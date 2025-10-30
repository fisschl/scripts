//! # 工具模块 (utils)
//!
//! 提供文件处理工具集的公共功能，包括哈希计算、文件系统操作等。

use anyhow::{Context, Result};
use std::path::Path;
use tokio::fs;
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
/// - 使用 8KB 缓冲区进行流式读取，支持大文件处理
/// - Base58 编码避免在文件系统中出现无效字符
///
/// # 示例
///
/// ```rust
/// use file_utils::utils::calculate_file_hash;
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
    let mut buffer = [0; 8192]; // 8KB 缓冲区，平衡内存使用和性能

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

/// 删除文件或目录
///
/// 根据路径类型自动选择删除方法：
/// - 文件：使用 `remove_file`
/// - 目录：使用 `remove_dir_all`（递归删除）
///
/// # 参数
///
/// * `path` - 要删除的文件或目录路径
///
/// # 返回值
///
/// * `Ok(())` - 删除成功
/// * `Err(anyhow::Error)` - 删除失败，包含详细错误信息
///
/// # 安全性
///
/// 此函数会永久删除文件和目录，请确保：
/// - 数据已备份或不再需要
/// - 路径正确，避免误删
///
/// # 示例
///
/// ```rust
/// use file_utils::utils::remove_path;
/// use std::path::Path;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let path = Path::new("./old_file.txt");
///     remove_path(path).await?;
///     println!("删除成功");
///     Ok(())
/// }
/// ```
pub async fn remove_path<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();

    if path.is_file() {
        // 删除单个文件
        fs::remove_file(path)
            .await
            .with_context(|| format!("删除文件失败: {}", path.display()))?;
    } else if path.is_dir() {
        // 递归删除整个目录
        fs::remove_dir_all(path)
            .await
            .with_context(|| format!("删除目录失败: {}", path.display()))?;
    }
    Ok(())
}

/// 确保目录存在，如果不存在则创建
///
/// 检查指定目录是否存在，如果不存在则递归创建所有必要的父目录。
/// 这确保了目标目录可以安全地用于文件存储。
///
/// # 参数
///
/// * `dir_path` - 要检查和创建的目录路径
///
/// # 返回值
///
/// * `Ok(())` - 目录已存在或创建成功
/// * `Err(anyhow::Error)` - 创建目录失败，包含详细错误信息
///
/// # 示例
///
/// ```rust
/// use file_utils::utils::ensure_directory_exists;
/// use std::path::Path;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let target_dir = Path::new("./backup/videos");
///     ensure_directory_exists(target_dir).await?;
///     println!("目录准备就绪");
///     Ok(())
/// }
/// ```
pub async fn ensure_directory_exists<P: AsRef<Path>>(dir_path: P) -> Result<()> {
    let dir_path = dir_path.as_ref();

    // 检查目录是否已存在
    if !dir_path.exists() {
        // 递归创建目录及其所有父目录
        fs::create_dir_all(dir_path)
            .await
            .with_context(|| format!("创建目录失败: {}", dir_path.display()))?;
    }
    Ok(())
}
