//! S3 上传命令模块
//!
//! 提供将本地目录覆盖式上传到 S3 远程目录的功能

use anyhow::{Context, Result};
use aws_config::BehaviorVersion;
use aws_sdk_s3::Client;
use aws_sdk_s3::primitives::ByteStream;
use mime_guess;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};
use walkdir::WalkDir;

/// S3 配置信息结构体
///
/// # 功能概述
/// 该结构体封装了连接和操作 S3 存储服务所需的所有配置信息，
/// 支持标准 AWS S3 以及兼容 S3 协议的其他存储服务（如 MinIO、阿里云 OSS、腾讯云 COS 等）。
///
/// # 字段说明
/// - `access_key_id`: AWS 访问密钥 ID，用于身份验证的公钥部分
/// - `secret_access_key`: AWS 秘密访问密钥，用于身份验证的私钥部分，必须妥善保管
/// - `region`: S3 服务所在的地理区域，如 "us-east-1"、"cn-north-1" 等
/// - `bucket`: S3 存储桶名称，作为文件存储的顶层容器
/// - `endpoint_url`: 自定义终端节点 URL，用于连接非 AWS 的 S3 兼容服务
///
/// # 使用场景
/// - 配置 AWS S3 服务连接
/// - 配置 MinIO 等私有 S3 服务
/// - 配置阿里云 OSS、腾讯云 COS 等公有云 S3 兼容服务
/// - 在不同环境（开发、测试、生产）间切换存储配置
///
/// # 安全注意
/// - 访问密钥信息必须保密，避免在日志中明文输出
/// - 建议使用环境变量或安全配置管理服务存储敏感信息
/// - 在生产环境中应使用最小权限原则配置 IAM 权限
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct S3Config {
    /// AWS 访问密钥 ID，用于身份验证的公钥标识
    pub access_key_id: String,
    /// AWS 秘密访问密钥，用于身份验证的私钥，必须严格保密
    pub secret_access_key: String,
    /// S3 服务所在的地理区域，例如 "us-east-1"、"cn-north-1"
    pub region: String,
    /// S3 存储桶名称，作为文件存储的顶层命名空间
    pub bucket: String,
    /// 自定义 endpoint URL，用于兼容其他 S3 服务（如 MinIO、阿里云 OSS 等）
    /// 对于标准 AWS S3，通常设置为 "https://s3.amazonaws.com"
    pub endpoint_url: String,
}

/// S3 上传参数结构体
///
/// # 功能概述
/// 该结构体封装了执行一次完整的 S3 同步操作所需的所有参数，
/// 作为前端与后端之间的数据传输载体，确保所有必要信息都能正确传递。
///
/// # 字段说明
/// - `s3_config`: S3 连接和认证配置，包含访问密钥、区域、存储桶等信息
/// - `local_dir`: 本地源目录的完整路径，指定要同步的本地文件夹
/// - `remote_dir`: 远程目标目录路径，作为 S3 存储桶中的对象前缀
///
/// # 路径规范
/// - `local_dir`: 使用本地文件系统路径格式（如："C:\\Users\\Documents\\website" 或 "/home/user/website"）
/// - `remote_dir`: 使用 S3 对象键格式，通常以斜杠结尾（如："website/" 或 "backup/2024/"）
///
/// # 使用场景
/// - 网站静态资源部署：将本地构建的网页文件上传到 S3
/// - 文件备份：将本地重要目录备份到 S3 存储
/// - 内容分发：将资源文件同步到 S3 供 CDN 分发
/// - 数据归档：将历史数据上传到 S3 进行长期保存
///
/// # 数据验证
/// - 本地目录必须存在且为有效目录
/// - 远程目录路径必须符合 S3 对象键命名规范
/// - S3 配置必须包含有效的认证信息和存储桶名称
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3UploadParams {
    /// S3 连接和认证配置信息，用于建立与 S3 服务的连接
    pub s3_config: S3Config,
    /// 本地源目录路径，指定要同步的本地文件夹完整路径
    pub local_dir: String,
    /// 远程目标目录路径，作为 S3 存储桶中的对象键前缀
    /// 通常以斜杠结尾，如 "website/" 或 "backup/2024/"
    pub remote_dir: String,
}

