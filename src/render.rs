use crate::{config, GraphOptions};
use anyhow::Result;
use datafusion::{
    arrow::{array, datatypes::DataType},
    prelude::ExecutionContext,
};
use serde::{Serialize, Serializer};
use std::path::PathBuf;
use std::{
    collections::HashMap,
    fmt::Debug,
    fs::File,
    io::{copy, Cursor},
    path::Path,
};

#[derive(Debug, Clone)]
pub enum ColumnType {
    Float64(f64),
    Int64(i64),
    String(String),
}

impl Serialize for ColumnType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            ColumnType::Float64(v) => serializer.serialize_f64(*v),
            ColumnType::Int64(v) => serializer.serialize_i64(*v),
            ColumnType::String(v) => serializer.serialize_str(v.as_str()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ColumnMap {
    pub store: HashMap<String, Vec<ColumnType>>,
}

impl ColumnMap {
    fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
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
                                .map(|x| ColumnType::String(x.unwrap().to_string()))
                                .collect::<Vec<ColumnType>>(),
                        );
                    }

                    DataType::Float64 => {
                        cm.store.insert(
                            name,
                            data.downcast_ref::<array::Float64Array>()
                                .unwrap()
                                .iter()
                                .map(|x| ColumnType::Float64(x.unwrap() as f64))
                                .collect::<Vec<ColumnType>>(),
                        );
                    }

                    DataType::Float32 => {
                        cm.store.insert(
                            name,
                            data.downcast_ref::<array::Float32Array>()
                                .unwrap()
                                .iter()
                                .map(|x| ColumnType::Float64(x.unwrap() as f64))
                                .collect::<Vec<ColumnType>>(),
                        );
                    }

                    DataType::UInt64 => {
                        cm.store.insert(
                            name,
                            data.downcast_ref::<array::UInt64Array>()
                                .unwrap()
                                .iter()
                                .map(|x| ColumnType::Int64(x.unwrap() as i64))
                                .collect::<Vec<ColumnType>>(),
                        );
                    }

                    DataType::Int64 => {
                        cm.store.insert(
                            name,
                            data.downcast_ref::<array::Int64Array>()
                                .unwrap()
                                .iter()
                                .map(|x| ColumnType::Int64(x.unwrap()))
                                .collect::<Vec<ColumnType>>(),
                        );
                    }

                    DataType::UInt32 => {
                        cm.store.insert(
                            name,
                            data.downcast_ref::<array::UInt32Array>()
                                .unwrap()
                                .iter()
                                .map(|x| ColumnType::Int64(x.unwrap() as i64))
                                .collect::<Vec<ColumnType>>(),
                        );
                    }

                    DataType::Int32 => {
                        cm.store.insert(
                            name,
                            data.downcast_ref::<array::Int32Array>()
                                .unwrap()
                                .iter()
                                .map(|x| ColumnType::Int64(x.unwrap() as i64))
                                .collect::<Vec<ColumnType>>(),
                        );
                    }

                    DataType::UInt16 => {
                        cm.store.insert(
                            name,
                            data.downcast_ref::<array::UInt16Array>()
                                .unwrap()
                                .iter()
                                .map(|x| ColumnType::Int64(x.unwrap() as i64))
                                .collect::<Vec<ColumnType>>(),
                        );
                    }

                    DataType::Int16 => {
                        cm.store.insert(
                            name,
                            data.downcast_ref::<array::Int16Array>()
                                .unwrap()
                                .iter()
                                .map(|x| ColumnType::Int64(x.unwrap() as i64))
                                .collect::<Vec<ColumnType>>(),
                        );
                    }

                    DataType::UInt8 => {
                        cm.store.insert(
                            name,
                            data.downcast_ref::<array::UInt8Array>()
                                .unwrap()
                                .iter()
                                .map(|x| ColumnType::Int64(x.unwrap() as i64))
                                .collect::<Vec<ColumnType>>(),
                        );
                    }

                    DataType::Int8 => {
                        cm.store.insert(
                            name,
                            data.downcast_ref::<array::Int8Array>()
                                .unwrap()
                                .iter()
                                .map(|x| ColumnType::Int64(x.unwrap() as i64))
                                .collect::<Vec<ColumnType>>(),
                        );
                    }

                    _ => (),
                }
            }
        }

        Ok(cm)
    }
}

