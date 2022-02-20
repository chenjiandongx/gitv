mod gitimpl;
mod record;
mod config;

use std::any::Any;
use std::fs::File;
use std::iter::Filter;
pub use gitimpl::*;
pub use record::*;

use anyhow::Result;
// use async_trait::async_trait;
use datafusion::arrow::array;
use datafusion::arrow::array::{Int64Array, StringArray, UInt64Array};
use datafusion::arrow::datatypes::{DataType, SchemaRef};
use datafusion::prelude::*;
use serde::Serialize;


#[tokio::main]
async fn main() -> Result<()> {
    let c = config::load_config("./config.yaml")?;
    println!("{:#?}", c);

    // let repos = vec![repo, repo1];
    //
    // let f = File::open("./repos.yaml")?;
    // let k: Repos = serde_yaml::from_reader(f)?;
    //
    // println!("{:#?}", k);

    // CsvSerializer::serialize("./database".to_string(), "chenjiandongx".to_string(), k.repositories).unwrap();
    //
    //
    // // create local execution context
    // let mut ctx = ExecutionContext::new();
    //
    // // register csv file with the execution context
    // ctx.register_csv(
    //     "chenjiandongx",
    //     "./database/chenjiandongx.csv",
    //     CsvReadOptions::new(),
    // )
    //     .await?;
    //
    // let df = ctx
    //     .sql("select \
    //             repo_name, author_name, sum(insertion) as insertion, sum(deletion) as deletion \
    //     from chenjiandongx \
    //     where metric='CHANGE' group by author_name, repo_name order by insertion desc limit 10")
    //     .await?;
    // df.show().await?;
    //
    // let df = ctx
    //     .sql("select \
    //         author_email, count(author_email) as commit_count from chenjiandongx \
    //     where metric='COMMIT' group by author_email order by commit_count desc limit 10")
    //     .await?;
    // df.show().await?;
    //
    // let df = ctx
    //     .sql("select repo_name, count(1) from chenjiandongx where metric='TAG' group by repo_name limit 1")
    //     .await?;
    // df.show().await?;
    //
    // for val in df.collect().await? {
    //     if val.num_rows() == 0 {
    //         continue;
    //     }
    //     if val.columns().is_empty() {
    //         continue;
    //     }
    //
    //     println!("{:#?}", val.schema());
    //
    //     val.schema();
    //
    //     let x: Vec<Vec<&dyn Any>> = vec![];
    //     for col in 0..val.num_columns() {
    //         if col == 0 {
    //             let repo_name = val
    //                 .column(col)
    //                 .as_any()
    //                 .downcast_ref::<StringArray>()
    //                 .unwrap()
    //                 .iter()
    //                 .filter(|x| x.is_some());
    //
    //             for r in repo_name {
    //                 println!("repo_name: {:#?}", r.unwrap())
    //             }
    //             // repo_name
    //         }
    //
    //         if col == 1 {
    //             let repo_name = val
    //                 .column(col)
    //                 .as_any()
    //                 .downcast_ref::<UInt64Array>()
    //                 .unwrap()
    //                 .iter()
    //                 .filter(|x| x.is_some());
    //
    //             for r in repo_name {
    //                 println!("repo_name: {:#?}", r.unwrap())
    //             }
    //         }
    //     }
    // }
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