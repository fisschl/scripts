//! # S3 工具模块
//!
//! 提供 S3 对象存储管理功能，包括文件上传、目录同步等操作。

use anyhow::{Context, Result};
use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;
use std::collections::HashSet;
use std::path::Path;
use tokio::fs;

/// S3 管理器
///
/// 封装 AWS S3 客户端，提供便捷的文件和目录上传方法。
///
/// # 示例
///
/// ```rust
/// use scripts::utils::s3::S3Manager;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let manager = S3Manager::new(
///         "my-access-key-id",
///         "my-secret-access-key",
///         "us-east-1",
///         Some("https://s3.example.com")
///     ).await?;
///     manager.upload_file("my-bucket", "local.txt", "remote/path/file.txt").await?;
///     Ok(())
/// }
/// ```
pub struct S3Manager {
    client: Client,
}

impl S3Manager {
    /// 创建 S3Manager 实例
    ///
    /// 使用显式传递的凭证和配置创建 S3 客户端。
    ///
    /// # 参数
    ///
    /// * `access_key_id` - AWS 访问密钥 ID
    /// * `secret_access_key` - AWS 密钥
    /// * `region` - AWS 区域（如 us-east-1）
    /// * `endpoint_url` - 可选的自定义端点 URL（用于兼容 S3 的服务）
    ///
    /// # 返回值
    ///
    /// * `Ok(S3Manager)` - 成功创建的 S3 管理器实例
    /// * `Err(anyhow::Error)` - 配置加载失败
    ///
    /// # 示例
    ///
    /// ```rust
    /// use scripts::utils::s3::S3Manager;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let manager = S3Manager::new(
    ///         "AKIAIOSFODNN7EXAMPLE",
    ///         "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
    ///         "us-east-1",
    ///         None
    ///     ).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn new(
        access_key_id: &str,
        secret_access_key: &str,
        region: &str,
        endpoint_url: Option<&str>,
    ) -> Result<Self> {
        println!("  → 初始化 S3 客户端: region={}", region);

        use aws_config::BehaviorVersion;
        use aws_sdk_s3::config::{Credentials, Region};

        let credentials =
            Credentials::new(access_key_id, secret_access_key, None, None, "s3-manager");

        let mut config_builder = aws_config::defaults(BehaviorVersion::latest())
            .credentials_provider(credentials)
            .region(Region::new(region.to_string()));

        if let Some(endpoint) = endpoint_url {
            config_builder = config_builder.endpoint_url(endpoint);
        }

        let config = config_builder.load().await;
        let client = Client::new(&config);

        Ok(Self { client })
    }

    /// 上传本地文件到 S3
    ///
    /// 使用流式传输将本地文件上传到 S3 存储桶，自动检测文件 MIME 类型。
    ///
    /// # 参数
    ///
    /// * `bucket` - S3 存储桶名称
    /// * `local_path` - 本地文件路径
    /// * `s3_key` - S3 对象键（路径）
    ///
    /// # 返回值
    ///
    /// * `Ok(())` - 上传成功
    /// * `Err(anyhow::Error)` - 本地文件不存在或上传失败
    ///
    /// # 示例
    ///
    /// ```rust
    /// use std::path::Path;
    /// manager.upload_file("my-bucket", Path::new("local.txt"), "remote/file.txt").await?;
    /// ```
    pub async fn upload_file(&self, bucket: &str, local_path: &Path, s3_key: &str) -> Result<()> {
        // 检查本地文件是否存在
        if !local_path.exists() {
            anyhow::bail!("本地文件不存在: {}", local_path.display());
        }
        if !local_path.is_file() {
            anyhow::bail!("路径不是文件: {}", local_path.display());
        }

        // 获取文件大小
        let metadata = fs::metadata(local_path)
            .await
            .with_context(|| format!("无法获取文件信息: {}", local_path.display()))?;
        let file_size = metadata.len();

        // 猜测 MIME 类型
        let content_type = mime_guess::from_path(local_path)
            .first_or_octet_stream()
            .to_string();

        // 创建字节流（流式上传）
        let body = ByteStream::from_path(local_path)
            .await
            .with_context(|| format!("无法创建文件流: {}", local_path.display()))?;

        // 上传到 S3
        self.client
            .put_object()
            .bucket(bucket)
            .key(s3_key)
            .body(body)
            .content_type(&content_type)
            .content_length(file_size as i64)
            .send()
            .await
            .with_context(|| format!("上传文件到 S3 失败: {}", s3_key))?;

        Ok(())
    }

    /// 上传目录到 S3
    ///
    /// 将本地目录的所有内容同步到 S3 指定前缀下。
    /// 同步逻辑：
    /// 1. 列举本地所有文件
    /// 2. 列举 S3 指定前缀下的所有对象
    /// 3. 上传所有本地文件
    /// 4. 删除 S3 中多余的对象（确保 S3 与本地完全一致）
    ///
    /// # 参数
    ///
    /// * `bucket` - S3 存储桶名称
    /// * `local_dir` - 本地目录路径
    /// * `s3_prefix` - S3 对象键前缀（相当于目录路径）
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
    /// manager.upload_dir("my-bucket", Path::new("./dist"), "website/").await?;
    /// ```
    pub async fn upload_dir(&self, bucket: &str, local_dir: &Path, s3_prefix: &str) -> Result<()> {
        // 检查本地目录是否存在
        if !local_dir.exists() {
            anyhow::bail!("本地目录不存在: {}", local_dir.display());
        }
        if !local_dir.is_dir() {
            anyhow::bail!("路径不是目录: {}", local_dir.display());
        }

        // 标准化 S3 前缀（确保以 / 结尾）
        let s3_prefix = if s3_prefix.is_empty() {
            String::new()
        } else {
            format!("{}/", s3_prefix.trim_end_matches('/'))
        };

        // 列举本地文件（相对路径）
        let local_files = crate::utils::filesystem::list_local_files(local_dir)?;
        println!("  → 本地文件数量: {}", local_files.len());

        // 列举 S3 对象（相对路径）
        let s3_objects = self.list_objects(bucket, &s3_prefix).await?;
        println!("  → S3 对象数量: {}", s3_objects.len());

        // 上传所有本地文件
        for rel_path in &local_files {
            let local_file = local_dir.join(rel_path);
            let s3_key = format!("{}{}", s3_prefix, rel_path);
            self.upload_file(bucket, &local_file, &s3_key).await?;
            println!("  ✓ 上传: {}", rel_path);
        }

        // 删除 S3 多余对象
        let local_set: HashSet<_> = local_files.iter().collect();
        for s3_rel_path in &s3_objects {
            if !local_set.contains(s3_rel_path) {
                let s3_key = format!("{}{}", s3_prefix, s3_rel_path);
                self.delete_object(bucket, &s3_key).await?;
                println!("  ✓ 删除 S3: {}", s3_rel_path);
            }
        }

        Ok(())
    }

    /// 列举 S3 指定前缀下的所有对象（返回相对路径）
    ///
    /// # 参数
    ///
    /// * `bucket` - S3 存储桶名称
    /// * `prefix` - S3 对象键前缀
    ///
    /// # 返回值
    ///
    /// * `Ok(Vec<String>)` - 所有对象的相对路径列表
    /// * `Err(anyhow::Error)` - 列举失败
    async fn list_objects(&self, bucket: &str, prefix: &str) -> Result<Vec<String>> {
        let mut objects = Vec::new();
        let mut continuation_token: Option<String> = None;

        loop {
            let mut request = self.client.list_objects_v2().bucket(bucket).prefix(prefix);

            if let Some(token) = continuation_token {
                request = request.continuation_token(token);
            }

            let response = request
                .send()
                .await
                .with_context(|| format!("列举 S3 对象失败: {}", prefix))?;

            // 提取相对路径
            if let Some(contents) = &response.contents {
                objects.extend(contents.iter().filter_map(|object| {
                    object
                        .key()
                        .and_then(|key| key.strip_prefix(prefix))
                        .filter(|rel_path| !rel_path.is_empty())
                        .map(|rel_path| rel_path.to_string())
                }));
            }

            // 检查是否还有更多对象
            if response.is_truncated() == Some(true) {
                continuation_token = response.next_continuation_token().map(|s| s.to_string());
            } else {
                break;
            }
        }

        Ok(objects)
    }

    /// 删除 S3 对象
    ///
    /// # 参数
    ///
    /// * `bucket` - S3 存储桶名称
    /// * `s3_key` - S3 对象键
    ///
    /// # 返回值
    ///
    /// * `Ok(())` - 删除成功
    /// * `Err(anyhow::Error)` - 删除失败
    async fn delete_object(&self, bucket: &str, s3_key: &str) -> Result<()> {
        self.client
            .delete_object()
            .bucket(bucket)
            .key(s3_key)
            .send()
            .await
            .with_context(|| format!("删除 S3 对象失败: {}", s3_key))?;

        Ok(())
    }
}
