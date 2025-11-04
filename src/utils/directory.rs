//! # 目录管理模块
//!
//! 提供目录的创建和验证功能。

use anyhow::{Context, Result};
use std::path::Path;
use tokio::fs;

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
/// use file_utils::utils::directory::ensure_directory_exists;
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
