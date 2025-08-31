mod utils;

use clap::Parser;
use crate::utils::hash::calculate_file_hash;

/// 文件哈希计算器 - 基于 Blake3 算法的高效文件哈希计算工具
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// 要计算哈希值的文件路径
    #[arg(required = true, help = "指定要计算哈希值的文件路径")]
    file_path: String,
}

fn main() {
    // 解析命令行参数
    let args = Args::parse();
    
    // 计算文件哈希值
    match calculate_file_hash(&args.file_path) {
        Ok(hash) => println!("文件哈希值: {}", hash),
        Err(err) => eprintln!("计算哈希值失败: {}", err),
    }
}
