use crate::{config::AuthorMapping, Author, Repository};
use anyhow::{anyhow, Result};
use async_process::Command;
use async_trait::async_trait;
use lazy_static::lazy_static;
use std::{
    collections::HashMap,
    fs,
    path::Path,
    process::exit,
    sync::{Arc, Mutex},
    time,
};
use tracing::{error, info};

/// 文件变更记录
#[derive(Debug, Clone, Default)]
pub struct FileExtChange {
    /// 文件扩展名
    pub ext: String,
    /// 文件改动增加行数
    pub insertion: i64,
    /// 文件改动删除函数
    pub deletion: i64,
}

/// 提交记录
#[derive(Debug, Clone, Default)]
pub struct Commit {
    /// 仓库名称
    pub repo: String,
    /// Commit hash
    pub hash: String,
    /// 提交作者
    pub author: Author,
    /// 提交日期
    pub datetime: String,
    /// 变动文件数
    pub change_files: i64,
    /// 文件变更记录
    pub changes: Vec<FileExtChange>,
}

/// 文件统计
#[derive(Debug, Clone, Default)]
pub struct FileExtStat {
    /// 文件扩展名
    pub ext: String,
    /// 文件体积大小
    pub size: i64,
    /// 文件数量
    pub files: i64,
}

/// Tags 统计数据
#[derive(Debug, Clone, Default)]
pub struct TagStats {
    /// Tag hash
    pub hash: String,
    /// 版本号
    pub tag: String,
    /// 提交时间
    pub datetime: String,
    /// 文件统计
    pub stats: Vec<FileExtStat>,
}

/// Gitter 定义了 `git` 实现接口
#[async_trait]
pub trait Gitter: Send + Sync {
    async fn clone_or_pull(&self, repos: Vec<Repository>) -> Result<()>;
    async fn checkout(&self, repo: &Repository) -> Result<()>;
    async fn commits(
        &self,
        repo: &Repository,
        author_mappings: Vec<AuthorMapping>,
    ) -> Result<Vec<Commit>>;
    async fn tags(
        &self,
        repo: &Repository,
        author_mappings: Vec<AuthorMapping>,
    ) -> Result<Vec<TagStats>>;
}

lazy_static! {
    static ref COMMIT_INFO_REGEXP: regex::Regex =
        regex::Regex::new(r"<(.*?)> <(.*)> <(.*)> <(.*?)>").unwrap();
    static ref COMMIT_CHANGE_REGEXP: regex::Regex =
        regex::Regex::new(r"([0-9-]+)\t([0-9-]+)\t(.*)").unwrap();
}

/// `git` 可执行文件抽象，使用本地的 `git` 命令
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

    async fn git_log(repo: &Repository, args: &[&str]) -> Result<Vec<String>> {
        Self::git(repo, "log", args, '\n').await
    }

    async fn git_show_ref(repo: &Repository, args: &[&str]) -> Result<Vec<String>> {
        Self::git(repo, "show-ref", args, '\n').await
    }

    async fn git_ls_tree(repo: &Repository, args: &[&str]) -> Result<Vec<String>> {
        Self::git(repo, "ls-tree", args, '\u{0}').await
    }

    async fn git_checkout(repo: &Repository, args: &[&str]) -> Result<Vec<String>> {
        Self::git(repo, "checkout", args, '\u{0}').await
    }
}

/// Parser 负责解析 git 命令输出
struct Parser;

