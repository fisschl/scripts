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

/// 同步执行系统命令
///
/// # 功能说明
/// 提供通用的命令执行接口，允许前端调用任意的系统命令。
/// 支持指定工作目录，捕获命令的标准输出和错误输出。
///
/// # 参数
/// - `command`: 要执行的命令名称（如 "git"、"npm" 等）
/// - `args`: 命令参数数组，每个元素作为独立参数传递
/// - `working_dir`: 命令执行的工作目录，必须传入有效路径
///
/// # 返回值
/// - 成功时返回 `CommandResult`，包含：
///   - `exit_code`: 进程退出码（`None` 表示进程异常终止）
///   - `stdout`: 标准输出的字符串形式
///   - `stderr`: 标准错误的字符串形式
/// - 失败时返回错误信息字符串
///
/// # 注意事项
/// - 命令执行是同步的，会阻塞当前线程直到完成
/// - 工作目录必须存在且可访问
/// - 大量输出可能会导致内存占用较高
/// - 调用者需要确保命令参数的安全性，避免命令注入风险
#[tauri::command]
pub fn execute_command_sync(
    command: String,
    args: Vec<String>,
    working_dir: String,
) -> Result<CommandResult, String> {
    let mut cmd = Command::new(&command);
    cmd.args(&args);
    cmd.current_dir(&working_dir);

    match cmd.output() {
        Ok(output) => {
            let result = CommandResult {
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            };

            Ok(result)
        }
        Err(e) => Err(e.to_string()),
    }
}
