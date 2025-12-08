//! # 部署工具 (deploy)
//!
//! 读取 JSON 配置文件并按顺序执行部署步骤。
//! 支持通过 SSH 连接到远程服务器，执行文件上传和远程命令操作。
//!
//! 配置文件示例（JSON）：
//! ```json
//! {
//!   "providers": {
//!     "prod": {
//!       "type": "ssh",
//!       "host": "example.com",
//!       "user": "deploy",
//!       "port": 22,
//!       "password": "your-password"
//!     }
//!   },
//!   "steps": [
//!     {
//!       "type": "upload",
//!       "name": "上传二进制",
//!       "provider": "prod",
//!       "local": "./dist/app",
//!       "remote": "/opt/app/app",
//!       "mode": "755"
//!     },
//!     {
//!       "type": "command",
//!       "name": "重启服务",
//!       "provider": "prod",
//!       "workdir": "/opt/app",
//!       "commands": [
//!         "systemctl stop app",
//!         "systemctl start app",
//!         "systemctl status app --no-pager"
//!       ]
//!     }
//!   ]
//! }
//! ```

use crate::utils::ssh::SSHServer;
use anyhow::{Context, Result};
use clap::Args;
use serde::Deserialize;
use std::collections::HashMap;

use std::path::{Path, PathBuf};
use tokio::fs;

/// 命令行参数结构体
///
/// 使用 clap 的 Args API 自动解析命令行参数，
/// 提供类型安全和自动生成的帮助信息。
#[derive(Args, Debug)]
#[command(name = "deploy")]
#[command(version = "0.1.0")]
#[command(
    about = "读取 JSON 配置文件并执行部署步骤",
    long_about = "通过 SSH 连接远程服务器，按配置文件中定义的步骤顺序执行文件上传和命令操作。任意步骤失败时立即停止。"
)]
pub struct DeployArgs {
    /// JSON 格式的部署配置文件路径
    ///
    /// 配置文件包含 providers（服务器连接信息）和 steps（部署步骤）。
    #[arg(
        short = 'c',
        long,
        value_name = "CONFIG",
        help = "JSON 格式的部署配置文件路径",
        long_help = "指定包含 providers 和 steps 的 JSON 配置文件"
    )]
    pub config: PathBuf,
}

/// 顶层配置结构
///
/// 包含服务器配置映射表和步骤列表。
#[derive(Debug, Deserialize)]
pub struct DeployConfig {
    /// 服务器配置映射表
    pub providers: HashMap<String, ProviderConfig>,
    /// 步骤列表
    pub steps: Vec<Step>,
}

/// 服务器连接配置
///
/// 使用带标签的枚举以支持多种远程类型。
#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ProviderConfig {
    Ssh {
        host: String,
        user: String,
        port: u16,
        password: String,
    },
    // 未来可扩展更多类型，如 Sftp、Http 等
}

/// 步骤定义（枚举类型）
///
/// 1. Upload: 文件上传到远程服务器
/// 2. Command: 在远程服务器执行命令
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Step {
    /// 文件上传步骤
    Upload {
        name: String,
        provider: String,
        local: String,
        remote: String,
        mode: Option<String>,
    },
    /// 远程命令执行步骤
    Command {
        name: String,
        provider: String,
        workdir: String,
        commands: Vec<String>,
    },
}

