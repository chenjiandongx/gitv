use crate::{config, Repository};
use anyhow::Result;
use serde::Deserialize;
use std::path::Path;

pub(crate) struct GithubRepoFetcher;

static GITHUB_API: &str = "https://api.github.com/user/repos";

#[derive(Debug, Deserialize)]
struct RepoResponse {
    full_name: String,
    clone_url: String,
    default_branch: String,
}

impl GithubRepoFetcher {
    pub async fn repositories(opts: &config::Github) -> Result<Vec<Repository>> {
        let mut finish = false;
        let mut page: u16 = 1;
        let mut repos = vec![];
        let visibility = opts.visibility.clone().unwrap_or_default();
        let affiliation = opts.affiliation.clone().unwrap_or_default();

        while !finish {
            let params = [
                ("per_page", "100"),
                ("page", &page.to_string()),
                ("visibility", visibility.as_str()),
                ("affiliation", affiliation.as_str()),
            ];

            let response = reqwest::Client::new()
                .get(GITHUB_API)
                .query(&params)
                .bearer_auth(&opts.token)
                .header("User-Agent", "rust/reqwest")
                .header("Accept", "application/vnd.github.v3+json")
                .send()
                .await?
                .json::<Vec<RepoResponse>>()
                .await?;

            page += 1;
            if response.len() < 100 {
                finish = true
            }

            let exclude_orgs = opts.exclude_orgs.clone().unwrap_or_default();
            let exclude_repos = opts.exclude_repos.clone().unwrap_or_default();
            for repo in response {
                let mut ignore = false;
                for excluded in exclude_orgs.iter() {
                    if repo.full_name.starts_with(excluded) {
                        ignore = true;
                        break;
                    }
                }
                for excluded in exclude_repos.iter() {
                    if repo.full_name.starts_with(excluded) {
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
                        path: Path::new(&opts.base_dir.clone())
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
