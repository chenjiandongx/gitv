use crate::{config, RenderConfig};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use datafusion::{
    arrow::{array, datatypes::DataType},
    prelude::ExecutionContext,
};
use serde::Serialize;
use serde_yaml::{Number, Value};
use std::{
    collections::HashMap,
    fmt::Debug,
    fs::File,
    io::{copy, Cursor, Write},
    path::{Path, PathBuf},
};
use tera::{Context, Tera};
use tokio::time;
use tracing::info;

#[derive(Debug, Serialize)]
pub struct ColumnMap {
    store: HashMap<String, Vec<Value>>,
}

impl ColumnMap {
    fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }

    fn get(&self, k: &str) -> Option<Value> {
        let mut values = vec![];
        for v in self.store.get(k)? {
            values.push(v.clone())
        }
        Some(Value::Sequence(values))
    }
}

struct Engine {
    ctx: ExecutionContext,
}

impl Engine {
    fn new(ctx: ExecutionContext) -> Self {
        Self { ctx }
    }

    async fn select(&mut self, sql: &str) -> Result<ColumnMap> {
        let mut cm = ColumnMap::new();
        let ctx = &mut self.ctx;
        let df = ctx.sql(sql).await?;

        for val in df.collect().await? {
            if val.num_rows() == 0 {
                continue;
            }
            if val.columns().is_empty() {
                continue;
            }

            let schema = val.schema();
            let fields = schema.fields();
            for (idx, column) in val.columns().iter().enumerate() {
                let data = column.as_any();
                let field = &fields[idx];
                let name = field.name().to_string();
                match field.data_type() {
                    DataType::Utf8 => {
                        cm.store.insert(
                            name,
                            data.downcast_ref::<array::StringArray>()
                                .unwrap()
                                .iter()
                                .map(|x| Value::String(x.unwrap().to_string()))
                                .collect::<Vec<Value>>(),
                        );
                    }

                    DataType::Float64 => {
                        cm.store.insert(
                            name,
                            data.downcast_ref::<array::Float64Array>()
                                .unwrap()
                                .iter()
                                .map(|x| Value::Number(Number::from(x.unwrap() as f64)))
                                .collect::<Vec<Value>>(),
                        );
                    }

                    DataType::Float32 => {
                        cm.store.insert(
                            name,
                            data.downcast_ref::<array::Float32Array>()
                                .unwrap()
                                .iter()
                                .map(|x| Value::Number(Number::from(x.unwrap() as f64)))
                                .collect::<Vec<Value>>(),
                        );
                    }

                    DataType::UInt64 => {
                        cm.store.insert(
                            name,
                            data.downcast_ref::<array::UInt64Array>()
                                .unwrap()
                                .iter()
                                .map(|x| Value::Number(Number::from(x.unwrap() as u64)))
                                .collect::<Vec<Value>>(),
                        );
                    }

                    DataType::Int64 => {
                        cm.store.insert(
                            name,
                            data.downcast_ref::<array::Int64Array>()
                                .unwrap()
                                .iter()
                                .map(|x| Value::Number(Number::from(x.unwrap() as i64)))
                                .collect::<Vec<Value>>(),
                        );
                    }

                    DataType::UInt32 => {
                        cm.store.insert(
                            name,
                            data.downcast_ref::<array::UInt32Array>()
                                .unwrap()
                                .iter()
                                .map(|x| Value::Number(Number::from(x.unwrap() as u64)))
                                .collect::<Vec<Value>>(),
                        );
                    }

                    DataType::Int32 => {
                        cm.store.insert(
                            name,
                            data.downcast_ref::<array::Int32Array>()
                                .unwrap()
                                .iter()
                                .map(|x| Value::Number(Number::from(x.unwrap() as i64)))
                                .collect::<Vec<Value>>(),
                        );
                    }

                    DataType::UInt16 => {
                        cm.store.insert(
                            name,
                            data.downcast_ref::<array::UInt16Array>()
                                .unwrap()
                                .iter()
                                .map(|x| Value::Number(Number::from(x.unwrap() as u64)))
                                .collect::<Vec<Value>>(),
                        );
                    }

                    DataType::Int16 => {
                        cm.store.insert(
                            name,
                            data.downcast_ref::<array::Int16Array>()
                                .unwrap()
                                .iter()
                                .map(|x| Value::Number(Number::from(x.unwrap() as i16)))
                                .collect::<Vec<Value>>(),
                        );
                    }

                    DataType::UInt8 => {
                        cm.store.insert(
                            name,
                            data.downcast_ref::<array::UInt8Array>()
                                .unwrap()
                                .iter()
                                .map(|x| Value::Number(Number::from(x.unwrap() as u64)))
                                .collect::<Vec<Value>>(),
                        );
                    }

                    DataType::Int8 => {
                        cm.store.insert(
                            name,
                            data.downcast_ref::<array::Int8Array>()
                                .unwrap()
                                .iter()
                                .map(|x| Value::Number(Number::from(x.unwrap() as i64)))
                                .collect::<Vec<Value>>(),
                        );
                    }
                    _ => (),
                }
            }
        }

        Ok(cm)
    }
}