/// 命令执行函数
///
/// 负责协调整个部署流程。
pub async fn run(args: DeployArgs) -> Result<()> {
    // 读取配置文件
    let config_path = args
        .config
        .canonicalize()
        .with_context(|| format!("无法访问配置文件: {}", args.config.display()))?;

    let config_content = fs::read_to_string(&config_path)
        .await
        .with_context(|| format!("无法读取配置文件: {}", config_path.display()))?;

    // 解析 JSON 配置
    let config: DeployConfig =
        serde_json::from_str(&config_content).with_context(|| "JSON 解析失败")?;

    // 转换步骤配置为步骤枚举
    let steps = &config.steps;

    // 显示启动信息
    println!("{} Deploy 部署工具 {}", "=".repeat(15), "=".repeat(15));
    println!("配置文件: {}", config_path.display());
    println!("Provider 数量: {}", config.providers.len());
    println!("步骤数量: {}", steps.len());

    // 创建 SSH 连接哈希表
    let mut connections: HashMap<String, SSHServer> = HashMap::new();

    // 遍历 provider，依次创建连接
    println!("建立 SSH 连接...");
    for (name, provider_config) in &config.providers {
        match provider_config {
            ProviderConfig::Ssh {
                host,
                user,
                port,
                password,
            } => {
                let server = SSHServer::new(host, *port, user, password)
                    .await
                    .with_context(|| format!("创建 provider '{}' 的连接失败", name))?;
                connections.insert(name.clone(), server);
            }
        }
    }

    // 执行步骤
    for (index, step) in steps.iter().enumerate() {
        let step_num = index + 1;
        let total_steps = steps.len();

        match step {
            Step::Upload {
                name,
                provider,
                local,
                remote,
                mode,
            } => {
                println!("[步骤 {}/{}] {}", step_num, total_steps, name);
                let server = connections
                    .get(provider)
                    .with_context(|| format!("Provider '{}' 未定义", provider))?;
                execute_upload_step(server, provider, local, remote, mode.as_deref())
                    .await
                    .with_context(|| format!("步骤 {}/{} 执行失败", step_num, total_steps))?;
            }
            Step::Command {
                name,
                provider,
                workdir,
                commands,
            } => {
                println!("[步骤 {}/{}] {}", step_num, total_steps, name);
                let server = connections
                    .get(provider)
                    .with_context(|| format!("Provider '{}' 未定义", provider))?;
                execute_command_step(server, provider, workdir, commands)
                    .await
                    .with_context(|| format!("步骤 {}/{} 执行失败", step_num, total_steps))?;
            }
        }
    }

    // 关闭所有连接
    for (provider, server) in connections {
        println!("  → 关闭 {} 的连接", provider);
        if let Err(e) = server.close().await {
            eprintln!("警告: 关闭连接 {} 失败: {}", provider, e);
        }
    }

    // 显示完成信息
    println!("操作成功完成！");
    Ok(())
}

/// 执行上传步骤
///
/// 通过 SFTP 上传文件或目录到远程服务器。
/// 支持文件和目录两种模式，目录模式会同步整个目录内容。
async fn execute_upload_step(
    server: &SSHServer,
    provider: &str,
    local: &str,
    remote: &str,
    mode: Option<&str>,
) -> Result<()> {
    println!("  → Provider: {}", provider);
    println!("  → 本地: {}", local);
    println!("  → 远程: {}", remote);

    let local_path = Path::new(local);
    if !local_path.exists() {
        anyhow::bail!("本地路径不存在: {}", local);
    }

    if local_path.is_file() {
        // 文件上传模式
        server.upload_file(local_path, remote).await?;
        println!("  ✓ 上传成功");
    } else if local_path.is_dir() {
        // 目录同步模式
        println!("  → 目录同步模式");
        server.upload_dir(local_path, remote).await?;
        println!("  ✓ 目录同步完成");
    } else {
        anyhow::bail!("不支持的本地路径类型: {}", local);
    }

    // 设置文件权限（如果指定）
    if let Some(file_mode) = mode {
        let chmod_cmd = if local_path.is_dir() {
            format!("chmod -R {} {}", file_mode, remote)
        } else {
            format!("chmod {} {}", file_mode, remote)
        };
        server.exec_command("/", &chmod_cmd).await?;
        println!("  ✓ 权限设置成功: {}", file_mode);
    }

    Ok(())
}

/// 执行命令步骤
///
/// 在远程服务器的指定工作目录下执行命令列表。
async fn execute_command_step(
    server: &SSHServer,
    provider: &str,
    workdir: &str,
    commands: &[String],
) -> Result<()> {
    println!("  → Provider: {}", provider);
    println!("  → 工作目录: {}", workdir);

    // 执行每个命令
    if commands.is_empty() {
        anyhow::bail!("命令列表为空");
    }

    let cmd_refs: Vec<&str> = commands.iter().map(|s| s.as_str()).collect();
    server.exec_commands(workdir, &cmd_refs).await?;

    println!("  ✓ 命令执行成功");
    Ok(())
}
