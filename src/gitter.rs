use crate::{config::AuthorMapping, Author, Repository};
use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug, Clone, Default)]
pub struct FileExtChange {
    pub ext: String,
    pub insertion: i64,
    pub deletion: i64,
}

#[derive(Debug, Clone, Default)]
pub struct Commit {
    pub repo: String,
    pub hash: String,
    pub author: Author,
    pub datetime: String,
    pub change_files: i64,
    pub changes: Vec<FileExtChange>,
}

#[derive(Debug, Clone, Default)]
pub struct FileExtStat {
    pub ext: String,
    pub size: i64,
    pub files: i64,
}

#[derive(Debug, Clone, Default)]
pub struct TagStats {
    pub hash: String,
    pub tag: String,
    pub datetime: String,
    pub stats: Vec<FileExtStat>,
}

#[async_trait]
pub trait Gitter: Send + Sync {
    async fn clone_or_pull(&self, repos: Vec<Repository>) -> Result<()>;
    async fn checkout(&self, repo: &Repository) -> Result<()>;
    async fn commits(
        &self,
        repo: &Repository,
        author_mappings: Vec<AuthorMapping>,
    ) -> Result<Vec<Commit>>;
    async fn tags(
        &self,
        repo: &Repository,
        author_mappings: Vec<AuthorMapping>,
    ) -> Result<Vec<TagStats>>;
}
