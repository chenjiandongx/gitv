use crate::{record::*, AuthorMapping, Database, Gitter, InitAction, RecordSerializer};
use anyhow::Result;
use async_trait::async_trait;
use std::{
    path::Path,
    sync::{Arc, Mutex},
    time,
};
use tokio::sync;
use tracing::info;

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
            gitter.checkout(&repo).await.unwrap();

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
        let uri = database.location(ext);
        let mut wtr = csv::Writer::from_path(Path::new(uri.as_str())).unwrap();
        let mut n: usize = 0;
        while let Some(record) = rx.recv().await {
            n += 1;
            wtr.serialize(record).unwrap();
            if n >= FLUSH_SIZE {
                wtr.flush().unwrap();
            }
        }
        wtr.flush().unwrap();
    });

    for handle in handles {
        handle.await.unwrap();
    }
    drop(tx);

    rev.await.unwrap();
    Ok(())
}

#[derive(Debug)]
pub struct CsvSerializer<T: Gitter + Clone> {
    gitter: T,
}

impl<T: Gitter + Clone> CsvSerializer<T> {
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
                persist_records(
                    gitter,
                    extension,
                    database,
                    author_mappings.unwrap_or_default(),
                )
                .await
                .unwrap();
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }
        Ok(())
    }
}
