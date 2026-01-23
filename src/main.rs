//! # 文件处理工具集 (scripts)
//!
//! 一个集成了多种文件处理功能的命令行工具，支持子命令模式。

use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod utils;

/// 主命令结构体
///
/// 使用 clap 的 Parser API 自动解析命令行参数，
/// 提供类型安全和自动生成的帮助信息。
#[derive(Parser, Debug)]
#[command(name = "scripts")]
#[command(version = "0.1.0")]
#[command(
    about = "文件处理工具集",
    long_about = "多功能文件处理命令行工具。使用子命令 --help 查看详细说明。"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// 子命令枚举
///
/// 定义了所有支持的子命令，每个子命令对应一个具体的功能模块。
#[derive(Subcommand, Debug)]
enum Commands {
    /// 使用 7-Zip 压缩文件和目录,然后删除原始项目
    CompressDelete(commands::compress_delete::CompressDeleteArgs),
    /// 将文件从源目录复制到目标目录，使用哈希值重命名
    HashCopy(commands::hash_copy::HashCopyArgs),
    /// 查找软件卸载残留
    ResidueSearch(commands::residue_search::ResidueSearchArgs),
    /// 查找目录中未被使用的文件
    UnusedFiles(commands::unused_files::UnusedFilesArgs),
    /// 将视频文件转码为 WebM AV1 格式
    VideoTranscode(commands::video_transcode::VideoTranscodeArgs),
}

/// 主函数
///
/// 程序入口点，负责解析命令行参数并调用相应的子命令处理函数。
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::CompressDelete(args) => commands::compress_delete::run(args).await,
        Commands::HashCopy(args) => commands::hash_copy::run(args).await,
        Commands::ResidueSearch(args) => commands::residue_search::run(args).await,
        Commands::UnusedFiles(args) => commands::unused_files::run(args).await,
        Commands::VideoTranscode(args) => commands::video_transcode::run(args).await,
    }
}
