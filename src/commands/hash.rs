use crate::utils::hash::calculate_file_hash;
use console::{Emoji, style};

/// 执行文件哈希计算命令
///
/// # 参数
/// - `file_path`: 要计算哈希值的文件路径
///
/// # 返回值
/// 返回 `Result<(), anyhow::Error>`
pub async fn execute_hash(file_path: String) -> Result<(), anyhow::Error> {
    let hash = calculate_file_hash(&file_path)?;

    println!();
    println!(
        "{} {} {}",
        Emoji("🔍", ""),
        style("文件哈希值:").bold().cyan(),
        style(&file_path).yellow().bold()
    );
    println!("{} {}", Emoji("📋", ""), style(&hash).green().bold());
    println!();

    Ok(())
}
