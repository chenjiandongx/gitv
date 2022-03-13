use crate::config;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use datafusion::{
    arrow::{array, datatypes::DataType},
    prelude::ExecutionContext,
};
use lazy_static::lazy_static;
use rand::prelude::*;
use serde::Serialize;
use serde_yaml::{Mapping, Number, Value};
use std::{
    collections::HashMap,
    fmt::Debug,
    fs::File,
    io::Write,
    ops::Range,
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
    Html,
    Unsupported,
}

impl From<&str> for RenderMode {
    fn from(s: &str) -> Self {
        match s {
            "table" => RenderMode::Table,
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
        RenderMode::Html => Box::new(ChartRender::new(ctx, config)),
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
    config: config::RenderAction,
    engine: Engine,
}

impl ChartRender {
    fn new(ctx: ExecutionContext, config: config::RenderAction) -> Self {
        Self {
            config,
            engine: Engine::new(ctx),
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
            let dest = Path::new(&display_config.destination).join(chart_config.name.clone());
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

static KEY_LABELS: &str = "labels";
static KEY_DATASETS: &str = "datasets";
static KET_DATA: &str = "data";
static KEY_COLORS: &str = "backgroundColor";
static KEY_PLUGINS: &str = "plugins";
static KEY_DATALABELS: &str = "datalabels";
static KEY_FORMATTER: &str = "formatter";

static VALUE_RANDOM: &str = "random";

static TEMPLATE_CHART: &str = include_str!("../static/chart.tpl");
static CONTENT_COLORS: &str = include_str!("../static/colors.yaml");
static CONTENT_FUNCTIONS: &str = include_str!("../static/functions.yaml");

lazy_static! {
    static ref COLORS: HashMap<String, Vec<Value>> = include_colors();
    static ref FUNCTIONS: HashMap<String, Value> = include_functions();
}

fn include_colors() -> HashMap<String, Vec<Value>> {
    let values: Value = serde_yaml::from_str(CONTENT_COLORS).unwrap();
    let mappings = values.as_mapping().unwrap().clone();
    let mut hm = HashMap::new();
    for (k, v) in mappings {
        hm.insert(
            k.as_str().unwrap().to_string(),
            v.as_sequence().unwrap().clone(),
        );
    }
    hm
}

fn include_functions() -> HashMap<String, Value> {
    let values: Value = serde_yaml::from_str(CONTENT_FUNCTIONS).unwrap();
    let mappings = values.as_mapping().unwrap().clone();
    let mut hm = HashMap::new();
    for (k, v) in mappings {
        hm.insert(
            k.as_str().unwrap().to_string(),
            Value::String(v.as_str().unwrap().to_string()),
        );
    }
    hm
}

impl ChartRender {
    fn parse_variable<S: Into<String>>(s: S) -> Option<(usize, String)> {
        let s = s.into();
        let l = s.find("${")?;
        let r = s.find('}')?;

        if r > l + 2 {
            let var = s[l + 2..r].to_string();
            let fields = var.splitn(2, ':').collect::<Vec<&str>>();
            if fields.len() <= 1 {
                return Some((0, fields[0].to_string()));
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
        self.hanlde_data_section(mappings.unwrap(), &cms);

        let options_section = chart_config.options.clone();
        let mut options_section = options_section.unwrap_or_default();
        let mappings = options_section.as_mapping_mut();
        if mappings.is_some() {
            self.hanlde_options_section(mappings.unwrap());
        }

        let content = serde_json::to_string(&Chart {
            chart_type: chart_config.chart_type,
            data: data_section,
            options: options_section,
        })
        .unwrap_or_default();

        dest.set_extension("html");

        let mut ctx = Context::new();
        ctx.insert("width", &chart_config.width);
        ctx.insert("height", &chart_config.height);
        ctx.insert("title", &chart_config.name);
        ctx.insert("config", &content);
        ctx.insert("chart_id", &chart_config.name);

        let deps = self.config.display.dependency.clone().unwrap_or_default();
        ctx.insert("dependencies", &deps.list());
        ctx.insert("register", &deps.register());

        let mut f = File::create(dest.clone())?;
        let content = Tera::default().render_str(TEMPLATE_CHART, &ctx)?;
        f.write_all(self.cleanup_content(content).as_bytes())?;
        info!("render html: {:?}", dest);
        Ok(())
    }

    fn cleanup_content(&self, s: String) -> String {
        s.replace(r#""{{%"#, "")
            .replace(r#"%}}""#, "")
            .replace(",,", ",")
    }

    fn hanlde_options_section(&mut self, mappings: &mut Mapping) {
        for (key, val) in mappings {
            let key = key.as_str().unwrap_or_default();
            if key == KEY_PLUGINS {
                let plugins = val.as_mapping_mut();
                if plugins.is_none() {
                    continue;
                }

                for (pk, pv) in plugins.unwrap() {
                    let pk = pk.as_str().unwrap_or_default();
                    if pk == KEY_DATALABELS {
                        let datalabels = pv.as_mapping_mut();
                        if datalabels.is_none() {
                            continue;
                        }
                        self.handle_datalabels_field(datalabels.unwrap());
                    }
                }
            }
        }
    }

    fn handle_datalabels_field(&mut self, mappings: &mut Mapping) -> Option<()> {
        for (key, val) in mappings {
            let key = key.as_str().unwrap_or_default();
            if key == KEY_FORMATTER {
                let (_, var) = Self::parse_variable(val.as_str().unwrap_or_default())?;
                let function = FUNCTIONS.get(&var);
                if let Some(f) = function {
                    *val = f.clone();
                }
            }
        }
        Some(())
    }

    fn hanlde_data_section(&mut self, mappings: &mut Mapping, cms: &[ColumnMap]) {
        for (key, val) in mappings {
            let key = key.as_str().unwrap_or_default();
            if key == KEY_LABELS {
                self.handle_labels_field(val, &cms);
            }
            if key == KEY_DATASETS {
                self.handle_datasets_field(val, &cms);
            }
        }
    }

    fn handle_labels_field(&mut self, val: &mut Value, cms: &[ColumnMap]) -> Option<()> {
        let (index, variable) = Self::parse_variable(val.as_str().unwrap_or_default())?;
        if index < cms.len() {
            if let Some(v) = cms[index].get(&variable) {
                *val = v;
            }
        }
        Some(())
    }

    fn handle_datasets_field(&mut self, val: &mut Value, cms: &[ColumnMap]) -> Option<()> {
        let seq = val.as_sequence_mut()?;
        for dataset in seq {
            let dataset = dataset.as_mapping_mut();
            if dataset.is_none() {
                continue;
            }

            for (dk, dv) in dataset.unwrap() {
                let dk = dk.as_str().unwrap_or_default();
                if dk == KET_DATA {
                    let pattern = dv.as_str().unwrap_or_default();
                    let col = Self::parse_variable(pattern.to_string());
                    if col.is_none() {
                        continue;
                    }

                    let (index, variable) = col.unwrap();
                    if index < cms.len() {
                        if let Some(v) = cms[index].get(&variable) {
                            *dv = v.clone();
                        }
                    }
                }
                if dk == KEY_COLORS {
                    if let Some(v) = self.handle_colors_field(dv) {
                        *dv = Value::Sequence(v.to_vec());
                    }
                }
            }
        }
        Some(())
    }

    fn handle_colors_field(&mut self, val: &mut Value) -> Option<&[Value]> {
        let (_, var) = Self::parse_variable(val.as_str().unwrap_or_default())?;
        if var == VALUE_RANDOM {
            let mut rng = rand::thread_rng();
            let n: usize = rng.gen();
            return Some(COLORS.values().skip(n % COLORS.len()).next()?);
        }
        Some(COLORS.get(&var)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_variable() {
        let (index, var) = ChartRender::parse_variable("${0:foo}").unwrap();
        assert_eq!((index, var), (0, "foo".to_string()));

        let (index, var) = ChartRender::parse_variable("${0:foo:bar}").unwrap();
        assert_eq!((index, var), (0, "foo:bar".to_string()));

        let (index, var) = ChartRender::parse_variable("${foo}").unwrap();
        assert_eq!((index, var), (0, "foo".to_string()));

        assert_eq!(ChartRender::parse_variable("${}"), None);
        assert_eq!(ChartRender::parse_variable("${"), None);
        assert_eq!(ChartRender::parse_variable("{}"), None);
        assert_eq!(ChartRender::parse_variable("}${"), None);
    }
}
