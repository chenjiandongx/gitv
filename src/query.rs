use crate::config;
use anyhow::Result;
use datafusion::arrow::array;
use datafusion::arrow::datatypes::DataType;
use datafusion::prelude::ExecutionContext;

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

#[derive(Debug)]
pub struct ColumnResult {
    pub labels: Vec<LabelsColumn>,
    pub values: Vec<ValuesColumn>,
}

async fn exec(query: &config::Query, ctx: &mut ExecutionContext) -> Result<ColumnResult> {
    let mut result = ColumnResult {
        labels: vec![],
        values: vec![],
    };

    let df = ctx.sql(&query.sql).await?;
    df.show().await;
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

pub async fn select(c: config::Config, ctx: &mut ExecutionContext) -> Result<Vec<ColumnResult>> {
    let mut result = vec![];
    for query in c.render.queries {
        result.push(exec(&query, ctx).await?);
    }
    Ok(result)
}
