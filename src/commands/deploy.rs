//! # 部署工具 (deploy)
//!
//! 读取 JSON 配置文件并按顺序执行部署步骤。
//! 支持通过 SSH 连接到远程服务器，执行文件上传和远程命令操作。

use anyhow::{Context, Result};
use clap::Args;
use russh::client;
use russh_keys::key;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncReadExt;

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
    /// 配置文件包含 provider（服务器连接信息）和 steps（部署步骤）。
    #[arg(
        short = 'c',
        long,
        value_name = "CONFIG",
        help = "JSON 格式的部署配置文件路径",
        long_help = "指定包含 provider 和 steps 的 JSON 配置文件"
    )]
    pub config: PathBuf,
}

/// 顶层配置结构
///
/// 包含服务器配置映射表和步骤列表。
#[derive(Debug, Deserialize)]
pub struct DeployConfig {
    /// 服务器配置映射表
    #[serde(rename = "provider")]
    pub providers: HashMap<String, ProviderConfig>,
    /// 步骤列表
    pub steps: Vec<Step>,
}

/// 服务器连接配置
///
/// 使用带标签的枚举以支持多种远程类型。
#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "lowercase")]
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
/// 定义了两种部署步骤类型。
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
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

/// SSH 客户端处理器
///
/// 实现 russh 的客户端处理器接口。
pub struct ClientHandler;

#[async_trait::async_trait]
impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // 在生产环境中应验证服务器密钥
        // 这里为了简化直接接受所有密钥
        Ok(true)
    }
}

/// SSH 连接管理器
///
/// 管理到远程服务器的 SSH 连接，支持延迟连接和缓存复用。
pub struct ConnectionManager {
    /// 缓存的 SSH 连接
    connections: HashMap<String, Arc<client::Handle<ClientHandler>>>,
    /// Provider 配置
    providers: HashMap<String, ProviderConfig>,
}

impl ConnectionManager {
    /// 创建新的连接管理器
    pub fn new(providers: HashMap<String, ProviderConfig>) -> Self {
        Self {
            connections: HashMap::new(),
            providers,
        }
    }

    /// 获取或建立到指定 provider 的连接
    ///
    /// 如果连接已存在则直接返回，否则建立新连接并缓存。
    pub async fn get_connection(
        &mut self,
        provider_name: &str,
    ) -> Result<Arc<client::Handle<ClientHandler>>> {
        // 检查缓存
        if let Some(conn) = self.connections.get(provider_name) {
            return Ok(conn.clone());
        }

        // 获取 provider 配置
        let config = self
            .providers
            .get(provider_name)
            .with_context(|| format!("Provider '{}' 未定义", provider_name))?;

        // 根据 provider 类型建立连接
        match config {
            ProviderConfig::Ssh {
                host,
                user,
                port,
                password,
            } => {
                println!("  → 建立 SSH 连接: {}@{}:{}", user, host, port);

                let client_config = client::Config::default();
                let sh = ClientHandler;

                let mut session =
                    client::connect(Arc::new(client_config), (host.as_str(), *port), sh)
                        .await
                        .with_context(|| format!("无法连接到 {}:{}", host, port))?;

                // 密码认证
                let auth_res = session
                    .authenticate_password(user.clone(), password.clone())
                    .await
                    .with_context(|| format!("SSH 认证失败: {}@{}", user, host))?;

                if !auth_res {
                    anyhow::bail!("SSH 密码认证失败: {}@{}", user, host);
                }

                let handle = Arc::new(session);
                self.connections
                    .insert(provider_name.to_string(), handle.clone());

                Ok(handle)
            }
        }
    }

    /// 关闭所有连接
    pub async fn close_all(&mut self) {
        for (_name, handle) in self.connections.drain() {
            let _ = handle
                .disconnect(russh::Disconnect::ByApplication, "", "")
                .await;
        }
    }
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

    // 验证配置

    // 转换步骤配置为步骤枚举
    let steps = &config.steps;

    // 显示启动信息
    println!("{} Deploy 部署工具 {}", "=".repeat(15), "=".repeat(15));
    println!("配置文件: {}", config_path.display());
    println!("Provider 数量: {}", config.providers.len());
    println!("步骤数量: {}\n", steps.len());

    // 初始化连接管理器
    let mut conn_mgr = ConnectionManager::new(config.providers.clone());

    // 执行步骤
    for (index, step) in steps.iter().enumerate() {
        let step_num = index + 1;
        let total_steps = steps.len();

        execute_step(step, step_num, total_steps, &mut conn_mgr)
            .await
            .with_context(|| format!("步骤 {}/{} 执行失败", step_num, total_steps))?;
    }

    // 关闭所有连接
    conn_mgr.close_all().await;

    // 显示完成信息
    println!("\n操作成功完成！");
    Ok(())
}

