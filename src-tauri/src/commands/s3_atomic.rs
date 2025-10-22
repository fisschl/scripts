//! S3 原子操作命令模块
//!
//! 提供基础的 S3 操作命令，供前端组合使用

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
    /// S3 实例唯一标识符 (主键)
    pub s3_instance_id: String,
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

/// 获取 S3 客户端缓存实例
///
/// 返回全局唯一的 S3 客户端缓存实例，用于提高性能和减少资源消耗。
///
/// # 缓存特性
///
/// - **缓存时间**: 1分钟（60秒），客户端超过1分钟未使用会自动过期
/// - **最大容量**: 50个不同的 S3 客户端，超过容量时会根据LRU策略移除最少使用的客户端
/// - **缓存键**: 基于 endpoint_url 进行缓存键匹配，相同终端节点会复用同一个客户端
/// - **线程安全**: 支持并发访问，内部使用同步机制保证线程安全
/// - **自动过期**: 客户端会在指定时间后自动移除，释放资源
///
/// # 性能优势
///
/// - **减少网络开销**: 避免重复创建客户端时的握手和认证过程
/// - **连接池复用**: 复用底层的 HTTP 连接池，提高请求效率
/// - **减少认证开销**: 减少重复的 AWS 凭证验证请求
/// - **内存优化**: 通过容量限制防止内存无限增长
fn get_s3_client_cache() -> &'static Cache<String, Client> {
    static CLIENT_CACHE: OnceLock<Cache<String, Client>> = OnceLock::new();
    CLIENT_CACHE.get_or_init(|| {
        Cache::builder()
            .time_to_live(Duration::from_secs(60)) // 1分钟缓存时间
            .max_capacity(50) // 最多缓存50个客户端
            .build()
    })
}

/// 获取指定 S3 服务的客户端实例
///
/// 首先检查缓存中是否存在指定 s3_instance_id 的客户端，如果存在则直接返回；
/// 如果不存在，则根据配置创建新的 S3 客户端并将其缓存以供后续使用。
///
/// # 参数
///
/// * `s3_instance_id` - S3 实例的唯一标识符，用作缓存键
/// * `app` - Tauri 应用句柄，用于访问配置存储
///
/// # 返回值
///
/// * `Ok(Client)` - 成功获取或创建的 S3 客户端实例
/// * `Err(anyhow::Error)` - 失败时的错误信息，包含详细的上下文
///
/// # 错误处理
///
/// 此函数会处理以下类型的错误：
/// - 配置存储访问失败
/// - S3 配置不存在或格式错误
/// - AWS 凭证无效或权限不足
/// - 网络连接或配置加载失败
///
/// # 线程安全
///
/// 此函数是线程安全的，可以同时从多个异步任务中调用。
/// 内部缓存使用同步机制保证并发访问的安全性。
pub async fn get_cached_s3_client(s3_instance_id: &str, app: &tauri::AppHandle) -> Result<Client> {
    let cache = get_s3_client_cache();

    let client = cache
        .try_get_with(s3_instance_id.to_string(), async move {
            create_s3_client_from_config(s3_instance_id, app).await
        })
        .await
        .map_err(|e: Arc<anyhow::Error>| anyhow::anyhow!("{}", e))?;

    Ok(client)
}

/// 根据配置创建 S3 客户端
///
/// 从应用配置中读取指定 s3_instance_id 的 S3 配置信息，并创建对应的 S3 客户端。
/// 此函数封装了配置读取、解析和客户端创建的完整流程。
///
/// # 参数
///
/// * `s3_instance_id` - S3 实例的唯一标识符，用于匹配配置
/// * `app` - Tauri 应用句柄，用于访问配置存储
///
/// # 返回值
///
/// * `Ok(Client)` - 成功创建的 S3 客户端实例
/// * `Err(anyhow::Error)` - 失败时的错误信息，包含详细的上下文
///
/// # 配置查找流程
///
/// 1. 从应用的 "s3-config.json" 配置文件中读取配置
/// 2. 从配置中获取 "s3-instances" 数组
/// 3. 解析 S3 配置列表
/// 4. 根据传入的 s3_instance_id 查找匹配的配置项
/// 5. 使用找到的配置创建 AWS 凭证和客户端
async fn create_s3_client_from_config(
    s3_instance_id: &str,
    app: &tauri::AppHandle,
) -> Result<Client> {
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
        .find(|config| config.s3_instance_id == s3_instance_id)
        .ok_or_else(|| anyhow::anyhow!("未找到s3_instance_id为 {} 的S3配置", s3_instance_id))?;

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
}

