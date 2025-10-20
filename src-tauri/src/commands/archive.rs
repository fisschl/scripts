//! 压缩和解压模块
//!
//! 提供前端可调用的文件压缩和解压命令，支持7z格式

use std::path::Path;
use std::process::Command;
use tauri::command;

/// 查找系统中的7z可执行文件路径
///
/// 在Windows系统中查找已安装的7-Zip软件的可执行文件。
/// 按优先级顺序检查常见的安装位置。
///
/// # 返回值
///
/// * `Ok(String)` - 成功时返回7z.exe的完整路径
/// * `Err(String)` - 失败时返回错误描述
///
/// # 查找位置
///
/// 1. 系统PATH环境变量中的7z命令
/// 2. 常见的安装目录：
///    - C:\Program Files\7-Zip\7z.exe
///    - C:\Program Files (x86)\7-Zip\7z.exe
fn find_7z_executable() -> Result<String, String> {
    // 首先检查系统PATH中是否有7z命令
    if let Ok(output) = Command::new("7z").arg("--help").output() {
        if output.status.success() {
            return Ok("7z".to_string());
        }
    }

    // 常见的7-Zip安装路径
    let common_paths = vec![
        "C:\\Program Files\\7-Zip\\7z.exe",
        "C:\\Program Files (x86)\\7-Zip\\7z.exe",
    ];

    for path in common_paths {
        if Path::new(path).exists() {
            return Ok(path.to_string());
        }
    }

    Err("未找到7-Zip安装，请确保已安装7-Zip软件".to_string())
}

/// 使用7z压缩文件或目录
///
/// 使用系统安装的7-Zip软件对指定的文件或目录进行压缩。
/// 压缩包将保存在与源文件相同的目录中，使用相同的文件名（扩展名为.7z）。
///
/// # 参数
///
/// * `source_path` - 要压缩的文件或目录的完整路径
///
/// # 返回值
///
/// * `Ok(String)` - 压缩成功，返回压缩包的完整路径
/// * `Err(String)` - 失败时返回错误描述
///
/// # 行为
///
/// * 压缩包保存在源文件同目录下，文件名与源文件相同，扩展名为.7z
/// * 使用默认压缩级别
/// * 不设置密码保护
/// * 如果源文件是目录，将递归压缩所有内容
/// * 如果目标压缩包已存在，将覆盖
#[command]
pub fn compress_with_7z(source_path: String) -> Result<String, String> {
    let source_path = Path::new(&source_path);

    // 验证源路径存在
    if !source_path.exists() {
        return Err("源文件或目录不存在".to_string());
    }

    // 查找7z可执行文件
    let seven_zip = find_7z_executable()?;

    // 确定压缩包路径
    let source_name = source_path
        .file_name()
        .ok_or("无法获取源文件名")?
        .to_string_lossy();

    let archive_path = source_path
        .with_file_name(format!("{}.7z", source_name))
        .to_string_lossy()
        .to_string();

    // 构建7z压缩命令
    let output = Command::new(&seven_zip)
        .arg("a") // 添加到压缩包
        .arg(&archive_path) // 压缩包路径
        .arg(source_path.to_string_lossy().as_ref()) // 源文件/目录路径
        .output()
        .map_err(|e| format!("执行7z命令失败: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("7z压缩失败: {}", stderr));
    }

    Ok(archive_path)
}
