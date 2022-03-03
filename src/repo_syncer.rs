use crate::Repository;
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait RepoSyncer {
    async fn repositories(&self) -> Result<Vec<Repository>>;
}
