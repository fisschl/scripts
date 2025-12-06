//! # SSH 工具模块
//!
//! 提供 SSH 连接管理功能，包括会话创建、认证等操作。

use anyhow::{Context, Result};
use russh::client;
use russh_keys::key;
use std::io::{self, Write};
use std::path::Path;
use std::sync::Arc;
use tokio::fs;

/// SSH 客户端处理器
///
/// 实现 russh 的客户端处理器接口，用于 SSH 连接过程中的密钥验证。
/// 在生产环境中应该实现严格的密钥验证，此处为演示目的直接接受所有密钥。
pub struct ClientHandler;

#[async_trait::async_trait]
impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        // 这里为了简化直接接受所有密钥
        Ok(true)
    }
}

/// SSH 会话类型别名
pub type SshSession = Arc<client::Handle<ClientHandler>>;

/// 创建 SSH 会话
///
/// 与配置结构无关的独立函数，接收原始连接参数建立 SSH 连接。
/// 使用用户名和密码进行身份认证。
///
/// # 参数
///
/// * `host` - 远程主机地址
/// * `port` - SSH 服务端口（通常为 22）
/// * `user` - 登录用户名
/// * `password` - 登录密码
///
/// # 返回值
///
/// * `Ok(SshSession)` - 建立的 SSH 会话，可复用于多个操作
/// * `Err(anyhow::Error)` - 连接或认证失败
///
/// # 示例
///
/// ```rust
/// use file_utils::utils::ssh::create_ssh_session;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let session = create_ssh_session("example.com", 22, "user", "pass").await?;
///     Ok(())
/// }
/// ```
pub async fn create_ssh_session(
    host: &str,
    port: u16,
    user: &str,
    password: &str,
) -> Result<SshSession> {
    println!("  → 建立 SSH 连接: {}@{}:{}", user, host, port);

    let client_config = client::Config::default();
    let sh = ClientHandler;

    let mut session = client::connect(Arc::new(client_config), (host, port), sh)
        .await
        .with_context(|| format!("无法连接到 {}:{}", host, port))?;

    // 密码认证
    let auth_res = session
        .authenticate_password(user, password)
        .await
        .with_context(|| format!("SSH 认证失败: {}@{}", user, host))?;

    if !auth_res {
        anyhow::bail!("SSH 密码认证失败: {}@{}", user, host);
    }

    Ok(Arc::new(session))
}

/// 远程执行命令
///
/// 在指定工作目录下执行单条命令，返回退出码。
/// 会实时输出命令的标准输出和标准错误，便于调试和监控。
///
/// # 参数
///
/// * `session` - SSH 会话引用
/// * `workdir` - 执行命令的工作目录
/// * `cmd` - 要执行的命令
///
/// # 返回值
///
/// * `Ok(i32)` - 命令的退出码（0 表示成功）
/// * `Err(anyhow::Error)` - 执行失败或网络错误
///
/// # 示例
///
/// ```rust
/// use file_utils::utils::ssh::create_ssh_session;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let session = create_ssh_session("example.com", 22, "user", "pass").await?;
///     let exit_code = crate::utils::ssh::exec_remote_cmd(&session, "/tmp", "ls -la").await?;
///     println!("命令退出码: {}", exit_code);
///     Ok(())
/// }
/// ```
pub async fn exec_remote_cmd(session: &SshSession, workdir: &str, cmd: &str) -> Result<i32> {
    let full_cmd = format!("cd {} && {}", workdir, cmd);
    let mut channel = session.channel_open_session().await?;
    channel.exec(true, full_cmd.as_bytes()).await?;

    let mut exit_code = 0;
    while let Some(msg) = channel.wait().await {
        match msg {
            russh::ChannelMsg::Data { ref data } => {
                let chunk = String::from_utf8_lossy(data);
                print!("{}", chunk);
                let _ = io::stdout().flush();
            }
            russh::ChannelMsg::ExtendedData { ref data, ext: 1 } => {
                let chunk = String::from_utf8_lossy(data);
                eprint!("{}", chunk);
                let _ = io::stderr().flush();
            }
            russh::ChannelMsg::ExitStatus { exit_status } => {
                exit_code = exit_status as i32;
            }
            _ => {}
        }
    }
    Ok(exit_code)
}

