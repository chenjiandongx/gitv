use crate::{AuthorMapping, CreateAction, Database, Gitter};
use anyhow::Result;
use async_trait::async_trait;
use chrono::prelude::*;
use serde::Serialize;
use std::{
    process::exit,
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

fn datetime_rfc339(datetime: &str) -> String {
    match DateTime::parse_from_rfc2822(datetime) {
        Ok(t) => t.to_rfc3339().to_string(),
        Err(_) => "".to_string(),
    }
}

/// 定义 Record 序列化接口
#[async_trait]
pub trait RecordSerializer {
    async fn serialize(&self, config: CreateAction) -> Result<()>;
}

static BUFFER_SIZE: usize = 1000;
static FLUSH_SIZE: usize = 500;

async fn persist_records<T: 'static + Gitter + Clone>(
    gitter: T,
    database: Database,
    author_mappings: Vec<AuthorMapping>,
) -> Result<()> {
    let repos = database.load()?;
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
                error!("Failed to execute git checkout command, error: {}", e);
                exit(1);
            }

            match gitter.commits(&repo, author_mappings.clone()).await {
                Ok(commits) => {
                    for commit in commits {
                        let domain = commit.author.domain();
                        let common_record = Record {
                            repo_name: repo.name.clone(),
                            branch: branch.clone().unwrap_or_default(),
                            datetime: datetime_rfc339(&commit.datetime),
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
                Err(e) => {
                    error!("Failed to analyzer repo commits, error: {}", e);
                    exit(1)
                }
            }

            match gitter.tags(&repo, author_mappings.clone()).await {
                Ok(tagstats) => {
                    for tagstat in tagstats {
                        let record = Record {
                            metric: RECORD_TAG.to_string(),
                            repo_name: repo.name.clone(),
                            datetime: datetime_rfc339(&tagstat.datetime),
                            tag: tagstat.tag,
                            ..Default::default()
                        };
                        if tx.send(record).await.is_err() {
                            return;
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to analyzer repo tags, error: {}", e);
                    exit(1)
                }
            }

            let mut lock = mutex.lock().unwrap();
            *lock += 1;
            let n = lock;
            info!(
                "[{}/{}] git analyze '{}' => elapsed {:#?}",
                n,
                total,
                repo.name,
                now.elapsed(),
            )
        });
        handles.push(handle)
    }

    let rev = tokio::spawn(async move {
        let mut wtr = csv::Writer::from_path(database.path).unwrap();
        let mut n: usize = 0;

        while let Some(record) = rx.recv().await {
            n += 1;
            if let Err(e) = wtr.serialize(record) {
                error!("Failed to serialize record, error: {}", e);
                exit(1)
            };

            if n >= FLUSH_SIZE {
                if let Err(e) = wtr.flush() {
                    error!("Failed to flush cached to disk, error: {}", e);
                    exit(1)
                }
            }
        }

        if let Err(e) = wtr.flush() {
            error!("Failed to flush cached to disk, error: {}", e);
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
    async fn serialize(&self, config: CreateAction) -> Result<()> {
        let mut handles = vec![];
        for database in config.databases {
            let gitter = self.gitter.clone();
            let database = database.clone();
            let author_mappings = config.author_mappings.clone();

            let handle = tokio::spawn(async move {
                let r =
                    persist_records(gitter, database, author_mappings.unwrap_or_default()).await;
                if let Err(e) = r {
                    error!("Failed to persist records, error: {}", e);
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
