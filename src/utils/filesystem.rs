//! # 文件系统操作模块
//!
//! 提供文件和目录的创建、删除等文件系统操作功能。

use std::path::Path;
use walkdir::WalkDir;

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
/// use scripts::utils::filesystem::get_file_extension;
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

/// 计算目录的实际大小（字节数）
///
/// 使用 WalkDir 遍历目录，累加所有文件的大小。
/// 权限不足时自动跳过，不会抛出异常。
///
/// # 参数
///
/// * `path` - 要计算大小的目录路径
///
/// # 返回值
///
/// * `u64` - 目录总大小（字节数），如果无法访问则返回 0
///
/// # 示例
///
/// ```rust
/// use scripts::utils::filesystem::calculate_dir_size;
/// use std::path::Path;
///
/// let size = calculate_dir_size(Path::new("./src"));
/// println!("目录大小: {} 字节", size);
/// ```
pub fn calculate_dir_size<P: AsRef<Path>>(path: P) -> u64 {
    WalkDir::new(path.as_ref())
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
        .sum()
}
