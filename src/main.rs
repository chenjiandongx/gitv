mod charts;
mod config;
mod gitter;
mod gitter_binary;
mod query;
mod query_functions;
mod record;
mod record_csv;
mod repo_fetcher;
mod repo_github;

use crate::record_csv::CsvSerializer;
use crate::repo_fetcher::RepoFetcher;
use anyhow::Result;
use chrono::Local;
use clap::{Args, Parser, Subcommand};
use config::*;
use datafusion::prelude::*;
use futures::StreamExt;
use gitter::*;
use gitter_binary::*;
use query_functions::*;
use record::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::{io, time};
use tracing::Level;
use tracing::*;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::FormatTime;

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// init flag
    #[clap(short, long)]
    init: bool,

    /// sync flag
    #[clap(short, long)]
    fetch: bool,

    /// render flag
    #[clap(short, long)]
    render: bool,

    /// shell flag
    #[clap(short = 'S', long)]
    shell: bool,

    /// docs here
    path: String,
}

struct App {
    // github_fechter
}

struct LocalTimer;

impl FormatTime for LocalTimer {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        write!(w, "{}", Local::now().format("[%Y-%m-%d %H:%M:%S]"))
    }
}

fn setup_logger() {
    let format = tracing_subscriber::fmt::format()
        .with_level(false)
        .with_timer(LocalTimer)
        .with_target(false);

    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_writer(io::stdout)
        .with_ansi(true)
        .event_format(format)
        .init();
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_logger();
    let cli: Cli = Cli::parse();
    println!("path: {}", cli.path);

    // let f = File::create("./database.yaml").unwrap();
    let c: Config = config::load_config(&cli.path).unwrap();

    // tokio::

    if cli.fetch {
        let repo_fetcher = RepoFetcher::new(c.fetch.clone());
        repo_fetcher.fetch().await?;
    }
    if cli.init {
        print!("hii");
        let serializer = CsvSerializer::new(GitBinaryImpl);
        serializer.serialize(c.init.clone()).await?
    }

    // time::s(Duration::from_secs(2));
    // let serializer = record_csv::CsvSerializer::new(GitBinaryImpl);
    // serializer.serialize()
    // println!("{:#?}", cli.path);
    Ok(())
}
