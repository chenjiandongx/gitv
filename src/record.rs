use crate::{config::Repository, AuthorMapping, CreateAction, Database, GitImpl};
use anyhow::Result;
use async_trait::async_trait;
use chrono::DateTime;
use serde::Serialize;
use std::{
    fs::File,
    path::Path,
    process::exit,
    sync::{Arc, Mutex},
};
use tokio::{
    sync::{self, mpsc::Sender},
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

fn datetime_rfc339(datetime: &str) -> String {
    match DateTime::parse_from_rfc2822(datetime) {
        Ok(t) => t.to_rfc3339().to_string(),
        Err(_) => "".to_string(),
    }
}

/// 定义 Record 序列化接口
#[async_trait]
pub trait RecordSerializer {
    async fn serialize(config: CreateAction) -> Result<()>;
}

/// Csv 序列化实现
#[derive(Debug)]
pub struct CsvSerializer;

impl CsvSerializer {
    async fn serialize_commits(
        tx: Sender<RecordType>,
        repo: &Repository,
        author_mappings: Vec<AuthorMapping>,
    ) -> Result<()> {
        for commit in GitImpl::commits(&repo, author_mappings).await? {
            let record = RecordCommit {
                repo_name: repo.name.clone(),
                branch: repo.branch.clone().unwrap_or_default(),
                datetime: datetime_rfc339(&commit.datetime),
                author_name: commit.author.name.clone(),
                author_email: commit.author.email.clone(),
                author_domain: commit.author.domain(),
            };
            if tx.send(RecordType::Commit(record)).await.is_err() {
                return Ok(());
            }

            for fc in commit.changes {
                let record = RecordChange {
                    repo_name: repo.name.clone(),
                    branch: repo.branch.clone().unwrap_or_default(),
                    datetime: datetime_rfc339(&commit.datetime),
                    author_name: commit.author.name.clone(),
                    author_email: commit.author.email.clone(),
                    author_domain: commit.author.domain(),
                    ext: fc.ext,
                    insertion: fc.insertion,
                    deletion: fc.deletion,
                };
                if tx.send(RecordType::Change(record)).await.is_err() {
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    async fn serialize_tags(
        tx: Sender<RecordType>,
        repo: &Repository,
        author_mappings: Vec<AuthorMapping>,
    ) -> Result<()> {
        for tag in GitImpl::tags(repo, author_mappings).await? {
            let record = RecordTag {
                repo_name: repo.name.clone(),
                datetime: datetime_rfc339(&tag.datetime),
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
        let snapshot = GitImpl::snapshot(repo).await?;
        for stat in snapshot.stats {
            let record = RecordSnapshot {
                repo_name: repo.name.clone(),
                branch: repo.branch.clone().unwrap_or_default(),
                datetime: datetime_rfc339(&snapshot.datetime),
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

    async fn serialize_records(
        database: Database,
        author_mappings: Vec<AuthorMapping>,
    ) -> Result<()> {
        const BUFFER_SIZE: usize = 1000;
        let repos = database.load()?;
        let total = repos.len();

        let (tx, mut rx) = sync::mpsc::channel::<RecordType>(BUFFER_SIZE);
        let mutex = Arc::new(Mutex::new(0));
        let mut handles = vec![];

        GitImpl::clone_or_pull(repos.clone()).await?;
        for repo in repos {
            let repo = repo.clone();
            let author_mappings = author_mappings.clone();
            let tx = tx.clone();
            let mutex = mutex.clone();

            let handle = tokio::spawn(async move {
                let now = time::Instant::now();
                if let Err(e) = GitImpl::checkout(&repo).await {
                    println!("Failed to execute git checkout command, error: {}", e);
                    exit(1);
                }

                if let Err(e) =
                    Self::serialize_commits(tx.clone(), &repo, author_mappings.clone()).await
                {
                    println!("Failed to analyze repo commits, error: {}", e);
                    exit(1)
                }

                if let Err(e) =
                    Self::serialize_tags(tx.clone(), &repo, author_mappings.clone()).await
                {
                    println!("Failed to analyze repo tags, error: {}", e);
                    exit(1)
                }

                if let Err(e) = Self::serialize_snapshot(tx.clone(), &repo).await {
                    println!("Failed to analyze repo snapshot, error: {}", e);
                    exit(1)
                }

                if let Err(e) = Self::serialize_active(tx.clone(), &repo).await {
                    println!("Failed to analyze repo active, error: {}", e);
                    exit(1)
                }

                let mut lock = mutex.lock().unwrap();
                *lock += 1;
                let n = lock;
                println!(
                    "[{}/{}] git analyze '{}' => elapsed {:#?}",
                    n,
                    total,
                    repo.name,
                    now.elapsed(),
                )
            });
            handles.push(handle)
        }

        const FLUSH_SIZE: usize = 500;

        let rev = tokio::spawn(async move {
            let dir = &database.dir;
            let mut commit_wtr = CsvWriter::must_new(dir, RecordCommit::name(), FLUSH_SIZE);
            let mut change_wtr = CsvWriter::must_new(dir, RecordChange::name(), FLUSH_SIZE);
            let mut tag_wtr = CsvWriter::must_new(dir, RecordTag::name(), FLUSH_SIZE);
            let mut snapshot_wtr = CsvWriter::must_new(dir, RecordSnapshot::name(), FLUSH_SIZE);
            let mut active_wtr = CsvWriter::must_new(dir, RecordActive::name(), FLUSH_SIZE);

            while let Some(record) = rx.recv().await {
                match record {
                    RecordType::Commit(commit) => commit_wtr.must_write(commit),
                    RecordType::Change(change) => change_wtr.must_write(change),
                    RecordType::Tag(tag) => tag_wtr.must_write(tag),
                    RecordType::Snapshot(snapshot) => snapshot_wtr.must_write(snapshot),
                    RecordType::Active(active) => active_wtr.must_write(active),
                }
            }

            commit_wtr.must_flush();
            change_wtr.must_flush();
            tag_wtr.must_flush();
            snapshot_wtr.must_flush();
            active_wtr.must_flush();
        });

        for handle in handles {
            handle.await?;
        }
        drop(tx);

        rev.await?;
        Ok(())
    }
}

struct CsvWriter {
    wtr: csv::Writer<File>,
    size: usize,
    curr: usize,
}

impl CsvWriter {
    fn must_new(dir: &str, name: String, size: usize) -> Self {
        let wtr = match csv::Writer::from_path(Path::new(dir).join(format!("{}.csv", name))) {
            Ok(wtr) => wtr,
            Err(e) => {
                println!("Failed to create {} writer, error: {}", name, e);
                exit(1)
            }
        };
        Self { wtr, size, curr: 0 }
    }

    fn must_write<T: Serialize>(&mut self, record: T) {
        self.curr += 1;
        if let Err(e) = self.wtr.serialize(record) {
            println!("Failed to serialize record, error: {}", e);
            exit(1)
        }
        if self.curr >= self.size {
            self.must_flush();
            self.curr = 0;
        }
    }

    fn must_flush(&mut self) {
        if let Err(e) = self.wtr.flush() {
            println!("Failed to flush record, error: {}", e);
            exit(1)
        }
    }
}

#[async_trait]
impl RecordSerializer for CsvSerializer {
    async fn serialize(config: CreateAction) -> Result<()> {
        let mut handles = vec![];
        for database in config.databases {
            let database = database.clone();
            let author_mappings = config.author_mappings.clone().unwrap_or_default();

            let handle = tokio::spawn(async move {
                if let Err(e) = Self::serialize_records(database, author_mappings).await {
                    println!("Failed to persist records, error: {}", e);
                    exit(1)
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await?;
        }
        Ok(())
    }
}
