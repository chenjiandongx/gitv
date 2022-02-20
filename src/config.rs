use std::fs::File;
use serde::Deserialize;
use anyhow::Result;
use crate::gitimpl::Repository;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AuthorMapping {
    pub source: String,
    pub destination: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Database {
    pub name: String,
    pub directory: String,
    pub mode: String,
    pub repositories: Vec<Repository>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    pub author_mappings: Vec<AuthorMapping>,
    pub databases: Vec<Database>,
}

pub fn load_config(c: &'static str) -> Result<Config> {
    let f = File::open(c)?;
    let config: Config = serde_yaml::from_reader(f)?;
    Ok(config)
}