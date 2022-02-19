mod gitimpl;
mod record;

pub use gitimpl::*;
pub use record::*;

use anyhow::Result;
use async_trait::async_trait;
use datafusion::arrow::array;
use datafusion::arrow::array::{StringArray, UInt64Array};
use datafusion::arrow::datatypes::DataType;
use datafusion::prelude::*;
use serde::Serialize;

struct Analyzer {
    repo: Repository,
    recorder: Box<dyn RecordSerializer>,
}

impl Analyzer {
    // new()
    fn fetch_repo(&self) {
        self.recorder.write(self.repo.clone());
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let repo = Repository::new(
        "pyecharts:pyecharts",
        "",
        "/Users/chenjiandongx/project/python/pyecharts",
        vec![],
    );

    let analyzer = Analyzer {
        recorder: Box::new(CsvSerializer {
            git: Box::new(GitBinary { repo: &repo }),
        }),
    };

    analyzer.recorder.write(repo)?;

    // create local execution context
    let mut ctx = ExecutionContext::new();

    // register csv file with the execution context
    ctx.register_csv(
        "pyecharts:pyecharts",
        "./pyecharts:pyecharts.csv",
        CsvReadOptions::new(),
    )
    .await?;

    // ctx.register_avro()

    let df = ctx
        .sql("select author_name, sum(insertion) as insertion, sum(deletion) as deletion from 'pyecharts:pyecharts' where metric='change' group by author_name order by insertion desc limit 10")
        .await?;
    df.show().await?;

    let df = ctx
        .sql("select author_email, count(author_email) as commit_count from 'pyecharts:pyecharts' where metric='commit' group by author_email order by commit_count desc limit 10")
        .await?;
    df.show().await?;

    let df = ctx
        .sql("select tag from 'pyecharts:pyecharts' where metric='tag' limit 10")
        .await?;
    df.show().await?;

    // for val in df.collect().await? {
    //     // println!("{:#?}", val.schema());
    //     if val.num_rows() == 0 {
    //         continue;
    //     }
    //     if val.columns().is_empty() {
    //         continue;
    //     }
    //
    //     for col in 0..val.num_columns() {
    //         // val.column(col).data_type()
    //         // val.column(col).as_any().
    //     }
    //     // println!("");
    //     let author_name = val
    //         .column(0)
    //         .as_any()
    //         .downcast_ref::<StringArray>()
    //         .unwrap()
    //         .value(0)
    //         .to_string();
    //
    //     // println!("{:#?}: {:#?}", author_name, count);
    // }
    Ok(())
}