enum RenderMode {
    Table,
    Image,
    Html,
    Unsupported,
}

impl From<&str> for RenderMode {
    fn from(s: &str) -> Self {
        match s {
            "table" => RenderMode::Table,
            "image" => RenderMode::Image,
            "html" => RenderMode::Html,
            _ => RenderMode::Unsupported,
        }
    }
}

#[async_trait]
pub trait ResultRender {
    async fn render(&mut self) -> Result<()>;
}

pub fn create_render(ctx: ExecutionContext, config: config::RenderAction) -> Box<dyn ResultRender> {
    match RenderMode::from(config.display.render_mode.as_str()) {
        mode @ (RenderMode::Image | RenderMode::Html) => {
            Box::new(ChartRender::new(mode, ctx, config))
        }
        RenderMode::Table | RenderMode::Unsupported => Box::new(TableRender::new(ctx, config)),
    }
}

struct TableRender {
    config: config::RenderAction,
    ctx: ExecutionContext,
}

impl TableRender {
    fn new(ctx: ExecutionContext, config: config::RenderAction) -> Self {
        Self { ctx, config }
    }
}

#[async_trait]
impl ResultRender for TableRender {
    async fn render(&mut self) -> Result<()> {
        let display = self.config.display.clone();
        let queries = display.queries.clone();
        for query in queries {
            for sql in query.statements {
                let now = time::Instant::now();
                println!("SQL: {}", sql);
                let df = self.ctx.sql(&sql).await?;
                df.show().await?;
                println!("Query OK, elapsed: {:#?}", now.elapsed())
            }
        }
        Ok(())
    }
}

struct ChartRender {
    mode: RenderMode,
    api: String,
    config: config::RenderAction,
    engine: Engine,
}

impl ChartRender {
    fn new(mode: RenderMode, ctx: ExecutionContext, config: config::RenderAction) -> Self {
        let dependency = config.display.dependency.clone().unwrap_or_default();
        match mode {
            RenderMode::Image => {
                let route = "/chart";
                let api = dependency.quickchart_api + route;
                Self {
                    mode,
                    api,
                    config,
                    engine: Engine::new(ctx),
                }
            }
            RenderMode::Html => {
                let api = dependency.chart_js;
                Self {
                    mode,
                    api,
                    config,
                    engine: Engine::new(ctx),
                }
            }
            _ => unreachable!(),
        }
    }

    fn extension(&self) -> String {
        match self.mode {
            RenderMode::Image => self.config.display.render_config.format.clone(),
            RenderMode::Html => "html".to_string(),
            _ => unreachable!(),
        }
    }
}

