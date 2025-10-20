//! 文件系统操作模块
//!
//! 提供前端可调用的文件系统操作命令

use serde::Serialize;
use std::fs;
use std::path::Path;
use tauri::command;

/// 文件系统条目信息
///
/// 表示文件或目录的基本元数据信息
#[derive(Debug, Serialize)]
pub struct FileInfo {
    /// 条目的完整路径
    pub path: String,
    /// 是否为目录
    pub is_dir: bool,
    /// 文件大小（字节），目录通常为 0
    pub size: u64,
    /// 最后修改时间（ISO 8601 格式）
    pub last_modified: String,
}

/// 列举目录内容
///
/// 扫描指定目录并返回其中所有文件和子目录的信息。
/// 此操作是递归的，只列出直接子项，不会深入子目录。
///
/// # 参数
///
/// * `path` - 要扫描的目录路径
///
/// # 返回值
///
/// * `Ok(Vec<FileInfo>)` - 成功时返回目录条目信息列表
/// * `Err(CommandError)` - 失败时返回错误描述
///
/// # 错误
///
/// * 当目录不存在时返回错误
/// * 当路径不是目录时返回错误
/// * 当没有读取权限时返回错误
#[command]
pub fn list_directory(path: String) -> Result<Vec<FileInfo>, String> {
    let path = Path::new(&path);

    if !path.exists() {
        return Err("目录不存在".to_string());
    }

    if !path.is_dir() {
        return Err("路径不是目录".to_string());
    }

    let mut files = Vec::new();

    let entries = fs::read_dir(path).map_err(|e| format!("读取目录失败: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("读取目录条目失败: {}", e))?;
        let entry_path = entry.path();
        let metadata = entry
            .metadata()
            .map_err(|e| format!("获取文件元数据失败: {}", e))?;

        let last_modified = metadata
            .modified()
            .map_err(|e| format!("获取修改时间失败: {}", e))?
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| format!("时间转换失败: {}", e))?
            .as_millis();

        // 转换为 DateTime 并格式化为 ISO 8601
        let dt = chrono::DateTime::from_timestamp_millis(last_modified as i64)
            .ok_or_else(|| "转换时间戳失败".to_string())?
            .to_rfc3339();

        let file_info = FileInfo {
            path: entry_path.to_string_lossy().to_string(),
            is_dir: metadata.is_dir(),
            size: metadata.len(),
            last_modified: dt,
        };

        files.push(file_info);
    }

    Ok(files)
}

/// 复制文件
///
/// 将文件从源位置复制到目标位置。可以选择是否覆盖已存在的文件。
/// 如果目标目录不存在，会自动创建。
///
/// # 参数
///
/// * `from` - 源文件的完整路径
/// * `to` - 目标文件的完整路径
/// * `overwrite` - 是否覆盖已存在的文件，默认为 false
///
/// # 返回值
///
/// * `Ok(())` - 文件复制成功
/// * `Err(CommandError)` - 复制失败时的错误描述
///
/// # 行为
///
/// * 当源文件不存在时返回错误
/// * 当目标文件已存在且 `overwrite` 为 false 时，跳过复制并返回成功
/// * 当目标目录不存在时，自动创建目录结构
/// * 当目标文件已存在且 `overwrite` 为 true 时，覆盖目标文件
#[command]
pub fn copy_file(from: String, to: String, overwrite: Option<bool>) -> Result<(), String> {
    let from = Path::new(&from);
    let to = Path::new(&to);
    let overwrite = overwrite.unwrap_or(false);

    // 检查源文件是否存在
    if !from.exists() {
        return Err("源文件不存在".to_string());
    }

    // 检查目标文件是否已存在，如果已存在且不允许覆盖则直接返回成功
    if to.exists() && !overwrite {
        return Ok(());
    }

    // 确保目标目录存在
    if let Some(parent) = to.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
        }
    }

    // 复制文件
    fs::copy(from, to).map_err(|e| format!("复制文件失败: {}", e))?;

    Ok(())
}

/// 删除文件或目录
///
/// 删除指定路径的文件或目录。如果是目录，将递归删除其中的所有内容。
/// 如果路径不存在，视为操作成功。
///
/// # 参数
///
/// * `path` - 要删除的文件或目录路径
///
/// # 返回值
///
/// * `Ok(())` - 删除操作成功
/// * `Err(CommandError)` - 删除失败时的错误描述
///
/// # 行为
///
/// * 当路径不存在时，返回成功（幂等操作）
/// * 当路径是文件时，删除单个文件
/// * 当路径是目录时，递归删除目录及其所有内容
/// * 当没有删除权限时返回错误
#[command]
pub fn remove_path(path: String) -> Result<(), String> {
    let path = Path::new(&path);

    if !path.exists() {
        return Ok(()); // 路径不存在，视为成功
    }

    if path.is_file() {
        // 删除文件
        fs::remove_file(path).map_err(|e| format!("删除文件失败: {}", e))?;
    } else if path.is_dir() {
        // 递归删除目录
        fs::remove_dir_all(path).map_err(|e| format!("删除目录失败: {}", e))?;
    }

    Ok(())
}

/// 计算目录总大小
///
/// 递归计算指定目录及其所有子目录中所有文件的总大小。
/// 此函数会遍历整个目录树，累加所有文件的大小。
///
/// # 参数
///
/// * `path` - 要计算大小的目录路径
///
/// # 返回值
///
/// * `Ok(u64)` - 成功时返回目录的总大小（字节）
/// * `Err(String)` - 失败时返回错误描述
///
/// # 行为
///
/// * 当目录不存在时返回错误
/// * 当路径不是目录时返回错误
/// * 递归计算所有子目录中的文件大小
/// * 对于大型目录结构，计算时间可能较长
/// * 目录本身的大小不计算在内，只计算其中的文件大小
#[command]
pub fn calculate_directory_size(path: String) -> Result<u64, String> {
    let path = Path::new(&path);

    if !path.exists() {
        return Err("目录不存在".to_string());
    }

    if !path.is_dir() {
        return Err("路径不是目录".to_string());
    }

    let mut total_size = 0u64;

    fn calculate_size_recursive(dir: &Path, total_size: &mut u64) -> Result<(), String> {
        let entries = fs::read_dir(dir).map_err(|e| format!("读取目录失败: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("读取目录条目失败: {}", e))?;
            let entry_path = entry.path();

            if entry_path.is_dir() {
                // 递归处理子目录
                calculate_size_recursive(&entry_path, total_size)?;
            } else if entry_path.is_file() {
                // 获取文件大小并累加
                let metadata = entry_path
                    .metadata()
                    .map_err(|e| format!("获取文件元数据失败: {}", e))?;
                *total_size += metadata.len();
            }
        }

        Ok(())
    }

    calculate_size_recursive(path, &mut total_size)?;

    Ok(total_size)
}
