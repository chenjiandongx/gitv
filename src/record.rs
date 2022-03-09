use crate::{AuthorMapping, Database, Gitter, InitAction};
use anyhow::Result;
use async_trait::async_trait;
use serde::Serialize;
use std::process::exit;
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use tokio::{sync, time};
use tracing::{error, info};

pub static RECORD_COMMIT: &str = "COMMIT";
pub static RECORD_CHANGE: &str = "CHANGE";
pub static RECORD_TAG: &str = "TAG";

#[derive(Debug, Default, Serialize, Clone)]
pub struct Record {
    pub metric: String,
    pub repo_name: String,
    pub branch: String,
    pub datetime: String,
    pub author_name: String,
    pub author_email: String,
    pub author_domain: String,
    pub tag: String,
    pub ext: String,
    pub insertion: i64,
    pub deletion: i64,
    pub size: i64,
    pub files: i64,
}

/// 定义 Record 序列化接口
#[async_trait]
pub trait RecordSerializer {
    fn extension(&self) -> String;
    async fn serialize(&self, config: InitAction) -> Result<()>;
}

static BUFFER_SIZE: usize = 1000;
static FLUSH_SIZE: usize = 500;

async fn persist_records<T: 'static + Gitter + Clone>(
    gitter: T,
    ext: String,
    database: Database,
    author_mappings: Vec<AuthorMapping>,
) -> Result<()> {
    let repos = database.load().unwrap();
    let total = repos.len();

    let (tx, mut rx) = sync::mpsc::channel::<Record>(BUFFER_SIZE);
    let mutex = Arc::new(Mutex::new(0));
    let mut handles = vec![];

    gitter.clone_or_pull(repos.clone()).await?;
    for repo in repos {
        let repo = repo.clone();
        let author_mappings = author_mappings.clone();
        let gitter = gitter.clone();
        let tx = tx.clone();
        let mutex = mutex.clone();

        let handle = tokio::spawn(async move {
            let now = time::Instant::now();
            let branch = repo.branch.clone();
            if let Err(e) = gitter.checkout(&repo).await {
                error!("failed to execute git checkout command, err: {}", e);
                exit(1);
            }

            let commits = gitter.commits(&repo, author_mappings.clone()).await;
            if let Ok(commits) = commits {
                for commit in commits {
                    let domain = commit.author.domain();
                    let common_record = Record {
                        repo_name: repo.name.clone(),
                        branch: branch.clone().unwrap_or_default(),
                        datetime: commit.datetime,
                        author_name: commit.author.name,
                        author_email: commit.author.email,
                        author_domain: domain,
                        ..Default::default()
                    };

                    let mut commit_record = common_record.clone();
                    commit_record.metric = RECORD_COMMIT.to_string();
                    if tx.send(commit_record).await.is_err() {
                        return;
                    }

                    for fc in commit.changes {
                        let mut record = common_record.clone();
                        record.metric = RECORD_CHANGE.to_string();
                        record.ext = fc.ext;
                        record.insertion = fc.insertion;
                        record.deletion = fc.deletion;
                        if tx.send(record).await.is_err() {
                            return;
                        }
                    }
                }
            }

            let tag_stats = gitter.tags(&repo, author_mappings.clone()).await;
            if let Ok(tag_stats) = tag_stats {
                for tag_stat in tag_stats {
                    let record = Record {
                        metric: RECORD_TAG.to_string(),
                        repo_name: repo.name.clone(),
                        datetime: tag_stat.datetime,
                        tag: tag_stat.tag,
                        ..Default::default()
                    };
                    if tx.send(record).await.is_err() {
                        return;
                    }
                }
            }

            let mut lock = mutex.lock().unwrap();
            *lock += 1;
            let n = lock;
            info!(
                "[{}/{}] git analyze: elapsed {:#?} => {}",
                n,
                total,
                now.elapsed(),
                repo.remote
            )
        });
        handles.push(handle)
    }

    let rev = tokio::spawn(async move {
        let mut wtr = csv::Writer::from_path(database.location(ext)).unwrap();
        let mut n: usize = 0;

        while let Some(record) = rx.recv().await {
            n += 1;
            if let Err(e) = wtr.serialize(record) {
                error!("failed to serialize record, err: {}", e);
                exit(1)
            };

            if n >= FLUSH_SIZE {
                if let Err(e) = wtr.flush() {
                    error!("failed to flush cached to disk, err: {}", e);
                    exit(1)
                }
            }
        }

        if let Err(e) = wtr.flush() {
            error!("failed to flush cached to disk, err: {}", e);
            exit(1)
        }
    });

    for handle in handles {
        handle.await?;
    }
    drop(tx);

    rev.await?;
    Ok(())
}

/// Csv 序列化实现
#[derive(Debug)]
pub struct CsvSerializer<T> {
    gitter: T,
}

impl<T> CsvSerializer<T> {
    pub fn new(gitter: T) -> Self {
        Self { gitter }
    }
}

#[async_trait]
impl<T: 'static + Gitter + Clone> RecordSerializer for CsvSerializer<T> {
    fn extension(&self) -> String {
        String::from("csv")
    }

    async fn serialize(&self, config: InitAction) -> Result<()> {
        let mut handles = vec![];
        for database in config.databases {
            let gitter = self.gitter.clone();
            let extension = self.extension();
            let database = database.clone();
            let author_mappings = config.author_mappings.clone();

            let handle = tokio::spawn(async move {
                let r = persist_records(
                    gitter,
                    extension,
                    database,
                    author_mappings.unwrap_or_default(),
                )
                .await;
                if let Err(e) = r {
                    error!("failed to persist records, err: {}", e);
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
