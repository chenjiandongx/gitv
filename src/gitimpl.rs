use anyhow::{Error, Result};
use async_trait::async_trait;
use lazy_static::lazy_static;
use std::collections::HashMap;
use std::path::Path;
use std::sync;
use std::sync::Arc;
use async_process::Command;
use serde::{Deserialize};

lazy_static! {
    static ref COMMIT_INFO_REGEXP: regex::Regex =
        regex::Regex::new(r"<(.*?)> <(.*)> <(.*)> <(.*?)>").unwrap();
    static ref COMMIT_CHANGE_REGEXP: regex::Regex =
        regex::Regex::new(r"([0-9-]+)\t([0-9-]+)\t(.*)").unwrap();
}

#[derive(Debug, Clone, Default)]
pub struct Author {
    pub name: String,
    pub email: String,
    pub domain: String,
}

#[derive(Debug, Clone, Default)]
pub struct FileExtChange {
    pub ext: String,
    pub insertion: i64,
    pub deletion: i64,
}

#[derive(Debug, Clone, Default)]
pub struct Commit {
    pub repo: String,
    pub hash: String,
    pub author: Author,
    pub timestamp: i64,
    pub timezone: String,
    pub change_files: i64,
    pub changes: Vec<FileExtChange>,
}

impl TryFrom<&[String]> for Commit {
    type Error = anyhow::Error;

