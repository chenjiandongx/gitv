use crate::{config, GraphOptions, Query};
use anyhow::Result;
use datafusion::arrow::array;
use datafusion::arrow::datatypes::DataType;
use datafusion::prelude::ExecutionContext;
use serde::Serialize;
use std::path::Path;
use std::{
    fs::File,
    io::{copy, Cursor},
};

#[derive(Debug)]
pub struct ColumnResult {
    pub labels: Vec<LabelsColumn>,
    pub values: Vec<ValuesColumn>,
}

#[derive(Debug)]
pub struct LabelsColumn {
    pub column: String,
    pub data: Vec<String>,
}

#[derive(Debug)]
pub struct ValuesColumn {
    pub column: String,
    pub data: Vec<f64>,
}

struct Engine {
    ctx: ExecutionContext,
}

impl Engine {
    fn new(ctx: ExecutionContext) -> Self {
        Self { ctx }
    }

    async fn select(&mut self, sql: &str) -> Result<ColumnResult> {
        let mut result = ColumnResult {
            labels: vec![],
            values: vec![],
        };

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
                        result.labels.push(LabelsColumn {
                            column: name,
                            data: data
                                .downcast_ref::<array::StringArray>()
                                .unwrap()
                                .iter()
                                .map(|x| x.unwrap().to_string())
                                .collect::<Vec<String>>(),
                        });
                    }

                    DataType::Float64 => {
                        result.values.push(ValuesColumn {
                            column: name,
                            data: data
                                .downcast_ref::<array::Float64Array>()
                                .unwrap()
                                .iter()
                                .map(|x| x.unwrap() as f64)
                                .collect::<Vec<f64>>(),
                        });
                    }

                    DataType::Float32 => {
                        result.values.push(ValuesColumn {
                            column: name,
                            data: data
                                .downcast_ref::<array::Float32Array>()
                                .unwrap()
                                .iter()
                                .map(|x| x.unwrap() as f64)
                                .collect::<Vec<f64>>(),
                        });
                    }

                    DataType::UInt64 => {
                        result.values.push(ValuesColumn {
                            column: name,
                            data: data
                                .downcast_ref::<array::UInt64Array>()
                                .unwrap()
                                .iter()
                                .map(|x| x.unwrap() as f64)
                                .collect::<Vec<f64>>(),
                        });
                    }

                    DataType::Int64 => {
                        result.values.push(ValuesColumn {
                            column: name,
                            data: data
                                .downcast_ref::<array::Int64Array>()
                                .unwrap()
                                .iter()
                                .map(|x| x.unwrap() as f64)
                                .collect::<Vec<f64>>(),
                        });
                    }

                    DataType::UInt32 => {
                        result.values.push(ValuesColumn {
                            column: name,
                            data: data
                                .downcast_ref::<array::UInt32Array>()
                                .unwrap()
                                .iter()
                                .map(|x| x.unwrap() as f64)
                                .collect::<Vec<f64>>(),
                        });
                    }

                    DataType::Int32 => {
                        result.values.push(ValuesColumn {
                            column: name,
                            data: data
                                .downcast_ref::<array::Int32Array>()
                                .unwrap()
                                .iter()
                                .map(|x| x.unwrap() as f64)
                                .collect::<Vec<f64>>(),
                        });
                    }

                    DataType::UInt16 => {
                        result.values.push(ValuesColumn {
                            column: name,
                            data: data
                                .downcast_ref::<array::UInt16Array>()
                                .unwrap()
                                .iter()
                                .map(|x| x.unwrap() as f64)
                                .collect::<Vec<f64>>(),
                        });
                    }

                    DataType::Int16 => {
                        result.values.push(ValuesColumn {
                            column: name,
                            data: data
                                .downcast_ref::<array::Int16Array>()
                                .unwrap()
                                .iter()
                                .map(|x| x.unwrap() as f64)
                                .collect::<Vec<f64>>(),
                        });
                    }

                    DataType::UInt8 => {
                        result.values.push(ValuesColumn {
                            column: name,
                            data: data
                                .downcast_ref::<array::UInt8Array>()
                                .unwrap()
                                .iter()
                                .map(|x| x.unwrap() as f64)
                                .collect::<Vec<f64>>(),
                        });
                    }