/// 文件操作类型枚举
///
/// # 功能概述
/// 定义了在同步过程中可能执行的所有文件操作类型，用于描述对文件的具体操作行为。
/// 该枚举是同步算法的核心数据结构，每个操作都包含了执行所需的所有上下文信息。
///
/// # 操作类型说明
/// - `Upload`: 上传新文件，本地存在但远程不存在的文件需要上传到 S3
/// - `Overwrite`: 覆盖已存在的文件，本地和远程都存在但有差异的文件需要用本地版本覆盖
/// - `Delete`: 删除远程文件，远程存在但本地不存在的冗余文件需要从 S3 删除
///
/// # 数据结构特点
/// - 每个操作都包含执行所需的所有参数，避免在操作执行时再次查询
/// - 使用结构化数据而非简单枚举值，便于扩展和添加新的操作类型
/// - 操作类型设计覆盖了文件同步的所有场景，确保同步的完整性
///
/// # 使用场景
/// - 在 `generate_operation_queue` 函数中生成操作队列
/// - 在 `execute_operations` 函数中执行具体的文件操作
/// - 用于日志记录和同步结果统计
///
/// # 扩展性
/// 如果需要添加新的操作类型（如文件重命名、权限修改等），
/// 可以在此枚举中添加新的变体，并在相应的处理函数中实现逻辑
#[derive(Debug, Clone)]
enum FileOperation {
    /// 上传新文件操作
    ///
    /// # 参数
    /// - `local_path`: 本地文件的完整路径，指定要上传的源文件
    /// - `s3_key`: 文件在 S3 中的存储键（路径），作为文件在存储桶中的唯一标识
    Upload { local_path: PathBuf, s3_key: String },

    /// 覆盖已存在的文件操作
    ///
    /// # 参数
    /// - `local_path`: 本地文件的完整路径，提供新版本的内容
    /// - `s3_key`: 文件在 S3 中的存储键，指定要覆盖的目标文件
    Overwrite { local_path: PathBuf, s3_key: String },

    /// 删除远程文件操作
    ///
    /// # 参数
    /// - `s3_key`: 要删除的文件在 S3 中的存储键
    Delete { s3_key: String },
}

/// 创建并配置 S3 客户端
///
/// # 功能说明
/// 根据提供的 S3 配置信息，创建一个配置完整的 AWS S3 客户端实例。
/// 支持标准 AWS S3 服务以及兼容 S3 协议的其他存储服务（如 MinIO、阿里云 OSS、腾讯云 COS 等）。
///
/// # 参数
/// - `config`: S3 配置信息，包含访问密钥、区域、存储桶名称和自定义终端节点等
///
/// # 返回值
/// - 成功时返回配置好的 S3 客户端实例
/// - 失败时返回配置错误信息
///
/// # 配置详情
/// - 使用提供的访问密钥进行身份验证
/// - 设置指定的 AWS 区域
/// - 配置自定义终端节点 URL（用于兼容其他 S3 服务）
/// - 使用最新的 AWS 行为版本
///
/// # 使用场景
/// 这是所有 S3 操作的基础函数，在上传、下载、删除等操作前都需要先创建客户端
async fn create_s3_client(config: &S3Config) -> Result<Client> {
    let creds = aws_credential_types::Credentials::new(
        &config.access_key_id,
        &config.secret_access_key,
        None,
        None,
        "tauri-app",
    );

    let region = aws_config::Region::new(config.region.clone());

    let config_loader = aws_config::defaults(BehaviorVersion::latest())
        .region(region)
        .credentials_provider(creds)
        .endpoint_url(&config.endpoint_url);

    let aws_config = config_loader.load().await;
    Ok(Client::new(&aws_config))
}

