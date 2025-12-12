//! # Claude Code 配置命令
//!
//! 用于配置 @anthropic-ai/claude-code 的全局配置文件。

use anyhow::{Context, Result};
use clap::Args;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Claude Code 配置参数
#[derive(Args, Debug)]
#[command(name = "claude_code")]
#[command(version = "0.1.0")]
#[command(
    about = "配置 @anthropic-ai/claude-code 的全局配置文件",
    long_about = "自动修改 ~/.claude/settings.json 配置文件，支持 deepseek 和 moonshot 平台。\n需要提供 API 密钥参数。"
)]
pub struct ClaudeCodeArgs {
    /// 配置平台
    ///
    /// 指定要配置的平台类型，支持 deepseek 或 moonshot。
    /// deepseek: 使用 DeepSeek API
    /// moonshot: 使用 Moonshot API
    #[arg(
        short = 'p',
        long,
        value_name = "PLATFORM",
        help = "配置平台 (deepseek 或 moonshot)",
        long_help = "指定要配置的平台类型：\n- deepseek: 使用 DeepSeek API (需要设置 DEEPSEEK_API_KEY 环境变量)\n- moonshot: 使用 Moonshot API (需要设置 YOUR_MOONSHOT_API_KEY 环境变量)"
    )]
    pub platform: String,

    /// API 密钥
    ///
    /// 用于访问 API 的认证令牌。
    /// 必须提供此参数，用于配置 ANTHROPIC_AUTH_TOKEN。
    #[arg(
        short = 'k',
        long,
        value_name = "API_KEY",
        help = "API 密钥 (必须)",
        long_help = "用于访问 API 的认证令牌，必须提供此参数。\n将用于配置 ANTHROPIC_AUTH_TOKEN 环境变量。"
    )]
    pub api_key: String,

    /// 自动安装 @anthropic-ai/claude-code
    ///
    /// 开启此选项会自动执行 npm install -g @anthropic-ai/claude-code@latest
    /// 确保 claude-code 命令可用。
    #[arg(
        short = 'i',
        long,
        help = "自动安装 @anthropic-ai/claude-code",
        long_help = "开启此选项会自动执行 npm install -g @anthropic-ai/claude-code@latest\n确保 claude-code 命令可用。"
    )]
    pub install: bool,
}

/// 配置 DeepSeek 平台
fn configure_deepseek(api_key: String, config_path: &PathBuf, config: &mut Value) -> Result<()> {
    let env_config = json!({
        "ANTHROPIC_AUTH_TOKEN": api_key,
        "ANTHROPIC_BASE_URL": "https://api.deepseek.com/anthropic",
        "ANTHROPIC_MODEL": "deepseek-chat",
        "ANTHROPIC_SMALL_FAST_MODEL": "deepseek-chat",
        "API_TIMEOUT_MS": "3000000",
        "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": 1
    });

    config["env"] = env_config;

    println!("✅ DeepSeek 平台配置完成!");
    println!("   基础 URL: https://api.deepseek.com/anthropic");
    println!("   模型: deepseek-chat");
    println!("   配置文件已保存至: {}", config_path.display());
    println!("\n使用说明:");
    println!("   1. 运行 claude-code 命令时，会自动使用此配置");

    Ok(())
}

/// 配置 Moonshot 平台
fn configure_moonshot(api_key: String, config_path: &PathBuf, config: &mut Value) -> Result<()> {
    let env_config = json!({
        "ANTHROPIC_AUTH_TOKEN": api_key,
        "ANTHROPIC_BASE_URL": "https://api.moonshot.cn/anthropic",
        "ANTHROPIC_MODEL": "kimi-k2-thinking-turbo",
        "ANTHROPIC_SMALL_FAST_MODEL": "kimi-k2-thinking-turbo",
        "API_TIMEOUT_MS": "3000000",
        "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": 1
    });

    config["env"] = env_config;

    println!("✅ Moonshot 平台配置完成!");
    println!("   基础 URL: https://api.moonshot.cn/anthropic");
    println!("   模型: kimi-k2-thinking-turbo");
    println!("   配置文件已保存至: {}", config_path.display());
    println!("\n使用说明:");
    println!("   1. 运行 claude-code 命令时，会自动使用此配置");

    Ok(())
}

/// 安装 @anthropic-ai/claude-code
fn install_claude_code() -> Result<()> {
    println!("正在安装 @anthropic-ai/claude-code...");

    let mut child = Command::new("npm")
        .args(["install", "-g", "@anthropic-ai/claude-code@latest"])
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .context("无法执行 npm 命令，请确保 Node.js 和 npm 已安装")?;

    let status = child.wait().context("等待 npm 命令完成失败")?;

    if status.success() {
        println!("✅ @anthropic-ai/claude-code 安装成功!");
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "安装失败，退出码: {}",
            status.code().unwrap_or(-1)
        ))
    }
}

/// 获取默认配置文件路径
fn get_default_config_path() -> Result<PathBuf> {
    let home_dir = dirs::home_dir().context("无法获取用户主目录")?;
    Ok(home_dir.join(".claude").join("settings.json"))
}

/// 读取现有配置文件
fn read_existing_config(config_path: &PathBuf) -> Result<Value> {
    if config_path.exists() {
        let content = fs::read_to_string(config_path)
            .context(format!("无法读取配置文件: {}", config_path.display()))?;
        serde_json::from_str(&content).context("配置文件格式错误")
    } else {
        Ok(json!({}))
    }
}

/// 创建配置目录
fn ensure_config_dir(config_path: &PathBuf) -> Result<()> {
    if let Some(parent) = config_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .context(format!("无法创建配置目录: {}", parent.display()))?;
        }
    }
    Ok(())
}

/// 运行 Claude Code 配置命令
pub async fn run(args: ClaudeCodeArgs) -> Result<()> {
    println!("正在配置 Claude Code...");

    // 如果需要安装，先安装 claude-code
    if args.install {
        install_claude_code()?;
    }

    println!("平台: {}", args.platform);

    // 获取配置文件路径
    let config_path = get_default_config_path()?;

    println!("配置文件: {}", config_path.display());

    // 读取现有配置
    let mut config = read_existing_config(&config_path)?;

    // 根据平台调用不同的配置函数
    match args.platform.to_lowercase().as_str() {
        "deepseek" => configure_deepseek(args.api_key, &config_path, &mut config)?,
        "moonshot" => configure_moonshot(args.api_key, &config_path, &mut config)?,
        _ => unreachable!(),
    }

    // 确保配置目录存在
    ensure_config_dir(&config_path)?;

    // 写入配置文件
    let config_str = serde_json::to_string_pretty(&config).context("无法序列化配置为 JSON")?;

    fs::write(&config_path, config_str)
        .context(format!("无法写入配置文件: {}", config_path.display()))?;

    println!("\n如需切换平台，使用: scripts claude-code --platform <平台>");

    Ok(())
}
