use std::borrow::Borrow;
pub use crate::gitimpl::*;

use std::fs::File;
use std::sync::Arc;
use anyhow::Result;
use serde::Serialize;
use async_trait::async_trait;

static RECORD_COMMIT: &str = "COMMIT";
static RECORD_CHANGE: &str = "CHANGE";
static RECORD_TAG: &str = "TAG";

#[derive(Debug, Default, Serialize, Clone)]
struct Record {
    metric: String,
    hash: String,
    repo_name: String,
    timestamp: i64,
    timezone: String,
    author_name: String,
    author_email: String,
    author_domain: String,
    tag: String,
    ext: String,
    insertion: i64,
    deletion: i64,
    size: i64,
    files: i64,
}

#[async_trait]
pub trait RecordSerializer {
    async fn serialize(&self, path: String, database: String, repos: Vec<Repository>) -> Result<()>;
}

pub struct CsvSerializer {
    pub git: Arc<dyn GitImpl>,
}

#[async_trait]
impl<'a> RecordSerializer for CsvSerializer {
    async fn serialize(&self, path: String, database: String, repos: Vec<Repository>) -> Result<()> {
        // TODO(optimize): 判断文件是否存在
        let file = File::create(format!("{}/{}.csv", path, database))?;
        let mut wtr = csv::Writer::from_writer(file);

        for repo in repos.iter() {
            let commits = self.git.commits(repo).await;
            if let Ok(commits) = commits {
                for commit in commits {
                    let common_record = Record {
                        repo_name: repo.name.clone(),
                        timestamp: commit.timestamp,
                        hash: commit.hash,
                        timezone: commit.timezone,
                        author_name: commit.author.name,
                        author_email: commit.author.email,
                        author_domain: commit.author.domain,
                        ..Default::default()
                    };

                    let mut commit_record = common_record.clone();
                    commit_record.metric = RECORD_COMMIT.to_string();
                    wtr.serialize(commit_record)?;

                    for fc in commit.changes {
                        let mut r = common_record.clone();
                        r.metric = RECORD_CHANGE.to_string();
                        r.ext = fc.ext;
                        r.insertion = fc.insertion;
                        r.deletion = fc.deletion;
                        wtr.serialize(r)?;
                    }
                }
            }

            // let tag_stats = self.git.tags(repo.borrow()).await;
            // if let Ok(tag_stats) = tag_stats {
            //     for tag_stat in tag_stats {
            //         let record = Record {
            //             metric: RECORD_TAG.to_string(),
            //             repo_name: repo.name.clone(),
            //             timestamp: tag_stat.timestamp,
            //             timezone: tag_stat.timezone,
            //             tag: tag_stat.tag,
            //             ..Default::default()
            //         };
            //         wtr.serialize(record)?;
            //     }
            // }
        }

        // TODO(Optimize): 分批写入 避免内存过渡增长
        wtr.flush()?;
        Ok(())
    }
}
