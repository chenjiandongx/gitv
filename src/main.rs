mod config;
mod gitter;
mod gitter_binary;
mod query_executor;
mod record;
mod record_csv;
mod render_graph;
mod repo_fetcher;
mod repo_github;
mod shell;

use crate::{record_csv::CsvSerializer, render_graph::GraphRender, repo_fetcher::RepoFetcher};
use anyhow::Result;
use chrono::Local;
use clap::Parser;
use config::*;
use gitter::*;
use gitter_binary::*;
use query_executor::*;
use record::*;
use std::{io, process::exit};
use tracing::*;
use tracing_subscriber::fmt::{format::Writer, time::FormatTime};

// TODO(optimize): 拆分大函数
// TODO(docs): 完善代码注释
// TODO(feat): 新增代码函数统计
// TODO(optimize): 统一配置校验方式
// TODO(optimize): 更合理的错误处理方式

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
    #[clap(short, long)]
    shell: bool,

    /// docs here
    path: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_logger();

    let cli: Cli = Cli::parse();
    let c: Config = config::load_config(&cli.path).unwrap();

    if cli.fetch {
        let repo_fetcher = RepoFetcher::new(c.fetch);
        repo_fetcher.fetch().await?;
    }

    if cli.init {
        let serializer = CsvSerializer::new(GitBinaryImpl);
        serializer.serialize(c.init).await?
    }

    if cli.shell {
        let ctx = Executor::create_context(c.shell.executions).await?;
        shell::console_loop(ctx).await?;
        exit(0)
    }

    if cli.render {
        let executions = c.render.executions.clone();
        let ctx = Executor::create_context(executions).await?;
        let mut render = GraphRender::new(ctx, c.render);
        render.render().await.unwrap();
    }

    Ok(())
}
