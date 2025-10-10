//! 通用命令执行模块
//!
//! 提供前端可调用的通用命令执行接口

use serde::{Deserialize, Serialize};
use std::process::Command;

/// 命令执行结果
#[derive(Debug, Serialize, Deserialize)]
pub struct CommandResult {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// 执行命令
#[tauri::command]
pub fn execute_command_sync(
    command: String,
    args: Vec<String>,
    working_dir: Option<String>,
) -> Result<CommandResult, String> {
    let mut cmd = Command::new(&command);
    cmd.args(&args);

    if let Some(dir) = working_dir {
        cmd.current_dir(dir);
    }

    match cmd.output() {
        Ok(output) => {
            let result = CommandResult {
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            };

            Ok(result)
        }
        Err(e) => Err(format!("命令执行失败: {}", e)),
    }
}
