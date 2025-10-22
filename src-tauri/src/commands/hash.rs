//! 哈希计算模块
//!
//! 提供前端可调用的文件哈希计算命令

use tauri::command;

/// 计算文件哈希值
///
/// 使用 Blake3 算法计算文件的哈希值，并以 base58 格式返回。
/// 适用于文件完整性校验和去重等场景。
///
/// # 参数
///
/// * `file_path` - 要计算哈希值的文件路径
///
/// # 返回值
///
/// * `Ok(String)` - 成功时返回 base58 编码的哈希值
/// * `Err(CommandError)` - 失败时返回错误描述
///
/// # 行为
///
/// * 当文件不存在时返回错误
/// * 当没有读取权限时返回错误
/// * 对于大文件，计算时间可能与文件大小成正比
/// * 哈希值不包含路径信息，仅基于文件内容
#[command]
pub async fn file_hash(file_path: String) -> Result<String, String> {
    use blake3::{Hash, Hasher};
    use bs58::encode;
    use tokio::io::AsyncReadExt;

    let path = std::path::PathBuf::from(file_path);
    let mut hasher = Hasher::new();

    // 64KB
    const HASH_BUFFER_SIZE: usize = 64 * 1024;

    let mut file = tokio::fs::File::open(&path)
        .await
        .map_err(|e| format!("打开文件失败: {}", e))?;

    let mut buffer = vec![0u8; HASH_BUFFER_SIZE];
    loop {
        match file.read(&mut buffer).await {
            Ok(0) => break,
            Ok(n) => hasher.update(&buffer[..n]),
            Err(e) => return Err(format!("读取文件失败: {}", e)),
        };
    }

    // 计算最终哈希值并转换为字节数组
    let hash: Hash = hasher.finalize();
    let hash_bytes: [u8; 32] = hash.into();

    // 使用base58编码哈希值
    // base58编码在比特币等加密货币中广泛使用，具有较好的可读性和紧凑性
    let encoded = encode(&hash_bytes).into_string();

    Ok(encoded)
}
