mod config;
mod executor;
mod fetcher;
mod gitter;
mod record;
mod render;
mod shell;

use anyhow::Result;
use chrono::Local;
use clap::{IntoApp, Parser};
use config::*;
use executor::*;
use fetcher::*;
use gitter::*;
use record::*;
use std::{io, process::exit};
use tracing::*;
use tracing_subscriber::fmt::{format::Writer, time::FormatTime};

struct LocalTimer;

impl FormatTime for LocalTimer {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        write!(w, "{}", Local::now().format("[%Y-%m-%d %H:%M:%S]"))
    }
}

fn init_logger() {
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
#[clap(version, about, long_about = None)]
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

    /// config file path (default: gitv.yaml)
    path: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_logger();

    let cli: Cli = Cli::parse();
    if !cli.create && !cli.fetch && !cli.render && !cli.shell {
        Cli::command().print_help().unwrap();
        exit(0)
    }

    let c: Config = match config::load_config(&cli.path.unwrap_or("gitv.yaml".to_string())) {
        Err(e) => {
            error!("Load config error: {}", e);
            exit(1);
        }
        Ok(c) => c,
    };

    if cli.create && c.create.is_some() {
        let serializer = CsvSerializer::new(BinaryGitter);
        if let Err(e) = serializer.serialize(c.create.unwrap()).await {
            error!("Create database error: {}", e);
            exit(1);
        };
        exit(0)
    }

    if cli.fetch && c.fetch.is_some() {
        let repo_fetcher = RepoFetcher::new(c.fetch.unwrap());
        if let Err(e) = repo_fetcher.fetch().await {
            error!("Fetch repos error: {}", e);
            exit(1);
        };
        exit(0)
    }

    if cli.shell && c.shell.is_some() {
        let ctx = Executor::create_context(c.shell.unwrap().executions).await;
        let ctx = match ctx {
            Err(e) => {
                error!("Create executor context error: {}", e);
                exit(1)
            }
            Ok(ctx) => ctx,
        };

        if let Err(e) = shell::console_loop(ctx).await {
            error!("Shell console loop error: {}", e);
            exit(1);
        };
        exit(0)
    }

    if cli.render && c.render.is_some() {
        let render_config = c.render.unwrap();
        let executions = render_config.executions.clone();
        let ctx = match Executor::create_context(executions).await {
            Err(e) => {
                error!("Create executor context error: {}", e);
                exit(1)
            }
            Ok(ctx) => ctx,
        };

        if let Err(e) = render::create_render(ctx, render_config).render().await {
            error!("Render output error: {}", e);
            exit(1);
        }
        exit(0)
    }

    Ok(())
}