/// 上传单个文件
///
/// 使用流式传输将本地文件上传到远程服务器。
/// 自动确保远程父目录存在，并验证传输的完整性。
///
/// # 特性
///
/// * 流式传输：适合大文件上传，内存占用小
/// * 完整性验证：对比传输字节数与源文件大小
/// * 自动创建父目录：无需手动创建远程目录
/// * 实时错误反馈：传输失败时立即返回错误
///
/// # 参数
///
/// * `session` - SSH 会话引用（用于创建父目录）
/// * `sftp` - SFTP 会话引用
/// * `local_path` - 本地文件路径
/// * `remote_path` - 远程文件目标路径
///
/// # 返回值
///
/// * `Ok(())` - 上传成功
/// * `Err(anyhow::Error)` - 本地文件不存在、目录创建失败或传输不完整
///
/// # 示例
///
/// ```rust
/// use file_utils::utils::ssh::{create_ssh_session, upload_single_file};
/// use std::path::Path;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let session = create_ssh_session("example.com", 22, "user", "pass").await?;
///     let channel = session.channel_open_session().await?;
///     channel.request_subsystem(true, "sftp").await?;
///     let sftp = russh_sftp::client::SftpSession::new(channel.into_stream()).await?;
///     
///     upload_single_file(&session, &sftp, Path::new("local.txt"), "/tmp/remote.txt").await?;
///     Ok(())
/// }
/// ```
pub async fn upload_single_file(
    session: &SshSession,
    sftp: &russh_sftp::client::SftpSession,
    local_path: &Path,
    remote_path: &str,
) -> Result<()> {
    // 检查本地文件是否存在
    if !local_path.exists() {
        anyhow::bail!("本地文件不存在: {}", local_path.display());
    }
    if !local_path.is_file() {
        anyhow::bail!("路径不是文件: {}", local_path.display());
    }

    // 确保远程父目录存在
    if let Some(parent_idx) = remote_path.rfind('/') {
        let parent_dir = &remote_path[..parent_idx];
        if !parent_dir.is_empty() {
            ensure_remote_dir_exists(session, parent_dir).await?;
        }
    }

    // 获取文件大小
    let metadata = fs::metadata(local_path)
        .await
        .with_context(|| format!("无法获取文件信息: {}", local_path.display()))?;
    let file_size = metadata.len();

    // 流式打开本地文件
    let mut local_file = fs::File::open(local_path)
        .await
        .with_context(|| format!("无法打开本地文件: {}", local_path.display()))?;

    // 创建远程文件
    let mut remote_file = sftp
        .create(remote_path)
        .await
        .with_context(|| format!("无法创建远程文件: {}", remote_path))?;

    // 流式复制（使用 tokio::io::copy 进行高效传输）
    let bytes_copied = tokio::io::copy(&mut local_file, &mut remote_file)
        .await
        .with_context(|| format!("上传文件失败: {}", remote_path))?;

    // 确保数据写入
    remote_file.sync_all().await?;

    // 验证传输完整性
    if bytes_copied != file_size {
        anyhow::bail!(
            "文件传输不完整: 期望 {} 字节，实际 {} 字节",
            file_size,
            bytes_copied
        );
    }

    Ok(())
}