/// 上传单个文件到 S3 存储桶
///
/// # 功能说明
/// 将本地文件系统中的单个文件上传到指定的 S3 存储桶中，使用对象的 key 作为存储路径。
/// 该函数会处理文件的读取、流式传输以及上传过程中的错误处理。
///
/// # 参数
/// - `client`: S3 客户端实例，用于执行上传操作
/// - `bucket`: S3 存储桶名称，指定文件要上传到的目标存储桶
/// - `local_path`: 本地文件的完整路径，指定要上传的源文件
/// - `s3_key`: 文件在 S3 中的存储键（路径），作为文件在存储桶中的唯一标识
///
/// # 返回值
/// - 成功时返回 ()，表示文件已成功上传到 S3
/// - 失败时返回错误信息，包含文件读取失败或上传失败的具体原因
///
/// # 错误处理
/// - 本地文件不存在或无法读取时会返回错误
/// - 网络问题导致的上传失败会返回错误
/// - S3 服务端返回的错误会包含详细的错误信息
///
/// # 性能特点
/// - 使用流式传输，支持大文件上传
/// - 自动处理文件内容的字节流转换
/// - 提供详细的错误上下文信息，便于问题定位
/// - 自动检测并设置 Content-Type，确保文件在 S3 中正确显示
async fn upload_file_to_s3(
    client: &Client,
    bucket: &str,
    local_path: &Path,
    s3_key: &str,
) -> Result<()> {
    let body = ByteStream::from_path(local_path)
        .await
        .with_context(|| format!("读取文件失败: {}", local_path.display()))?;

    // 根据文件扩展名自动检测 MIME 类型
    let mime_type = mime_guess::from_path(local_path)
        .first_or_octet_stream()
        .to_string();

    client
        .put_object()
        .bucket(bucket)
        .key(s3_key)
        .content_type(&mime_type)
        .body(body)
        .send()
        .await
        .with_context(|| {
            format!(
                "上传文件失败: {} (Content-Type: {}) -> {}",
                local_path.display(),
                mime_type,
                s3_key
            )
        })?;

    Ok(())
}

/// 同步本地目录到 S3 远程目录
///
/// # 功能概述
/// 该函数实现了本地目录到 S3 远程目录的完整同步功能。通过对比本地和远程文件列表，
/// 智能生成最优的操作队列，确保以最小的操作成本实现本地和远程的完全一致。
///
/// # 同步策略
/// 采用"覆盖式"同步策略，具体规则如下：
/// - **上传**：本地存在但远程不存在的文件，执行上传操作
/// - **覆盖**：本地和远程都存在的文件，直接覆盖，不比较文件内容
/// - **删除**：远程存在但本地不存在的文件，执行删除操作
///
/// # 同步流程
/// 1. **扫描本地文件**：递归遍历本地目录，构建文件映射表
/// 2. **获取远程文件**：通过 S3 API 获取远程目录的文件列表
/// 3. **生成操作队列**：对比本地和远程文件，生成最优操作序列
/// 4. **批量执行操作**：按照生成的队列，顺序执行上传、覆盖、删除操作
///
/// # 事件系统
/// 在同步过程中，会通过 Tauri 事件系统发送 "s3-sync-progress" 事件到前端，
/// 包含当前的操作状态和进度信息，便于前端实时显示同步进度。
///
/// # 性能特点
/// - 简单高效的存在性检测，不比较文件内容
/// - 批量操作，减少网络请求次数
/// - 实时事件推送，便于监控同步状态
///
/// # 错误处理
/// - 本地目录扫描失败会返回错误
/// - S3 连接和文件列表获取失败会返回错误
/// - 文件上传、删除操作失败会返回错误
/// - 所有错误都包含详细的上下文信息
///
/// # 参数
/// - `params`: S3 上传参数，包含 S3 配置、本地目录路径和远程目录路径
/// - `app_handle`: Tauri 应用句柄，用于发送进度事件到前端
///
/// # 返回值
/// - 成功时返回 `Ok(())`，表示同步操作成功完成
/// - 失败时返回 `Err(String)`，包含详细的错误信息
pub async fn sync_directory_to_s3(
    params: S3UploadParams,
    app_handle: AppHandle,
) -> Result<(), String> {
    let S3UploadParams {
        s3_config,
        local_dir,
        remote_dir,
    } = params;

    // 验证本地目录存在
    let local_path = PathBuf::from(&local_dir);
    if !local_path.exists() {
        return Err(format!("本地目录不存在: {}", local_dir));
    }
    if !local_path.is_dir() {
        return Err(format!("路径不是目录: {}", local_dir));
    }

    // 创建 S3 客户端
    let client = create_s3_client(&s3_config)
        .await
        .map_err(|e| format!("创建 S3 客户端失败: {}", e))?;

    // 移除前导斜杠（如果有的话）
    let remote_dir = remote_dir.trim_start_matches('/');

    // 确保远程目录路径以 / 结尾
    let remote_prefix = if remote_dir.ends_with('/') {
        remote_dir.to_string()
    } else {
        format!("{}/", remote_dir)
    };

    app_handle
        .emit("s3-sync-progress", "开始分析本地和远程文件差异...")
        .ok();

    // 1. 获取本地文件映射
    let local_files =
        build_local_file_map(&local_path).map_err(|e| format!("扫描本地文件失败: {}", e))?;

    app_handle
        .emit(
            "s3-sync-progress",
            &format!("发现本地文件: {} 个", local_files.len()),
        )
        .ok();

    // 2. 获取远程文件列表
    let remote_files = get_remote_files(&client, &s3_config.bucket, &remote_prefix)
        .await
        .map_err(|e| format!("获取远程文件列表失败: {}", e))?;

    app_handle
        .emit(
            "s3-sync-progress",
            &format!("发现远程文件: {} 个", remote_files.len()),
        )
        .ok();

    // 3. 生成操作队列
    let operations = generate_operation_queue(&local_files, &remote_files, &remote_prefix)
        .map_err(|e| format!("生成操作队列失败: {}", e))?;

    app_handle
        .emit(
            "s3-sync-progress",
            &format!("生成操作队列: {} 个操作", operations.len()),
        )
        .ok();

    if operations.is_empty() {
        app_handle
            .emit("s3-sync-progress", "本地和远程文件完全一致，无需同步")
            .ok();
        return Ok(());
    }

    // 4. 执行操作队列
    app_handle
        .emit("s3-sync-progress", "开始执行同步操作...")
        .ok();
    execute_operations(&client, &s3_config.bucket, operations, &app_handle)
        .await
        .map_err(|e| format!("执行同步操作失败: {}", e))?;

    app_handle.emit("s3-sync-progress", "同步完成！").ok();

    Ok(())
}

