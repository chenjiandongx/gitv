use crate::{config, repo_github::GithubRepoFetcher};
use anyhow::Result;
use std::fs::File;
use tokio::time;
use tracing::info;

pub struct RepoFetcher {
    opts: config::FetchAction,
}

impl RepoFetcher {
    pub fn new(opts: config::FetchAction) -> Self {
        Self { opts }
    }

    pub async fn fetch(&self) -> Result<()> {
        self.fetch_github().await
    }

    async fn fetch_github(&self) -> Result<()> {
        info!("start to fetch github repos...");
        let now = time::Instant::now();
        let mut handles = vec![];
        for config in self.opts.github.clone().unwrap_or_default() {
            let handle = tokio::spawn(async move {
                let repos = GithubRepoFetcher::repositories(&config).await.unwrap();
                let f = File::create(&config.output).unwrap();
                serde_yaml::to_writer(f, &repos).unwrap();
                info!("save database file '{}'", &config.output);
            });
            handles.push(handle);
        }
        for handle in handles {
            handle.await.unwrap();
        }

        info!(
            "all github repositories have been fetched, elapsed: {:#?}",
            now.elapsed()
        );

        Ok(())
    }
}
