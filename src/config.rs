use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::fs::File;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
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
    pub remote: Option<String>,
    pub path: String,
    pub forks_count: Option<usize>,
    pub stargazers_count: Option<usize>,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq, Hash)]
pub struct Author {
    pub name: String,
    pub email: String,
}

impl Author {
    pub fn domain(&self) -> String {
        let email = self.email.clone();
        let fields = email.splitn(2, '@').collect::<Vec<&str>>();
        fields.last().unwrap_or(&"").to_string()
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Database {
    pub dir: String,
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

#[derive(Debug, Clone, Default, Deserialize)]
pub struct FetchAction {
    pub github: Option<Vec<Github>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Github {
    pub clone_dir: String,
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
#[serde(rename_all = "camelCase")]
pub struct Execution {
    pub db_name: String,
    pub dir: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RenderAction {
    pub executions: Vec<Execution>,
    pub display: Display,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Display {
    pub destination: String,
    pub render_mode: String,
    pub dependency: Option<Dependency>,
    pub queries: Vec<Query>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Dependency {
    chartjs: String,
    stacked100: String,
    datalabels: String,
}

static REGISTER_STACKED100: &str = "Chart.register(ChartjsPluginStacked100)";
static REGISTER_DATALABELS: &str = "Chart.register(ChartDataLabels)";

impl Dependency {
    pub fn list(&self) -> Vec<String> {
        let mut data = vec![];
        if self.chartjs.is_empty() {
            data.push(Self::default().chartjs);
        } else {
            data.push(self.chartjs.clone())
        }
        if !self.stacked100.is_empty() {
            data.push(self.stacked100.clone())
        }
        if !self.datalabels.is_empty() {
            data.push(self.datalabels.clone())
        }
        data
    }

    pub fn register(&self) -> Vec<&'static str> {
        let mut data = vec![];
        if !self.stacked100.is_empty() {
            data.push(REGISTER_STACKED100);
        }
        if !self.datalabels.is_empty() {
            data.push(REGISTER_DATALABELS);
        }
        data
    }
}

static DEPENDENCY_CHARTJS: &str = "https://cdn.bootcdn.net/ajax/libs/Chart.js/3.7.1/chart.min.js";
static DEPENDENCY_STACKED100: &str = "https://cdn.jsdelivr.net/npm/chartjs-plugin-stacked100@1.0";
static DEPENDENCY_DATALABELS: &str = "https://cdn.jsdelivr.net/npm/chartjs-plugin-datalabels@2.0.0";

impl Default for Dependency {
    fn default() -> Self {
        Self {
            chartjs: String::from(DEPENDENCY_CHARTJS),
            stacked100: String::from(DEPENDENCY_STACKED100),
            datalabels: String::from(DEPENDENCY_DATALABELS),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Query {
    pub statements: Vec<String>,
    pub chart: Option<ChartConfig>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ChartConfig {
    #[serde(rename(deserialize = "type"))]
    pub chart_type: String,
    pub width: String,
    pub height: String,
    pub name: String,
    pub options: Option<Value>,
    pub data: Value,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Config {
    pub create: Option<CreateAction>,
    pub fetch: Option<FetchAction>,
    pub shell: Option<ShellAction>,
    pub render: Option<RenderAction>,
}

pub fn load_config(c: &str) -> Result<Config> {
    let f = File::open(c)?;
    let config: Config = serde_yaml::from_reader(f)?;
    Ok(config)
}