/// 从 S3 存储桶中删除指定的对象
///
/// # 功能说明
/// 根据提供的对象键（路径），从指定的 S3 存储桶中删除对应的对象（文件）。
/// 该操作是不可逆的，删除后无法恢复，请谨慎使用。
///
/// # 参数
/// - `client`: S3 客户端实例，用于执行删除操作
/// - `bucket`: S3 存储桶名称，指定要从中删除对象的存储桶
/// - `key`: 要删除的对象键（路径），标识存储桶中要删除的具体对象
///
/// # 返回值
/// - 成功时返回 `Ok(())`，表示对象已成功删除
/// - 失败时返回错误信息，包含删除失败的具体原因
///
/// # 安全注意
/// - 删除操作是永久性的，无法撤销
/// - 请确保 key 路径正确，避免误删重要文件
/// - 建议在生产环境中使用版本控制，以防误删
///
/// # 错误处理
/// - 对象不存在时会返回错误
/// - 权限不足时会返回错误
/// - 网络问题导致删除失败会返回错误
/// - 存储桶不存在时会返回错误
///
/// # 使用场景
/// 主要用于同步过程中清理远程存在但本地不存在的冗余文件，保持存储空间的整洁
async fn delete_s3_object(client: &Client, bucket: &str, key: &str) -> Result<()> {
    client
        .delete_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .with_context(|| format!("删除 S3 对象失败: {}", key))?;

    Ok(())
}

