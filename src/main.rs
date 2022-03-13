mod config;
mod executor;
mod fetcher;
mod gitter;
mod record;
mod render;
mod shell;

use anyhow::Result;
use chrono::Local;
use clap::Parser;
use config::*;
use executor::*;
use fetcher::*;
use gitter::*;
use record::*;
use std::{io, process::exit};
use tracing::*;
use tracing_subscriber::fmt::{format::Writer, time::FormatTime};

// TODO(feat): 新增代码函数统计
// TODO(optimize): 统一配置校验方式

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
    /// Retrieve repos and create new databases
    #[clap(short, long)]
    create: bool,

    /// Fetch repos metadata from remote source (github)
    #[clap(short, long)]
    fetch: bool,

    /// Render query result as the given mode (htlm, table)
    #[clap(short, long)]
    render: bool,

    /// Load data and enter into a new spawn shell
    #[clap(short, long)]
    shell: bool,

    /// config file path (default: gitx.yaml)
    path: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    setup_logger();

    let cli: Cli = Cli::parse();
    let c: Config = match config::load_config(&cli.path.unwrap_or("gitx.yaml".to_string())) {
        Err(e) => {
            error!("load config error: {}", e);
            exit(1);
        }
        Ok(c) => c,
    };

    if cli.fetch {
        let repo_fetcher = RepoFetcher::new(c.fetch);
        if let Err(e) = repo_fetcher.fetch().await {
            error!("fetch repos error: {}", e);
            exit(1);
        };
        exit(0)
    }

    if cli.create {
        let serializer = CsvSerializer::new(BinaryGitter);
        if let Err(e) = serializer.serialize(c.create).await {
            error!("serialize records error: {}", e);
            exit(1);
        };
        exit(0)
    }

    if cli.shell {
        let ctx = Executor::create_context(c.shell.executions).await;
        let ctx = match ctx {
            Err(e) => {
                error!("create executor context error: {}", e);
                exit(1)
            }
            Ok(ctx) => ctx,
        };

        if let Err(e) = shell::console_loop(ctx).await {
            error!("shell console loop error: {}", e);
            exit(1);
        };
        exit(0)
    }

    if cli.render {
        let executions = c.render.executions.clone();
        let ctx = Executor::create_context(executions).await;
        let ctx = match ctx {
            Err(e) => {
                error!("create executor context error: {}", e);
                exit(1)
            }
            Ok(ctx) => ctx,
        };

        let mut render = render::create_render(ctx, c.render);
        if let Err(e) = render.render().await {
            error!("render output error: {}", e);
            exit(1);
        }
        exit(0)
    }

    Ok(())
}
