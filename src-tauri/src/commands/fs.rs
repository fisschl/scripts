//! 文件系统操作模块
//!
//! 提供前端可调用的文件系统操作命令

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tauri::command;

/// 文件信息
#[derive(Debug, Serialize)]
pub struct FileInfo {
    /// 文件路径
    path: String,
    /// 是否为目录
    is_dir: bool,
    /// 文件大小（字节）
    size: u64,
}

/// 列举目录内容命令参数
#[derive(Debug, Deserialize)]
pub struct ListDirectoryArgs {
    /// 目录路径
    path: String,
}

/// 复制文件命令参数
#[derive(Debug, Deserialize)]
pub struct CopyFileArgs {
    /// 源文件路径
    from: String,
    /// 目标文件路径
    to: String,
    /// 目标存在时是否覆盖
    overwrite: bool,
}

/// 列举目录下所有文件和子目录
///
/// # 参数
/// - `args`: 包含目录路径和是否递归的参数
///
/// # 返回值
/// - 成功时返回文件信息列表
/// - 失败时返回错误信息字符串
#[command]
pub fn list_directory(args: ListDirectoryArgs) -> Result<Vec<FileInfo>, String> {
    let path = Path::new(&args.path);

    if !path.exists() {
        return Err("目录不存在".to_string());
    }

    if !path.is_dir() {
        return Err("路径不是目录".to_string());
    }

    let mut files = Vec::new();

    let entries = fs::read_dir(path).map_err(|e| format!("读取目录失败: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("遍历目录条目失败: {}", e))?;
        let entry_path = entry.path();
        let metadata = entry
            .metadata()
            .map_err(|e| format!("获取文件元数据失败: {}", e))?;

        let file_info = FileInfo {
            path: entry_path.to_string_lossy().to_string(),
            is_dir: metadata.is_dir(),
            size: metadata.len(),
        };

        files.push(file_info);
    }

    Ok(files)
}

/// 复制文件
///
/// # 参数
/// - `args`: 包含源路径、目标路径和是否覆盖的参数
///
/// # 返回值
/// - 成功时返回 Ok(())
/// - 失败时返回错误信息字符串
#[command]
pub fn copy_file(args: CopyFileArgs) -> Result<(), String> {
    let from = Path::new(&args.from);
    let to = Path::new(&args.to);

    // 检查源文件是否存在
    if !from.exists() {
        return Err("源文件不存在".to_string());
    }

    // 检查目标文件是否已存在，如果已存在且不允许覆盖则直接返回成功
    if to.exists() && !args.overwrite {
        return Ok(());
    }

    // 确保目标目录存在
    if let Some(parent) = to.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| format!("创建目标目录失败: {}", e))?;
        }
    }

    // 复制文件
    fs::copy(from, to).map_err(|e| format!("复制文件失败: {}", e))?;

    Ok(())
}