/// 获取远程 S3 存储桶中的文件列表
///
/// # 功能说明
/// 通过分页方式获取指定 S3 存储桶中符合前缀条件的所有对象（文件）列表。
/// 该函数只返回文件路径列表，不包含文件的元信息（如大小、修改时间等）。
///
/// # 参数
/// - `client`: S3 客户端实例，用于执行列表查询操作
/// - `bucket`: S3 存储桶名称，指定要查询的目标存储桶
/// - `prefix`: 对象键前缀，用于筛选特定目录下的文件（如 "images/"）
///
/// # 返回值
/// - 成功时返回 `HashSet<String>`，包含所有文件的完整路径列表
/// - 失败时返回错误信息，包含网络请求失败的具体原因
///
/// # 分页处理
/// - 自动处理 S3 的分页响应，支持获取超过 1000 个文件的大型目录
/// - 使用 continuation_token 机制确保获取完整文件列表
/// - 每次请求获取一页数据，直到获取完所有文件
///
/// # 数据提取
/// - **对象键**：文件在 S3 中的完整路径，作为 HashSet 的元素
/// - **路径格式**：保持 S3 中的原始路径格式，包含前缀部分
///
/// # 错误处理
/// - 网络请求失败会返回错误
/// - 权限不足无法访问存储桶时会返回错误
/// - 存储桶不存在时会返回错误
///
/// # 使用场景
/// 主要用于同步过程中的远程文件列表获取：
/// - 与本地文件列表进行对比分析
/// - 确定哪些文件在远程存在但本地不存在（需要删除）
/// - 计算文件差异，生成最优同步操作队列
///
/// # 性能特点
/// - 分页获取，避免一次性加载大量数据导致内存问题
/// - 只获取文件路径信息，减少数据传输量
/// - 高效的 HashSet 存储，便于后续快速查找和对比
async fn get_remote_files(client: &Client, bucket: &str, prefix: &str) -> Result<HashSet<String>> {
    let mut remote_files = HashSet::new();
    let mut continuation_token = None;

    loop {
        let mut request = client.list_objects_v2().bucket(bucket).prefix(prefix);

        if let Some(token) = continuation_token {
            request = request.continuation_token(token);
        }

        let response = request.send().await?;

        let contents = response.contents();
        if !contents.is_empty() {
            for obj in contents {
                if let Some(key) = obj.key() {
                    remote_files.insert(key.to_string());
                }
            }
        }

        if response.is_truncated() == Some(true) {
            continuation_token = response.next_continuation_token().map(|s| s.to_string());
        } else {
            break;
        }
    }

    Ok(remote_files)
}

/// 构建本地文件映射表：将指定目录下的所有文件建立相对路径到完整路径的映射
///
/// # 功能说明
/// 该函数递归遍历指定目录，收集所有文件（不包括目录），并为每个文件建立映射关系：
/// - Key：文件相对于根目录的相对路径（使用正斜杠 `/` 作为路径分隔符，确保跨平台兼容性）
/// - Value：文件的完整绝对路径
///
/// # 参数
/// - `local_dir`: 要扫描的本地目录路径
///
/// # 返回值
/// - 成功时返回 HashMap，包含所有文件的映射关系
/// - 失败时返回错误信息（如目录遍历失败、路径计算错误等）
///
/// # 使用场景
/// 在文件同步过程中，该映射表用于：
/// 1. 快速查找本地是否存在某个文件
/// 2. 计算文件在远程存储中的对应路径
/// 3. 与远程文件列表进行对比，确定需要上传、覆盖或删除的文件
///
/// # 注意事项
/// - 只收集文件，跳过目录
/// - 自动将 Windows 反斜杠转换为正斜杠，确保路径格式统一
/// - 如果目录不存在或权限不足，会返回相应的错误
fn build_local_file_map(local_dir: &Path) -> Result<HashMap<String, PathBuf>> {
    let mut local_files = HashMap::new();

    for entry in WalkDir::new(local_dir) {
        let entry = entry.with_context(|| "遍历本地目录失败")?;
        let path = entry.path();

        if path.is_file() {
            let relative_path = path
                .strip_prefix(local_dir)
                .with_context(|| "计算相对路径失败")?;

            let relative_str = relative_path.to_string_lossy().replace('\\', "/");
            local_files.insert(relative_str, path.to_path_buf());
        }
    }

    Ok(local_files)
}