    fn try_from(lines: &[String]) -> Result<Self> {
        let mut changes: HashMap<String, FileExtChange> = HashMap::new();
        let mut count: i64 = 0;

        let mut commit = Commit::default();
        for (idx, line) in lines.iter().enumerate() {
            if idx == 0 {
                let caps = COMMIT_INFO_REGEXP.captures(line.as_str());
                if caps.is_none() {
                    return Err(Error::msg(format!("invalid commit format: {}", line)));
                };

                let caps = caps.unwrap();
                for i in 0..caps.len() {
                    match i {
                        1 => {
                            let fields = caps
                                .get(i)
                                .unwrap()
                                .as_str()
                                .split_ascii_whitespace()
                                .collect::<Vec<&str>>();
                            if fields.len() == 2 {
                                commit.timestamp = fields[0].parse::<i64>()?;
                                commit.timezone = fields[1].to_string();
                            }
                        }
                        2 => {
                            commit.hash = caps.get(i).unwrap().as_str().to_string();
                        }
                        3 => {
                            commit.author.name = caps.get(i).unwrap().as_str().to_string();
                        }
                        4 => {
                            commit.author.email = caps.get(i).unwrap().as_str().to_string();
                            let email = commit.author.email.clone();
                            let fields = email.splitn(2, '@').collect::<Vec<&str>>();
                            if fields.len() >= 2 {
                                commit.author.domain = fields.get(1).unwrap().to_string();
                            }
                        }
                        _ => (),
                    }
                }
                continue;
            }

            count += 1;
            let mut change = FileExtChange::default();
            let caps = COMMIT_CHANGE_REGEXP.captures(line.as_str());
            if caps.is_none() {
                return Err(Error::msg(format!("invalid change format: {}", line)));
            }

            let caps = caps.unwrap();
            for i in 0..caps.len() {
                match i {
                    1 => {
                        change.insertion =
                            caps.get(i).unwrap().as_str().parse::<i64>().unwrap_or(0);
                    }
                    2 => {
                        change.deletion = caps.get(i).unwrap().as_str().parse::<i64>().unwrap_or(0);
                    }
                    3 => {
                        let p = Path::new(caps.get(i).unwrap().as_str());
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
        Ok(commit)
    }
}

#[derive(Debug, Clone, Default)]
pub struct TagStats {
    pub hash: String,
    pub tag: String,
    pub timestamp: i64,
    pub timezone: String,
    pub stats: FileExtStats,
}

#[derive(Debug, Clone, Default)]
pub struct FileExtStat {
    pub ext: String,
    pub size: i64,
    pub files: i64,
}

#[derive(Debug, Clone, Default)]
pub struct FileExtStats {
    stats: Vec<FileExtStat>,
}

impl TryFrom<&[String]> for FileExtStats {
    type Error = anyhow::Error;

    fn try_from(lines: &[String]) -> Result<Self> {
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

        let mut fs = FileExtStats::default();
        for (k, v) in stats {
            fs.stats.push(FileExtStat {
                ext: k,
                size: v.size,
                files: v.files,
            })
        }
        Ok(fs)
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Repository {
    pub name: String,
    pub branches: Option<Vec<String>>,
    pub remote: String,
    pub path: String,
}

#[async_trait]
pub trait GitImpl: Send + Sync {
    async fn clone(&self, repo: &Repository) -> Result<()>;
    async fn commits(&self, repo: &Repository) -> Result<Vec<Commit>>;
    async fn tags(&self, repo: &Repository) -> Result<Vec<TagStats>>;
    async fn current_branch(&self, repo: &Repository) -> Result<String>;
}

pub struct GitBinary;

impl GitBinary {
    async fn sub_command(&self, repo: &Repository, command: &str, args: &[&str], delimiter: char) -> Result<Vec<String>> {
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

    async fn git_branch(&self, repo: &Repository, args: &[&str]) -> Result<Vec<String>> {
        self.sub_command(repo, "branch", args, '\n').await
    }

    async fn git_log(&self, repo: &Repository, args: &[&str]) -> Result<Vec<String>> {
        self.sub_command(repo, "log", args, '\n').await
    }

    async fn git_show_ref(&self, repo: &Repository, args: &[&str]) -> Result<Vec<String>> {
        self.sub_command(repo, "show-ref", args, '\n').await
    }

    async fn git_ls_tree(&self, repo: &Repository, args: &[&str]) -> Result<Vec<String>> {
        self.sub_command(repo, "ls-tree", args, '\u{0}').await
    }

    async fn git_clone(&self, repo: &Repository) -> Result<()> {
        let mut c = Command::new("git");
        c.args(&["clone", repo.remote.as_str(), repo.path.as_str()]);
        c.output().await?;
        Ok(())
    }
}

struct GitBinaryImpl;

#[async_trait]
impl<'a> GitImpl for GitBinary {
    async fn clone(&self, repo: &Repository) -> Result<()> {
        self.git_clone(repo).await
    }

    // TODO(optimize): 按时间切割 并发执行
    // https://stackoverflow.com/questions/11856983/why-git-authordate-is-different-from-commitdate
    // https://stackoverflow.com/questions/37311494/how-to-get-git-to-show-commits-in-a-specified-date-range-for-author-date
    async fn commits(&self, repo: &Repository) -> Result<Vec<Commit>> {
        let lines = self.git_log(repo, &[
            "--no-merges",
            "--date=raw",
            "--pretty=format:<%ad> <%H> <%aN> <%aE>",
            "--numstat",
            "HEAD",
        ]).await?;

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
            if let Ok(mut commit) = Commit::try_from(&lines[l..r]) {
                commit.repo = repo.name.to_string();
                commits.push(commit);
            }
        }

        Ok(commits)
    }

    // TODO(optimize): 并发执行
    async fn tags(&self, repo: &Repository) -> Result<Vec<TagStats>> {
        let lines = self.git_show_ref(repo, &["--tags"]).await?;
        let r: Vec<TagStats> = vec![];
        let mut mutex = Arc::new(tokio::sync::Mutex::new(r));
        for line in lines {
            let fields: Vec<&str> = line.splitn(2, ' ').collect();
            if fields.len() < 2 {
                continue;
            }

            let lock = mutex.clone();
            tokio::spawn(async move {
                let (hash, tag) = (fields[0], &fields[1]["refs/tags/".len()..]);
                let lines = self.git_ls_tree(repo, &["-r", "-l", "-z", hash]).await.unwrap();
                let file_ext_stats = FileExtStats::try_from(lines.as_slice()).unwrap();

                let log = self.git_log(repo, &[
                    "--date=raw",
                    "--pretty=format:<%ad> <%H> <%aN> <%aE>",
                    "--numstat",
                    hash,
                    "-1",
                ]).await.unwrap();

                let mut tag_stat = TagStats {
                    stats: file_ext_stats,
                    ..Default::default()
                };
                if let Ok(commit) = Commit::try_from(&log[..]) {
                    tag_stat.tag = tag.to_string();
                    tag_stat.hash = hash.to_string();
                    tag_stat.timestamp = commit.timestamp;
                    tag_stat.timezone = commit.timezone;
                }
                let mut stats = lock.lock().await;
                // let mut stats = lock.lock().await;
                // let mut stats = stats.unwrap();
                stats.push(tag_stat);
            });
        }
        Ok(vec![])
    }

    async fn current_branch(&self, repo: &Repository) -> Result<String> {
        let lines = self.git_branch(repo, &["--show-current"]).await?;
        let branch = if !lines.is_empty() {
            Ok(lines.first().unwrap().clone())
        } else {
            Err(Error::msg("unknown curren branch"))
        };

        branch
    }
}
