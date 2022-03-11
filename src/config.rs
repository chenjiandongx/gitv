use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::{
    fs::File,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CreateAction {
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
    pub base_dir: String,
    pub source: String,
    pub files: Option<Vec<String>>,
    pub repos: Option<Vec<Repository>>,
}

impl Database {
    pub fn load(&self) -> Result<Vec<Repository>> {
        let mut repos = vec![];
        if self.repos.is_some() {
            repos.extend(self.repos.clone().unwrap());
        }

        if self.files.is_some() {
            for file in self.files.clone().unwrap() {
                let f = File::open(&file)?;
                let r: Vec<Repository> = serde_yaml::from_reader(f)?;
                repos.extend(r);
            }
        }
        Ok(repos)
    }
}

impl Database {
    pub fn location(&self, ext: String) -> PathBuf {
        Path::new(self.base_dir.as_str()).join(format!(
            "{}.{}",
            self.table_name.clone(),
            ext.as_str()
        ))
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct FetchAction {
    pub github: Option<Vec<Github>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Github {
    pub base_dir: String,
    pub destination: String,
    pub token: String,
    pub exclude_orgs: Option<Vec<String>>,
    pub exclude_repos: Option<Vec<String>>,
    pub visibility: Option<String>,
    pub affiliation: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ShellAction {
    pub executions: Vec<Execution>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Execution {
    pub table_name: String,
    pub file: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RenderAction {
    pub executions: Vec<Execution>,
    pub display: Display,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Display {
    pub destination: String,
    pub render_mode: String,
    pub render_api: String,
    pub render_config: ChartConfig,
    pub queries: Vec<Query>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChartConfig {
    #[serde(rename(serialize = "backgroundColor"))]
    pub background_color: String,
    pub width: i32,
    pub height: i32,
    pub format: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Query {
    pub statements: Vec<String>,
    pub chart: Option<Chart>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Chart {
    #[serde(rename(deserialize = "type"))]
    pub chart_type: String,
    pub name: String,
    pub options: Option<Value>,
    pub data: Value,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    pub create: CreateAction,
    pub fetch: FetchAction,
    pub shell: ShellAction,
    pub render: RenderAction,
}

pub fn load_config(c: &str) -> Result<Config> {
    let f = File::open(c)?;
    let config: Config = serde_yaml::from_reader(f)?;
    Ok(config)
}
