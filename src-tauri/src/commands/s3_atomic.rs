//! S3 原子操作命令模块
//!
//! 提供基础的 S3 操作命令，供前端组合使用

use anyhow::Result;
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;
use serde::{Deserialize, Serialize};
use std::path::Path;
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

/// 创建已认证的 S3 客户端
///
/// 根据终端节点 URL 从应用存储中查找 S3 配置，并创建经过认证的客户端。
/// 支持标准 AWS S3 以及兼容 S3 API 的第三方存储服务。
///
/// # 参数
///
/// * `endpoint_url` - S3 服务的终端节点 URL
/// * `app` - Tauri 应用句柄，用于访问全局状态和 S3 配置
///
/// # 返回值
///
/// * `Ok(Client)` - 成功创建的 S3 客户端实例
/// * `Err(String)` - 配置查找失败或客户端创建失败
///
/// # 错误
///
/// * 未找到匹配的 S3 配置
/// * 配置解析失败
/// * 认证信息无效
/// * 网络连接失败
async fn create_authenticated_s3_client(
    endpoint_url: String,
    app: &tauri::AppHandle,
) -> Result<Client, String> {
    // 获取 store，使用与前端相同的配置文件名
    let store = app.store("s3-config.json")
        .map_err(|e| format!("加载S3配置失败: {}", e))?;

    // 获取所有 S3 实例配置
    let instances_value = store
        .get("s3-instances")
        .ok_or_else(|| "未找到S3配置".to_string())?;

    let instances: Vec<S3Config> =
        serde_json::from_value(instances_value).map_err(|e| format!("解析S3配置失败: {}", e))?;

    // 查找匹配的配置
    let config = instances
        .into_iter()
        .find(|config| config.endpoint_url == endpoint_url)
        .ok_or_else(|| format!("未找到endpoint_url为 {} 的S3配置", endpoint_url))?;

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
/// * `Err(String)` - 失败时返回错误描述
///
/// # 错误
///
/// * 认证失败或权限不足
/// * 网络连接问题
/// * 服务配置错误
#[tauri::command]
pub async fn list_buckets(
    endpoint_url: String,
    app: tauri::AppHandle,
) -> Result<Vec<String>, String> {
    let client = create_authenticated_s3_client(endpoint_url, &app)
        .await
        .map_err(|e| format!("创建 S3 客户端失败: {}", e))?;

    let response = client
        .list_buckets()
        .send()
        .await
        .map_err(|e| format!("列举存储桶失败: {}", e))?;

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
/// * `Err(String)` - 失败时返回错误描述
///
/// # 行为
///
/// * 每次调用返回最多 1000 个对象（S3 API 限制）
/// * 返回的对象按字典序排列
/// * 包含完整的对象元数据信息和分页令牌
/// * 前端需要根据 `is_truncated` 和 `next_continuation_token` 来决定是否继续获取
#[tauri::command]
pub async fn list_objects(
    endpoint_url: String,
    bucket: String,
    prefix: Option<String>,
    continuation_token: Option<String>,
    app: tauri::AppHandle,
) -> Result<ListObjectsResponse, String> {
    let client = create_authenticated_s3_client(endpoint_url, &app)
        .await
        .map_err(|e| format!("创建 S3 客户端失败: {}", e))?;

    let mut request = client.list_objects_v2().bucket(&bucket);

    if let Some(prefix) = &prefix {
        request = request.prefix(prefix);
    }

    if let Some(token) = continuation_token {
        request = request.continuation_token(token);
    }

    let response = request
        .send()
        .await
        .map_err(|e| format!("列举对象失败: {}", e))?;

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
/// * `Err(String)` - 上传失败时的错误描述
///
/// # 行为
///
/// * 自动检测文件 MIME 类型并设置 Content-Type
/// * 支持任意大小的文件上传
/// * 会覆盖已存在的同名对象
/// * 上传是原子操作，要么完全成功，要么完全失败
#[tauri::command]
pub async fn upload_file(
    endpoint_url: String,
    bucket: String,
    local_path: String,
    s3_key: String,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let client = create_authenticated_s3_client(endpoint_url, &app)
        .await
        .map_err(|e| format!("创建 S3 客户端失败: {}", e))?;

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
        .map_err(|e| format!("上传文件失败: {}", e))?;

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
/// * `Err(String)` - 删除失败时的错误描述
///
/// # 行为
///
/// * 永久删除对象，无法恢复
/// * 如果对象不存在，某些服务可能返回成功
/// * 需要适当的删除权限
/// * 操作是原子的，要么完全成功，要么完全失败
#[tauri::command]
pub async fn delete_object(
    endpoint_url: String,
    bucket: String,
    s3_key: String,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let client = create_authenticated_s3_client(endpoint_url, &app)
        .await
        .map_err(|e| format!("创建 S3 客户端失败: {}", e))?;

    client
        .delete_object()
        .bucket(&bucket)
        .key(&s3_key)
        .send()
        .await
        .map_err(|e| format!("删除对象失败: {}", e))?;

    Ok(())
}

