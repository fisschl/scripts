//! # 文件复制并重命名工具 (file_copy_rename)
//!
//! 一个简洁高效的 Rust 命令行工具，用于将源目录中的文件复制到目标目录，
//! 并使用 Blake3 哈希值重命名以避免重复。
//!
//! ## 功能特性
//!
//! - **哈希重命名**：使用 Blake3 哈希 + Base58 编码生成唯一文件名
//! - **重复检测**：自动跳过已存在的文件，避免重复复制
//! - **灵活过滤**：支持自定义文件扩展名过滤
//! - **移动模式**：可选择复制后删除源文件
//! - **递归处理**：递归遍历源目录的所有子目录
//! - **智能跳过**：自动跳过隐藏文件和目录
//! - **错误恢复**：单个文件失败不影响其他文件处理
//!
//! ## 使用方法
//!
//! ```bash
//! file_copy_rename [OPTIONS]
//! ```
//!
//! ## 参数说明
//!
//! - `[--source, -s] <DIRECTORY>`: 源目录路径，默认为 `./source`
//! - `[--target, -t] <DIRECTORY>`: 目标目录路径，默认为 `./target`
//! - `[--extensions, -e] <EXTENSIONS>`: 文件扩展名（逗号分隔，不带点），默认为常见视频格式
//! - `[--move, -m]`: 启用移动模式（复制后删除源文件）
//!
//! ## 示例
//!
//! ```bash
//! # 复制默认目录的默认格式文件
//! file_copy_rename
//!
//! # 复制指定目录的图片文件
//! file_copy_rename --source ./photos --target ./backup --extensions jpg,png,gif
//!
//! # 移动视频文件
//! file_copy_rename --source ./videos --target ./archive --extensions mp4,avi --move
//!
//! # 使用短选项
//! file_copy_rename -s ./source -t ./target -e "mp4,webm" -m
//! ```
//!
//! ## 工作流程
//!
//! 1. 递归扫描源目录中的所有文件
//! 2. 过滤隐藏文件和不符合扩展名要求的文件
//! 3. 计算文件的 Blake3 哈希值
//! 4. 使用哈希值生成目标文件名
//! 5. 复制文件到目标目录
//! 6. 如果启用移动模式，删除源文件
//!
//! ## 注意事项
//!
//! - 启用移动模式会永久删除源文件，请确保有备份
//! - 相同内容的文件只会生成一个副本（基于哈希去重）
//! - 目标目录如果不存在会自动创建

use anyhow::{Context, Result};
use clap::Parser;
use file_utils::utils::{directory::ensure_directory_exists, hash::calculate_file_hash};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// 命令行参数结构体
///
/// 使用 clap 的 Derive API 自动解析命令行参数，
/// 提供类型安全和自动生成的帮助信息。
#[derive(Parser, Debug)]
#[command(name = "file_copy_rename")]
#[command(version = "0.1.0")]
#[command(about = "将文件从源目录复制到目标目录，使用哈希值重命名")]
struct Args {
    /// 源目录路径
    ///
    /// 包含要复制的文件的目录。工具会递归遍历这个目录。
    /// 默认为 "./source"。
    #[arg(short = 's', long, default_value = "./source")]
    source: PathBuf,

    /// 目标目录路径
    ///
    /// 复制文件的输出目录。如果不存在会自动创建。
    /// 默认为 "./target"。
    #[arg(short = 't', long, default_value = "./target")]
    target: PathBuf,

    /// 要处理的文件扩展名
    ///
    /// 指定要处理的文件扩展名，多个扩展名用逗号分隔。
    /// 例如：mp4,webm,m4v
    /// 默认为常见视频格式。
    #[arg(short = 'e', long, default_value = "mp4,webm,m4v,avi,mkv,mov")]
    extensions: String,

    /// 移动模式
    ///
    /// 启用后，复制成功会删除源文件（相当于移动操作）。
    /// 默认为禁用（仅复制）。
    #[arg(short = 'm', long)]
    move_after_copy: bool,
}

