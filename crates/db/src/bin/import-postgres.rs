use std::{env, error::Error, process::ExitCode};

use akasha_db::import_postgres_to_sqlite;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("数据库导入失败：{error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let sqlite_path = argument_value("--sqlite-path")?;
    let postgres_url = env::var("AKASHA_IMPORT_POSTGRES_URL")
        .map_err(|_| "缺少 AKASHA_IMPORT_POSTGRES_URL 环境变量")?;

    let summary = import_postgres_to_sqlite(&postgres_url, sqlite_path.clone()).await?;
    println!(
        "SQLite 导入完成：{} 张表，{} 条记录，目标文件 {}",
        summary.table_count, summary.row_count, sqlite_path
    );
    Ok(())
}

fn argument_value(name: &str) -> Result<String, Box<dyn Error>> {
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == name {
            return arguments
                .next()
                .ok_or_else(|| format!("参数 {name} 缺少值").into());
        }
    }

    Err(format!("缺少参数 {name}").into())
}
