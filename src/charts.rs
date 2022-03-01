use crate::config;
use anyhow::Result;
use serde::Serialize;
use std::fs::File;
use std::io::copy;
use std::io::Cursor;

#[derive(Debug, Serialize)]
pub struct Dataset {
    label: String,
    data: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Data {
    pub labels: Vec<String>,
    pub datasets: Vec<Dataset>,
}

impl Data {
    pub fn new() -> Self {
        Self {
            labels: vec![],
            datasets: vec![],
        }
    }
}

#[derive(Debug, Serialize)]
struct Chart {
    #[serde(rename(serialize = "type"))]
    chart_type: String,
    data: Data,
}

#[derive(Debug, Serialize)]
struct Parameters {
    #[serde(rename(serialize = "backgroundColor"))]
    background_color: String,
    width: i32,
    height: i32,
    format: String,
    chart: Chart,
}

struct ChartGenerator {
    api: &'static str,
    options: config::ChartOptions,
}

impl ChartGenerator {
    pub async fn gen_bar_chart<P: Serialize>(&self, payload: P, dest: String) -> Result<()> {
        let response = reqwest::Client::new()
            .post(self.api)
            .json(&payload)
            .send()
            .await?;

        let mut f = File::create(dest)?;
        let mut content = Cursor::new(response.bytes().await?);
        copy(&mut content, &mut f)?;
        Ok(())
    }
}
