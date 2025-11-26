//! # 哈希计算模块
//!
//! 提供文件哈希计算功能，使用 Blake3 算法和 Base58 编码。

use anyhow::{Context, Result};
use std::path::Path;
use tokio::io::AsyncReadExt;

/// 计算文件的 Blake3 哈希值并使用 Base58 编码
///
/// 对文件内容进行 Blake3 哈希计算，然后将哈希值编码为 Base58 格式。
/// 这样生成的文件名既唯一又便于文件系统使用。
///
/// # 参数
///
/// * `file_path` - 要计算哈希的文件路径
///
/// # 返回值
///
/// * `Ok(String)` - Base58 编码的哈希值
/// * `Err(anyhow::Error)` - 计算哈希失败，包含详细错误信息
///
/// # 技术细节
///
/// - 使用 Blake3 哈希算法，提供高性能和安全性
/// - 使用 64KB 缓冲区进行流式读取，优化大文件处理性能
/// - Base58 编码避免在文件系统中出现无效字符
///
/// # 示例
///
/// ```rust
/// use file_utils::utils::hash::calculate_file_hash;
/// use std::path::Path;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let file = Path::new("./video.mp4");
///     let hash = calculate_file_hash(file).await?;
///     println!("文件哈希: {}", hash);
///     Ok(())
/// }
/// ```
pub async fn calculate_file_hash<P: AsRef<Path>>(file_path: P) -> Result<String> {
    let file_path = file_path.as_ref();

    // 异步打开文件进行读取
    let mut file = tokio::fs::File::open(file_path)
        .await
        .with_context(|| format!("打开文件失败: {}", file_path.display()))?;

    // 创建 Blake3 哈希器
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0; 65536]; // 64KB 缓冲区，优化大文件性能

    // 流式读取文件内容并更新哈希
    loop {
        let n = file
            .read(&mut buffer)
            .await
            .with_context(|| format!("读取文件失败: {}", file_path.display()))?;
        if n == 0 {
            break; // 文件读取完毕
        }
        hasher.update(&buffer[..n]);
    }

    // 完成哈希计算并进行 Base58 编码
    let hash = hasher.finalize();
    let hash_bytes = hash.as_bytes();
    Ok(bs58::encode(hash_bytes).into_string())
}