#[async_trait]
impl ResultRender for ChartRender {
    async fn render(&mut self) -> Result<()> {
        let display_config = self.config.display.clone();
        let queries = display_config.queries.clone();
        for query in queries {
            let mut cms = vec![];
            for sql in query.statements {
                cms.push(self.engine.select(&sql).await.unwrap())
            }
            if query.chart.is_none() {
                continue;
            }

            let chart_config = query.chart.unwrap();
            let dest =
                Path::new(display_config.destination.as_str()).join(chart_config.name.clone());
            self.render_chart(chart_config, cms, dest).await?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
struct Chart {
    #[serde(rename(serialize = "type"))]
    chart_type: String,
    data: Value,
    options: Value,
}

#[derive(Debug, Serialize)]
struct Parameters {
    #[serde(flatten)]
    conf: RenderConfig,
    chart: Chart,
}

impl Parameters {
    pub fn json_content(&self) -> String {
        serde_json::to_string(&self.chart).unwrap_or_default()
    }
}

static KEY_LABELS: &str = "labels";
static KEY_DATASETS: &str = "datasets";
static KET_DATA: &str = "data";

static CHART_TEMPLATE: &str = include_str!("../static/chart.tpl.html");

impl ChartRender {
    fn parse_variable(&self, s: String) -> Option<(usize, String)> {
        let l = s.find("${")?;
        let r = s.find('}')?;

        if r > l + 2 {
            let var = s[l + 2..r].to_string();
            let fields = var.splitn(2, ':').collect::<Vec<&str>>();
            if fields.len() < 2 {
                return None;
            }
            if let Ok(index) = fields[0].parse::<usize>() {
                return Some((index, fields[1].to_string()));
            }
        }
        None
    }

    async fn render_chart(
        &mut self,
        chart_config: config::ChartConfig,
        cms: Vec<ColumnMap>,
        mut dest: PathBuf,
    ) -> Result<()> {
        if cms.is_empty() {
            return Ok(());
        }

        let mut data_section = chart_config.data.clone();
        let mappings = data_section.as_mapping_mut();
        if mappings.is_none() {
            return Err(anyhow!("data section should be mappings type"));
        }

        for (key, val) in mappings.unwrap() {
            let key = key.as_str().unwrap_or_default();
            if key == KEY_LABELS {
                self.handle_labels_section(val, &cms);
            }
            if key == KEY_DATASETS {
                self.handle_datasets_section(val, &cms);
            }
        }

        let param = Parameters {
            conf: self.config.display.render_config.clone(),
            chart: Chart {
                chart_type: chart_config.chart_type,
                data: data_section,
                options: chart_config.options.unwrap_or_default(),
            },
        };

        dest.set_extension(self.extension());
        match self.mode {
            RenderMode::Html => {
                let mut ctx = Context::new();
                ctx.insert("title", &chart_config.name);
                ctx.insert("config", &param.json_content());
                ctx.insert("chart_id", &chart_config.name);
                ctx.insert("chart_js", &self.api);

                let mut f = File::create(dest.clone())?;
                let content = Tera::default().render_str(CHART_TEMPLATE, &ctx)?;
                f.write_all(content.as_bytes())?;
                info!("render html: {:?}", dest);
            }
            RenderMode::Image => {
                let response = reqwest::Client::new()
                    .post(&self.api)
                    .json(&param)
                    .send()
                    .await?;

                let mut f = File::create(dest.clone())?;
                let mut content = Cursor::new(response.bytes().await?);
                copy(&mut content, &mut f)?;
                info!("render image: {:?}", dest);
            }

            _ => unreachable!(),
        }
        Ok(())
    }

    fn handle_labels_section(&mut self, val: &mut Value, cms: &[ColumnMap]) -> Option<()> {
        let pattern = val.as_str().unwrap_or_default();
        let col = self.parse_variable(pattern.to_string())?;
        let (index, variable) = col;
        if index < cms.len() {
            if let Some(v) = cms[index].get(&variable) {
                *val = v;
            }
        }
        Some(())
    }

    fn handle_datasets_section(&mut self, val: &mut Value, cms: &[ColumnMap]) -> Option<()> {
        let seq = val.as_sequence_mut()?;
        for obj in seq {
            let obj = obj.as_mapping_mut();
            if obj.is_none() {
                continue;
            }

            for (obj_key, obj_val) in obj.unwrap() {
                let obj_key = obj_key.as_str().unwrap_or_default();
                if obj_key != KET_DATA {
                    continue;
                }

                let pattern = obj_val.as_str().unwrap_or_default();
                let col = self.parse_variable(pattern.to_string());
                if col.is_none() {
                    continue;
                }

                let (index, variable) = col.unwrap();
                if index < cms.len() {
                    if let Some(v) = cms[index].get(&variable) {
                        *obj_val = v.clone();
                    }
                }
            }
        }
        Some(())
    }
}
