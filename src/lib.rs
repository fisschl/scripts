//! 文件哈希计算器库
//! 
//! 提供高性能的文件哈希计算功能，基于 Blake3 算法和 base32-crockford 编码。
//! 
//! ## 主要功能
//! 
//! - 使用 Blake3 算法计算文件哈希值
//! - 支持 base32-crockford 编码格式输出
//! - 提供分块读取功能，支持大文件处理
//! - 完整的错误处理机制

pub mod utils;
pub use utils::hash::calculate_file_hash;