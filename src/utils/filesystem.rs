//! # 文件系统操作模块
//!
//! 提供文件和目录的删除操作功能。

use anyhow::{Context, Result};
use std::path::Path;
use tokio::fs;

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
/// use file_utils::utils::filesystem::remove_path;
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
