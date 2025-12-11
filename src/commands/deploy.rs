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
//!     },
//!     "s3-storage": {
//!       "type": "s3",
//!       "access-key-id": "AKIAIOSFODNN7EXAMPLE",
//!       "secret-access-key": "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
//!       "region": "us-east-1",
//!       "endpoint-url": "https://s3.amazonaws.com"
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

use crate::utils::s3::S3Manager;
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

/// 部署配置顶层结构体
///
/// 从 JSON 文件反序列化，包含全部部署所需信息。
/// 包括服务器连接配置映射表和按顺序执行的部署步骤列表。
#[derive(Debug, Deserialize)]
pub struct DeployConfig {
    /// 服务器提供者配置映射表
    ///
    /// 键为提供者名称（如 "prod", "s3-storage"），值为对应的连接配置。
    /// 支持 SSH 和 S3 两种类型的提供者。
    pub providers: HashMap<String, ProviderConfig>,
    /// 部署步骤列表
    ///
    /// 按顺序执行的部署步骤，可以是文件上传或远程命令执行。
    pub steps: Vec<Step>,
}

/// 服务器提供者连接配置枚举
///
/// 使用标签化枚举（internally tagged enum）区分不同类型的远程连接配置。
/// 目前支持 SSH 和 S3 两种连接类型。
#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ProviderConfig {
    /// SSH 连接配置
    ///
    /// 用于通过 SSH 协议连接远程服务器。
    Ssh {
        /// 远程服务器主机名或 IP 地址
        host: String,
        /// SSH 登录用户名
        user: String,
        /// SSH 端口号（通常为 22）
        port: u16,
        /// SSH 登录密码
        password: String,
    },
    /// S3 对象存储连接配置
    ///
    /// 用于连接 AWS S3 或兼容 S3 接口的对象存储服务。
    S3 {
        /// AWS 访问密钥 ID（Access Key ID）
        ///
        /// 用于身份验证的访问密钥标识符。
        access_key_id: String,
        /// AWS 秘密访问密钥（Secret Access Key）
        ///
        /// 与 Access Key ID 配对的秘密密钥，用于签名验证。
        secret_access_key: String,
        /// AWS 区域（Region）
        ///
        /// 指定 S3 服务所在的区域，如 "us-east-1"。
        region: String,
        /// S3 服务端点 URL
        ///
        /// AWS S3 或兼容 S3 服务的 API 端点地址。
        endpoint_url: String,
    },
}

