use crate::{config::Repository, gitimp::*, AuthorMapping, CreateAction, Database, GitImpl};
use anyhow::Result;
use async_trait::async_trait;
use serde::Serialize;
use std::{
    fs::File,
    path::Path,
    sync::{Arc, Mutex},
};
use tokio::{
    sync::{self, mpsc::Sender},
    task::JoinHandle,
    time,
};

#[derive(Debug, Serialize, Clone)]
pub enum RecordType {
    Commit(RecordCommit),
    Change(RecordChange),
    Tag(RecordTag),
    Snapshot(RecordSnapshot),
    Active(RecordActive),
}

#[derive(Debug, Default, Serialize, Clone)]
pub struct RecordCommit {
    pub repo_name: String,
    pub hash: String,
    pub branch: String,
    pub datetime: String,
    pub author_name: String,
    pub author_email: String,
    pub author_domain: String,
}

impl RecordCommit {
    pub fn name() -> String {
        String::from("commit")
    }
}

#[derive(Debug, Default, Serialize, Clone)]
pub struct RecordChange {
    pub repo_name: String,
    pub hash: String,
    pub branch: String,
    pub datetime: String,
    pub author_name: String,
    pub author_email: String,
    pub author_domain: String,
    pub ext: String,
    pub insertion: usize,
    pub deletion: usize,
}

impl RecordChange {
    pub fn name() -> String {
        String::from("change")
    }
}

#[derive(Debug, Default, Serialize, Clone)]
pub struct RecordTag {
    pub repo_name: String,
    pub branch: String,
    pub datetime: String,
    pub tag: String,
}

impl RecordTag {
    pub fn name() -> String {
        String::from("tag")
    }
}

#[derive(Debug, Default, Serialize, Clone)]
pub struct RecordSnapshot {
    pub repo_name: String,
    pub branch: String,
    pub datetime: String,
    pub ext: String,
    pub code: usize,
    pub comments: usize,
    pub blanks: usize,
}

impl RecordSnapshot {
    pub fn name() -> String {
        String::from("snapshot")
    }
}

#[derive(Debug, Default, Serialize, Clone)]
pub struct RecordActive {
    pub repo_name: String,
    pub forks: usize,
    pub stars: usize,
}

impl RecordActive {
    pub fn name() -> String {
        String::from("active")
    }
}

/// 定义 Record 序列化接口
#[async_trait]
pub trait RecordSerializer {
    async fn serialize(config: CreateAction) -> Result<()>;
}

const BUFFER_SIZE: usize = 1000;

/// Csv 序列化实现
#[derive(Debug)]
pub struct CsvSerializer;

impl CsvSerializer {
    async fn send_commit_records(
        tx: &Sender<RecordType>,
        repo: &Repository,
        commits: Vec<Commit>,
    ) -> Result<()> {
        for commit in commits {
            let record = RecordCommit {
                repo_name: repo.name.clone(),
                hash: commit.hash.clone(),
                branch: repo.branch.clone().unwrap_or_default(),
                datetime: commit.datetime.to_rfc339(),
                author_name: commit.author.name.clone(),
                author_email: commit.author.email.clone(),
                author_domain: commit.author.domain(),
            };
            if tx.send(RecordType::Commit(record)).await.is_err() {
                return Ok(());
            };

            for fc in commit.changes {
                let record = RecordChange {
                    repo_name: repo.name.clone(),
                    hash: commit.hash.clone(),
                    branch: repo.branch.clone().unwrap_or_default(),
                    datetime: commit.datetime.to_rfc339(),
                    author_name: commit.author.name.clone(),
                    author_email: commit.author.email.clone(),
                    author_domain: commit.author.domain(),
                    ext: fc.ext,
                    insertion: fc.insertion,
                    deletion: fc.deletion,
                };
                if tx.send(RecordType::Change(record)).await.is_err() {
                    return Ok(());
                };
            }
        }
        Ok(())
    }

