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
use std::{fs::File, io::Write, process::exit};

#[derive(Debug, Parser)]
#[clap(about = "\nA git repos analyzing and visualizing tool built in Rust.")]
#[clap(version)]
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

    /// Generate the example config file (default: gitv.example.yaml)
    #[clap(short, long)]
    gernerate: bool,

    /// config file path (default: gitv.yaml)
    path: Option<String>,
}

static DEFAULT_CONFIG: &str = include_str!("../static/gitv.example.yaml");

#[tokio::main]
async fn main() -> Result<()> {
    let cli: Cli = Cli::parse();
    if !cli.create && !cli.fetch && !cli.render && !cli.shell && !cli.gernerate {
        Cli::command().print_help().unwrap();
        exit(0)
    }

    if cli.gernerate {
        let p = &cli.path.unwrap_or_else(|| "gitv.example.yaml".to_string());
        let mut f = match File::create(p) {
            Err(e) => {
                println!("Create config file error: {}", e);
                exit(1)
            }
            Ok(f) => f,
        };

        if let Err(e) = f.write_all(DEFAULT_CONFIG.as_bytes()) {
            println!("Write config file error: {}", e);
            exit(1)
        }
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
