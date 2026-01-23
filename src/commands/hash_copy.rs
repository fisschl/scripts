//! # 哈希复制工具 (hash_copy)
//!
//! 一个简洁高效的 Rust 命令行工具，用于将源目录中的文件复制到目标目录，
//! 并使用 Blake3 哈希值重命名以避免重复。

use crate::utils::filesystem::get_file_extension;
use crate::utils::hash::calculate_file_hash;
use anyhow::{Context, Result};
use clap::Args;
use std::path::{Path, PathBuf};
use trash;
use walkdir::WalkDir;

/// 命令行参数结构体
///
/// 使用 clap 的 Args API 自动解析命令行参数，
/// 提供类型安全和自动生成的帮助信息。
#[derive(Args, Debug)]
#[command(name = "hash_copy")]
#[command(version = "0.1.0")]
#[command(
    about = "将文件从源目录复制到目标目录，使用哈希值重命名",
    long_about = "递归遍历源目录，将匹配的文件复制到目标目录，并用 Blake3 哈希重命名以避免重复。可选移动模式在复制成功后删除源文件。"
)]
pub struct HashCopyArgs {
    /// 源目录路径
    ///
    /// 包含要复制的文件的目录。工具会递归遍历这个目录。
    /// 默认为 "./source"。
    #[arg(
        short = 's',
        long,
        default_value = "./source",
        value_name = "SOURCE_DIR",
        help = "源目录",
        long_help = "递归遍历该目录中的文件。默认 ./source。"
    )]
    pub source: PathBuf,

    /// 目标目录路径
    ///
    /// 复制文件的输出目录。如果不存在会自动创建。
    /// 默认为 "./target"。
    #[arg(
        short = 't',
        long,
        default_value = "./target",
        value_name = "TARGET_DIR",
        help = "目标目录",
        long_help = "复制到该目录；若不存在将自动创建。默认 ./target。"
    )]
    pub target: PathBuf,

    /// 要处理的文件扩展名
    ///
    /// 指定要处理的文件扩展名，多个扩展名用逗号分隔。
    /// 例如：mp4,webm,m4v
    /// 默认为常见视频格式。
    #[arg(
        short = 'e',
        long,
        default_value = "mp4,webm,m4v,avi,mkv,mov",
        value_name = "EXTENSIONS",
        help = "要处理的扩展名列表",
        long_help = "逗号分隔，不带点，大小写不敏感。例如：mp4,webm,m4v。"
    )]
    pub extensions: String,

    /// 移动模式
    ///
    /// 启用后，复制成功会删除源文件（相当于移动操作）。
    /// 默认为禁用（仅复制）。
    #[arg(
        short = 'm',
        long,
        help = "启用移动模式",
        long_help = "开启后在复制成功后删除源文件（相当于移动）。默认关闭，仅复制不删除源文件。"
    )]
    pub move_after_copy: bool,
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
pub async fn process_file(
    file_path: &Path,
    target_dir: &Path,
    move_after_copy: bool,
) -> Result<()> {
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
    let ext = get_file_extension(file_path);

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
        trash::delete(file_path)
            .with_context(|| format!("无法将源文件移动到回收站: {}", file_path.display()))?;

        println!("已将源文件移动到回收站: {}", file_name);
    }

    Ok(())
}

/// 命令执行函数
///
/// 负责协调整个文件复制和重命名流程：
/// 1. 验证源目录和目标目录
/// 2. 确保目标目录存在
/// 3. 递归处理源目录中的所有文件
/// 4. 对每个文件计算哈希并复制/移动
///
/// # 参数
///
/// * `args` - 命令行参数
///
/// # 返回值
///
/// * `Ok(())` - 程序成功执行
/// * `Err(anyhow::Error)` - 程序执行失败
pub async fn run(args: HashCopyArgs) -> anyhow::Result<()> {
    // 验证源目录和目标目录不能相同
    if args.source == args.target {
        anyhow::bail!("源目录和目标目录不能相同");
    }

    // 验证源目录是否存在
    if !args.source.exists() {
        anyhow::bail!("源目录不存在: {}", args.source.display());
    }

    // 显示程序信息
    println!("{} 哈希复制工具 {}", "=".repeat(15), "=".repeat(15));
    println!("源目录: {}", args.source.display());
    println!("目标目录: {}", args.target.display());
    println!();

    // 确保目标目录存在
    if !args.target.exists() {
        tokio::fs::create_dir_all(&args.target)
            .await
            .with_context(|| format!("创建目录失败: {}", args.target.display()))?;
    }

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
            let ext = get_file_extension(entry.path());

            if allowed_extensions.contains(&ext) {
                Some(entry)
            } else {
                None
            }
        })
        .collect();

    // 处理收集到的文件，遇到失败直接返回错误
    for entry in files_to_process {
        process_file(entry.path(), &args.target, args.move_after_copy)
            .await
            .with_context(|| format!("处理 {} 失败", entry.path().display()))?;
    }

    println!("操作成功完成！");
    Ok(())
}