/// 智能生成文件同步操作队列
///
/// # 功能概述
/// 这是同步算法的核心函数，通过对比本地文件映射和远程文件列表，
/// 智能生成最优的操作队列，确保以最小的操作成本实现本地和远程的完全一致。
///
/// # 算法策略
/// 采用两阶段对比算法：
/// 1. **本地文件处理阶段**：遍历所有本地文件，决定每个文件的操作类型
/// 2. **远程文件清理阶段**：遍历远程文件，清理本地不存在的冗余文件
///
/// # 操作类型判定
/// - **上传** (`FileOperation::Upload`)：本地存在但远程不存在的新文件
/// - **覆盖** (`FileOperation::Overwrite`)：本地和远程都存在的文件（无论是否相同，都执行覆盖）
/// - **删除** (`FileOperation::Delete`)：远程存在但本地不存在的冗余文件
///
/// # 差异检测机制
/// 仅通过文件名存在性判断，不比较文件内容和元信息
///
/// # 参数
/// - `local_files`: 本地文件映射表（相对路径 -> 完整路径）
/// - `remote_files`: 远程文件列表（对象键集合）
/// - `remote_prefix`: 远程目录前缀，用于构建完整的 S3 对象键
///
/// # 返回值
/// - 成功时返回操作队列，按最优顺序排列的同步操作列表
/// - 失败时返回错误信息（如文件元信息读取失败）
///
/// # 优化策略
/// - 优先处理本地文件，确保新文件和更新文件得到及时处理
/// - 批量生成操作，便于后续批量执行
/// - 不跳过任何文件，确保所有文件都被同步
///
/// # 错误处理
/// - 读取本地文件元信息失败会返回错误
/// - 路径计算错误会返回错误
/// - 确保在操作执行前发现所有潜在问题
///
/// # 使用场景
/// 主要用于同步前的操作规划阶段，为批量执行阶段提供完整的操作指令序列
fn generate_operation_queue(
    local_files: &HashMap<String, PathBuf>,
    remote_files: &HashSet<String>,
    remote_prefix: &str,
) -> Result<Vec<FileOperation>> {
    let mut operations = Vec::new();

    // 1. 处理本地文件：上传或覆盖
    for (relative_path, local_path) in local_files {
        let s3_key = format!("{}{}", remote_prefix, relative_path);

        if remote_files.contains(&s3_key) {
            // 远程已存在，直接覆盖，不比较文件差异
            operations.push(FileOperation::Overwrite {
                local_path: local_path.clone(),
                s3_key,
            });
        } else {
            // 远程不存在，需要上传
            operations.push(FileOperation::Upload {
                local_path: local_path.clone(),
                s3_key,
            });
        }
    }

    // 2. 处理需要删除的远程文件
    for s3_key in remote_files {
        // 提取相对路径（移除前缀）
        if let Some(relative_path) = s3_key.strip_prefix(remote_prefix) {
            let relative_path = relative_path.to_string();

            // 如果本地不存在这个文件，则需要删除远程文件
            if !local_files.contains_key(&relative_path) {
                operations.push(FileOperation::Delete {
                    s3_key: s3_key.clone(),
                });
            }
        }
    }

    Ok(operations)
}