impl Parser {
    fn parse_commit_info(
        commit: &mut Commit,
        line: &str,
        author_mappings: Vec<AuthorMapping>,
    ) -> Result<()> {
        let caps = COMMIT_INFO_REGEXP.captures(line);
        if caps.is_none() {
            return Err(anyhow!("Invalid commit format: {}", line));
        };

        let caps = caps.unwrap();
        for i in 0..caps.len() {
            let cap = caps.get(i).unwrap().as_str().to_string();
            match i {
                1 => commit.datetime = cap,
                2 => commit.hash = cap,
                3 => commit.author.name = cap,
                4 => commit.author.email = cap,
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
                return Err(anyhow!("Invalid change format: {}", line));
            }

            let caps = caps.unwrap();
            for i in 0..caps.len() {
                let cap = caps.get(i).unwrap().as_str();
                match i {
                    1 => change.insertion = cap.parse::<i64>().unwrap_or(0),
                    2 => change.deletion = cap.parse::<i64>().unwrap_or(0),
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
            // 忽略 submodules
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

        let mut data = vec![];
        for (k, v) in stats {
            data.push(FileExtStat {
                ext: k,
                size: v.size,
                files: v.files,
            })
        }
        Ok(data)
    }
}

/// Gitter 的 Binary 实现
#[derive(Copy, Clone)]
pub struct BinaryGitter;

#[async_trait]
impl Gitter for BinaryGitter {
    async fn clone_or_pull(&self, repos: Vec<Repository>) -> Result<()> {
        let mut handles = vec![];
        let mutex = Arc::new(Mutex::new(0));
        let total = repos.len();

        for repo in repos {
            let repo = repo.clone();
            let mutex = mutex.clone();

            let handle = tokio::spawn(async move {
                let now = time::Instant::now();
                if Path::new(&repo.path).exists() {
                    if let Err(e) = GitExecutable::git_pull(&repo).await {
                        error!("failed to execute git pull command, err: {}", e);
                        exit(1)
                    };

                    let mut lock = mutex.lock().unwrap();
                    *lock += 1;
                    let n = *lock;
                    info!(
                        "[{}/{}] git pull: elapsed {:#?} => {}",
                        n,
                        total,
                        now.elapsed(),
                        &repo.remote,
                    )
                } else {
                    if let Err(e) = GitExecutable::git_clone(&repo).await {
                        error!("failed to execute git clone command, err: {}", e);
                        exit(1)
                    };

                    let mut lock = mutex.lock().unwrap();
                    *lock += 1;
                    let n = *lock;
                    info!(
                        "[{}/{}] git clone: elapsed {:#?} => {}",
                        n,
                        total,
                        now.elapsed(),
                        &repo.remote,
                    )
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await?;
        }
        Ok(())
    }

    async fn checkout(&self, repo: &Repository) -> Result<()> {
        if repo.branch.is_some() {
            let branch = repo.branch.clone().unwrap();
            if !branch.is_empty() {
                GitExecutable::git_checkout(repo, &[&branch]).await?;
            }
        }
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
            if let Ok(mut commit) = Parser::parse_commit(&lines[l..r], author_mappings.clone()) {
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
        let mutex = Arc::new(tokio::sync::Mutex::new(vec![]));
        let mut handles = vec![];

        let lines = GitExecutable::git_show_ref(repo, &["--tags"]).await?;
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
                let lines = GitExecutable::git_ls_tree(&repo, &["-r", "-l", "-z", hash]).await;
                let lines = match lines {
                    Err(e) => {
                        error!("failed to execute git ls-tree command, err: {}", e);
                        exit(1)
                    }
                    Ok(lines) => lines,
                };

                let file_ext_stats = match Parser::parse_file_ext_stats(&lines) {
                    Err(e) => {
                        error!("failed to parse file ext stats, err: {}", e);
                        exit(1)
                    }
                    Ok(lines) => lines,
                };

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
                .await;
                let log = match log {
                    Err(e) => {
                        error!("failed to execute git log command, err: {}", e);
                        exit(1)
                    }
                    Ok(log) => log,
                };

                let mut tag_stats = TagStats {
                    stats: file_ext_stats,
                    ..Default::default()
                };
                if let Ok(commit) = Parser::parse_commit(&log[..], author_mappings) {
                    tag_stats.tag = tag.to_string();
                    tag_stats.hash = hash.to_string();
                    tag_stats.datetime = commit.datetime;
                }
                let mut data = lock.lock().await;
                data.push(tag_stats);
            });
            handles.push(handle)
        }

        for handle in handles {
            handle.await?;
        }
        let s = mutex.lock().await;
        Ok(s.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_commit() {
        let output = r#"<Mon Nov 8 23:34:49 2021 +0800> <414915edea035738cc314c8ffab7eccf4e608045> <chenjiandongx> <chenjiandongx@qq.com>
19	0	.gitignore
21	0	LICENSE
1	0	README.md
99	0	conn_darwin.go
396	0	conn_linux.go
71	0	conn_windows.go
65	0	dns.go
18	0	go.mod
52	0	go.sum
335	0	pcap.go
261	0	stat.go
250	0	ui.go"#;
        let lines: Vec<String> = output.split('\n').map(|line| line.to_string()).collect();
        let commit = Parser::parse_commit(&lines, vec![]).unwrap();

        let author = Author {
            name: "chenjiandongx".to_string(),
            email: "chenjiandongx@qq.com".to_string(),
        };
        assert_eq!(commit.author, author);
        assert_eq!("qq.com".to_string(), author.domain());
        assert_eq!("Mon Nov 8 23:34:49 2021 +0800", commit.datetime);
        assert_eq!("414915edea035738cc314c8ffab7eccf4e608045", commit.hash);
        assert_eq!(12, commit.change_files);
        assert_eq!(5, commit.changes.len());

        let changes = commit.changes;
        assert_eq!(0, changes.iter().map(|c| c.deletion).sum::<i64>());
        assert_eq!(1588, changes.iter().map(|c| c.insertion).sum::<i64>());
    }

    #[test]
    fn test_parse_file_ext_stats() {
        let output = r#"100644 blob fc15aee1cb60737ea15ce83b88d0fac349f9d0ff   12827	ui.go"#;
        let fes = Parser::parse_file_ext_stats(&vec![output.to_string()]);
        let fes = fes.unwrap();
        assert_eq!(1, fes.len());

        let first = fes.first().unwrap();
        assert_eq!("go", first.ext);
        assert_eq!(1, first.files);
        assert_eq!(12827, first.size);
    }
}
