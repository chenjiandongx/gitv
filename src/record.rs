use crate::gitimpl::{GitImpl, Repository};
use anyhow::Result;
use async_trait::async_trait;
use serde::Serialize;
use std::fs::File;

static RECORD_COMMIT: &str = "COMMIT";
static RECORD_CHANGE: &str = "CHANGE";
static RECORD_TAG: &str = "TAG";

#[derive(Debug, Default, Serialize, Clone)]
struct Record {
    metric: String,
    hash: String,
    repo_name: String,
    datetime: String,
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
    async fn serialize(&self, path: String, database: String, repos: &[Repository]) -> Result<()>;
}

pub struct CsvSerializer {
    pub git: Box<dyn GitImpl>,
}

#[async_trait]
impl<'a> RecordSerializer for CsvSerializer {
    async fn serialize(&self, path: String, database: String, repos: &[Repository]) -> Result<()> {
        // TODO(optimize): 判断文件是否存在 或者有多种文件创建模式可选？
        let file = File::create(format!("{}/{}.csv", path, database))?;
        let mut wtr = csv::Writer::from_writer(file);

        for repo in repos.iter() {
            let commits = self.git.commits(repo).await;
            if let Ok(commits) = commits {
                for commit in commits {
                    let common_record = Record {
                        repo_name: repo.name.clone(),
                        datetime: commit.datetime,
                        hash: commit.hash,
                        author_name: commit.author.name,
                        author_email: commit.author.email,
                        author_domain: commit.author.domain,
                        ..Default::default()
                    };

                    let mut commit_record = common_record.clone();
                    commit_record.metric = RECORD_COMMIT.to_string();
                    wtr.serialize(commit_record)?;

                    for fc in commit.changes {
                        let mut record = common_record.clone();
                        record.metric = RECORD_CHANGE.to_string();
                        record.ext = fc.ext;
                        record.insertion = fc.insertion;
                        record.deletion = fc.deletion;
                        wtr.serialize(record)?;
                    }
                }
            }
            wtr.flush()?;

            let tag_stats = self.git.tags(repo).await;
            if let Ok(tag_stats) = tag_stats {
                for tag_stat in tag_stats {
                    let record = Record {
                        metric: RECORD_TAG.to_string(),
                        repo_name: repo.name.clone(),
                        datetime: tag_stat.datetime,
                        tag: tag_stat.tag,
                        ..Default::default()
                    };
                    wtr.serialize(record)?;
                }
            }
            wtr.flush()?;
        }

        Ok(())
    }
}
