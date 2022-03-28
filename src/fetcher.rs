use crate::{config, Repository};
use anyhow::Result;
use serde::Deserialize;
use std::{fs::File, path::Path};
use tokio::{task::JoinHandle, time};

#[derive(Debug, Clone)]
enum GithubConfig {
    Authenticated(config::GithubAuthenticated),
    User(config::GithubUser),
    Org(config::GithubOrg),
}

impl GithubConfig {
    fn destination(&self) -> String {
        match self {
            GithubConfig::Authenticated(c) => c.destination.clone(),
            GithubConfig::User(c) => c.destination.clone(),
            GithubConfig::Org(c) => c.destination.clone(),
        }
    }
}

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

    async fn fetch_github(&self) -> Result<()> {
        println!("start to fetch github repos...");
        let now = time::Instant::now();

        let mut configs = vec![];
        for config in self.opts.github_authenticated.clone().unwrap_or_default() {
            configs.push(GithubConfig::Authenticated(config));
        }
        for config in self.opts.github_user.clone().unwrap_or_default() {
            configs.push(GithubConfig::User(config));
        }
        for config in self.opts.github_org.clone().unwrap_or_default() {
            configs.push(GithubConfig::Org(config));
        }

        let mut handles: Vec<JoinHandle<Result<(), anyhow::Error>>> = vec![];
        for config in configs {
            let config = config.clone();
            let handle = tokio::spawn(async move {
                let repos = match config {
                    GithubConfig::Authenticated(ref config) => {
                        GithubRepoFetcher::authenticated_repos(config).await?
                    }
                    GithubConfig::User(ref config) => GithubRepoFetcher::user_repos(config).await?,
                    GithubConfig::Org(ref config) => GithubRepoFetcher::org_repos(config).await?,
                };

                let f = File::create(&config.destination())?;
                serde_yaml::to_writer(f, &repos)?;
                println!("save database file '{}'", &config.destination());
                Ok(())
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await??;
        }

        println!(
            "[github]: all repos have been fetched, elapsed: {:#?}",
            now.elapsed()
        );
        Ok(())
    }
}

/// Github Fetcher 实现
struct GithubRepoFetcher;

enum GithubApi {
    Authenticated,
    User,
    Org,
}

impl GithubApi {
    fn url(&self, s: &str) -> String {
        match self {
            GithubApi::Authenticated => String::from("https://api.github.com/user/repos"),
            GithubApi::User => format!("https://api.github.com/users/{}/repos", s),
            GithubApi::Org => format!("https://api.github.com/orgs/{}/repos", s),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
struct GithubRepoResponse {
    full_name: String,
    clone_url: String,
    default_branch: String,
    forks_count: usize,
    stargazers_count: usize,
}

impl GithubRepoFetcher {
    fn exclude_orgs_filter(exclude_orgs: &[String], repo: &Repository) -> bool {
        for excluded in exclude_orgs.iter() {
            if repo.name.starts_with(excluded) {
                println!("[excludeOrgs] skip repo '{}' ", repo.name);
                return true;
            }
        }
        false
    }

    fn exclude_repos_filter(exclude_repos: &[String], repo: &Repository) -> bool {
        for excluded in exclude_repos.iter() {
            if repo.name.starts_with(excluded) {
                println!("[excludeRepos] skip repo '{}' ", repo.name);
                return true;
            }
        }
        false
    }

    async fn authenticated_repos(config: &config::GithubAuthenticated) -> Result<Vec<Repository>> {
        let visibility = config.visibility.clone();
        let affiliation = config.affiliation.clone();
        let params = vec![
            ("visibility", visibility.unwrap_or_default()),
            ("affiliation", affiliation.unwrap_or_default()),
        ];
        let api = GithubApi::Authenticated;

        let repos = Self::repositories(&config.clone_dir, params, &api.url(""), &config.token)
            .await?
            .into_iter()
            .filter(|repo| {
                !(Self::exclude_orgs_filter(&config.clone().exclude_orgs.unwrap_or_default(), repo)
                    || Self::exclude_repos_filter(
                        &config.clone().exclude_repos.unwrap_or_default(),
                        repo,
                    ))
            })
            .collect::<Vec<_>>();

        Ok(repos)
    }

    async fn org_repos(config: &config::GithubOrg) -> Result<Vec<Repository>> {
        let params = vec![("type", config.typ.clone())];
        let api = GithubApi::Org;

        let repos = Self::repositories(
            &config.clone_dir,
            params,
            &api.url(&config.org),
            &config.token,
        )
        .await?
        .into_iter()
        .filter(|repo| {
            !Self::exclude_repos_filter(&config.clone().exclude_repos.unwrap_or_default(), repo)
        })
        .collect::<Vec<_>>();

        Ok(repos)
    }

    async fn user_repos(config: &config::GithubUser) -> Result<Vec<Repository>> {
        let params = vec![("type", config.typ.clone())];
        let api = GithubApi::User;

        let repos = Self::repositories(
            &config.clone_dir,
            params,
            &api.url(&config.username),
            &config.token,
        )
        .await?
        .into_iter()
        .filter(|repo| {
            !Self::exclude_repos_filter(&config.clone().exclude_repos.unwrap_or_default(), repo)
        })
        .collect::<Vec<_>>();

        Ok(repos)
    }

    async fn repositories(
        clone_dir: &str,
        params: Vec<(&str, String)>,
        url: &str,
        token: &str,
    ) -> Result<Vec<Repository>> {
        let mut finish = false;
        let mut page: u16 = 1;
        let mut repos = vec![];

        while !finish {
            println!("fetching github repos page: {}", page);
            let mut params = params.clone();
            params.push(("per_page", "100".to_string()));
            params.push(("page", page.to_string()));

            let response = reqwest::Client::new()
                .get(url)
                .query(&params)
                .bearer_auth(token)
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

            for repo in response {
                let name = repo.full_name;
                repos.push(Repository {
                    name: name.clone(),
                    branch: Some(repo.default_branch),
                    remote: Some(repo.clone_url),
                    path: Path::new(clone_dir)
                        .join(Path::new(&name))
                        .to_str()
                        .unwrap()
                        .to_string(),
                    forks_count: Some(repo.forks_count),
                    stargazers_count: Some(repo.stargazers_count),
                });
            }
        }

        println!("[github]: fetch total {} repos", repos.len());
        Ok(repos)
    }
}
