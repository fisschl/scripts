mod commands;
mod utils;

use clap::{Parser, Subcommand};
use commands::{execute_find_empty_s3_files, execute_hash};

/// 多功能工具 - 文件哈希计算和S3操作工具
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// 计算文件的哈希值
    Hash {
        /// 要计算哈希值的文件路径
        #[arg(help = "指定要计算哈希值的文件路径")]
        file_path: String,
    },
    /// 在S3中查找空文件
    FindEmptyS3Files {
        /// 可选的前缀路径，用于限制搜索范围
        #[arg(short, long, help = "指定搜索前缀路径")]
        prefix: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv::dotenv().ok();

    // 解析命令行参数
    let args = Args::parse();

    match args.command {
        Commands::Hash { file_path } => {
            execute_hash(file_path).await?;
        }
        Commands::FindEmptyS3Files { prefix } => {
            execute_find_empty_s3_files(prefix).await?;
        }
    }

    Ok(())
}
