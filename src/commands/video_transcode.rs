use crate::utils::filesystem::get_file_extension;
use crate::utils::media::{detect_available_encoder, transcode_to_webm_av1};
use anyhow::{Context, Result};
use clap::Args;
use std::path::{Path, PathBuf};

#[derive(Args, Debug)]
#[command(name = "video_transcode")]
#[command(version = "0.1.0")]
#[command(
    about = "将视频文件转码为 WebM AV1 格式",
    long_about = "扫描指定目录（最多嵌套三层）下的视频文件，转换为 WebM AV1 格式。转换后的文件路径与源文件一致，扩展名为 .webm。如果目标文件已存在，则覆盖。"
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
}

fn collect_video_files(source_dir: &Path, max_depth: usize) -> Vec<PathBuf> {
    let video_extensions = [
        "mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "m4v", "3gp", "ts", "mts", "m2ts",
    ];

    let mut video_files = Vec::new();

    for entry in walkdir::WalkDir::new(source_dir)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let ext = get_file_extension(path);
        if !ext.is_empty() && video_extensions.contains(&ext.as_str()) {
            video_files.push(path.to_path_buf());
        }
    }

    video_files
}

async fn transcode_video(source_path: &Path, encoder: &str) -> Result<()> {
    let ext = get_file_extension(source_path);
    if ext == "webm" {
        println!("跳过 WebM 文件: {}", source_path.display());
        return Ok(());
    }

    let output_path = source_path.with_extension("webm");
    transcode_to_webm_av1(source_path, &output_path, encoder).await
}

pub async fn run(args: VideoTranscodeArgs) -> Result<()> {
    let source_dir = args
        .source
        .canonicalize()
        .with_context(|| format!("无法访问源目录: {}", args.source.display()))?;

    if !source_dir.is_dir() {
        anyhow::bail!("源路径必须是目录: {}", source_dir.display());
    }

    let encoder = detect_available_encoder().map_err(|e| anyhow::anyhow!("{}", e))?;

    println!("{} 视频转码工具 {}", "=".repeat(15), "=".repeat(15));
    println!("源目录: {}", source_dir.display());
    println!("编码器: {}", encoder);
    println!("编码质量: CRF=25");
    println!("目标格式: WebM (AV1 + Opus)");
    println!();

    let video_files = collect_video_files(&source_dir, 3);

    if video_files.is_empty() {
        println!("没有找到视频文件");
        return Ok(());
    }

    println!("找到 {} 个视频文件\n", video_files.len());

    for (index, video_file) in video_files.iter().enumerate() {
        println!("进度: {}/{}", index + 1, video_files.len());
        transcode_video(video_file, &encoder).await?;
        println!();
    }

    println!("操作成功完成！");
    Ok(())
}