static DEFAULT_API: &str = "https://quickchart.io";

pub struct GraphRender {
    api: String,
    config: config::RenderAction,
    engine: Engine,
}

impl GraphRender {
    pub fn new(ctx: ExecutionContext, config: config::RenderAction) -> Self {
        let route = "/chart";
        let api = config.display.render_api.clone();
        let api = if api.is_empty() {
            DEFAULT_API.to_owned() + route
        } else {
            api + route
        };

        Self {
            api,
            config,
            engine: Engine::new(ctx),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Data {
    pub labels: Vec<ColumnType>,
    pub datasets: Vec<Dataset>,
}

#[derive(Debug, Serialize)]
pub struct Dataset {
    label: String,
    data: Vec<ColumnType>,
}

#[derive(Debug, Serialize)]
struct Chart {
    #[serde(rename(serialize = "type"))]
    chart_type: &'static str,
    data: Data,
}

#[derive(Debug, Serialize)]
struct Parameters {
    #[serde(flatten)]
    options: GraphOptions,
    chart: Chart,
}

const CHART_BAR: &str = "bar";
const CHART_LINE: &str = "line";

impl GraphRender {
    pub async fn render(&mut self) -> Result<()> {
        let display = self.config.display.clone();
        let queries = display.queries.clone();
        for query in queries {
            let mut cms = vec![];
            for sql in query.statements {
                cms.push(self.engine.select(&sql).await.unwrap())
            }

            let dest = Path::new(display.destination.as_str()).join(format!(
                "{}.{}",
                query.graph.name, display.render_options.format
            ));

            match query.graph.chart_type.as_str() {
                CHART_BAR => {
                    self.render_2d_axis_chart(CHART_BAR, cms, query.graph, dest)
                        .await?;
                }
                CHART_LINE => {
                    self.render_2d_axis_chart(CHART_LINE, cms, query.graph, dest)
                        .await?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub async fn render_2d_axis_chart(
        &mut self,
        chart_type: &'static str,
        cms: Vec<ColumnMap>,
        graph: config::Graph,
        dest: PathBuf,
    ) -> Result<()> {
        if cms.is_empty() || graph.series.is_empty() {
            return Ok(());
        }

        let mut labels = vec![];
        let mut datasets = vec![];
        for (i, series) in graph.series.iter().enumerate() {
            if i == 0 {
                let value = cms[0].store.get(&series.label);
                if let Some(lbs) = value {
                    labels = lbs.to_vec()
                }
            }

            let mut index: usize = 0;
            let mut dataset = &*series.dataset.clone();
            let fields: Vec<&str> = series.dataset.split(':').collect();
            if fields.len() > 1 {
                index = fields[0].parse::<usize>().unwrap_or_default();
                dataset = fields[1];
            }
            if index >= cms.len() {
                index = 0
            }

            let legend = series.legend.clone();
            let value = cms[index].store.get(dataset);
            if let Some(vs) = value {
                datasets.push(Dataset {
                    label: legend,
                    data: vs.to_vec(),
                })
            }
        }

        let param = Parameters {
            options: Default::default(),
            chart: Chart {
                chart_type,
                data: Data { labels, datasets },
            },
        };

        let response = reqwest::Client::new()
            .post(&self.api)
            .json(&param)
            .send()
            .await?;

        let mut f = File::create(dest)?;
        let mut content = Cursor::new(response.bytes().await?);
        copy(&mut content, &mut f)?;
        Ok(())
    }
}
