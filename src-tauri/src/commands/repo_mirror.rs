//! Git 仓库镜像工具命令模块
//!
//! 该模块提供 Tauri 命令用于克隆源仓库并同步所有分支和标签到目标仓库。
//! 主要功能包括：
//! - 克隆源仓库到临时目录
//! - 获取并处理所有远程分支
//! - 重命名远程仓库并添加新的目标仓库
//! - 推送所有分支和标签到目标仓库
//! - 发送进度通知到前端界面

use std::process::Command;
use tauri::{AppHandle, Emitter};

/// Git 仓库镜像命令
///
/// 克隆源仓库并同步所有分支和标签到目标仓库。该命令执行以下步骤：
/// 1. 创建临时目录用于克隆源仓库
/// 2. 克隆源仓库到临时目录
/// 3. 获取所有远程分支信息
/// 4. 为每个分支创建本地分支并设置跟踪
/// 5. 重命名原始远程仓库并添加新的目标仓库
/// 6. 推送所有分支到目标仓库
/// 7. 推送所有标签到目标仓库
/// 8. 清理临时文件并发送完成通知
///
/// # 参数
/// - `app_handle`: Tauri 应用句柄，用于发送进度通知事件
/// - `from`: 源仓库 URL (例如: https://github.com/user/repo.git)
/// - `to`: 目标仓库 URL (例如: https://gitlab.com/user/repo.git)
///
/// # 返回值
/// - `Ok(())`: 镜像操作成功完成
/// - `Err(String)`: 操作失败，包含详细的错误信息
///
/// # 事件通知
/// - `repo-mirror-info`: 进度信息通知，包含当前操作状态
/// - `repo-mirror-success`: 操作成功完成通知
///
/// # 错误处理
/// - 所有 Git 命令执行失败都会返回详细的错误信息
/// - 临时目录创建失败会返回错误
/// - 进度通知发送失败使用 `.unwrap()`，失败会导致应用崩溃（开发阶段便于调试）
pub fn repo_mirror(app_handle: AppHandle, from: String, to: String) -> Result<(), String> {
    // 创建临时目录用于克隆仓库
    // 使用 tempfile crate 创建带前缀的临时目录，确保操作完成后自动清理
    let temp_dir = tempfile::Builder::new()
        .prefix("repo-mirror-") // 目录前缀，便于识别
        .tempdir()
        .map_err(|e| format!("创建临时目录失败: {}", e))?;

    // 发送进度通知 - 开始克隆
    app_handle
        .emit("repo-mirror-info", "开始克隆源仓库...")
        .unwrap();

    let temp_path = temp_dir.path();

    // 从源仓库 URL 中提取项目名称
    // 例如: https://github.com/user/my-project.git -> "my-project"
    let project_name = from
        .split('/') // 按斜杠分割 URL
        .next_back() // 获取最后一部分
        .unwrap_or("repo") // 如果获取失败，使用默认名称 "repo"
        .trim_end_matches(".git"); // 移除 .git 后缀

    // 设置仓库路径
    let repo_path = temp_path.join(project_name);

    // 克隆源仓库到临时目录
    // 执行 git clone 命令将源仓库克隆到临时目录
    let output = Command::new("git")
        .arg("clone") // git clone 命令
        .arg(&from) // 源仓库 URL
        .arg(&repo_path) // 目标路径（临时目录）
        .output()
        .map_err(|e| format!("执行 git clone 命令失败: {}", e))?;

    // 检查克隆是否成功
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Git clone 失败: {}", stderr));
    }

    // 发送进度通知 - 克隆完成
    app_handle
        .emit("repo-mirror-info", "源仓库克隆完成，正在获取分支信息...")
        .unwrap();

    // 获取远程分支列表
    // 执行 git branch -r 命令列出所有远程分支
    let output = Command::new("git")
        .args(["branch", "-r"]) // 列出远程分支
        .current_dir(&repo_path) // 在仓库目录中执行命令
        .output()
        .map_err(|e| format!("执行 git branch -r 命令失败: {}", e))?;

    // 检查命令是否成功执行
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Git branch -r 失败: {}", stderr));
    }

    // 解析远程分支输出
    // 处理 git branch -r 命令的输出，提取有效的分支名称
    let output_str = String::from_utf8_lossy(&output.stdout);
    let remote_branches: Vec<&str> = output_str
        .lines() // 按行分割输出
        .map(|line| line.trim()) // 去除每行首尾空白
        .filter(|line| {
            !line.is_empty()   // 过滤空行
            && !line.contains("->")      // 过滤指向其他分支的指针行
            && !line.contains("HEAD")
        }) // 过滤 HEAD 引用
        .map(|line| line.strip_prefix("origin/").unwrap_or(line)) // 移除 "origin/" 前缀
        .collect();

    // 发送进度通知 - 开始处理分支
    app_handle
        .emit(
            "repo-mirror-info",
            format!("开始处理 {} 个分支...", remote_branches.len()),
        )
        .unwrap();

    // 为每个远程分支创建本地分支并设置跟踪
    // 遍历所有远程分支，为每个分支创建对应的本地分支
    for (index, branch) in remote_branches.iter().enumerate() {
        // 执行 git checkout -b <branch> origin/<branch> 命令
        // 创建本地分支并设置跟踪到远程分支
        let output = Command::new("git")
            .args(["checkout", "-b", branch, &format!("origin/{}", branch)])
            .current_dir(&repo_path)
            .output()
            .map_err(|e| format!("执行 git checkout 创建分支 {} 失败: {}", branch, e))?;

        // 检查分支创建是否成功
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("创建分支 {} 时出错: {}", branch, stderr));
        }

        // 发送分支处理进度通知
        app_handle
            .emit(
                "repo-mirror-info",
                format!(
                    "正在处理分支: {} ({}/{})",
                    branch,
                    index + 1,
                    remote_branches.len()
                ),
            )
            .unwrap();
    }

    // 发送进度通知 - 分支处理完成，开始配置远程仓库
    app_handle
        .emit("repo-mirror-info", "所有分支处理完成，正在配置远程仓库...")
        .unwrap();

    // 重命名原始远程仓库
    // 将原始的 origin 远程仓库重命名为 old-origin，避免与新目标仓库冲突
    let output = Command::new("git")
        .args(["remote", "rename", "origin", "old-origin"]) // 重命名远程仓库
        .current_dir(&repo_path)
        .output()
        .map_err(|e| format!("执行 git remote rename 命令失败: {}", e))?;

    // 检查重命名是否成功
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("重命名远程仓库 origin 失败: {}", stderr));
    }

    // 添加新的远程仓库
    // 添加目标仓库作为新的 origin 远程仓库
    let output = Command::new("git")
        .args(["remote", "add", "origin", &to]) // 添加新的远程仓库
        .current_dir(&repo_path)
        .output()
        .map_err(|e| format!("执行 git remote add 命令失败: {}", e))?;

    // 检查添加远程仓库是否成功
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("添加新的远程仓库 origin 失败: {}", stderr));
    }

    // 发送进度通知 - 远程仓库配置完成，开始推送
    app_handle
        .emit("repo-mirror-info", "远程仓库配置完成，开始推送所有分支...")
        .unwrap();

    // 推送所有分支到目标仓库
    // 使用 --set-upstream 设置上游分支，--all 推送所有分支
    let output = Command::new("git")
        .args(["push", "--set-upstream", "origin", "--all"]) // 推送所有分支并设置上游
        .current_dir(&repo_path)
        .output()
        .map_err(|e| format!("执行 git push --all 命令失败: {}", e))?;

    // 检查推送是否成功
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("推送所有分支失败: {}", stderr));
    }

    // 发送进度通知 - 分支推送完成，开始推送标签
    app_handle
        .emit("repo-mirror-info", "所有分支推送完成，正在推送标签...")
        .unwrap();

    // 推送所有标签到目标仓库
    // 使用 --tags 选项推送所有标签到目标仓库
    let output = Command::new("git")
        .args(["push", "--set-upstream", "origin", "--tags"]) // 推送所有标签
        .current_dir(&repo_path)
        .output()
        .map_err(|e| format!("执行 git push --tags 命令失败: {}", e))?;

    // 检查推送标签是否成功
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("推送标签失败: {}", stderr));
    }

    // 发送进度通知 - 所有操作完成
    app_handle
        .emit("repo-mirror-info", "所有操作完成，正在清理临时文件...")
        .unwrap();

    // 清理临时目录
    // 通过 drop 临时目录句柄来确保目录被删除
    // tempfile crate 会在 temp_dir 离开作用域时自动删除临时目录
    drop(temp_dir);

    // 发送完成通知
    app_handle
        .emit("repo-mirror-success", "仓库镜像操作完成！")
        .unwrap();

    Ok(())
}
