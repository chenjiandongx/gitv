use crate::InitAction;
use anyhow::Result;
use async_trait::async_trait;
use serde::Serialize;

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

#[async_trait]
pub trait RecordSerializer {
    fn extension(&self) -> String;
    async fn serialize(&self, config: InitAction) -> Result<()>;
}
