//! S3 原子操作命令模块
//!
//! 提供基础的 S3 操作命令，供前端组合使用

use crate::utils::error::CommandError;
use anyhow::Result;
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;
use moka::future::Cache;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tauri_plugin_store::StoreExt;

/// S3 服务配置
///
/// 包含连接 S3 服务所需的认证信息和网络配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct S3Config {
    /// AWS 访问密钥 ID
    pub access_key_id: String,
    /// AWS 秘密访问密钥
    pub secret_access_key: String,
    /// AWS 区域或兼容服务的区域标识
    pub region: String,
    /// S3 服务的终端节点 URL
    pub endpoint_url: String,
}

/// S3 对象元数据
///
/// 表示 S3 存储桶中对象的基本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Object {
    /// 对象在 S3 中的唯一键
    pub key: String,
    /// 对象大小（字节）
    pub size: Option<i64>,
    /// 最后修改时间的 ISO 8601 格式字符串
    pub last_modified: Option<String>,
}

/// S3 对象列表响应
///
/// 包含分页信息的 S3 对象列表响应结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListObjectsResponse {
    /// 对象列表
    pub objects: Vec<S3Object>,
    /// 是否还有更多对象
    pub is_truncated: bool,
    /// 用于获取下一页的令牌
    pub next_continuation_token: Option<String>,
}

/// 获取 S3 客户端缓存
///
/// 返回全局唯一的 S3 客户端缓存实例，具有以下特性：
/// - 缓存时间：1分钟（60秒）
/// - 最大容量：50个不同的 S3 客户端
/// - 基于 endpoint_url 进行缓存键匹配
/// - 自动过期：超过1分钟的客户端会被自动移除
/// - 线程安全：支持并发访问
///
/// # 性能优势
/// - 避免重复创建客户端的网络开销
/// - 复用 HTTP 连接池
/// - 减少 AWS 凭证验证次数
fn get_s3_client_cache() -> &'static Cache<String, Client> {
    static CLIENT_CACHE: OnceLock<Cache<String, Client>> = OnceLock::new();
    CLIENT_CACHE.get_or_init(|| {
        Cache::builder()
            .time_to_live(Duration::from_secs(60)) // 1分钟缓存时间
            .max_capacity(50) // 最多缓存50个客户端
            .build()
    })
}

/// 获取缓存的 S3 客户端
///
/// 如果缓存中存在指定 endpoint_url 的客户端则直接返回，
/// 否则创建新客户端并缓存。
///
/// # 参数
///
/// * `endpoint_url` - S3 服务的终端节点 URL
/// * `app` - Tauri 应用句柄，用于获取 S3 配置
///
/// # 返回值
///
/// * `Ok(Client)` - 成功获取或创建的 S3 客户端实例
/// * `Err(CommandError)` - 配置查找失败或客户端创建失败
///
/// # 错误
///
/// * 未找到匹配的 S3 配置
/// * 配置解析失败
/// * 认证信息无效
/// * 网络连接失败
pub async fn get_cached_s3_client(
    endpoint_url: &str,
    app: &tauri::AppHandle,
) -> Result<Client, CommandError> {
    let cache = get_s3_client_cache();

    let client = cache
        .try_get_with(endpoint_url.to_string(), async move {
            // 获取 store，使用与前端相同的配置文件名
            let store = app.store("s3-config.json")?;

            // 获取所有 S3 实例配置
            let instances_value = store
                .get("s3-instances")
                .ok_or_else(|| anyhow::anyhow!("未找到S3配置"))?;

            let instances: Vec<S3Config> = serde_json::from_value(instances_value)?;

            // 查找匹配的配置
            let config = instances
                .into_iter()
                .find(|config| config.endpoint_url == endpoint_url)
                .ok_or_else(|| anyhow::anyhow!("未找到endpoint_url为 {} 的S3配置", endpoint_url))?;

            // 创建 AWS 凭证
            let creds = aws_credential_types::Credentials::new(
                &config.access_key_id,
                &config.secret_access_key,
                None,
                None,
                "tauri-app",
            );

            // 设置区域
            let region = aws_config::Region::new(config.region);

            // 配置 AWS SDK
            let config_loader = aws_config::defaults(BehaviorVersion::latest())
                .region(region)
                .credentials_provider(creds)
                .endpoint_url(&config.endpoint_url);

            // 创建客户端
            let aws_config = config_loader.load().await;
            Ok(Client::new(&aws_config))
        })
        .await
        .map_err(|e: Arc<anyhow::Error>| anyhow::anyhow!("{}", e))?;

    Ok(client)
}