/// 执行单个步骤
///
/// 根据步骤类型调用相应的执行函数。
async fn execute_step(
    step: &Step,
    step_num: usize,
    total_steps: usize,
    conn_mgr: &mut ConnectionManager,
) -> Result<()> {
    match step {
        Step::Upload {
            name,
            provider,
            local,
            remote,
            mode,
        } => {
            execute_upload_step(
                name,
                provider,
                local,
                remote,
                mode.as_deref(),
                step_num,
                total_steps,
                conn_mgr,
            )
            .await
        }
        Step::Command {
            name,
            provider,
            workdir,
            commands,
        } => {
            execute_command_step(
                name,
                provider,
                workdir,
                commands,
                step_num,
                total_steps,
                conn_mgr,
            )
            .await
        }
    }
}

/// 执行上传步骤
///
/// 通过 SFTP 上传文件到远程服务器。
async fn execute_upload_step(
    name: &str,
    provider: &str,
    local: &str,
    remote: &str,
    mode: Option<&str>,
    step_num: usize,
    total_steps: usize,
    conn_mgr: &mut ConnectionManager,
) -> Result<()> {
    println!("[步骤 {}/{}] {}", step_num, total_steps, name);
    println!("  → Provider: {}", provider);
    println!("  → 本地: {}", local);
    println!("  → 远程: {}", remote);

    // 获取连接
    let session = conn_mgr.get_connection(provider).await?;

    // 检查本地文件
    let local_path = Path::new(local);
    if !local_path.exists() {
        anyhow::bail!("本地文件不存在: {}", local);
    }

    // 读取本地文件内容
    let mut file_content = Vec::new();
    let mut file = fs::File::open(local_path)
        .await
        .with_context(|| format!("无法打开本地文件: {}", local))?;
    file.read_to_end(&mut file_content)
        .await
        .with_context(|| format!("无法读取本地文件: {}", local))?;

    // 创建 SFTP 通道
    let channel = session.channel_open_session().await?;
    channel.request_subsystem(true, "sftp").await?;

    let sftp = russh_sftp::client::SftpSession::new(channel.into_stream()).await?;

    // 上传文件
    let mut remote_file = sftp
        .create(remote)
        .await
        .with_context(|| format!("无法创建远程文件: {}", remote))?;

    tokio::io::copy(&mut file_content.as_slice(), &mut remote_file)
        .await
        .with_context(|| format!("上传文件失败: {}", remote))?;

    remote_file.sync_all().await?;
    drop(remote_file);
    drop(sftp);

    println!("  ✓ 上传成功");

    // 设置文件权限（如果指定）
    if let Some(file_mode) = mode {
        let chmod_cmd = format!("chmod {} {}", file_mode, remote);
        let mut channel = session.channel_open_session().await?;
        channel.exec(true, chmod_cmd.as_bytes()).await?;

        let mut exit_code = None;
        while let Some(msg) = channel.wait().await {
            match msg {
                russh::ChannelMsg::ExitStatus { exit_status } => {
                    exit_code = Some(exit_status);
                }
                _ => {}
            }
        }

        if exit_code != Some(0) {
            anyhow::bail!("权限设置失败: chmod {} {}", file_mode, remote);
        }

        println!("  ✓ 权限设置成功: {}", file_mode);
    }

    println!();
    Ok(())
}

/// 执行命令步骤
///
/// 在远程服务器的指定工作目录下执行命令列表。
async fn execute_command_step(
    name: &str,
    provider: &str,
    workdir: &str,
    commands: &[String],
    step_num: usize,
    total_steps: usize,
    conn_mgr: &mut ConnectionManager,
) -> Result<()> {
    println!("[步骤 {}/{}] {}", step_num, total_steps, name);
    println!("  → Provider: {}", provider);
    println!("  → 工作目录: {}", workdir);

    // 获取连接
    let session = conn_mgr.get_connection(provider).await?;

    // 执行每个命令
    if commands.is_empty() {
        anyhow::bail!("命令列表为空");
    }
    for command in commands {
        println!("  → 执行命令: {}", command);

        // 在指定工作目录下执行命令
        let full_command = format!("cd {} && {}", workdir, command);

        let mut channel = session.channel_open_session().await?;
        channel.exec(true, full_command.as_bytes()).await?;

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut exit_code = None;

        while let Some(msg) = channel.wait().await {
            match msg {
                russh::ChannelMsg::Data { ref data } => {
                    stdout.extend_from_slice(data);
                }
                russh::ChannelMsg::ExtendedData { ref data, ext: 1 } => {
                    stderr.extend_from_slice(data);
                }
                russh::ChannelMsg::ExitStatus { exit_status } => {
                    exit_code = Some(exit_status);
                }
                _ => {}
            }
        }

        // 检查退出码
        if exit_code != Some(0) {
            let stderr_str = String::from_utf8_lossy(&stderr);
            anyhow::bail!(
                "命令执行失败: {}\n  → 退出码: {}\n  → 错误输出: {}",
                command,
                exit_code.unwrap_or(u32::MAX),
                stderr_str
            );
        }

        println!("  ✓ 命令执行成功");
    }

    println!();
    Ok(())
}
