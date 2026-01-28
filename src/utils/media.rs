//! # 媒体工具模块
//!
//! 提供媒体处理相关的工具函数，例如测试编码器可用性。

use anyhow::{Context, Result};
use std::process::{Command as StdCommand, Stdio};

/// 确保 ffmpeg 可用
///
/// 检测系统中是否安装了 ffmpeg，如果未安装则使用 winget 自动安装。
///
/// # 返回值
///
/// * `Ok(())` - ffmpeg 可用或安装成功
/// * `Err(anyhow::Error)` - 安装失败
///
/// # 技术细节
///
/// - 通过执行 `ffmpeg -version` 检测 ffmpeg 是否可用
/// - 若不可用，使用 `winget install ffmpeg` 进行安装
/// - winget 安装时 stdout/stderr 继承到当前终端，方便用户查看安装进度
///
/// # 示例
///
/// ```rust
/// use scripts::utils::media::ensure_ffmpeg;
///
/// fn main() -> anyhow::Result<()> {
///     ensure_ffmpeg()?;
///     println!("ffmpeg 已就绪");
///     Ok(())
/// }
/// ```
pub fn ensure_ffmpeg() -> Result<()> {
    // 检测 ffmpeg 是否可用
    let check = StdCommand::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    match check {
        Ok(status) if status.success() => return Ok(()),
        _ => {}
    }

    // ffmpeg 不可用，使用 winget 安装
    println!("ffmpeg 未安装，正在使用 winget 安装...");

    let install_status = StdCommand::new("winget")
        .arg("install")
        .arg("ffmpeg")
        .arg("--disable-interactivity")
        .arg("--accept-source-agreements")
        .arg("--accept-package-agreements")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("执行 winget 失败，请确保 winget 可用")?;

    if !install_status.success() {
        anyhow::bail!("winget 安装 ffmpeg 失败");
    }

    println!("ffmpeg 安装完成");
    Ok(())
}

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
        .arg("testsrc=duration=1:size=320x240:rate=1")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("-frames:v")
        .arg("1")
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