/// 批量执行文件同步操作队列
///
/// # 功能概述
/// 按照生成的操作队列，顺序执行每个同步操作（上传、覆盖、删除），
/// 并通过 Tauri 事件系统向前端发送实时进度信息，确保每个操作都能被跟踪和监控。
///
/// # 执行流程
/// 1. **顺序执行**：按照操作队列的顺序，逐个执行每个操作
/// 2. **实时事件**：每执行一个操作，都会通过事件系统发送进度信息到前端
/// 3. **错误处理**：如果某个操作失败，会立即停止执行并返回错误
/// 4. **成功确认**：每个操作成功后，继续执行下一个操作
///
/// # 操作类型处理
/// - **上传操作**：调用 `upload_file_to_s3` 函数上传新文件，发送 "上传: 文件路径" 事件
/// - **覆盖操作**：调用 `upload_file_to_s3` 函数覆盖已存在文件，发送 "覆盖: 文件路径" 事件
/// - **删除操作**：调用 `delete_s3_object` 函数删除远程文件，发送 "删除: 文件路径" 事件
///
/// # 参数
/// - `client`: S3 客户端实例，用于执行具体的上传和删除操作
/// - `bucket`: S3 存储桶名称，指定操作的目标存储桶
/// - `operations`: 操作队列，包含要执行的所有同步操作
/// - `app_handle`: Tauri 应用句柄，用于发送进度事件到前端
///
/// # 返回值
/// - 成功时返回 `Ok(())`，表示所有操作都已成功执行
/// - 失败时返回错误信息，包含失败操作的具体原因和上下文
///
/// # 错误处理
/// - 任何操作失败都会立即停止后续执行
/// - 提供详细的错误上下文，包括失败的文件路径和操作类型
/// - 确保部分失败时能够准确报告问题
///
/// # 事件系统
/// - 每个操作执行前都会通过 "s3-sync-progress" 事件发送操作类型和文件路径
/// - 便于前端实时监控同步进度和操作状态
/// - 事件数据为字符串类型，包含操作描述信息
///
/// # 性能特点
/// - 顺序执行，确保操作的可预测性
/// - 实时事件推送，便于监控长时间运行的同步任务
/// - 详细的错误信息，便于快速定位和解决问题
///
/// # 使用场景
/// 主要用于同步过程的最后阶段，批量执行生成的同步操作队列
async fn execute_operations(
    client: &Client,
    bucket: &str,
    operations: Vec<FileOperation>,
    app_handle: &AppHandle,
) -> Result<()> {
    for operation in operations {
        match operation {
            FileOperation::Upload { local_path, s3_key } => {
                app_handle
                    .emit("s3-sync-progress", &format!("上传: {}", s3_key))
                    .ok();
                upload_file_to_s3(client, bucket, &local_path, &s3_key)
                    .await
                    .with_context(|| {
                        format!("上传文件失败: {} -> {}", local_path.display(), s3_key)
                    })?;
            }
            FileOperation::Overwrite { local_path, s3_key } => {
                app_handle
                    .emit("s3-sync-progress", &format!("覆盖: {}", s3_key))
                    .ok();
                upload_file_to_s3(client, bucket, &local_path, &s3_key)
                    .await
                    .with_context(|| {
                        format!("覆盖文件失败: {} -> {}", local_path.display(), s3_key)
                    })?;
            }
            FileOperation::Delete { s3_key } => {
                app_handle
                    .emit("s3-sync-progress", &format!("删除: {}", s3_key))
                    .ok();
                delete_s3_object(client, bucket, &s3_key).await?;
            }
        }
    }

    Ok(())
}

/// 公开的 S3 上传函数，供 commands.rs 调用
///
/// # 功能概述
/// 这是整个 S3 上传模块的入口函数，负责接收前端传来的 JSON 格式参数，
/// 解析后调用同步函数执行实际的文件上传任务。该函数提供了统一的对外接口，
/// 隐藏了内部复杂的同步逻辑实现细节。
///
/// # 参数处理
/// - 接收 JSON 字符串格式的 `S3UploadParams` 参数
/// - 使用 `serde_json` 进行反序列化，将 JSON 转换为 Rust 结构体
/// - 如果参数解析失败，会返回详细的错误信息，包括解析失败的具体原因
///
/// # 执行流程
/// 1. **参数解析**：将 JSON 字符串解析为 `S3UploadParams` 结构体
/// 2. **同步执行**：调用 `sync_directory_to_s3` 函数执行实际的同步操作
///
/// # 错误处理
/// - **参数解析错误**：JSON 格式不正确或缺少必要字段时返回解析错误
/// - **同步执行错误**：文件读取、网络传输、S3 操作等失败时返回具体错误
/// - **错误信息详细**：所有错误都包含清晰的错误描述，便于前端处理和显示
///
/// # 使用场景
/// 主要用于 Tauri 应用的命令调用，通过 `invoke` 函数从 JavaScript/TypeScript 代码调用，
/// 实现本地目录到 S3 存储桶的完整同步功能。
///
/// # 注意事项
/// - 该函数是异步的，需要在异步上下文中调用
/// - 参数必须是有效的 JSON 字符串，且符合 `S3UploadParams` 结构体格式
/// - 同步过程可能会比较耗时，建议在前端显示进度或加载状态
///
/// # 参数
/// - `params`: S3 上传参数的 JSON 字符串，格式必须符合 `S3UploadParams` 结构体定义
///
/// # 返回值
/// - 成功时返回 `Ok(())`，表示同步操作成功完成
/// - 失败时返回 `Err(String)`，包含详细的错误信息字符串
pub async fn upload_to_s3(params: String, app_handle: AppHandle) -> Result<(), String> {
    // 解析 JSON 参数
    let upload_params: S3UploadParams =
        serde_json::from_str(&params).map_err(|e| format!("解析参数失败: {}", e))?;

    // 执行同步
    sync_directory_to_s3(upload_params, app_handle).await
}
