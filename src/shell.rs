use anyhow::{anyhow, Context, Result};
use datafusion::{arrow::util::pretty, prelude::ExecutionContext};
use rustyline::{error::ReadlineError, Editor};
use std::path::PathBuf;

/// 记录 gitx shell 的语句执行历史，默认路径为 ~/.gitx
fn history_path() -> Result<PathBuf> {
    let mut home =
        dirs::home_dir().ok_or_else(|| anyhow!("Failed to locate user home directory"))?;
    home.push(".gitx");
    Ok(home)
}

/// 持续循环读取并执行 sql 语句，监听 `Ctrl+C`、`q`、`Q` 作为退出信号
pub async fn console_loop(mut ctx: ExecutionContext) -> anyhow::Result<()> {
    let history = history_path();
    let mut readline = Editor::<()>::new();
    if let Ok(ref history) = history {
        readline.load_history(&history).unwrap_or(());
    }

    loop {
        match readline.readline("gitx(sql)> ") {
            Ok(line) => {
                readline.add_history_entry(line.as_str());
                match line.as_ref() {
                    "exit" | "quit" | "q" => {
                        println!("Good bye!");
                        break;
                    }
                    s => {
                        if s.is_empty() {
                            println!("gitx(sql)> ");
                            continue
                        }
                        match ctx.sql(s).await {
                        Ok(batches) => match batches.collect().await {
                            Ok(batches) => {
                                pretty::print_batches(&batches)?;
                            }
                            Err(e) => println!("Error: {}", e),
                        },
                        Err(e) => {
                            println!("Error: {}", e);
                        }
                    }}
                }
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                println!("Good bye!");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    if let Ok(history) = history {
        return readline
            .save_history(&history)
            .context("Failed to save query history");
    }
    Ok(())
}
