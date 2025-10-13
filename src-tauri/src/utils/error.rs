//! 错误处理模块
//!
//! 提供统一的错误类型定义，简化错误处理流程

use serde::{Deserialize, Serialize};

/// 统一的命令错误类型
///
/// 包装 anyhow 错误，提供与 Tauri 命令接口兼容的错误类型。
/// 自动实现从 anyhow::Error 的转换，简化错误处理。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandError {
    pub msg: String,
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl std::error::Error for CommandError {}

impl From<anyhow::Error> for CommandError {
    fn from(err: anyhow::Error) -> Self {
        CommandError {
            msg: err.to_string(),
        }
    }
}

impl From<String> for CommandError {
    fn from(s: String) -> Self {
        CommandError { msg: s }
    }
}

impl From<&str> for CommandError {
    fn from(s: &str) -> Self {
        CommandError { msg: s.to_string() }
    }
}

/// 为常见的标准错误类型提供转换实现
impl From<std::io::Error> for CommandError {
    fn from(err: std::io::Error) -> Self {
        CommandError {
            msg: err.to_string(),
        }
    }
}

/// 为系统时间错误提供转换实现
impl From<std::time::SystemTimeError> for CommandError {
    fn from(err: std::time::SystemTimeError) -> Self {
        CommandError {
            msg: err.to_string(),
        }
    }
}
