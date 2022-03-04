use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::Path;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct InitAction {
    pub author_mappings: Option<Vec<AuthorMapping>>,
    pub databases: Vec<Database>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct AuthorMapping {
    pub source: Author,
    pub destination: Author,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Repository {
    pub name: String,
    pub branch: Option<String>,
    pub remote: String,
    pub path: String,
}

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

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Database {
    pub table_name: String,
    pub path: String,
    pub source: String,
    pub files: Option<Vec<String>>,
    pub repos: Option<Vec<Repository>>,
}

impl Database {
    pub fn location(&self, ext: String) -> String {
        let p = Path::new(self.path.as_str()).join(format!(
            "{}.{}",
            self.table_name.clone(),
            ext.as_str()
        ));
        p.to_str().unwrap().to_string()
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SyncAction {
    pub github: Option<Vec<Github>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Github {
    pub path: String,
    pub token: String,
    pub exclude_org: Option<String>,
    pub visibility: Option<String>,
    pub affiliation: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RenderAction {
    pub options: ChartOptions,
    pub queries: Vec<Query>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ChartOptions {
    #[serde(rename(deserialize = "backgroundColor"))]
    pub background_color: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub format: Option<String>,
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
    pub options: Option<ChartOptions>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    pub init: InitAction,
    pub sync: SyncAction,
    pub render: RenderAction,
}

pub fn load_config(c: &'static str) -> Result<Config> {
    let f = File::open(c)?;
    let config: Config = serde_yaml::from_reader(f)?;
    Ok(config)
}
