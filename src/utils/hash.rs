use base32::{Alphabet, encode};
use blake3::{Hash, Hasher};
use std::fs::File;
use std::io::{self, BufReader, Read};
use std::path::Path;

/// 使用 Blake3 算法计算文件的哈希值
///
/// 该函数读取指定路径的文件内容，使用高性能的 Blake3 哈希算法计算其哈希值，
/// 并使用 base32-crockford 编码格式返回结果字符串。特别适合用于文件完整性验证、
/// 文件唯一标识等场景。
///
/// # 参数
/// - `file_path`: 实现了 `AsRef<Path>` trait 的文件路径，可以是 `String`、`&str` 或 `PathBuf` 等
///
/// # 返回值
/// - **成功时**：返回文件的 Blake3 哈希值，以 base32-crockford 编码的小写字符串形式
/// - **失败时**：返回 `io::Error` 类型的错误
///
/// # 错误处理
/// 可能的错误包括：
/// - 文件不存在
/// - 没有权限访问文件
/// - 读取文件过程中发生I/O错误
///
/// # 示例
/// ```
/// use scripts::utils::hash::calculate_file_hash;
///
/// fn main() {
///     match calculate_file_hash("path/to/file.txt") {
///         Ok(hash) => println!("文件哈希值: {}", hash),
///         Err(err) => eprintln!("计算哈希值失败: {}", err),
///     }
/// }
/// ```
pub fn calculate_file_hash<P: AsRef<Path>>(file_path: P) -> Result<String, io::Error> {
    // 创建Blake3哈希器
    let mut hasher = Hasher::new();

    // 打开文件
    let file = File::open(file_path)?;
    let mut reader = BufReader::new(file);

    // 定义缓冲区（8KB，适合大多数文件读取场景）
    let mut buffer = [0; 8192];

    // 分块读取文件并更新哈希
    // 使用循环读取直到文件结束，避免一次性加载大文件到内存
    while let Ok(bytes_read) = reader.read(&mut buffer) {
        if bytes_read == 0 {
            break; // 文件读取完毕
        }
        hasher.update(&buffer[..bytes_read]);
    }

    // 计算最终哈希值并转换为字节数组
    let hash: Hash = hasher.finalize();
    let hash_bytes: [u8; 32] = hash.into();

    // 使用base32-crockford编码哈希值并转换为小写
    // base32-crockford编码具有更好的可读性和纠错能力
    let encoded = encode(Alphabet::Crockford, &hash_bytes).to_lowercase();

    Ok(encoded)
}
