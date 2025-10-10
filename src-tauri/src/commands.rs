//! 前端命令模块
//!
//! 专门编写暴露给前端的函数，通过 Tauri 命令与前端交互

pub mod fs;
mod repo_mirror;
mod s3_upload;
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

/// 仓库镜像同步功能
///
/// # 参数
/// - `app_handle`: Tauri 应用句柄，用于发送进度事件和访问应用资源
/// - `from`: 源仓库路径或URL
/// - `to`: 目标仓库路径
///
/// # 返回值
/// - 成功时返回 Ok(())
/// - 失败时返回错误信息字符串
///
/// # 功能说明
/// 该函数用于将源仓库的内容镜像同步到目标位置，支持本地到本地、本地到远程的仓库同步
#[command]
pub fn repo_mirror(app_handle: tauri::AppHandle, from: String, to: String) -> Result<(), String> {
    repo_mirror::repo_mirror(app_handle, from, to)
}

/// 将本地目录覆盖式上传到 S3 远程目录
///
/// # 参数
/// - `params`: S3 上传参数的 JSON 字符串，包含 S3 配置、本地目录路径和远程目录路径
/// - `app_handle`: Tauri 应用句柄，用于发送进度事件
///
/// # 返回值
/// - 成功时返回 Ok(())
/// - 失败时返回错误信息字符串
///
/// # 进度事件
/// 在上传过程中，会通过 Tauri 事件系统发送 "s3-sync-progress" 事件到前端，
/// 包含当前的操作状态和文件信息
#[command]
pub async fn upload_to_s3(params: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    s3_upload::upload_to_s3(params, app_handle).await
}
