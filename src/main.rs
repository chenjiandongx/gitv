mod charts;
mod config;
mod gitter;
mod gitter_binary;
mod query;
mod record;
mod record_csv;
mod register_functions;
mod repo_github;
mod repo_syncer;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use config::*;
use datafusion::prelude::*;
use gitter::*;
use gitter_binary::*;
use record::*;
use register_functions::*;
use serde::{Deserialize, Serialize};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// init database
    Init(InitArgs),

    /// sync repos
    Sync(SyncArgs),

    // render something
    Render(RenderArgs),
}

#[derive(Args)]
struct InitArgs {
    path: Option<String>,
}

#[derive(Args)]
struct SyncArgs {
    path: Option<String>,
}

#[derive(Args)]
struct RenderArgs {
    path: Option<String>,
}

fn main() {
    let cli = Cli::parse();
    println!("{}", "hello world");
}
