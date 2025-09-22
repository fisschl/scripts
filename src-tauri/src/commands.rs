//! 前端命令模块
//!
//! 专门编写暴露给前端的函数，通过 Tauri 命令与前端交互

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
