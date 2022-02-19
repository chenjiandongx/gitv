pub use crate::gitimpl::*;
use std::path::Path;

use anyhow::Result;
use serde::Serialize;

static RECORD_METRIC: &str = "metric";
static RECORD_CHANGE: &str = "change";
static RECORD_TAG: &str = "tag";

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

pub trait RecordSerializer {
    fn serialize(&self, path: String, database: String, repos: Vec<&Repository>) -> Result<()>;
}

pub struct CsvSerializer {
    pub git: Box<dyn GitImpl>,
}

impl<'a> RecordSerializer for CsvSerializer {
    fn serialize(&self, path: String, database: String, repos: Vec<&Repository>) -> Result<()> {
        let name = repo.name.clone();
        let name = format!("{}.csv", name);
        let p = Path::new(name.as_str());
        csv::Writer::from_writer();
        let mut wtr = csv::Writer::from_path(p).unwrap();

        let commander = GitBinary::new(repo);
        let commits = commander.commits();
        if let Ok(commits) = commits {
            for commit in commits {
                let common_record = Record {
                    repo_name: name.clone(),
                    timestamp: commit.timestamp,
                    hash: commit.hash,
                    timezone: commit.timezone,
                    author_name: commit.author.name,
                    author_email: commit.author.email,
                    author_domain: commit.author.domain,
                    ..Default::default()
                };

                let mut commit_record = common_record.clone();
                commit_record.metric = RECORD_METRIC.to_string();
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

        let tag_stats = commander.tags();
        if let Ok(tag_stats) = tag_stats {
            for tag_stat in tag_stats {
                let record = Record {
                    metric: RECORD_TAG.to_string(),
                    repo_name: name.clone(),
                    timestamp: tag_stat.timestamp,
                    timezone: tag_stat.timezone,
                    tag: tag_stat.tag,
                    ..Default::default()
                };
                wtr.serialize(record)?;
            }
        }
        wtr.flush()?;

        Ok(())
    }
}
