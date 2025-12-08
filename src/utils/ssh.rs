//! # SSH 工具模块
//!
//! 提供 SSH 连接管理功能，包括会话创建、认证等操作。

use anyhow::{Context, Result};
use russh::client;
use russh_keys::key;
use std::collections::HashSet;
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

/// SSH 服务器操作封装
///
/// 封装了 SSH 会话和 SFTP 会话，提供便捷的远程操作方法。
/// 直接持有会话的所有权，使用更简洁，无需生命周期参数。
///
/// # 示例
///
/// ```rust
/// use scripts::utils::ssh::{create_ssh_session, SSHServer};
/// use std::path::Path;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let session = create_ssh_session("example.com", 22, "user", "pass").await?;
///     let channel = session.channel_open_session().await?;
///     channel.request_subsystem(true, "sftp").await?;
///     let sftp = russh_sftp::client::SftpSession::new(channel.into_stream()).await?;
///     
///     let server = SSHServer::new(session, sftp);
///     server.exec_command("/tmp", "ls -la").await?;
///     Ok(())
/// }
/// ```
pub struct SSHServer {
    session: SshSession,
    sftp: russh_sftp::client::SftpSession,
}

impl SSHServer {
    /// 创建 SSHServer 实例
    ///
    /// 自动建立 SSH 连接并初始化 SFTP 会话。
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
    /// * `Ok(SSHServer)` - 成功创建的 SSH 服务器实例
    /// * `Err(anyhow::Error)` - 连接、认证或 SFTP 初始化失败
    ///
    /// # 示例
    ///
    /// ```rust
    /// use scripts::utils::ssh::SSHServer;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let server = SSHServer::new("example.com", 22, "user", "pass").await?;
    ///     server.exec_command("/tmp", "ls -la").await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn new(host: &str, port: u16, user: &str, password: &str) -> Result<Self> {
        println!("  → 建立 SSH 连接: {}@{}:{}", user, host, port);

        // 创建 SSH 客户端配置
        let client_config = client::Config::default();
        let sh = ClientHandler;

        // 建立 SSH 连接
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

        let session = Arc::new(session);

        // 创建 SFTP 会话
        let channel = session.channel_open_session().await?;
        channel.request_subsystem(true, "sftp").await?;
        let sftp = russh_sftp::client::SftpSession::new(channel.into_stream()).await?;

