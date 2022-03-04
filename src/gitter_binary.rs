use crate::config::AuthorMapping;
use crate::gitter::*;
use crate::Repository;
use anyhow::{Error, Result};
use async_process::Command;
use async_trait::async_trait;
use lazy_static::lazy_static;
use std::sync::Arc;
use std::{collections::HashMap, fs, path::Path};

lazy_static! {
    static ref COMMIT_INFO_REGEXP: regex::Regex =
        regex::Regex::new(r"<(.*?)> <(.*)> <(.*)> <(.*?)>").unwrap();
    static ref COMMIT_CHANGE_REGEXP: regex::Regex =
        regex::Regex::new(r"([0-9-]+)\t([0-9-]+)\t(.*)").unwrap();
}

struct GitExecutable;

impl GitExecutable {
    async fn git(
        repo: &Repository,
        command: &str,
        args: &[&str],
        delimiter: char,
    ) -> Result<Vec<String>> {
        let mut args = args.to_vec();
        args.insert(0, command);

        let mut c = Command::new("git");
        c.args(&[
            format!("--git-dir={}/.git", repo.path),
            format!("--work-tree={}", repo.path),
        ]);
        c.args(args);

        let out = c.output().await?.stdout;
        let lines = String::from_utf8(out)?
            .split(delimiter)
            .filter(|x| !x.is_empty())
            .map(|x| x.to_string())
            .collect();

        Ok(lines)
    }

    async fn git_clone(repo: &Repository) -> Result<()> {
        if let Some(p) = Path::new(&repo.path).parent() {
            fs::create_dir_all(p)?
        }

        let mut c = Command::new("git");
        c.args(&["clone", repo.remote.as_str(), repo.path.as_str()])
            .output()
            .await?;
        Ok(())
    }

    async fn git_pull(repo: &Repository) -> Result<()> {
        let args = vec![];
        let lines = Self::git(repo, "pull", &args, '\n').await;
        if lines.is_err() {
            Err(lines.err().unwrap())
        } else {
            Ok(())
        }
    }

    async fn git_branch(repo: &Repository, args: &[&str]) -> Result<Vec<String>> {
        Self::git(repo, "branch", args, '\n').await
    }

    async fn git_log(repo: &Repository, args: &[&str]) -> Result<Vec<String>> {
        Self::git(repo, "log", args, '\n').await
    }

    async fn git_show_ref(repo: &Repository, args: &[&str]) -> Result<Vec<String>> {
        Self::git(repo, "show-ref", args, '\n').await
    }

    async fn git_ls_tree(repo: &Repository, args: &[&str]) -> Result<Vec<String>> {
        Self::git(repo, "ls-tree", args, '\u{0}').await
    }
}

struct Parse;

impl Parse {
    fn parse_commit_info(
        commit: &mut Commit,
        line: &str,
        author_mappings: Vec<AuthorMapping>,
    ) -> Result<()> {
        let caps = COMMIT_INFO_REGEXP.captures(line);
        if caps.is_none() {
            return Err(Error::msg(format!("invalid commit format: {}", line)));
        };

        let caps = caps.unwrap();
        for i in 0..caps.len() {
            let cap = caps.get(i).unwrap().as_str().to_string();
            match i {
                1 => {
                    commit.datetime = cap;
                }
                2 => {
                    commit.hash = cap;
                }
                3 => {
                    commit.author.name = cap;
                }
                4 => {
                    commit.author.email = cap;
                }
                _ => (),
            }
        }

        for author_mapping in author_mappings.iter() {
            if commit.author == author_mapping.source {
                commit.author = author_mapping.destination.clone();
                break;
            }
        }
        Ok(())
    }

    fn parse_commit_changes(commit: &mut Commit, lines: &[String]) -> Result<()> {
        let mut count = 0;
        let mut changes: HashMap<String, FileExtChange> = HashMap::new();

        for line in lines.iter() {
            count += 1;
            let mut change = FileExtChange::default();
            let caps = COMMIT_CHANGE_REGEXP.captures(line.as_str());
            if caps.is_none() {
                return Err(Error::msg(format!("invalid change format: {}", line)));
            }

            let caps = caps.unwrap();
            for i in 0..caps.len() {
                let cap = caps.get(i).unwrap().as_str();
                match i {
                    1 => {
                        change.insertion = cap.parse::<i64>().unwrap_or(0);
                    }
                    2 => {
                        change.deletion = cap.parse::<i64>().unwrap_or(0);
                    }
                    3 => {
                        let p = Path::new(cap);
                        if p.extension().is_some() {
                            change.ext = p.extension().unwrap().to_str().unwrap().to_string();
                        } else {
                            change.ext = "".to_string();
                        }
                    }
                    _ => (),
                }
            }

            let c = changes.entry(change.ext.clone()).or_insert(FileExtChange {
                ext: change.ext,
                ..Default::default()
            });
            c.insertion += change.insertion;
            c.deletion += change.deletion;
        }

        let mut cs = vec![];
        for c in changes {
            cs.push(c.to_owned().1);
        }
        commit.changes = cs;
        commit.change_files = count;
        Ok(())
    }