/// 处理单个文件
///
/// 对单个文件执行复制/移动流程：
/// 1. 计算文件哈希值
/// 2. 生成基于哈希的目标文件名
/// 3. 复制文件到目标目录
/// 4. 如果启用移动模式，删除源文件
///
/// # 参数
///
/// * `file_path` - 要处理的文件路径
/// * `target_dir` - 目标目录路径
/// * `move_after_copy` - 是否在复制后删除源文件
///
/// # 返回值
///
/// * `Ok(())` - 处理成功
/// * `Err(anyhow::Error)` - 处理失败
async fn process_file(file_path: &Path, target_dir: &Path, move_after_copy: bool) -> Result<()> {
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .context("无效的文件名")?;

    println!("处理: {}", file_name);

    // 计算文件哈希
    let hash = calculate_file_hash(file_path)
        .await
        .context("计算文件哈希失败")?;

    // 获取文件扩展名（不带点，小写）
    let ext = file_path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    // 生成目标文件名
    let target_filename = if ext.is_empty() {
        hash
    } else {
        format!("{}.{}", hash, ext)
    };

    let target_path = target_dir.join(&target_filename);

    // 检查目标文件是否已存在
    if target_path.exists() {
        println!("目标已存在: {}", target_filename);
        return Ok(());
    }

    // 复制文件
    tokio::fs::copy(file_path, &target_path)
        .await
        .with_context(|| format!("复制文件到 {} 失败", target_path.display()))?;

    println!("复制完成: {} -> {}", file_name, target_filename);

    // 如果启用了移动模式，复制成功后删除源文件
    if move_after_copy {
        tokio::fs::remove_file(file_path)
            .await
            .with_context(|| format!("删除源文件失败: {}", file_path.display()))?;

        println!("删除源文件: {}", file_name);
    }

    Ok(())
}

/// 主函数
///
/// 程序入口点，负责协调整个文件复制和重命名流程：
/// 1. 解析命令行参数
/// 2. 验证源目录和目标目录
/// 3. 确保目标目录存在
/// 4. 递归处理源目录中的所有文件
/// 5. 对每个文件计算哈希并复制/移动
///
/// # 错误处理
///
/// - 源目录和目标目录相同：程序退出
/// - 源目录不存在：程序退出
/// - 单个文件处理失败：记录错误但继续处理其他文件
///
/// # 返回值
///
/// * `Ok(())` - 程序成功执行
/// * `Err(anyhow::Error)` - 程序执行失败
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 解析命令行参数
    let args = Args::parse();

    // 验证源目录和目标目录不能相同
    if args.source == args.target {
        anyhow::bail!("源目录和目标目录不能相同");
    }

    // 验证源目录是否存在
    if !args.source.exists() {
        anyhow::bail!("源目录不存在: {}", args.source.display());
    }

    // 显示程序信息
    println!("{} 文件复制并重命名工具 {}", "=".repeat(15), "=".repeat(15));
    println!("源目录: {}", args.source.display());
    println!("目标目录: {}", args.target.display());
    println!();

    // 确保目标目录存在
    ensure_directory_exists(&args.target).await?;

    // 解析文件扩展名参数（不带点）
    let allowed_extensions: Vec<String> = args
        .extensions
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    if allowed_extensions.is_empty() {
        anyhow::bail!("扩展名列表不能为空");
    }

    println!("文件扩展名: {}", allowed_extensions.join(", "));
    println!();

    // 使用函数式编程风格收集符合条件的文件
    let files_to_process: Vec<walkdir::DirEntry> = WalkDir::new(&args.source)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            // 跳过隐藏文件和目录
            !name.starts_with('.')
        })
        .filter_map(Result::ok) // 忽略遍历错误
        .filter(|entry| entry.file_type().is_file()) // 只要文件
        .filter_map(|entry| {
            // 检查文件扩展名（不带点，小写）
            let ext = entry
                .path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_lowercase())
                .unwrap_or_default();

            if allowed_extensions.contains(&ext) {
                Some(entry)
            } else {
                None
            }
        })
        .collect();

    // 处理收集到的文件
    for entry in files_to_process {
        if let Err(e) = process_file(entry.path(), &args.target, args.move_after_copy).await {
            println!("处理 {} 失败: {}", entry.path().display(), e);
        }
    }

    println!("操作成功完成！");
    Ok(())
}
