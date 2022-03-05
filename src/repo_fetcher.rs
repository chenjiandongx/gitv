use crate::config;
use crate::repo_github::GithubRepoFetcher;
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
        for opt in self.opts.github.clone().unwrap_or_default() {
            let handle = tokio::spawn(async move {
                let repos = GithubRepoFetcher::repositories(&opt).await.unwrap();
                let f = File::create(&opt.output).unwrap();
                serde_yaml::to_writer(f, &repos).unwrap();
                info!("save database file '{}'", &opt.output);
            });
            handles.push(handle);
        }
        futures::future::join_all(handles).await;

        info!(
            "all github repositories have been fetched, elapsed: {:#?}",
            now.elapsed()
        );

        Ok(())
    }
}
