mod config;
mod executor;
mod fetcher;
mod gitimp;
mod record;
mod render;
mod shell;

use anyhow::Result;
use clap::{IntoApp, Parser};
use config::*;
use executor::*;
use fetcher::*;
use gitimp::*;
use record::*;
use std::process::exit;

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
    let cli: Cli = Cli::parse();
    if !cli.create && !cli.fetch && !cli.render && !cli.shell {
        Cli::command().print_help().unwrap();
        exit(0)
    }

    let c: Config = match config::load_config(&cli.path.unwrap_or_else(|| "gitv.yaml".to_string()))
    {
        Err(e) => {
            println!("Load config error: {}", e);
            exit(1);
        }
        Ok(c) => c,
    };

    if cli.create && c.create.is_some() {
        if let Err(e) = CsvSerializer::serialize(c.create.unwrap()).await {
            println!("Create database error: {}", e);
            exit(1);
        };
        exit(0)
    }

    if cli.fetch && c.fetch.is_some() {
        let repo_fetcher = RepoFetcher::new(c.fetch.unwrap());
        if let Err(e) = repo_fetcher.fetch().await {
            println!("Fetch repos error: {}", e);
            exit(1);
        };
        exit(0)
    }

    if cli.shell && c.shell.is_some() {
        let ctx = Executor::create_context(c.shell.unwrap().executions).await;
        let ctx = match ctx {
            Err(e) => {
                println!("Create executor context error: {}", e);
                exit(1)
            }
            Ok(ctx) => ctx,
        };

        if let Err(e) = shell::console_loop(ctx).await {
            println!("Shell console loop error: {}", e);
            exit(1);
        };
        exit(0)
    }

    if cli.render && c.render.is_some() {
        let render_config = c.render.unwrap();
        let executions = render_config.executions.clone();
        let ctx = match Executor::create_context(executions).await {
            Err(e) => {
                println!("Create executor context error: {}", e);
                exit(1)
            }
            Ok(ctx) => ctx,
        };

        if let Err(e) = render::create_render(ctx, render_config).render().await {
            println!("Render output error: {}", e);
            exit(1);
        }
        exit(0)
    }

    Ok(())
}