                    DataType::Int8 => {
                        result.values.push(ValuesColumn {
                            column: name,
                            data: data
                                .downcast_ref::<array::Int8Array>()
                                .unwrap()
                                .iter()
                                .map(|x| x.unwrap() as f64)
                                .collect::<Vec<f64>>(),
                        });
                    }

                    _ => (),
                }
            }
        }

        Ok(result)
    }
}

enum ChartType {
    Bar,
    Line,
}

impl From<ChartType> for String {
    fn from(t: ChartType) -> Self {
        match t {
            ChartType::Bar => String::from("bar"),
            ChartType::Line => String::from("line"),
        }
    }
}

static DEFAULT_API: &str = "https://quickchart.io";
static DEFAULT_ROUTE: &str = "/chart";

pub struct GraphRender {
    api: String,
    config: config::RenderAction,
    engine: Engine,
}

impl GraphRender {
    pub fn new(ctx: ExecutionContext, config: config::RenderAction) -> Self {
        let api = config.display.render_api.clone();
        let api = if api.is_empty() {
            DEFAULT_API.to_owned() + DEFAULT_ROUTE.clone()
        } else {
            api + DEFAULT_ROUTE.clone()
        };

        Self {
            api,
            config,
            engine: Engine { ctx },
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Data {
    pub labels: Vec<String>,
    pub datasets: Vec<Dataset>,
}

#[derive(Debug, Serialize)]
pub struct Dataset {
    label: String,
    data: Vec<f64>,
}

#[derive(Debug, Serialize)]
struct Chart {
    #[serde(rename(serialize = "type"))]
    chart_type: String,
    data: Data,
}

#[derive(Debug, Serialize)]
struct Parameters {
    #[serde(flatten)]
    options: GraphOptions,
    chart: Chart,
}

impl GraphRender {
    pub async fn render(&mut self) -> Result<()> {
        let queries = self.config.display.queries.clone();
        for query in queries {
            let mut crs = vec![];
            for sql in query.statements {
                crs.push(self.engine.select(&sql).await.unwrap())
            }

            let dest = Path::new(self.config.display.destination.as_str()).join(format!(
                "{}.{}",
                query.graph.name, self.config.display.render_options.format
            ));
            let dest = dest.to_str().unwrap();

            match query.graph.chart_type.as_str() {
                "bar" => {
                    self.render_2d_axis_chart("bar".to_string(), crs, query.graph, dest)
                        .await?;
                }
                "line" => {
                    self.render_2d_axis_chart("line".to_string(), crs, query.graph, dest)
                        .await?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub async fn render_2d_axis_chart(
        &mut self,
        chart_type: String,
        crs: Vec<ColumnResult>,
        graph: config::Graph,
        dest: &str,
    ) -> Result<()> {
        if crs.is_empty() || graph.series.is_empty() {
            return Ok(());
        }

        let mut labels = vec![];
        let mut datasets = vec![];
        for (i, series) in graph.series.iter().enumerate() {
            // graph 的 label 默认只取第一个 因为目前只支持单 X 轴
            if i == 0 {
                for l in &crs[0].labels {
                    if l.column == series.label {
                        labels = l.data.clone();
                    }
                }
            }
            let legend = series.legend.clone();

            let mut index: usize = 0;
            let mut dataset = &*series.dataset.clone();
            let fields: Vec<&str> = series.dataset.split(':').collect();
            if fields.len() > 1 {
                index = fields[0].parse::<usize>().unwrap_or_default();
                dataset = fields[1];
            }
            if index >= crs.len() {
                index = 0
            }

            let mut values = vec![];
            for v in &crs[index].values {
                if v.column == dataset {
                    values = v.data.clone();
                }
            }
            datasets.push(Dataset {
                label: legend,
                data: values,
            })
        }

        let param = Parameters {
            options: Default::default(),
            chart: Chart {
                chart_type,
                data: Data { labels, datasets },
            },
        };

        println!("param: {:#?}", param);

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