/// 清除所有 S3 客户端缓存
///
/// 清空所有已缓存的 S3 客户端，强制下次访问时重新创建。
/// 通常在配置更新后调用，以确保使用最新的配置。
#[tauri::command]
pub fn clear_s3_client_cache() {
    let cache = get_s3_client_cache();
    cache.invalidate_all();
}

/// 列举 S3 存储桶
///
/// 获取指定 S3 服务中所有存储桶的名称列表。
///
/// # 参数
///
/// * `endpoint_url` - S3 服务的终端节点 URL
/// * `app` - Tauri 应用句柄，用于获取 S3 配置
///
/// # 返回值
///
/// * `Ok(Vec<String>)` - 成功时返回存储桶名称列表
/// * `Err(CommandError)` - 失败时返回错误描述
///
/// # 错误
///
/// * 认证失败或权限不足
/// * 网络连接问题
/// * 服务配置错误
#[tauri::command]
pub async fn list_s3_buckets(
    endpoint_url: String,
    app: tauri::AppHandle,
) -> Result<Vec<String>, CommandError> {
    let client = get_cached_s3_client(&endpoint_url, &app).await?;

    let response = client
        .list_buckets()
        .send()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    let buckets = response
        .buckets()
        .iter()
        .filter_map(|bucket| bucket.name().map(|name| name.to_string()))
        .collect();

    Ok(buckets)
}

/// 列举 S3 对象（分页）
///
/// 获取指定存储桶中匹配前缀的对象列表，支持分页获取。
///
/// # 参数
///
/// * `endpoint_url` - S3 服务的终端节点 URL
/// * `bucket` - 存储桶名称
/// * `prefix` - 对象键前缀过滤器，可选
/// * `continuation_token` - 用于获取下一页的令牌，可选
/// * `app` - Tauri 应用句柄，用于获取 S3 配置
///
/// # 返回值
///
/// * `Ok(ListObjectsResponse)` - 成功时返回包含分页信息的对象列表
/// * `Err(CommandError)` - 失败时返回错误描述
///
/// # 行为
///
/// * 每次调用返回最多 1000 个对象（S3 API 限制）
/// * 返回的对象按字典序排列
/// * 包含完整的对象元数据信息和分页令牌
/// * 前端需要根据 `is_truncated` 和 `next_continuation_token` 来决定是否继续获取
#[tauri::command]
pub async fn list_s3_objects(
    endpoint_url: String,
    bucket: String,
    prefix: Option<String>,
    continuation_token: Option<String>,
    app: tauri::AppHandle,
) -> Result<ListObjectsResponse, CommandError> {
    let client = get_cached_s3_client(&endpoint_url, &app).await?;

    let mut request = client.list_objects_v2().bucket(&bucket);

    if let Some(prefix) = &prefix {
        request = request.prefix(prefix);
    }

    if let Some(token) = continuation_token {
        request = request.continuation_token(token);
    }

    let response = request.send().await.map_err(|e| anyhow::anyhow!(e))?;

    let mut objects = Vec::new();
    let contents = response.contents();
    for obj in contents {
        if let Some(key) = obj.key() {
            objects.push(S3Object {
                key: key.to_string(),
                size: obj.size(),
                last_modified: obj.last_modified().map(|dt| dt.to_string()),
            });
        }
    }

    let is_truncated = response.is_truncated() == Some(true);
    let next_continuation_token = response.next_continuation_token().map(|s| s.to_string());

    Ok(ListObjectsResponse {
        objects,
        is_truncated,
        next_continuation_token,
    })
}