        Ok(Self { session, sftp })
    }

    /// 执行单条远程命令
    ///
    /// 在指定工作目录下执行单条命令，实时输出执行结果。
    /// 命令执行失败（退出码非 0）时会返回包含 stderr 内容的错误。
    ///
    /// # 参数
    ///
    /// * `workdir` - 命令执行的工作目录
    /// * `cmd` - 要执行的命令
    ///
    /// # 返回值
    ///
    /// * `Ok(String)` - 命令执行成功，返回标准输出内容
    /// * `Err(anyhow::Error)` - 命令执行失败，错误信息包含标准错误输出
    ///
    /// # 示例
    ///
    /// ```rust
    /// // 成功时返回 stdout 内容
    /// let output = server.exec_command("/tmp", "ls -la").await?;
    /// println!("输出: {}", output);
    ///
    /// // 失败时返回包含 stderr 的错误
    /// // server.exec_command("/tmp", "ls non-existent").await?; // Err 包含 stderr
    /// ```
    pub async fn exec_command(&self, workdir: &str, cmd: &str) -> Result<String> {
        let full_cmd = format!("cd {} && {}", workdir, cmd);
        let mut channel = self.session.channel_open_session().await?;
        channel.exec(true, full_cmd.as_bytes()).await?;

        let mut stdout_data = String::new();
        let mut stderr_data = String::new();

        while let Some(msg) = channel.wait().await {
            match msg {
                russh::ChannelMsg::Data { ref data } => {
                    let chunk = String::from_utf8_lossy(data);
                    print!("{}", chunk);
                    let _ = io::stdout().flush();
                    stdout_data.push_str(&chunk);
                }
                russh::ChannelMsg::ExtendedData { ref data, ext: 1 } => {
                    let chunk = String::from_utf8_lossy(data);
                    eprint!("{}", chunk);
                    let _ = io::stderr().flush();
                    stderr_data.push_str(&chunk);
                }
                russh::ChannelMsg::ExitStatus { exit_status } => {
                    if exit_status == 0 {
                        return Ok(stdout_data);
                    } else {
                        anyhow::bail!("命令执行失败，退出码: {}\n{}", exit_status, stderr_data);
                    }
                }
                _ => {}
            }
        }

        anyhow::bail!("命令执行异常: 未收到退出码")
    }

    /// 批量执行多条远程命令
    ///
    /// 在一个交互式 shell 中按顺序执行所有命令，实时输出执行结果。
    /// 所有命令在同一个 shell 会话中执行，可以共享环境变量和工作目录。
    /// 如果某条命令失败（返回非零退出码），会立即停止并返回错误。
    ///
    /// # 参数
    ///
    /// * `workdir` - 所有命令执行的工作目录
    /// * `cmds` - 要执行的命令列表
    ///
    /// # 返回值
    ///
    /// * `Ok(String)` - 所有命令执行成功，返回标准输出内容
    /// * `Err(anyhow::Error)` - 某条命令执行失败，错误信息包含标准错误输出
    ///
    /// # 示例
    ///
    /// ```rust
    /// let commands = vec![
    ///     "mkdir -p /tmp/test",
    ///     "touch /tmp/test/file.txt",
    ///     "ls -la /tmp/test",
    /// ];
    /// let output = server.exec_commands("/tmp", &commands).await?;
    /// ```
    pub async fn exec_commands(&self, workdir: &str, cmds: &[&str]) -> Result<String> {
        // 启动交互式 shell（从标准输入读取命令）
        let mut channel = self.session.channel_open_session().await?;
        channel.exec(true, b"bash -s").await?;

        // 进入工作目录并开启错误立即退出
        channel.data(format!("cd {}\n", workdir).as_bytes()).await?;
        channel.data("set -e\n".as_bytes()).await?;

        // 逐条发送命令
        for cmd in cmds {
            println!("  → 执行命令: {}", cmd);
            channel.data(format!("{}\n", cmd).as_bytes()).await?;
        }

        // 退出 shell
        channel.data("exit\n".as_bytes()).await?;
        channel.eof().await?;

        let mut stdout_data = String::new();
        let mut stderr_data = String::new();

        while let Some(msg) = channel.wait().await {
            match msg {
                russh::ChannelMsg::Data { ref data } => {
                    let chunk = String::from_utf8_lossy(data);
                    print!("{}", chunk);
                    let _ = io::stdout().flush();
                    stdout_data.push_str(&chunk);
                }
                russh::ChannelMsg::ExtendedData { ref data, ext: 1 } => {
                    let chunk = String::from_utf8_lossy(data);
                    eprint!("{}", chunk);
                    let _ = io::stderr().flush();
                    stderr_data.push_str(&chunk);
                }
                russh::ChannelMsg::ExitStatus { exit_status } => {
                    if exit_status == 0 {
                        return Ok(stdout_data);
                    } else {
                        anyhow::bail!("命令批量执行失败，退出码: {}\n{}", exit_status, stderr_data);
                    }
                }
                _ => {}
            }
        }

        anyhow::bail!("命令执行异常: 未收到退出码")
    }

    /// 递归创建远程目录
    ///
    /// 创建远程目录及其所有必需的父目录。如果目录已存在则不执行任何操作。
    /// 等同于 `mkdir -p` 命令的行为。
    ///
    /// # 参数
    ///
    /// * `remote_dir` - 要创建的远程目录路径
    ///
    /// # 返回值
    ///
    /// * `Ok(())` - 目录创建成功或已存在
    /// * `Err(anyhow::Error)` - 目录创建失败
    ///
    /// # 示例
    ///
    /// ```rust
    /// server.mkdir_p("/home/user/path/to/dir").await?;
    /// ```
    pub async fn mkdir_p(&self, remote_dir: &str) -> Result<()> {
        if remote_dir.is_empty() {
            return Ok(());
        }

        // 使用 mkdir -p 命令递归创建目录
        let mkdir_cmd = format!("mkdir -p {}", remote_dir);
        self.exec_command("/", &mkdir_cmd).await?;
        Ok(())
    }

    /// 上传本地文件到远程服务器
    ///
    /// 使用流式传输将本地文件上传到远程服务器，自动创建远程父目录。
    ///
    /// # 参数
    ///
    /// * `local_path` - 本地文件路径
    /// * `remote_path` - 远程文件目标路径
    ///
    /// # 返回值
    ///
    /// * `Ok(())` - 上传成功
    /// * `Err(anyhow::Error)` - 本地文件不存在、目录创建失败或传输失败
    ///
    /// # 示例
    ///
    /// ```rust
    /// use std::path::Path;
    /// server.upload_file(Path::new("local.txt"), "/tmp/remote.txt").await?;
    /// ```
    pub async fn upload_file(&self, local_path: &Path, remote_path: &str) -> Result<()> {
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
                self.mkdir_p(parent_dir).await?;
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
        let mut remote_file = self
            .sftp
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

    /// 上传目录到远程服务器
    ///
    /// 将本地目录的所有内容同步到远程目录。
    /// 同步逻辑：
    /// 1. 确保远程目录存在
    /// 2. 上传所有本地文件
    /// 3. 删除远程多余的文件（确保远程目录与本地完全一致）
    ///
    /// # 参数
    ///
    /// * `local_dir` - 本地目录路径
    /// * `remote_dir` - 远程目录路径
    ///
    /// # 返回值
    ///
    /// * `Ok(())` - 目录同步成功
    /// * `Err(anyhow::Error)` - 本地目录不存在或同步失败
    ///
    /// # 示例
    ///
    /// ```rust
    /// use std::path::Path;
    /// server.upload_dir(Path::new("./local_dir"), "/remote/path").await?;
    /// ```
    pub async fn upload_dir(&self, local_dir: &Path, remote_dir: &str) -> Result<()> {
        // 检查本地目录是否存在
        if !local_dir.exists() {
            anyhow::bail!("本地目录不存在: {}", local_dir.display());
        }
        if !local_dir.is_dir() {
            anyhow::bail!("路径不是目录: {}", local_dir.display());
        }

        // 确保远程目录存在
        self.mkdir_p(remote_dir).await?;

        // 列举本地文件（相对路径）
        let local_files = crate::utils::filesystem::list_local_files(local_dir)?;
        println!("  → 本地文件数量: {}", local_files.len());

        // 列举远程文件（相对路径）
        let remote_files = self.list_files(remote_dir).await?;
        println!("  → 远程文件数量: {}", remote_files.len());

        // 上传所有本地文件
        for rel_path in &local_files {
            let local_file = local_dir.join(rel_path);
            let remote_file = format!("{}/{}", remote_dir.trim_end_matches('/'), rel_path);
            self.upload_file(&local_file, &remote_file).await?;
            println!("  ✓ 上传: {}", rel_path);
        }

        // 删除远程多余文件
        let local_set: HashSet<_> = local_files.iter().collect();
        for remote_rel_path in &remote_files {
            if !local_set.contains(remote_rel_path) {
                let remote_file =
                    format!("{}/{}", remote_dir.trim_end_matches('/'), remote_rel_path);
                let rm_cmd = format!("rm -f {}", remote_file);
                self.exec_command("/", &rm_cmd).await?;
                println!("  ✓ 删除远程: {}", remote_rel_path);
            }
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
    /// let files = server.list_files("/home/user/documents").await?;
    /// for file in files {
    ///     println!("文件: {}", file);
    /// }
    /// ```
    pub async fn list_files(&self, remote_dir: &str) -> Result<Vec<String>> {
        let mut files = Vec::new();
        let base = remote_dir;

        // 使用栈实现深度优先遍历，避免递归
        let mut stack = vec![remote_dir.to_string()];

        while let Some(current) = stack.pop() {
            let entries = match self.sftp.read_dir(&current).await {
                Ok(entries) => entries,
                Err(_) => continue, // 目录不存在或无法读取，跳过
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
                    // 将子目录压入栈中，继续遍历
                    stack.push(full_path);
                }
            }
        }

        Ok(files)
    }

    /// 关闭 SSH 连接
    ///
    /// 主动断开 SSH 会话,释放相关资源。
    /// 虽然 SSHServer 在 drop 时会自动清理资源,但显式调用 close 可以更早释放连接。
    ///
    /// # 返回值
    ///
    /// * `Ok(())` - 成功关闭连接
    /// * `Err(anyhow::Error)` - 关闭连接时发生错误
    ///
    /// # 示例
    ///
    /// ```rust
    /// let server = SSHServer::new("example.com", 22, "user", "pass").await?;
    /// // ... 执行操作 ...
    /// server.close().await?;
    /// ```
    pub async fn close(self) -> Result<()> {
        self.session
            .disconnect(russh::Disconnect::ByApplication, "", "")
            .await?;
        Ok(())
    }
}
