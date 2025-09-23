//! 前端命令模块
//!
//! 专门编写暴露给前端的函数，通过 Tauri 命令与前端交互

mod file_copy;
mod repo_mirror;

use crate::utils::hash;
use std::path::PathBuf;
use tauri::command;

/// 计算文件的 Blake3 哈希值
///
/// # 参数
/// - `file_path`: 文件路径字符串
///
/// # 返回值
/// - 成功时返回文件的 base32-crockford 编码的哈希值字符串
/// - 失败时返回错误信息字符串
#[command]
pub fn calculate_file_hash(file_path: String) -> Result<String, String> {
    let path = PathBuf::from(file_path);

    hash::calculate_file_hash(&path).map_err(|e| format!("计算文件哈希值失败: {}", e))
}

#[command]
pub fn repo_mirror(app_handle: tauri::AppHandle, from: String, to: String) -> Result<(), String> {
    repo_mirror::repo_mirror(app_handle, from, to)
}

/// 根据扩展名列表复制文件
///
/// # 参数
/// - `app_handle`: Tauri 应用句柄，用于发送进度事件
/// - `from`: 源目录路径
/// - `to`: 目标目录路径  
/// - `extensions`: 要复制的文件扩展名数组，例如 vec!["mp4".to_string(), "jpg".to_string()]
///
/// # 返回值
/// - 成功时返回复制的文件数量
/// - 失败时返回错误信息字符串
#[command]
pub fn copy_files_with_options(
    app_handle: tauri::AppHandle,
    from: String,
    to: String,
    extensions: Vec<String>,
) -> Result<u64, String> {
    file_copy::copy_files_with_options(app_handle, from, to, extensions)
}