    async fn serialize_commits_sectional(
        tx: Sender<RecordType>,
        repo: &Repository,
        author_mappings: Vec<AuthorMapping>,
        hashs: Vec<String>,
    ) -> Result<()> {
        let concurrency = num_cpus::get();

        let mut txs = vec![];
        let mut rxs = vec![];
        for _ in 0..concurrency {
            let ch = sync::mpsc::channel::<String>(BUFFER_SIZE);
            txs.push(ch.0);
            rxs.push(ch.1);
        }

        let mut handles = vec![];
        for _ in 0..concurrency {
            let repo = repo.clone();
            let mappings = author_mappings.clone();
            let tx = tx.clone();
            let mut lines_rx = rxs.remove(0);

            let handle: JoinHandle<Result<(), anyhow::Error>> = tokio::spawn(async move {
                while let Some(hash) = lines_rx.recv().await {
                    let commits = GitImpl::commits(&repo, &mappings, &hash)?;
                    Self::send_commit_records(&tx, &repo, commits).await?;
                }
                Ok(())
            });
            handles.push(handle);
        }

        for (idx, hash) in hashs.into_iter().enumerate() {
            txs[idx % concurrency].send(hash).await?;
        }

        for tx in txs.into_iter().take(concurrency) {
            drop(tx);
        }

        for handle in handles {
            handle.await??;
        }
        Ok(())
    }

    async fn serialize_commits(
        tx: Sender<RecordType>,
        repo: &Repository,
        author_mappings: Vec<AuthorMapping>,
    ) -> Result<()> {
        let hashs = GitImpl::commits_hash(repo)?;
        if hashs.len() > 5000 {
            Self::serialize_commits_sectional(tx, repo, author_mappings, hashs).await?
        } else {
            let commits = GitImpl::commits(repo, &author_mappings, "")?;
            Self::send_commit_records(&tx, repo, commits).await?;
        }
        Ok(())
    }

    async fn serialize_tags(
        tx: Sender<RecordType>,
        repo: &Repository,
        author_mappings: Vec<AuthorMapping>,
    ) -> Result<()> {
        for tag in GitImpl::tags(repo, author_mappings)? {
            let record = RecordTag {
                repo_name: repo.name.clone(),
                datetime: tag.datetime.to_rfc339(),
                tag: tag.tag,
                branch: repo.branch.clone().unwrap_or_default(),
            };
            if tx.send(RecordType::Tag(record)).await.is_err() {
                return Ok(());
            }
        }
        Ok(())
    }

    async fn serialize_snapshot(tx: Sender<RecordType>, repo: &Repository) -> Result<()> {
        let snapshot = GitImpl::snapshot(repo)?;
        for stat in snapshot.stats {
            let record = RecordSnapshot {
                repo_name: repo.name.clone(),
                branch: repo.branch.clone().unwrap_or_default(),
                datetime: snapshot.datetime.to_rfc339(),
                ext: stat.ext,
                code: stat.code,
                comments: stat.comments,
                blanks: stat.blanks,
            };
            if tx.send(RecordType::Snapshot(record)).await.is_err() {
                return Ok(());
            }
        }
        Ok(())
    }

    async fn serialize_active(tx: Sender<RecordType>, repo: &Repository) -> Result<()> {
        let record = RecordActive {
            repo_name: repo.name.clone(),
            forks: repo.forks_count.unwrap_or_default(),
            stars: repo.stargazers_count.unwrap_or_default(),
        };
        if tx.send(RecordType::Active(record)).await.is_err() {
            return Ok(());
        }
        Ok(())
    }

    async fn analyze_repo(
        tx: Sender<RecordType>,
        repo: &Repository,
        author_mappings: Vec<AuthorMapping>,
    ) -> Result<()> {
        let mut handles: Vec<JoinHandle<Result<(), anyhow::Error>>> = vec![];
        for i in 0..4usize {
            let repo = repo.clone();
            let tx = tx.clone();
            let mappings = author_mappings.clone();
            match i {
                0 => {
                    handles.push(tokio::spawn(async move {
                        Self::serialize_commits(tx.clone(), &repo, mappings).await
                    }));
                }
                1 => {
                    handles.push(tokio::spawn(async move {
                        Self::serialize_snapshot(tx.clone(), &repo).await
                    }));
                }
                2 => {
                    handles.push(tokio::spawn(async move {
                        Self::serialize_tags(tx.clone(), &repo, mappings).await
                    }));
                }
                3 => {
                    handles.push(tokio::spawn(async move {
                        Self::serialize_active(tx.clone(), &repo).await
                    }));
                }
                _ => unreachable!(),
            }
        }
        for handle in handles {
            handle.await??;
        }
        Ok(())
    }

