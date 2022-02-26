mod config;
mod gitbinary;
mod gitimpl;
mod record;
mod register_udf;

use crate::register_udf::udf_weekday;
use anyhow::Result;
use chrono::{Datelike, TimeZone, Utc};
pub use config::*;
use datafusion::arrow::array::{StringArray, UInt64Array};
use datafusion::prelude::*;
pub use gitbinary::*;
pub use record::*;
pub use register_udf::*;

#[tokio::main]
async fn main() -> Result<()> {
    let c: Config = config::load_config("./config.yaml")?;
    println!("{:#?}", c);

    let repos = &c.databases[0].repositories;
    let serializer = CsvSerializer {
        git: Box::new(GitBinaryImpl),
    };

    // let mappings = vec![];
    // let mappings = mappings.as_slice();
    serializer
        .serialize(
            "./database".to_string(),
            "dongdongx".to_string(),
            repos,
            c.author_mappings,
        )
        .await?;

    // create local execution context
    let mut ctx = ExecutionContext::new();
    ctx.register_udf(udf_year());
    ctx.register_udf(udf_month());
    ctx.register_udf(udf_weekday());
    ctx.register_udf(udf_hour());
    ctx.register_udf(udf_timezone());
    ctx.register_udf(udf_date_day());
    ctx.register_udf(udf_date_month());
    ctx.register_udaf(udf_active_days());
    ctx.register_udaf(udf_active_longest());

    // register csv file with the execution context
    ctx.register_csv(
        "chenjiandongx",
        "./database/dongdongx.csv",
        CsvReadOptions::new(),
    )
    .await?;

    let df = ctx
        .sql("select active_longest(datetime) as longest, author_name from chenjiandongx where metric='COMMIT' group by author_name")
        .await?;
    df.show().await?;

    let df = ctx
        .sql("select date_month(datetime) as year, active_days(datetime) from chenjiandongx group by year order by year")
        .await?;
    df.show().await?;

    let df = ctx
        .sql("select date_month(datetime) as year, count(distinct(date_day(datetime))) from chenjiandongx group by year order by year")
        .await?;
    df.show().await?;

    let _df = ctx
        .sql(
            "select \
                repo_name, author_name, sum(insertion) as insertion, sum(deletion) as deletion \
        from chenjiandongx \
        where metric='CHANGE' group by author_name, repo_name order by insertion desc limit 10",
        )
        .await?;
    // df.show().await?;

    let _df = ctx
        .sql(
            "select \
            author_email, count(author_email) as commit_count from chenjiandongx \
        where metric='COMMIT' group by author_email order by commit_count desc limit 10",
        )
        .await?;
    // df.show().await?;

    let df = ctx
        .sql("select repo_name, count(1) from chenjiandongx where metric='TAG' group by repo_name limit 1")
        .await?;
    // df.show().await?;

    for val in df.collect().await? {
        if val.num_rows() == 0 {
            continue;
        }
        if val.columns().is_empty() {
            continue;
        }

        let _fields = val.schema().fields();

        // x -> labels:
        // y -> values:

        // val.schema().clone()    ;
        let dt = Utc.timestamp(1_500_000_000, 0);
        dt.weekday().to_string();

        for col in 0..val.num_columns() {
            if col == 0 {
                let repo_name = val
                    .column(col)
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .unwrap()
                    .iter()
                    .filter(|x| x.is_some());

                for r in repo_name {
                    println!("repo_name: {:#?}", r.unwrap())
                }
            }

            if col == 1 {
                let repo_name = val
                    .column(col)
                    .as_any()
                    .downcast_ref::<UInt64Array>()
                    .unwrap()
                    .iter()
                    .filter(|x| x.is_some());

                for r in repo_name {
                    println!("repo_name: {:#?}", r.unwrap())
                }
            }
        }
    }

    Ok(())
}

// 用语句描述查询 SQL？
// datatype 转换

// enum DT {
//     StringArray,
//     UInt64Array,
// }
//
// fn fx(schema: SchemaRef) {
//     for field in schema.fields() {
//         match field {
//             DataType::UInt64 => DT::UInt64Array
//         }
//     }
//     match schema.fields()
// }
