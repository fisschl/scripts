//! 视频转码命令模块
//!
//! 本模块提供将视频文件转码为 AV1 格式的功能。
//! 支持 WebM (AV1 + Opus) 和 MP4 (AV1 + AAC) 两种容器格式。
//!
//! # 功能特性
//!
//! - 递归扫描目录,最多支持 3 层嵌套
//! - 支持多种输入视频格式 (mp4, mkv, avi, mov 等)
//! - 转码为 AV1 编码,质量参数 CRF=25
//! - 保留原始文件路径,根据目标格式更新扩展名
//! - 如果目标文件已存在则覆盖

use crate::utils::filesystem::get_file_extension;
use crate::utils::media::{transcode_to_mp4_av1, transcode_to_webm_av1};
use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use std::fmt::Debug;
use std::path::{Path, PathBuf};

/// 目标视频格式
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum TargetFormat {
    /// WebM 格式 (AV1 + Opus)
    #[default]
    Webm,
    /// MP4 格式 (AV1 + AAC)
    Mp4,
}

/// 视频转码命令行参数
#[derive(Args, Debug)]
#[command(name = "video_transcode")]
#[command(version = "0.1.0")]
#[command(
    about = "将视频文件转码为 AV1 格式",
    long_about = "扫描指定目录(最多嵌套三层)下的视频文件,转换为 AV1 格式。支持 WebM 和 MP4 两种容器格式。转换后的文件路径与源文件一致,扩展名根据目标格式变化。如果目标文件已存在,则覆盖。"
)]
pub struct VideoTranscodeArgs {
    /// 源目录路径
    #[arg(
        short = 's',
        long,
        value_name = "SOURCE_DIRECTORY",
        help = "源目录路径（必须为目录）",
        long_help = "指定要扫描的源目录，工具会扫描该目录及其子目录（最多三层）中的视频文件。"
    )]
    pub source: PathBuf,

    /// 目标格式
    #[arg(
        short = 'f',
        long,
        value_enum,
        default_value_t = TargetFormat::Webm,
        help = "目标视频格式",
        long_help = "指定转码后的目标格式：webm (AV1 + Opus) 或 mp4 (AV1 + AAC)。"
    )]
    pub format: TargetFormat,
}

/// 收集指定目录下的所有视频文件
///
/// # 参数
///
/// * `source_dir` - 源目录路径
/// * `max_depth` - 最大扫描深度
///
/// # 返回
///
/// 返回找到的所有视频文件路径列表
fn collect_video_files(source_dir: &Path, max_depth: usize) -> Vec<PathBuf> {
    // 支持的视频文件扩展名列表
    let video_extensions = [
        "mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "m4v", "3gp", "ts", "mts", "m2ts",
    ];

    let mut video_files = Vec::new();

    // 递归遍历目录,收集所有视频文件
    for entry in walkdir::WalkDir::new(source_dir)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // 跳过非文件项
        if !path.is_file() {
            continue;
        }

        // 检查文件扩展名是否为视频格式
        let ext = get_file_extension(path);
        if !ext.is_empty() && video_extensions.contains(&ext.as_str()) {
            video_files.push(path.to_path_buf());
        }
    }

    video_files
}

/// 转码单个视频文件为指定格式
///
/// # 参数
///
/// * `source_path` - 源视频文件路径
/// * `format` - 目标格式 (WebM 或 MP4)
///
/// # 返回
///
/// 转码成功返回 `Ok(())`,失败返回错误信息
///
/// # 错误
///
/// 当转码过程失败时返回错误
async fn transcode_video(source_path: &Path, format: TargetFormat) -> Result<()> {
    match format {
        TargetFormat::Webm => {
            let output_path = source_path.with_extension("webm");
            transcode_to_webm_av1(source_path, &output_path).await
        },
        TargetFormat::Mp4 => {
            let output_path = source_path.with_extension("mp4");
            transcode_to_mp4_av1(source_path, &output_path).await
        },
    }
}

/// 执行视频转码命令
///
/// # 参数
///
/// * `args` - 命令行参数,包含源目录和目标格式
///
/// # 返回
///
/// 执行成功返回 `Ok(())`,失败返回错误信息
///
/// # 错误
///
/// - 当源目录不存在或无法访问时返回错误
/// - 当源路径不是目录时返回错误
/// - 当转码过程失败时返回错误
pub async fn run(args: VideoTranscodeArgs) -> Result<()> {
    // 规范化源目录路径并检查可访问性
    let source_dir = args
        .source
        .canonicalize()
        .with_context(|| format!("无法访问源目录: {}", args.source.display()))?;

    // 确保源路径是目录而非文件
    if !source_dir.is_dir() {
        anyhow::bail!("源路径必须是目录: {}", source_dir.display());
    }

    // 打印转码任务信息
    println!("{} 视频转码工具 {}", "=".repeat(15), "=".repeat(15));
    println!("源目录: {}", source_dir.display());
    println!("编码质量: CRF=25");
    println!();

    // 收集所有视频文件(最多扫描 3 层目录)
    let video_files = collect_video_files(&source_dir, 3);

    if video_files.is_empty() {
        println!("没有找到视频文件");
        return Ok(());
    }

    println!("找到 {} 个视频文件\n", video_files.len());

    // 逐个转码视频文件
    for (index, video_file) in video_files.iter().enumerate() {
        println!("进度: {}/{}", index + 1, video_files.len());
        transcode_video(video_file, args.format).await?;
        println!();
    }

    println!("操作成功完成！");
    Ok(())
}
