use crate::{config::AuthorMapping, Author, Repository};
use anyhow::{anyhow, Result};
use async_process::Command;
use chrono::{DateTime, TimeZone, Utc};
use lazy_static::lazy_static;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
    process::exit,
    sync::{Arc, Mutex},
    time,
};
use tokei::{Config, Languages};

/// 提交记录
#[derive(Debug, Clone, Default, Hash, Eq, PartialEq)]
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

impl Commit {
    pub fn new() -> Self {
        Self::default()
    }
}

/// 文件变更记录
#[derive(Debug, Clone, Default, Hash, Eq, PartialEq)]
pub struct FileExtChange {
    /// 文件扩展名
    pub ext: String,
    /// 文件改动增加行数
    pub insertion: usize,
    /// 文件改动删除函数
    pub deletion: usize,
}

impl FileExtChange {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Tags 数据
#[derive(Debug, Clone, Default)]
pub struct Tag {
    /// 版本号
    pub tag: String,
    /// 提交时间
    pub datetime: String,
}

#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    /// 提交时间
    pub datetime: String,
    /// 文件统计数据
    pub stats: Vec<FileExtStat>,
}

/// 文件统计
#[derive(Debug, Clone, Default)]
pub struct FileExtStat {
    /// 文件扩展名
    pub ext: String,
    /// 文件代码行数
    pub code: usize,
    /// 文件注释行数
    pub comments: usize,
    /// 文件空格行数
    pub blanks: usize,
}

lazy_static! {
    static ref COMMIT_INFO_REGEXP: regex::Regex =
        regex::Regex::new(r"<(.*?)> <(.*)> <(.*)> <(.*?)>").unwrap();
    static ref COMMIT_CHANGE_REGEXP: regex::Regex =
        regex::Regex::new(r"([0-9-]+)\t([0-9-]+)\t(.*)").unwrap();
}

/// `git` 可执行文件抽象，使用本地的 `git` 命令
struct Git;

