use crate::config;
use crate::git_impl::{RepoSourcer, Repository};
use anyhow::Result;
use async_trait::*;
use serde::Deserialize;
use std::path::Path;

// `github` subcommand
// args:
// 1) -v --visibility
// 2) -a --affiliation
// 3) -i --include-org
// 4) -p --path

pub struct GithubSourcer<'a> {
    api: &'static str,
    pub opts: &'a config::Github,
}

impl<'a> GithubSourcer<'a> {
    pub fn new(opt: &'a config::Github) -> Self {
        Self {
            api: "https://api.github.com/user/repos",
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
impl<'a> RepoSourcer for GithubSourcer<'a> {
    async fn repositories(&self) -> Result<Vec<Repository>> {
        let mut finish = false;
        let mut page: u16 = 1;
        let mut repos = vec![];
        while !finish {
            let params = [
                ("per_page", "100"),
                ("page", &page.to_string()),
                ("visibility", &self.opts.visibility),
                ("affiliation", &self.opts.affiliation),
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

            let orgs: Vec<&str> = self.opts.exclude_org.split(',').map(|x| x.trim()).collect();
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
                        branches: None,
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