    async fn serialize_records(
        database: Database,
        author_mappings: Vec<AuthorMapping>,
        disable_pull: bool,
    ) -> Result<()> {
        let repos = database.load()?;
        let total = repos.len();

        let (tx, mut rx) = sync::mpsc::channel::<RecordType>(BUFFER_SIZE);
        let mutex = Arc::new(Mutex::new(0));
        let mut handles: Vec<JoinHandle<Result<(), anyhow::Error>>> = vec![];

        GitImpl::clone_or_pull(repos.clone(), disable_pull).await?;
        for repo in repos {
            let repo = repo.clone();
            let mappings = author_mappings.clone();
            let tx = tx.clone();
            let mutex = mutex.clone();

            let handle = tokio::spawn(async move {
                let now = time::Instant::now();
                GitImpl::checkout(&repo)?;
                Self::analyze_repo(tx.clone(), &repo, mappings).await?;

                let mut lock = mutex.lock().unwrap();
                *lock += 1;
                let n = lock;
                println!(
                    "[{}/{}] git analyze '{}' => elapsed {:#?}",
                    n,
                    total,
                    repo.name,
                    now.elapsed(),
                );
                Ok(())
            });
            handles.push(handle)
        }

        let rev: JoinHandle<Result<(), anyhow::Error>> = tokio::spawn(async move {
            let dir = &database.dir;
            let mut commit_wtr = CsvWriter::try_new(dir, RecordCommit::name())?;
            let mut change_wtr = CsvWriter::try_new(dir, RecordChange::name())?;
            let mut tag_wtr = CsvWriter::try_new(dir, RecordTag::name())?;
            let mut snapshot_wtr = CsvWriter::try_new(dir, RecordSnapshot::name())?;
            let mut active_wtr = CsvWriter::try_new(dir, RecordActive::name())?;

            while let Some(record) = rx.recv().await {
                match record {
                    RecordType::Commit(commit) => commit_wtr.write(commit)?,
                    RecordType::Change(change) => change_wtr.write(change)?,
                    RecordType::Tag(tag) => tag_wtr.write(tag)?,
                    RecordType::Snapshot(snapshot) => snapshot_wtr.write(snapshot)?,
                    RecordType::Active(active) => active_wtr.write(active)?,
                }
            }

            commit_wtr.flush()?;
            change_wtr.flush()?;
            tag_wtr.flush()?;
            snapshot_wtr.flush()?;
            active_wtr.flush()?;
            Ok(())
        });

        for handle in handles {
            handle.await??;
        }
        drop(tx);

        rev.await??;
        Ok(())
    }
}

struct CsvWriter {
    wtr: csv::Writer<File>,
    size: usize,
    curr: usize,
}

const FLUSH_SIZE: usize = 500;

impl CsvWriter {
    fn try_new(dir: &str, name: String) -> Result<CsvWriter> {
        Ok(Self {
            wtr: csv::Writer::from_path(Path::new(dir).join(format!("{}.csv", name)))?,
            size: FLUSH_SIZE,
            curr: 0,
        })
    }

    fn write<T: Serialize>(&mut self, record: T) -> Result<()> {
        self.curr += 1;
        self.wtr.serialize(record)?;
        if self.curr >= self.size {
            self.flush()?;
            self.curr = 0;
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        self.wtr.flush()?;
        Ok(())
    }
}

#[async_trait]
impl RecordSerializer for CsvSerializer {
    async fn serialize(config: CreateAction) -> Result<()> {
        let mut handles = vec![];
        let disable_pull = config.disable_pull.unwrap_or(false);
        for database in config.databases {
            let database = database.clone();
            let author_mappings = config.author_mappings.clone().unwrap_or_default();

            let handle = tokio::spawn(async move {
                Self::serialize_records(database, author_mappings, disable_pull).await
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await??;
        }
        Ok(())
    }
}