impl Git {
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
        if repo.remote.is_some() {
            c.args(&["clone", repo.remote.as_ref().unwrap(), repo.path.as_str()])
                .output()
                .await?;
        }
        Ok(())
    }

    async fn git_pull(repo: &Repository) -> Result<Vec<String>> {
        Self::git(repo, "pull", &vec![], '\n').await
    }

    async fn git_log(repo: &Repository, args: &[&str]) -> Result<Vec<String>> {
        Self::git(repo, "log", args, '\n').await
    }

    async fn git_show_ref(repo: &Repository, args: &[&str]) -> Result<Vec<String>> {
        Self::git(repo, "show-ref", args, '\n').await
    }

    async fn git_rev_list(repo: &Repository, args: &[&str]) -> Result<Vec<String>> {
        Self::git(repo, "rev-list", args, '\n').await
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
            let mut change = FileExtChange::new();
            let caps = COMMIT_CHANGE_REGEXP.captures(line.as_str());
            if caps.is_none() {
                return Err(anyhow!("Invalid change format: {}", line));
            }

            let caps = caps.unwrap();
            for i in 0..caps.len() {
                let cap = caps.get(i).unwrap().as_str();
                match i {
                    1 => change.insertion = cap.parse::<usize>().unwrap_or_default(),
                    2 => change.deletion = cap.parse::<usize>().unwrap_or_default(),
                    3 => {
                        let p = Path::new(cap);
                        if p.extension().is_some() {
                            change.ext = p.extension().unwrap().to_str().unwrap().to_string();
                            let n = change.ext.len() as usize - 1;
                            if !change.ext.chars().nth(n).unwrap().is_ascii_alphanumeric() {
                                change.ext.remove(n);
                            }
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
        let mut commit = Commit::new();
        Self::parse_commit_info(&mut commit, &lines[0], author_mappings)?;
        Self::parse_commit_changes(&mut commit, &lines[1..])?;
        Ok(commit)
    }
}

#[derive(Copy, Clone)]
pub struct GitImpl;

impl GitImpl {
    async fn range_commits(repo: &Repository) -> Result<Vec<(String, String)>> {
        let lines = Git::git_rev_list(repo, &["--max-parents=0", "HEAD"]).await?;
        if lines.is_empty() {
            return Err(anyhow!("Empty git-rev-list output"));
        }
        let lines = Git::git_log(
            repo,
            &[
                "--date=rfc",
                "--pretty=format:<%ad> <%H> <%aN> <%aE>",
                &lines[0],
            ],
        )
        .await?;
        if lines.is_empty() {
            return Err(anyhow!("Failed to get first commit detailed"));
        }
        let mut first_commit = Commit::new();
        Parser::parse_commit_info(&mut first_commit, &lines[0], vec![])?;

        let lines = Git::git_log(
            repo,
            &[
                "--date=rfc",
                "--pretty=format:<%ad> <%H> <%aN> <%aE>",
                "HEAD",
            ],
        )
        .await?;
        if lines.is_empty() {
            return Err(anyhow!("Failed to get last commit detailed"));
        }
        let mut last_commit = Commit::new();
        Parser::parse_commit_info(&mut last_commit, &lines[0], vec![])?;

        let first_ts = DateTime::parse_from_rfc2822(&first_commit.datetime)?.timestamp();
        let last_ts = DateTime::parse_from_rfc2822(&last_commit.datetime)?.timestamp();

        const DURATION: i64 = 3600 * 24 * 120; // 120days
        let data = Self::calc_range(DURATION, first_ts, last_ts);

        if data.len() == 1 {
            let first = &data[0];
            if first.0 == first.1 {
                return Ok(vec![("".to_string(), "".to_string())]);
            }
        }
        Ok(data)
    }

    fn calc_range(duration: i64, mut start: i64, end: i64) -> Vec<(String, String)> {
        let format = "%Y-%m-%d";
        let mut data: Vec<(String, String)> = vec![];
        while start + duration <= end {
            data.push((
                Utc.timestamp(start, 0).format(format).to_string(),
                Utc.timestamp(start + duration, 0)
                    .format(format)
                    .to_string(),
            ));
            start += duration;
        }
        data.push((
            Utc.timestamp(start, 0).format(format).to_string(),
            Utc.timestamp(end, 0).format(format).to_string(),
        ));

        data
    }
}

impl GitImpl {
    pub async fn clone_or_pull(repos: Vec<Repository>) -> Result<()> {
        let mut handles = vec![];
        let mutex = Arc::new(Mutex::new(0));
        let total = repos.len();

        for repo in repos {
            let repo = repo.clone();
            let mutex = mutex.clone();

            let handle = tokio::spawn(async move {
                let now = time::Instant::now();
                if Path::new(&repo.path).exists() {
                    if let Err(e) = Git::git_pull(&repo).await {
                        println!("Failed to execute git pull command, error: {}", e);
                        exit(1)
                    };

                    let mut lock = mutex.lock().unwrap();
                    *lock += 1;
                    let n = *lock;
                    println!(
                        "[{}/{}] git pull '{}' => elapsed {:#?}",
                        n,
                        total,
                        &repo.name,
                        now.elapsed(),
                    )
                } else {
                    if let Err(e) = Git::git_clone(&repo).await {
                        println!("Failed to execute git clone command, error: {}", e);
                        exit(1)
                    };

                    let mut lock = mutex.lock().unwrap();
                    *lock += 1;
                    let n = *lock;
                    println!(
                        "[{}/{}] git clone '{}' => elapsed {:#?}",
                        n,
                        total,
                        &repo.name,
                        now.elapsed(),
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

    pub async fn checkout(repo: &Repository) -> Result<()> {
        if repo.branch.is_some() {
            let branch = repo.branch.clone().unwrap();
            if !branch.is_empty() {
                Git::git_checkout(repo, &[&branch]).await?;
            }
        }
        Ok(())
    }

    pub async fn commits(
        repo: &Repository,
        author_mappings: Vec<AuthorMapping>,
    ) -> Result<Vec<Commit>> {
        let mutex = Arc::new(Mutex::new(HashSet::new()));
        let mut handles = vec![];
        for (since, before) in Self::range_commits(repo).await? {
            let mutex = mutex.clone();
            let repo = repo.clone();
            let author_mappings = author_mappings.clone();
            let handle = tokio::spawn(async move {
                let lines = if since.is_empty() && before.is_empty() {
                    Git::git_log(
                        &repo,
                        &[
                            "--no-merges",
                            "--date=rfc",
                            "--pretty=format:<%ad> <%H> <%aN> <%aE>",
                            "--numstat",
                            "HEAD",
                        ],
                    )
                    .await
                } else {
                    Git::git_log(
                        &repo,
                        &[
                            "--no-merges",
                            "--date=rfc",
                            &format!("--since={}", since),
                            &format!("--before={}", before),
                            "--pretty=format:<%ad> <%H> <%aN> <%aE>",
                            "--numstat",
                            "HEAD",
                        ],
                    )
                    .await
                };

                if let Err(e) = lines {
                    println!("Failed to execute git log command, error: {}", e);
                    exit(1)
                };

                let lines = lines.unwrap();
                let mut indexes = vec![];
                for (idx, line) in lines.iter().enumerate() {
                    if line.starts_with('<') {
                        indexes.push(idx);
                    }
                }
                indexes.push(lines.len());

                let mut lock = mutex.lock().unwrap();
                for i in 1..indexes.len() {
                    let (l, r) = (indexes[i - 1], indexes[i]);
                    if let Ok(mut commit) =
                        Parser::parse_commit(&lines[l..r], author_mappings.clone())
                    {
                        commit.repo = repo.name.to_string();
                        lock.insert(commit);
                    }
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await?
        }

        let lock = mutex.lock().unwrap();
        let data: Vec<_> = lock.iter().map(|x| x.clone()).collect();
        Ok(data)
    }

    pub async fn snapshot(repo: &Repository) -> Result<Snapshot> {
        let lines = Git::git_log(
            &repo,
            &[
                "--no-merges",
                "--date=rfc",
                "--pretty=format:<%ad> <%H> <%aN> <%aE>",
                "HEAD",
            ],
        )
        .await?;

        if lines.is_empty() {
            return Err(anyhow!("Failed to get commit detailed"));
        }

        let mut commit = Commit::new();
        Parser::parse_commit_info(&mut commit, &lines[0], vec![])?;

        let mut languages = Languages::new();
        languages.get_statistics(&[repo.path.clone()], &vec![], &Config::default());

        let mut stats = vec![];
        for (ty, language) in languages {
            stats.push(FileExtStat {
                ext: ty.to_string().to_lowercase(),
                code: language.code,
                comments: language.comments,
                blanks: language.blanks,
            });
        }

        Ok(Snapshot {
            datetime: commit.datetime,
            stats,
        })
    }

    pub async fn tags(repo: &Repository, author_mappings: Vec<AuthorMapping>) -> Result<Vec<Tag>> {
        let mut records = vec![];
        let lines = Git::git_show_ref(repo, &["--tags"]).await?;
        for line in lines {
            let fields: Vec<String> = line.splitn(2, ' ').map(|x| x.to_string()).collect();
            if fields.len() < 2 {
                continue;
            }

            let hash = &fields[0];
            let tag = &fields[1]["refs/tags/".len()..];

            let logs = Git::git_log(
                &repo,
                &[
                    "--no-merges",
                    "--date=rfc",
                    "--pretty=format:<%ad> <%H> <%aN> <%aE>",
                    "-n",
                    "1",
                    hash,
                ],
            )
            .await?;

            if logs.is_empty() {
                continue;
            }

            let commit = Parser::parse_commit(&logs, author_mappings.clone())?;
            records.push(Tag {
                tag: tag.to_string(),
                datetime: commit.datetime,
            });
        }

        Ok(records)
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
        assert_eq!(0, changes.iter().map(|c| c.deletion).sum::<usize>());
        assert_eq!(1588, changes.iter().map(|c| c.insertion).sum::<usize>());
    }

    #[test]
    fn test_calc_range() {
        let duration: i64 = 3600 * 24;
        let start: i64 = 1647432000;
        let end: i64 = 1647432000 + (duration * 3) + 720;

        let range = GitImpl::calc_range(duration, start, end);
        let excepted = vec![
            (String::from("2022-03-16"), String::from("2022-03-17")),
            (String::from("2022-03-17"), String::from("2022-03-18")),
            (String::from("2022-03-18"), String::from("2022-03-19")),
            (String::from("2022-03-19"), String::from("2022-03-19")),
        ];
        assert_eq!(excepted, range);
    }
}