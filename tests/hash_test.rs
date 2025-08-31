//! 文件哈希计算器测试模块
//! 
//! 包含对 `calculate_file_hash` 函数的完整测试，验证其功能正确性、一致性和可靠性。
//! 测试涵盖了基本功能验证、相同内容文件的哈希一致性以及不同内容文件的哈希差异性。

use file_hasher::utils::hash::calculate_file_hash;
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