use crate::config::AuthorMapping;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct Author {
    pub name: String,
    pub email: String,
}

impl Author {
    pub fn domain(&self) -> String {
        let email = self.email.clone();
        let fields = email.splitn(2, '@').collect::<Vec<&str>>();
        fields.last().unwrap().to_string()
    }
}

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Repository {
    pub name: String,
    pub branches: Option<Vec<String>>,
    pub remote: String,
    pub path: String,
}

#[async_trait]
pub trait GitImpl: Send + Sync {
    async fn clone(&self, repo: &Repository) -> Result<()>;
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
    async fn current_branch(&self, repo: &Repository) -> Result<String>;
}

#[async_trait]
pub trait RepoSourcer {
    async fn repositories(&self) -> Result<Vec<Repository>>;
}