    fn parse_commit(lines: &[String], author_mappings: Vec<AuthorMapping>) -> Result<Commit> {
        let mut commit = Commit::default();
        Self::parse_commit_info(&mut commit, &lines[0], author_mappings)?;
        Self::parse_commit_changes(&mut commit, &lines[1..])?;
        Ok(commit)
    }

    fn parse_file_ext_stats(lines: &[String]) -> Result<Vec<FileExtStat>> {
        let mut stats: HashMap<String, FileExtStat> = HashMap::new();

        for line in lines {
            let fields: Vec<&str> = line.split_ascii_whitespace().collect();
            if fields.len() < 5 {
                continue;
            }
            if fields[0] == "106000" {
                continue;
            }

            let n = fields[3].parse::<i64>().unwrap_or(0);
            let p = Path::new(fields[4]);
            if p.extension().is_none() {
                continue;
            }
            let ext = p.extension().unwrap().to_str().unwrap().to_string();
            let s = stats
                .entry(ext.clone())
                .or_insert_with(FileExtStat::default);
            s.size += n;
            s.files += 1;
        }

        let mut fes = vec![];
        for (k, v) in stats {
            fes.push(FileExtStat {
                ext: k,
                size: v.size,
                files: v.files,
            })
        }
        Ok(fes)
    }
}

#[derive(Copy, Clone)]
pub struct GitBinaryImpl;

#[async_trait]
impl Gitter for GitBinaryImpl {
    async fn clone_or_pull(&self, repos: Vec<Repository>) -> Result<()> {
        let mut handles = vec![];
        for repo in repos {
            let repo = repo.clone();
            let handle = tokio::spawn(async move {
                if Path::new(&repo.path).exists() {
                    GitExecutable::git_pull(&repo).await
                } else {
                    GitExecutable::git_clone(&repo).await
                }
            });
            handles.push(handle);
        }
        futures::future::join_all(handles).await;
        Ok(())
    }

    // TODO(optimize): 按时间切割 并发执行
    // https://stackoverflow.com/questions/11856983/why-git-authordate-is-different-from-commitdate
    // https://stackoverflow.com/questions/37311494/how-to-get-git-to-show-commits-in-a-specified-date-range-for-author-date
    async fn commits(
        &self,
        repo: &Repository,
        author_mappings: Vec<AuthorMapping>,
    ) -> Result<Vec<Commit>> {
        let lines = GitExecutable::git_log(
            repo,
            &[
                "--no-merges",
                "--date=rfc",
                "--pretty=format:<%ad> <%H> <%aN> <%aE>",
                "--numstat",
                "HEAD",
            ],
        )
        .await?;

        let mut indexes = vec![];
        for (idx, line) in lines.iter().enumerate() {
            if line.starts_with('<') {
                indexes.push(idx);
            }
        }
        indexes.push(lines.len());

        let mut commits = vec![];
        for i in 1..indexes.len() {
            let (l, r) = (indexes[i - 1], indexes[i]);
            if let Ok(mut commit) = Parse::parse_commit(&lines[l..r], author_mappings.clone()) {
                commit.repo = repo.name.to_string();
                commits.push(commit);
            }
        }

        Ok(commits)
    }

    async fn tags(
        &self,
        repo: &Repository,
        author_mappings: Vec<AuthorMapping>,
    ) -> Result<Vec<TagStats>> {
        let stats: Vec<TagStats> = vec![];
        let mutex = Arc::new(tokio::sync::Mutex::new(stats));
        let lines = GitExecutable::git_show_ref(repo, &["--tags"]).await?;
        let mut handles = vec![];

        for line in lines {
            let fields: Vec<String> = line.splitn(2, ' ').map(|x| x.to_string()).collect();
            if fields.len() < 2 {
                continue;
            }
            let lock = mutex.clone();
            let repo = repo.clone();
            let author_mappings = author_mappings.clone();
            let handle = tokio::spawn(async move {
                let (hash, tag) = (&fields[0], &fields[1]["refs/tags/".len()..]);
                let lines = GitExecutable::git_ls_tree(&repo, &["-r", "-l", "-z", hash])
                    .await
                    .unwrap();
                let file_ext_stats = Parse::parse_file_ext_stats(&lines).unwrap();

                let log = GitExecutable::git_log(
                    &repo,
                    &[
                        "--date=rfc",
                        "--pretty=format:<%ad> <%H> <%aN> <%aE>",
                        "--numstat",
                        hash,
                        "-1",
                    ],
                )
                .await
                .unwrap();

                let mut tag_stat = TagStats {
                    stats: file_ext_stats,
                    ..Default::default()
                };
                if let Ok(commit) = Parse::parse_commit(&log[..], author_mappings) {
                    tag_stat.tag = tag.to_string();
                    tag_stat.hash = hash.to_string();
                    tag_stat.datetime = commit.datetime;
                }
                let mut data = lock.lock().await;
                data.push(tag_stat);
            });
            handles.push(handle)
        }
        futures::future::join_all(handles).await;

        let s = mutex.lock().await;
        Ok(s.to_vec())
    }

    async fn current_branch(&self, repo: &Repository) -> Result<String> {
        let lines = GitExecutable::git_branch(repo, &["--show-current"]).await?;
        let branch = if !lines.is_empty() {
            Ok(lines.first().unwrap().clone())
        } else {
            Err(Error::msg("unknown curren branch"))
        };

        branch
    }
}
