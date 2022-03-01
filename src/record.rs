use crate::git_impl::GitImpl;
use crate::Config;
use anyhow::Result;
use async_trait::async_trait;
use serde::Serialize;
use std::path::Path;

static RECORD_COMMIT: &str = "COMMIT";
static RECORD_CHANGE: &str = "CHANGE";
static RECORD_TAG: &str = "TAG";

#[derive(Debug, Default, Serialize, Clone)]
struct Record {
    metric: String,
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
    fn extension(&self) -> String;
    async fn serialize(&self, config: &Config) -> Result<()>;
}

pub struct CsvSerializer {
    pub git: Box<dyn GitImpl>,
}

impl CsvSerializer {
    pub fn new(git: Box<dyn GitImpl>) -> Self {
        Self { git }
    }
}

#[async_trait]
impl<'a> RecordSerializer for CsvSerializer {
    fn extension(&self) -> String {
        String::from("csv")
    }

    async fn serialize(&self, config: &Config) -> Result<()> {
        // TODO(optimize): 判断文件是否存在 或者有多种文件创建模式可选？
        let uri = config.database.uri(self.extension());
        let mut wtr = csv::Writer::from_path(Path::new(uri.as_str())).unwrap();

        for repo in config.database.repositories.iter() {
            let commits = self.git.commits(repo, config.author_mappings.clone()).await;
            if let Ok(commits) = commits {
                for commit in commits {
                    let domain = commit.author.domain();
                    let common_record = Record {
                        repo_name: repo.name.clone(),
                        datetime: commit.datetime,
                        author_name: commit.author.name,
                        author_email: commit.author.email,
                        author_domain: domain,
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

            let tag_stats = self.git.tags(repo, config.author_mappings.clone()).await;
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