/// 清除所有 S3 客户端缓存
///
/// 清空全局 S3 客户端缓存中的所有条目，强制下次访问时重新创建客户端。
/// 此操作通常在以下情况下使用：
/// - S3 配置信息更新后，需要使用新配置重新创建客户端
/// - 更换了 AWS 凭证，需要重新认证
/// - 怀疑客户端状态异常，需要强制刷新
/// - 进行故障排除或调试
///
/// # 效果
///
/// - 移除所有已缓存的 S3 客户端实例
/// - 释放客户端占用的内存和网络资源
/// - 下次调用 `get_cached_s3_client` 时会重新创建客户端
/// - 不会影响正在进行的 S3 操作
///
/// # 性能影响
///
/// 调用此函数后，后续的 S3 操作会因需要重新创建客户端而略有延迟，
/// 这是正常现象，也是使用缓存的权衡之一。
#[tauri::command]
pub fn clear_s3_client_cache() {
    let cache = get_s3_client_cache();
    cache.invalidate_all();
}

/// 列举指定 S3 服务中的所有存储桶
///
/// 查询并返回指定 s3_instance_id 对应的 S3 服务中所有存储桶的名称列表。
/// 此操作需要适当的 S3 访问权限。
///
/// # 参数
///
/// * `s3_instance_id` - S3 实例的唯一标识符，用于标识具体的 S3 服务
/// * `app` - Tauri 应用句柄，用于获取 S3 配置和客户端
///
/// # 返回值
///
/// * `Ok(Vec<String>)` - 成功时返回存储桶名称的列表，按字典序排列
/// * `Err(String)` - 失败时返回错误描述，可能包含权限不足、网络问题等信息
///
/// # 权限要求
///
/// 此操作需要 S3 服务的 `ListAllMyBuckets` 权限。
/// 如果权限不足，将返回相应的错误信息。
///
/// # 性能考虑
///
/// - 此操作涉及网络请求，响应时间取决于网络延迟和 S3 服务性能
/// - 如果存储桶数量很大（超过1000个），可能需要分页处理
/// - 建议缓存结果以避免频繁调用
#[tauri::command]
pub async fn list_s3_buckets(
    s3_instance_id: String,
    app: tauri::AppHandle,
) -> Result<Vec<String>, String> {
    let client = get_cached_s3_client(&s3_instance_id, &app)
        .await
        .map_err(|e| e.to_string())?;

    let response = client
        .list_buckets()
        .send()
        .await
        .map_err(|e| e.to_string())?;

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
#[tauri::command]
pub async fn list_s3_objects(
    s3_instance_id: String,
    bucket: String,
    prefix: Option<String>,
    continuation_token: Option<String>,
    app: tauri::AppHandle,
) -> Result<ListObjectsResponse, String> {
    let client = get_cached_s3_client(&s3_instance_id, &app)
        .await
        .map_err(|e| e.to_string())?;

    let mut request = client.list_objects_v2().bucket(&bucket);

    if let Some(prefix) = &prefix {
        request = request.prefix(prefix);
    }

    if let Some(token) = continuation_token {
        request = request.continuation_token(token);
    }

    let response = request.send().await.map_err(|e| e.to_string())?;

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
#[tauri::command]
pub async fn upload_file_to_s3(
    s3_instance_id: String,
    bucket: String,
    local_path: String,
    s3_key: String,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let client = get_cached_s3_client(&s3_instance_id, &app)
        .await
        .map_err(|e| e.to_string())?;

    let path = Path::new(&local_path);
    let body = ByteStream::from_path(path)
        .await
        .map_err(|e| format!("读取文件失败: {}", e))?;

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
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// 删除 S3 对象
///
/// 从指定存储桶中永久删除一个对象。此操作不可撤销。
#[tauri::command]
pub async fn delete_s3_object(
    s3_instance_id: String,
    bucket: String,
    s3_key: String,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let client = get_cached_s3_client(&s3_instance_id, &app)
        .await
        .map_err(|e| e.to_string())?;

    client
        .delete_object()
        .bucket(&bucket)
        .key(&s3_key)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// 从 S3 下载文件到本地
///
/// 将指定的 S3 对象下载到本地文件系统中。如果本地目录不存在，会自动创建。
#[tauri::command]
pub async fn download_file_from_s3(
    s3_instance_id: String,
    bucket: String,
    local_path: String,
    s3_key: String,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let client = get_cached_s3_client(&s3_instance_id, &app)
        .await
        .map_err(|e| e.to_string())?;

    // 获取 S3 对象
    let response = client
        .get_object()
        .bucket(&bucket)
        .key(&s3_key)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    // 确保本地目录存在
    let path = Path::new(&local_path);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("创建目录失败: {}", e))?;
    }

    // 将响应体转换为异步读取器并直接复制到文件
    let mut body = response.body.into_async_read();
    let mut file = tokio::fs::File::create(path)
        .await
        .map_err(|e| format!("创建文件失败: {}", e))?;

    tokio::io::copy(&mut body, &mut file)
        .await
        .map_err(|e| format!("文件复制失败: {}", e))?;

    Ok(())
}
