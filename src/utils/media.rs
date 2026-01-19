//! # 媒体工具模块
//!
//! 提供媒体处理相关的工具函数，例如测试编码器可用性。

use anyhow::{Context, Result};
use cached::proc_macro::cached;
use std::path::Path;
use std::process::{Command as StdCommand, Stdio};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// 测试指定的视频编码器是否可用
///
/// 使用 ffmpeg 测试编码器是否可用，通过生成一个 1 秒的测试视频并使用指定编码器进行编码。
///
/// # 参数
///
/// * `encoder` - 编码器名称，例如 "av1_nvenc", "svt-av1", "libsvtav1" 等
///
/// # 返回值
///
/// * `true` - 编码器可用
/// * `false` - 编码器不可用或 ffmpeg 未安装
///
/// # 技术细节
///
/// - 使用 ffmpeg 的 lavfi 滤镜生成测试源（1秒 320x240 视频）
/// - 使用 null 格式丢弃输出，只测试编码能力
/// - 编码成功返回 true，失败或 ffmpeg 未安装返回 false
///
/// # 示例
///
/// ```rust
/// use scripts::utils::media::test_encoder;
///
/// if test_encoder("svt-av1") {
///     println!("SVT-AV1 编码器可用");
/// } else {
///     println!("SVT-AV1 编码器不可用");
/// }
/// ```
pub fn test_encoder(encoder: &str) -> bool {
    let result = StdCommand::new("ffmpeg")
        .arg("-f")
        .arg("lavfi")
        .arg("-i")
        .arg("testsrc=duration=1:size=320x240")
        .arg("-c:v")
        .arg(encoder)
        .arg("-f")
        .arg("null")
        .arg("-y")
        .arg("-")
        .output();

    match result {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

/// 获取可用的 AV1 编码器（带缓存）
///
/// 按优先级顺序检测系统中可用的 AV1 编码器，首次检测后缓存结果。
///
/// # 编码器优先级
///
/// 1. `av1_nvenc` - NVIDIA GPU (NVENC)
/// 2. `av1_qsv` - Intel GPU (Quick Sync Video)
/// 3. `av1_amf` - AMD GPU (AMF)
/// 4. `svt-av1` - SVT-AV1 (Multi-thread)
/// 5. `libsvtav1` - SVT-AV1 (libsvtav1)
///
/// # 返回值
///
/// * `Ok(String)` - 可用编码器名称
/// * `Err(anyhow::Error)` - 未找到可用的 AV1 编码器
///
/// # 技术细节
///
/// - 使用 `cached` 宏缓存检测结果，避免重复检测
/// - 按优先级顺序测试编码器，返回第一个可用的编码器
///
/// # 示例
///
/// ```rust
/// use scripts::utils::media::detect_available_encoder;
///
/// match detect_available_encoder() {
///     Ok(encoder) => println!("使用编码器: {}", encoder),
///     Err(e) => eprintln!("错误: {}", e),
/// }
/// ```
#[cached]
pub fn detect_available_encoder() -> Result<String, String> {
    let priority_encoders = vec![
        ("av1_nvenc", "NVIDIA GPU (NVENC)"),
        ("av1_qsv", "Intel GPU (Quick Sync Video)"),
        ("av1_amf", "AMD GPU (AMF)"),
        ("svt-av1", "SVT-AV1 (Multi-thread)"),
        ("libsvtav1", "SVT-AV1 (libsvtav1)"),
    ];

    for (encoder, _desc) in priority_encoders {
        if test_encoder(encoder) {
            return Ok(encoder.to_string());
        }
    }

    Err("未找到可用的 AV1 编码器，请检查硬件驱动或安装支持 AV1 的 ffmpeg".to_string())
}

/// 将视频文件转码为 WebM AV1 格式
///
/// 使用指定的 AV1 编码器将视频文件转换为 WebM 格式，音频使用 Opus 编码。
///
/// # 参数
///
/// * `source_path` - 源视频文件路径
/// * `output_path` - 目标 WebM 文件路径
/// * `encoder` - AV1 编码器名称（如 "svt-av1", "av1_nvenc" 等）
///
/// # 返回值
///
/// * `Ok(())` - 转码成功
/// * `Err(anyhow::Error)` - 转码失败，包含详细错误信息
///
/// # 技术细节
///
/// - 使用 ffmpeg 进行转码
/// - 视频编码: AV1 (指定编码器), CRF=25
/// - 音频编码: Opus, 128k 码率
/// - 线程数: 0 (自动检测)
/// - `-y` 参数自动覆盖已存在的输出文件
///
/// # 示例
///
/// ```rust
/// use scripts::utils::media::transcode_to_webm_av1;
/// use std::path::Path;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let source = Path::new("input.mp4");
///     let output = Path::new("output.webm");
///     transcode_to_webm_av1(source, output, "svt-av1").await?;
///     Ok(())
/// }
/// ```
pub async fn transcode_to_webm_av1(
    source_path: &Path,
    output_path: &Path,
    encoder: &str,
) -> Result<()> {
    if !source_path.is_file() {
        anyhow::bail!("源文件不存在: {}", source_path.display());
    }

    let mut cmd = Command::new("ffmpeg");
    cmd.arg("-i")
        .arg(source_path)
        .arg("-threads")
        .arg("0")
        .arg("-c:v")
        .arg(encoder)
        .arg("-crf")
        .arg("25")
        .arg("-c:a")
        .arg("libopus")
        .arg("-b:a")
        .arg("128k")
        .arg("-y")
        .arg(output_path)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let mut child = cmd
        .spawn()
        .with_context(|| format!("启动 ffmpeg 失败: {}", source_path.display()))?;

    let status: std::process::ExitStatus = child
        .wait()
        .await
        .with_context(|| format!("等待 ffmpeg 完成 失败: {}", source_path.display()))?;

    if !status.success() {
        anyhow::bail!("ffmpeg 转码失败: {}", source_path.display());
    }

    println!("转码完成: {}", output_path.display());
    Ok(())
}
