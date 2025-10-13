//! 通用命令执行模块
//!
//! 提供前端可调用的通用命令执行接口

use serde::{Deserialize, Serialize};
use std::process::Command;

/// 命令执行结果
///
/// 包含已执行命令的完整输出信息和状态
#[derive(Debug, Serialize, Deserialize)]
pub struct CommandRunnerResult {
    /// 进程退出码，None 表示进程被信号终止
    pub exit_code: Option<i32>,
    /// 标准输出的完整内容
    pub stdout: String,
    /// 标准错误的完整内容
    pub stderr: String,
}

/// 同步执行系统命令
///
/// 提供通用的命令执行接口，允许前端调用任意的系统命令。
/// 支持指定工作目录，捕获命令的标准输出和错误输出。
///
/// # 参数
///
/// * `command` - 要执行的命令名称（如 "git"、"npm" 等）
/// * `args` - 命令参数数组，每个元素作为独立参数传递
/// * `working_dir` - 命令执行的工作目录，必须传入有效路径
///
/// # 返回值
///
/// * `Ok(CommandResult)` - 成功时返回命令执行结果，包含：
///   - `exit_code` - 进程退出码（None 表示进程异常终止）
///   - `stdout` - 标准输出的字符串形式
///   - `stderr` - 标准错误的字符串形式
/// * `Err(CommandError)` - 失败时返回错误信息
///
/// # 行为
///
/// * 命令执行是同步的，会阻塞当前线程直到完成
/// * 工作目录必须存在且可访问
/// * 自动处理输出编码，将字节转换为 UTF-8 字符串
/// * 捕获完整的输出内容，包括换行符
///
/// # 安全性
///
/// * 调用者需要确保命令参数的安全性
/// * 避免直接拼接用户输入作为命令参数
/// * 建议使用参数数组而非字符串拼接来防止命令注入
#[tauri::command]
pub fn execute_command_sync(
    command: String,
    args: Vec<String>,
    working_dir: String,
) -> Result<CommandRunnerResult, String> {
    let mut cmd = Command::new(&command);
    cmd.args(&args);
    cmd.current_dir(&working_dir);

    let output = cmd.output().map_err(|e| format!("命令执行失败: {}", e))?;

    let result = CommandRunnerResult {
        exit_code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    };

    Ok(result)
}
