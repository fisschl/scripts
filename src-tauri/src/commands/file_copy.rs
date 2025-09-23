//! 文件复制工具命令模块
//!
//! 该模块提供 Tauri 命令用于根据配置选项复制文件，支持按文件类型筛选、
//! 深层目录遍历、哈希重命名等功能。

use crate::utils::hash;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

/// 文件复制命令
///
/// 根据指定的扩展名从源目录复制文件到目标目录。主要功能包括：
/// - 深层目录遍历
/// - 按文件扩展名筛选
/// - 使用 Blake3 哈希重命名文件
/// - 跳过已存在的文件
/// - 实时发送进度事件到前端
/// - 复制失败时抛出异常（中断整个操作）
///
/// # 参数
/// - `app_handle`: Tauri 应用句柄，用于发送进度事件
/// - `from`: 源目录路径
/// - `to`: 目标目录路径
/// - `extensions`: 要复制的文件扩展名数组，例如 vec!["mp4".to_string(), "jpg".to_string()]
///
/// # 返回值
/// - `Ok(u64)`: 成功复制的文件数量
/// - `Err(String)`: 操作失败，包含详细的错误信息（包括文件复制失败的具体路径和原因）
///
/// # 文件命名规则
/// 复制后的文件使用 Blake3 哈希算法生成唯一文件名，保留原始扩展名
/// 如果目标目录已存在同名文件，则跳过该文件
///
/// # 进度事件
/// 在复制过程中，会通过 Tauri 事件系统发送 "file-copy-progress" 事件到前端，
/// 包含当前正在复制的文件名（不包含完整路径）
pub fn copy_files_with_options(
    app_handle: AppHandle,
    from: String,
    to: String,
    extensions: Vec<String>,
) -> Result<u64, String> {
    let from_path = PathBuf::from(&from);
    let to_path = PathBuf::from(&to);

    // 验证源目录存在且是目录
    if !from_path.exists() {
        return Err(format!("源目录不存在: {}", from));
    }
    if !from_path.is_dir() {
        return Err(format!("源路径不是目录: {}", from));
    }

    // 如果目标目录不存在，创建它
    if !to_path.exists() {
        fs::create_dir_all(&to_path).map_err(|e| format!("创建目标目录失败: {}", e))?;
    }

    // 将扩展名转换为小写，便于比较
    let allowed_extensions: Vec<String> = extensions
        .into_iter()
        .map(|ext| ext.to_lowercase())
        .collect();

    let mut copied_count = 0u64;

    // 遍历源目录中的所有文件
    for entry in WalkDir::new(&from_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let file_path = entry.path();
        let extension = file_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();

        // 检查文件扩展名是否在允许的列表中
        let should_copy = allowed_extensions.contains(&extension);

        if !should_copy {
            continue;
        }

        // 发送进度事件到前端，包含当前正在复制的文件名（不包含完整路径）
        let file_name = file_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("未知文件");
        app_handle.emit("file-copy-progress", file_name).unwrap();

        // 计算文件的哈希值作为新文件名
        let hash_result = hash::calculate_file_hash(file_path)
            .map_err(|e| format!("计算文件哈希值失败: {}", e))?;

        // 构建目标文件路径
        let target_file_name = format!("{}.{}", hash_result, extension);
        let target_path = to_path.join(&target_file_name);

        // 如果目标文件已存在，跳过
        if target_path.exists() {
            continue;
        }

        // 复制文件，如果失败则抛出异常
        fs::copy(file_path, &target_path).map_err(|e| {
            format!(
                "复制文件失败 {} -> {}: {}",
                file_path.display(),
                target_path.display(),
                e
            )
        })?;
        copied_count += 1;
    }

    Ok(copied_count)
}
