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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::write;
    use tempfile::NamedTempFile;

    /// 测试基本的文件哈希计算功能
    ///
    /// 验证函数能够正确计算文件的哈希值，并返回非空的小写字符串结果。
    #[test]
    fn test_calculate_file_hash() {
        // 创建测试文件
        let temp_file = NamedTempFile::new().expect("创建临时文件失败");
        let test_content = b"This is a test file for hashing";

        // 写入测试内容
        write(temp_file.path(), test_content).expect("写入临时文件失败");

        // 计算哈希值
        let hash = calculate_file_hash(temp_file.path()).expect("计算哈希值失败");

        // 验证返回值是一个字符串且不为空
        assert!(!hash.is_empty(), "哈希值不应该为空");

        // 验证返回值是小写的
        assert_eq!(hash, hash.to_lowercase(), "哈希值应该是小写的");
    }

    /// 测试相同内容的文件应产生相同的哈希值
    ///
    /// 验证哈希函数的一致性特性：对于相同内容的文件，无论文件路径或其他属性如何，
    /// 计算出的哈希值应该完全相同。这是哈希函数的基本属性之一。
    #[test]
    fn test_same_content_same_hash() {
        // 创建两个具有相同内容的临时文件
        let temp_file1 = NamedTempFile::new().expect("创建临时文件失败");
        let temp_file2 = NamedTempFile::new().expect("创建临时文件失败");
        let test_content = b"This is the same content for two files";

        // 向两个文件写入相同内容
        write(temp_file1.path(), test_content).expect("写入临时文件失败");
        write(temp_file2.path(), test_content).expect("写入临时文件失败");

        // 计算两个文件的哈希值
        let hash1 = calculate_file_hash(temp_file1.path()).expect("计算哈希值失败");
        let hash2 = calculate_file_hash(temp_file2.path()).expect("计算哈希值失败");

        // 验证两个相同内容的文件产生相同的哈希值
        assert_eq!(hash1, hash2, "相同内容的文件应该产生相同的哈希值");
    }

    /// 测试不同内容的文件应产生不同的哈希值
    ///
    /// 验证哈希函数的碰撞抵抗性：对于内容不同的文件，即使差异很小，
    /// 计算出的哈希值也应该不同。这是保证哈希值能够唯一标识文件内容的关键特性。
    #[test]
    fn test_different_content_different_hash() {
        // 创建两个具有不同内容的临时文件
        let temp_file1 = NamedTempFile::new().expect("创建临时文件失败");
        let temp_file2 = NamedTempFile::new().expect("创建临时文件失败");

        // 向两个文件写入不同内容
        write(temp_file1.path(), b"Content for file 1").expect("写入临时文件失败");
        write(temp_file2.path(), b"Content for file 2").expect("写入临时文件失败");

        // 计算两个文件的哈希值
        let hash1 = calculate_file_hash(temp_file1.path()).expect("计算哈希值失败");
        let hash2 = calculate_file_hash(temp_file2.path()).expect("计算哈希值失败");

        // 验证两个不同内容的文件产生不同的哈希值
        assert_ne!(hash1, hash2, "不同内容的文件应该产生不同的哈希值");
    }
}
