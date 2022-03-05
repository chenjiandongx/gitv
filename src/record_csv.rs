use crate::record::*;
use crate::{AuthorMapping, Database, Gitter, InitAction, RecordSerializer};
use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

async fn persist_records<T: Gitter>(
    gitter: T,
    ext: String,
    database: Database,
    author_mappings: Vec<AuthorMapping>,
) -> Result<()> {
    // TODO(optimize): 判断文件是否存在 或者有多种文件创建模式可选？
    let uri = database.location(ext);
    let mut wtr = csv::Writer::from_path(Path::new(uri.as_str()))?;

    let repos = database.load()?;
    gitter.clone_or_pull(repos.clone()).await?;

    for repo in &database.load().unwrap() {
        let branch = gitter.current_branch(repo).await.unwrap_or_default();
        let commits = gitter.commits(repo, author_mappings.clone()).await;
        if let Ok(commits) = commits {
            for commit in commits {
                let domain = commit.author.domain();
                let common_record = Record {
                    repo_name: repo.name.clone(),
                    branch: branch.clone(),
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

        let tag_stats = gitter.tags(repo, author_mappings.clone()).await;
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
        futures::future::join_all(handles).await;
        Ok(())
    }
}
