//! # Tar 归档工具 (tar_archive)
//!
//! 提供使用 tar 格式压缩和解压缩文件或目录的功能。
//! 支持 tar.zst (tar + zstd) 格式，提供高效的压缩比和速度。

use anyhow::{Context, Result};
use clap::Args;
use std::fs::File;
use std::path::{Path, PathBuf};
use tar::{Archive, Builder};
use zstd::stream::{Decoder, Encoder};

/// 命令行参数结构体
#[derive(Args, Debug)]
#[command(name = "tar")]
#[command(version = "0.1.0")]
#[command(
    about = "使用 tar 格式压缩或解压缩文件和目录",
    long_about = "支持 tar.zst（tar + zstd）。当 SOURCE 为 .tar.zst 时执行解压，否则执行压缩。"
)]
pub struct TarArchiveArgs {
    /// 源路径：要压缩的文件/目录，或要解压的 .tar.zst 文件
    /// 如果是 .tar.zst 文件则执行解压，否则执行压缩
    #[arg(
        value_name = "SOURCE",
        help = "源路径（文件/目录或 .tar.zst 归档）",
        long_help = "当传入 .tar.zst 文件时，将在其所在目录解压；当传入文件或目录时，将在父目录输出同名 .tar.zst。"
    )]
    pub source: PathBuf,

    /// 压缩级别 (1-22，默认 6)
    /// 仅在压缩时有效
    #[arg(
        short = 'l',
        long,
        default_value = "6",
        help = "压缩级别 (1-22)",
        long_help = "仅在压缩时有效。数值越大压缩比越高但速度越慢；推荐 6（默认）。"
    )]
    pub level: i32,
}

/// 压缩文件或目录到 tar.zst 格式
///
/// # 参数
///
/// * `source` - 要压缩的文件或目录路径
/// * `output` - 输出的 tar.zst 文件路径
/// * `level` - zstd 压缩级别
pub async fn compress_to_tar(source: &Path, output: &Path, level: i32) -> Result<()> {
    println!("正在压缩: {} -> {}", source.display(), output.display());

    // 创建输出文件
    let output_file =
        File::create(output).with_context(|| format!("无法创建输出文件: {}", output.display()))?;

    // 创建 zstd 编码器，直接写入输出文件
    let encoder = Encoder::new(output_file, level).context("创建 zstd 编码器失败")?;

    // 创建 tar 构建器，直接写入 zstd 编码器（流式处理）
    let mut tar_builder = Builder::new(encoder);

    if source.is_file() {
        // 压缩单个文件
        let file_name = source.file_name().context("无效的文件名")?;

        tar_builder
            .append_path_with_name(source, file_name)
            .with_context(|| format!("添加文件到 tar 失败: {}", source.display()))?;
    } else if source.is_dir() {
        // 压缩整个目录
        let dir_name = source.file_name().context("无效的目录名")?;

        tar_builder
            .append_dir_all(dir_name, source)
            .with_context(|| format!("添加目录到 tar 失败: {}", source.display()))?;
    } else {
        anyhow::bail!("源路径既不是文件也不是目录: {}", source.display());
    }

    // 完成 tar 构建，这会自动 finish tar 并 flush 数据到 encoder
    let encoder = tar_builder.into_inner().context("完成 tar 归档失败")?;

    // 完成 zstd 压缩
    encoder.finish().context("完成 zstd 压缩失败")?;

    println!("压缩完成: {}", output.display());
    Ok(())
}

/// 从 tar.zst 归档中解压缩
///
/// # 参数
///
/// * `archive_path` - tar.zst 归档文件路径
/// * `output_dir` - 解压到的目标目录
pub async fn extract_from_tar(archive_path: &Path, output_dir: &Path) -> Result<()> {
    println!(
        "正在解压: {} -> {}",
        archive_path.display(),
        output_dir.display()
    );

    // 打开 tar.zst 文件
    let archive_file = File::open(archive_path)
        .with_context(|| format!("无法打开归档文件: {}", archive_path.display()))?;

    // 创建 zstd 解码器，直接从文件读取
    let decoder = Decoder::new(archive_file).context("创建 zstd 解码器失败")?;

    // 创建 tar 解析器，直接从 zstd 解码器读取（流式处理）
    let mut archive = Archive::new(decoder);

    // 确保输出目录存在
    if !output_dir.exists() {
        std::fs::create_dir_all(output_dir)
            .with_context(|| format!("创建输出目录失败: {}", output_dir.display()))?;
    }

    // 解压 tar 归档（流式读取和写入）
    archive
        .unpack(output_dir)
        .with_context(|| format!("解压 tar 归档失败: {}", archive_path.display()))?;

    println!("解压完成: {}", output_dir.display());
    Ok(())
}

/// 命令执行函数
pub async fn run(args: TarArchiveArgs) -> Result<()> {
    // 将源路径规范化为绝对路径
    let source = args
        .source
        .canonicalize()
        .with_context(|| format!("源路径不存在: {}", args.source.display()))?;

    // 根据文件扩展名判断是压缩还是解压
    let is_extract = source.to_string_lossy().ends_with(".tar.zst");

    if is_extract {
        // 解压操作：输出到源文件所在目录
        let output_dir = source
            .parent()
            .context("无法获取源文件父目录")?
            .to_path_buf();
        extract_from_tar(&source, &output_dir).await?;
    } else {
        // 压缩操作：输出到源文件/目录的父目录
        let parent_dir = source.parent().context("无法获取源路径父目录")?;

        // 获取源文件/目录名称
        let source_name = source.file_name().context("无效的源路径")?;

        // 生成输出文件路径：与源文件同名（去掉原扩展名）+ .tar.zst
        let output_file = parent_dir.join(format!("{}.tar.zst", source_name.to_string_lossy()));

        compress_to_tar(&source, &output_file, args.level).await?;
    }

    Ok(())
}
