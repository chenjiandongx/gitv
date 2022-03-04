use crate::repo_syncer::RepoSyncer;
use crate::{config, Repository};
use anyhow::Result;
use async_trait::*;
use serde::Deserialize;
use std::path::Path;

pub struct GithubRepoSyncer<'a> {
    api: &'static str,
    pub opts: &'a config::Github,
}

const GITHUB_API: &str = "https://api.github.com/user/repos";

impl<'a> GithubRepoSyncer<'a> {
    pub fn new(opt: &'a config::Github) -> Self {
        Self {
            api: GITHUB_API,
            opts: opt,
        }
    }
}

#[derive(Debug, Deserialize)]
struct RepoResponse {
    full_name: String,
    clone_url: String,
}

#[async_trait]
impl<'a> RepoSyncer for GithubRepoSyncer<'a> {
    async fn repositories(&self) -> Result<Vec<Repository>> {
        let mut finish = false;
        let mut page: u16 = 1;
        let mut repos = vec![];
        let visibility = self.opts.visibility.clone().unwrap_or_default();
        let affiliation = self.opts.affiliation.clone().unwrap_or_default();
        while !finish {
            let params = [
                ("per_page", "100"),
                ("page", &page.to_string()),
                ("visibility", visibility.as_str()),
                ("affiliation", affiliation.as_str()),
            ];

            let response = reqwest::Client::new()
                .get(self.api)
                .query(&params)
                .bearer_auth(&self.opts.token)
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

            let orgs = match &self.opts.exclude_org {
                Some(orgs) => orgs.split(',').map(|x| x.trim()).collect(),
                None => vec![],
            };
            for repo in response {
                let mut ignore = false;
                for org in orgs.iter() {
                    if repo.full_name.starts_with(org) {
                        ignore = true;
                        break;
                    }
                }
                if !ignore {
                    let name = repo.full_name;
                    repos.push(Repository {
                        name: name.clone(),
                        branch: None,
                        remote: repo.clone_url,
                        path: Path::new(&self.opts.path.clone())
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