/// 确保远程目录存在
///
/// 递归创建远程目录路径（类似 `mkdir -p`）。
/// 如果目录已存在或创建成功，返回 Ok；创建失败则返回错误。
///
/// # 参数
///
/// * `session` - SSH 会话引用
/// * `remote_dir` - 要创建的远程目录路径
///
/// # 返回值
///
/// * `Ok(())` - 目录已存在或创建成功
/// * `Err(anyhow::Error)` - 目录创建失败
///
/// # 示例
///
/// ```rust
/// use file_utils::utils::ssh::{create_ssh_session, ensure_remote_dir_exists};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let session = create_ssh_session("example.com", 22, "user", "pass").await?;
///     ensure_remote_dir_exists(&session, "/var/app/backup").await?;
///     println!("目录准备就绪");
///     Ok(())
/// }
/// ```
pub async fn ensure_remote_dir_exists(session: &SshSession, remote_dir: &str) -> Result<()> {
    let mkdir_cmd = format!("mkdir -p {}", remote_dir);
    let exit_code = exec_remote_cmd(session, "/", &mkdir_cmd).await?;
    if exit_code != 0 {
        anyhow::bail!("创建远程目录失败: {}", remote_dir);
    }
    Ok(())
}

/// 列举远程目录下所有文件（返回相对路径）
///
/// 递归遍历远程目录树，返回所有文件的相对路径列表。
/// 路径分隔符统一使用正斜杠 `/`，便于跨平台处理。
/// 目录本身不包含在返回结果中，只返回文件。
///
/// # 参数
///
/// * `sftp` - SFTP 会话引用
/// * `remote_dir` - 要扫描的远程目录路径
///
/// # 返回值
///
/// * `Ok(Vec<String>)` - 所有文件的相对路径列表，按遍历顺序排列
/// * `Err(anyhow::Error)` - 目录读取失败（例如权限不足）
///
/// # 示例
///
/// ```rust
/// use file_utils::utils::ssh::{create_ssh_session, list_remote_files};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let session = create_ssh_session("example.com", 22, "user", "pass").await?;
///     let channel = session.channel_open_session().await?;
///     channel.request_subsystem(true, "sftp").await?;
///     let sftp = russh_sftp::client::SftpSession::new(channel.into_stream()).await?;
///     
///     let files = list_remote_files(&sftp, "/home/user/documents").await?;
///     for file in files {
///         println!("文件: {}", file);
///     }
///     Ok(())
/// }
/// ```
pub async fn list_remote_files(
    sftp: &russh_sftp::client::SftpSession,
    remote_dir: &str,
) -> Result<Vec<String>> {
    let mut files = Vec::new();
    list_remote_files_recursive(sftp, remote_dir, remote_dir, &mut files).await?;
    Ok(files)
}

/// 递归遍历远程目录，收集所有文件的相对路径
///
/// 这是 `list_remote_files` 的内部辅助函数，不应直接调用。
/// 递归处理所有子目录，跳过 `.` 和 `..` 条目。
///
/// # 参数
///
/// * `sftp` - SFTP 会话引用
/// * `base` - 基础目录路径（用于计算相对路径）
/// * `current` - 当前正在遍历的目录路径
/// * `files` - 用于收集结果的可变引用，由调用者提供
async fn list_remote_files_recursive(
    sftp: &russh_sftp::client::SftpSession,
    base: &str,
    current: &str,
    files: &mut Vec<String>,
) -> Result<()> {
    let entries = match sftp.read_dir(current).await {
        Ok(entries) => entries,
        Err(_) => return Ok(()), // 目录不存在或无法读取
    };

    for entry in entries {
        let name = entry.file_name();
        if name == "." || name == ".." {
            continue;
        }

        let full_path = format!("{}/{}", current.trim_end_matches('/'), name);
        let file_type = entry.file_type();

        if file_type.is_file() {
            let base_prefix = format!("{}/", base.trim_end_matches('/'));
            let rel_path = full_path
                .strip_prefix(&base_prefix)
                .unwrap_or(&full_path)
                .to_string();
            files.push(rel_path);
        } else if file_type.is_dir() {
            Box::pin(list_remote_files_recursive(sftp, base, &full_path, files)).await?;
        }
    }
    Ok(())
}
