use crate::{config, Repository};
use anyhow::Result;
use serde::Deserialize;
use std::process::exit;
use std::{fs::File, path::Path};
use tokio::time;
use tracing::{error, info};

/// 从不同数据源拉取 Repository 并写入本地磁盘
///
/// Fetcher Source: 目前只支持 Github
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

    /// 拉取 Github 仓库数据源
    async fn fetch_github(&self) -> Result<()> {
        info!("start to fetch github repos...");
        let now = time::Instant::now();
        let mut handles = vec![];

        for config in self.opts.github.clone().unwrap_or_default() {
            let handle = tokio::spawn(async move {
                let repos = GithubRepoFetcher::repositories(&config).await;
                let repos = match repos {
                    Err(e) => {
                        error!("failed to fetch github repositories, err: {}", e);
                        exit(1)
                    }
                    Ok(repos) => repos,
                };

                let f = File::create(&config.destination);
                let f = match f {
                    Err(e) => {
                        error!(
                            "failed to create file '{}', err: {}",
                            &config.destination, e
                        );
                        exit(1)
                    }
                    Ok(f) => f,
                };
                serde_yaml::to_writer(f, &repos).unwrap();
                info!("save database file '{}'", &config.destination);
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await?;
        }

        info!(
            "all github repositories have been fetched, elapsed: {:#?}",
            now.elapsed()
        );
        Ok(())
    }
}

/// Github Fetcher 实现
struct GithubRepoFetcher;

static GITHUB_API: &str = "https://api.github.com/user/repos";

#[derive(Debug, Deserialize)]
struct GithubRepoResponse {
    full_name: String,
    clone_url: String,
    default_branch: String,
}

impl GithubRepoFetcher {
    pub async fn repositories(config: &config::Github) -> Result<Vec<Repository>> {
        let mut finish = false;
        let mut page: u16 = 1;
        let mut repos = vec![];
        let visibility = config.visibility.clone().unwrap_or_default();
        let affiliation = config.affiliation.clone().unwrap_or_default();

        while !finish {
            let params = [
                ("per_page", "100"),
                ("page", &page.to_string()),
                ("visibility", visibility.as_str()),
                ("affiliation", affiliation.as_str()),
            ];

            info!("fetching github repos page: {}", page);
            let response = reqwest::Client::new()
                .get(GITHUB_API)
                .query(&params)
                .bearer_auth(&config.token)
                .header("User-Agent", "rust/reqwest")
                .header("Accept", "application/vnd.github.v3+json")
                .send()
                .await?
                .json::<Vec<GithubRepoResponse>>()
                .await?;

            page += 1;
            if response.len() < 100 {
                finish = true
            }

            let exclude_orgs = config.exclude_orgs.clone().unwrap_or_default();
            let exclude_repos = config.exclude_repos.clone().unwrap_or_default();
            for repo in response {
                let mut ignore = false;
                for excluded in exclude_orgs.iter() {
                    if repo.full_name.starts_with(excluded) {
                        info!("[exclude_orgs] skip repo '{}' ", repo.full_name);
                        ignore = true;
                        break;
                    }
                }
                for excluded in exclude_repos.iter() {
                    if repo.full_name.starts_with(excluded) {
                        info!("[exclude_repos] skip repo '{}' ", repo.full_name);
                        ignore = true;
                        break;
                    }
                }

                if !ignore {
                    let name = repo.full_name;
                    repos.push(Repository {
                        name: name.clone(),
                        branch: Some(repo.default_branch),
                        remote: repo.clone_url,
                        path: Path::new(&config.base_dir.clone())
                            .join(Path::new(&name))
                            .to_str()
                            .unwrap()
                            .to_string(),
                    });
                }
            }
        }
        Ok(repos)
    }
}
