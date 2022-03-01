use crate::git_impl::{Author, Repository};
use anyhow::Result;
use serde::Deserialize;
use std::fs::File;
use std::path::Path;

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct AuthorMapping {
    pub source: Author,
    pub destination: Author,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Database {
    pub name: String,
    pub path: String,
    pub repositories: Vec<Repository>,
}

impl Database {
    pub fn uri(&self, ext: String) -> String {
        let p =
            Path::new(self.path.as_str()).join(format!("{}.{}", self.name.clone(), ext.as_str()));
        p.to_str().unwrap().to_string()
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Render {
    pub options: ChartOptions,
    pub queries: Vec<Query>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ChartOptions {
    #[serde(rename(serialize = "backgroundColor"))]
    background_color: String,
    width: i32,
    height: i32,
    format: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Query {
    pub sql: String,
    pub charts: Option<Vec<Chart>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Chart {
    pub chart_type: String,
    pub title: String,
    pub labels: String,
    pub datasets: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Github {
    pub path: String,
    pub token: String,
    pub exclude_org: String,
    pub visibility: String,
    pub affiliation: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    pub author_mappings: Vec<AuthorMapping>,
    pub database: Database,
    pub render: Render,
    pub github: Github,
}

pub fn load_config(c: &'static str) -> Result<Config> {
    let f = File::open(c)?;
    let config: Config = serde_yaml::from_reader(f)?;
    Ok(config)
}
