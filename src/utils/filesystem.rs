//! # 文件系统操作模块
//!
//! 提供文件和目录的创建、删除等文件系统操作功能。

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
/// use file_utils::utils::filesystem::ensure_directory_exists;
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

/// 获取文件扩展名（小写）
///
/// 提取路径中的文件扩展名并转换为小写形式。
/// 如果文件没有扩展名，返回空字符串。
///
/// # 参数
///
/// * `path` - 文件路径
///
/// # 返回值
///
/// * `String` - 小写的文件扩展名（不含点号），如果无扩展名则返回空字符串
///
/// # 示例
///
/// ```rust
/// use file_utils::utils::filesystem::get_file_extension;
/// use std::path::Path;
///
/// let ext = get_file_extension(Path::new("document.PDF"));
/// assert_eq!(ext, "pdf");
///
/// let ext = get_file_extension(Path::new("archive.tar.GZ"));
/// assert_eq!(ext, "gz");
///
/// let ext = get_file_extension(Path::new("no_extension"));
/// assert_eq!(ext, "");
/// ```
pub fn get_file_extension<P: AsRef<Path>>(path: P) -> String {
    path.as_ref()
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase())
        .unwrap_or_default()
}

/// 列举本地目录下所有文件（返回相对路径）
///
/// 递归遍历指定目录，返回所有文件的相对路径列表。
/// 路径分隔符统一使用正斜杠 `/`，便于跨平台使用。
///
/// # 参数
///
/// * `dir` - 要扫描的目录路径
///
/// # 返回值
///
/// * `Ok(Vec<String>)` - 所有文件的相对路径列表
/// * `Err(anyhow::Error)` - 目录读取失败
///
/// # 示例
///
/// ```rust
/// use file_utils::utils::filesystem::list_local_files;
/// use std::path::Path;
///
/// fn main() -> anyhow::Result<()> {
///     let dir = Path::new("./src");
///     let files = list_local_files(dir)?;
///     for file in files {
///         println!("{}", file);
///     }
///     Ok(())
/// }
/// ```
pub fn list_local_files<P: AsRef<Path>>(dir: P) -> Result<Vec<String>> {
    let dir = dir.as_ref();
    let mut files = Vec::new();
    list_local_files_recursive(dir, dir, &mut files)?;
    Ok(files)
}

/// 递归遍历目录，收集所有文件的相对路径
fn list_local_files_recursive(base: &Path, current: &Path, files: &mut Vec<String>) -> Result<()> {
    for entry in std::fs::read_dir(current)
        .with_context(|| format!("无法读取目录: {}", current.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let rel_path = path
                .strip_prefix(base)
                .with_context(|| "无法获取相对路径")?
                .to_string_lossy()
                .replace('\\', "/");
            files.push(rel_path);
        } else if path.is_dir() {
            list_local_files_recursive(base, &path, files)?;
        }
    }
    Ok(())
}