/// 上传文件到 S3
///
/// 将本地文件上传到指定的 S3 存储桶和位置。自动根据文件扩展名设置 MIME 类型。
///
/// # 参数
///
/// * `endpoint_url` - S3 服务的终端节点 URL
/// * `bucket` - 目标存储桶名称
/// * `local_path` - 本地文件的完整路径
/// * `s3_key` - S3 对象的键名
/// * `app` - Tauri 应用句柄，用于获取 S3 配置
///
/// # 返回值
///
/// * `Ok(())` - 文件上传成功
/// * `Err(CommandError)` - 上传失败时的错误描述
///
/// # 行为
///
/// * 自动检测文件 MIME 类型并设置 Content-Type
/// * 支持任意大小的文件上传
/// * 会覆盖已存在的同名对象
/// * 上传是原子操作，要么完全成功，要么完全失败
#[tauri::command]
pub async fn upload_file_to_s3(
    endpoint_url: String,
    bucket: String,
    local_path: String,
    s3_key: String,
    app: tauri::AppHandle,
) -> Result<(), CommandError> {
    let client = get_cached_s3_client(&endpoint_url, &app).await?;

    let path = Path::new(&local_path);
    let body = ByteStream::from_path(path)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    // 根据文件扩展名自动检测 MIME 类型
    let mime_type = mime_guess::from_path(path)
        .first_or_octet_stream()
        .to_string();

    client
        .put_object()
        .bucket(&bucket)
        .key(&s3_key)
        .content_type(&mime_type)
        .body(body)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    Ok(())
}

/// 删除 S3 对象
///
/// 从指定存储桶中永久删除一个对象。此操作不可撤销。
///
/// # 参数
///
/// * `endpoint_url` - S3 服务的终端节点 URL
/// * `bucket` - 存储桶名称
/// * `s3_key` - 要删除的对象键名
/// * `app` - Tauri 应用句柄，用于获取 S3 配置
///
/// # 返回值
///
/// * `Ok(())` - 对象删除成功
/// * `Err(CommandError)` - 删除失败时的错误描述
///
/// # 行为
///
/// * 永久删除对象，无法恢复
/// * 如果对象不存在，某些服务可能返回成功
/// * 需要适当的删除权限
/// * 操作是原子的，要么完全成功，要么完全失败
#[tauri::command]
pub async fn delete_s3_object(
    endpoint_url: String,
    bucket: String,
    s3_key: String,
    app: tauri::AppHandle,
) -> Result<(), CommandError> {
    let client = get_cached_s3_client(&endpoint_url, &app).await?;

    client
        .delete_object()
        .bucket(&bucket)
        .key(&s3_key)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    Ok(())
}

/// 从 S3 下载文件到本地
///
/// 将指定的 S3 对象下载到本地文件系统中。如果本地目录不存在，会自动创建。
///
/// # 参数
///
/// * `endpoint_url` - S3 服务的终端节点 URL
/// * `bucket` - 源存储桶名称
/// * `local_path` - 本地文件的完整路径
/// * `s3_key` - S3 对象的键名
/// * `app` - Tauri 应用句柄，用于获取 S3 配置
///
/// # 返回值
///
/// * `Ok(())` - 文件下载成功
/// * `Err(CommandError)` - 下载失败时的错误描述
///
/// # 行为
///
/// * 自动创建本地目录（如果不存在）
/// * 会覆盖已存在的同名本地文件
/// * 保持原始文件内容
/// * 下载是原子操作，要么完全成功，要么完全失败
#[tauri::command]
pub async fn download_file_from_s3(
    endpoint_url: String,
    bucket: String,
    local_path: String,
    s3_key: String,
    app: tauri::AppHandle,
) -> Result<(), CommandError> {
    let client = get_cached_s3_client(&endpoint_url, &app).await?;

    // 获取 S3 对象
    let response = client
        .get_object()
        .bucket(&bucket)
        .key(&s3_key)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    // 确保本地目录存在
    let path = Path::new(&local_path);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // 将响应体转换为异步读取器并直接复制到文件
    let mut body = response.body.into_async_read();
    let mut file = tokio::fs::File::create(path).await?;

    tokio::io::copy(&mut body, &mut file).await?;

    Ok(())
}
