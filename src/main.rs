mod charts;
mod config;
mod git_binary;
mod git_impl;
mod query;
mod record;
mod register_udf;
mod repo_github;

use anyhow::Result;
use config::*;
use datafusion::prelude::*;
use git_binary::*;
use git_impl::*;
use record::*;
use register_udf::*;
use repo_github::*;
use serde::{Deserialize, Serialize};
use std::fs::File;

#[derive(Debug, Serialize, Deserialize)]
struct Payload {
    name: String,
    age: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct D {
    repositories: Vec<Repository>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let c: Config = config::load_config("./private.yaml")?;
    println!("config: {:#?}", c);

    let github_sourcer = GithubSourcer::new(&c.github);

    let repos = github_sourcer.repositories().await?;
    print!("repos: {:#?}", repos.len());

    let f = File::create("./database.yaml").unwrap();
    serde_yaml::to_writer(
        f,
        &D {
            repositories: repos,
        },
    )
    .unwrap();

    let serializer = CsvSerializer::new(Box::new(GitBinaryImpl));
    // serializer.serialize(&c).await?;

    let mut ctx = ExecutionContext::new();
    for udf in UDFS.iter() {
        ctx.register_udf(udf());
    }
    for udaf in UDAFS.iter() {
        ctx.register_udaf(udaf())
    }

    println!("u: {}", &c.database.uri(serializer.extension()));
    ctx.register_csv(
        &c.database.name,
        &c.database.uri(serializer.extension()),
        CsvReadOptions::new(),
    )
    .await?;

    let cr = query::select(c, &mut ctx).await?;
    println!("cr: {:#?}", cr);

    Ok(())
}