/// 部署步骤定义枚举
///
/// 表示部署过程中可以执行的不同类型的操作。
/// 目前支持文件上传和远程命令执行两种步骤类型。
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum Step {
    /// 文件上传步骤
    ///
    /// 将本地文件或目录上传到远程服务器（SSH）或 S3 存储桶。
    Upload {
        /// 步骤名称
        ///
        /// 用于在部署日志中标识此步骤，有助于识别执行进度。
        name: String,
        /// 目标提供者名称
        ///
        /// 引用 DeployConfig.providers 中定义的提供者配置名称。
        provider: String,
        /// 本地文件或目录路径
        ///
        /// 相对于当前工作目录的本地文件或目录路径。
        local: String,
        /// 远程目标路径
        ///
        /// 对于 SSH：远程服务器上的目标路径。
        /// 对于 S3：存储桶中的目标对象键（key）。
        remote: String,
        /// 文件权限模式（可选）
        ///
        /// 仅适用于 SSH 上传，设置远程文件的权限（如 "755"）。
        mode: Option<String>,
    },
    /// 远程命令执行步骤
    ///
    /// 在远程服务器上执行一系列命令（仅适用于 SSH 提供者）。
    Command {
        /// 步骤名称
        ///
        /// 用于在部署日志中标识此步骤，有助于识别执行进度。
        name: String,
        /// 目标提供者名称
        ///
        /// 引用 DeployConfig.providers 中定义的 SSH 提供者配置名称。
        provider: String,
        /// 工作目录
        ///
        /// 执行命令时所在的远程服务器目录路径。
        workdir: String,
        /// 命令列表
        ///
        /// 按顺序执行的远程命令字符串列表。
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

    // 创建 SSH 和 S3 连接哈希表
    let mut ssh_connections: HashMap<String, SSHServer> = HashMap::new();
    let mut s3_connections: HashMap<String, S3Manager> = HashMap::new();

    // 遍历 provider，依次创建连接
    println!("建立连接...");
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
                    .with_context(|| format!("创建 provider '{}' 的 SSH 连接失败", name))?;
                ssh_connections.insert(name.clone(), server);
            }
            ProviderConfig::S3 {
                access_key_id,
                secret_access_key,
                region,
                endpoint_url,
            } => {
                let manager =
                    S3Manager::new(access_key_id, secret_access_key, region, endpoint_url)
                        .await
                        .with_context(|| format!("创建 provider '{}' 的 S3 连接失败", name))?;
                s3_connections.insert(name.clone(), manager);
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

                // 查找 provider 配置以确定类型
                let provider_config = config
                    .providers
                    .get(provider)
                    .with_context(|| format!("Provider '{}' 未定义", provider))?;

                match provider_config {
                    ProviderConfig::Ssh { .. } => {
                        let server = ssh_connections
                            .get(provider)
                            .with_context(|| format!("Provider '{}' 未找到 SSH 连接", provider))?;
                        execute_ssh_upload(server, local, remote, mode.as_deref())
                            .await
                            .with_context(|| {
                                format!("步骤 {}/{} 执行失败", step_num, total_steps)
                            })?;
                    }
                    ProviderConfig::S3 { .. } => {
                        let manager = s3_connections
                            .get(provider)
                            .with_context(|| format!("Provider '{}' 未找到 S3 连接", provider))?;
                        execute_s3_upload(manager, local, remote)
                            .await
                            .with_context(|| {
                                format!("步骤 {}/{} 执行失败", step_num, total_steps)
                            })?;
                    }
                }
            }
            Step::Command {
                name,
                provider,
                workdir,
                commands,
            } => {
                println!("[步骤 {}/{}] {}", step_num, total_steps, name);
                let server = ssh_connections
                    .get(provider)
                    .with_context(|| format!("Provider '{}' 未定义", provider))?;
                execute_command_step(server, provider, workdir, commands)
                    .await
                    .with_context(|| format!("步骤 {}/{} 执行失败", step_num, total_steps))?;
            }
        }
    }

    // 关闭所有 SSH 连接
    for (provider, server) in ssh_connections {
        println!("  → 关闭 {} 的连接", provider);
        if let Err(e) = server.close().await {
            eprintln!("警告: 关闭连接 {} 失败: {}", provider, e);
        }
    }

    // 显示完成信息
    println!("操作成功完成！");
    Ok(())
}

/// 执行 SSH 文件上传
///
/// 通过 SFTP 协议上传文件或目录到远程 SSH 服务器。
/// 支持文件和目录两种模式，目录模式会同步整个目录内容。
async fn execute_ssh_upload(
    server: &SSHServer,
    local: &str,
    remote: &str,
    mode: Option<&str>,
) -> Result<()> {
    println!("  → 目标: SSH 服务器");
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

/// 执行 S3 文件上传
///
/// 上传本地文件或目录到 S3 存储桶。
/// remote 参数格式: "bucket-name/path/to/object"
async fn execute_s3_upload(manager: &S3Manager, local: &str, remote: &str) -> Result<()> {
    println!("  → 目标: S3 对象存储");
    println!("  → 本地: {}", local);
    println!("  → 远程: {}", remote);

    let local_path = Path::new(local);
    if !local_path.exists() {
        anyhow::bail!("本地路径不存在: {}", local);
    }

    // 解析 remote: "bucket-name/path/to/object"
    let (bucket, s3_prefix) = remote
        .split_once('/')
        .with_context(|| "S3 remote 格式错误，应为: bucket-name/path/to/object")?;

    if local_path.is_file() {
        // 上传单个文件
        let file_name = local_path
            .file_name()
            .and_then(|n| n.to_str())
            .context("无效的文件名")?;
        let s3_key = if s3_prefix.is_empty() {
            file_name.to_string()
        } else {
            format!("{}/{}", s3_prefix, file_name)
        };

        manager.upload_file(bucket, local_path, &s3_key).await?;
        println!("  ✓ 文件上传成功: s3://{}/{}", bucket, s3_key);
    } else if local_path.is_dir() {
        // 同步整个目录
        manager.upload_dir(bucket, local_path, s3_prefix).await?;
        println!("  ✓ 目录同步完成: s3://{}/{}", bucket, s3_prefix);
    } else {
        anyhow::bail!("不支持的本地路径类型: {}", local);
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
