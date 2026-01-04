//! # 压缩相关工具
//!
//! 提供基于 7-Zip 的通用压缩函数，例如将文件或目录压缩为 .7z。

use std::path::Path;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::OnceLock;

/// 7-Zip 可执行文件路径缓存
static SEVEN_ZIP_PATH: OnceLock<PathBuf> = OnceLock::new();

/// 查找系统中安装的 7-Zip 可执行文件（带缓存）
///
/// 首次调用时按优先级顺序查找 7-Zip：
/// 1. PATH 环境变量中的 `7z` 命令
/// 2. Windows 常见安装路径（Program Files 和 Program Files (x86)）
/// 3. 用户目录下的安装路径
///
/// 后续调用直接返回缓存结果，避免重复查找。
///
/// # Panics
///
/// 如果未找到 7-Zip 可执行文件，会 panic。
pub fn find_7z_executable() -> &'static Path {
    SEVEN_ZIP_PATH
        .get_or_init(find_7z_executable_inner)
        .as_path()
}

/// 实际执行 7-Zip 查找的内部函数
fn find_7z_executable_inner() -> PathBuf {
    if which::which("7z").is_ok() {
        return PathBuf::from("7z");
    }
    let home_dir = dirs::home_dir().unwrap();
    let common_paths = [
        PathBuf::from("C:\\Program Files\\7-Zip\\7z.exe"),
        PathBuf::from("C:\\Program Files (x86)\\7-Zip\\7z.exe"),
        PathBuf::from("C:\\7-Zip\\7z.exe"),
        home_dir.join("AppData\\Local\\Programs\\7-Zip\\7z.exe"),
        home_dir.join("7-Zip\\7z.exe"),
    ];
    for path in &common_paths {
        if path.exists() {
            return path.clone();
        }
    }
    panic!("未找到 7z 可执行文件。请从 https://www.7-zip.org/ 安装 7-Zip");
}

/// 使用 7-Zip 压缩文件或目录为 .7z
///
/// `item_path` 可以是文件或目录，`output_path` 为目标 .7z 文件路径。
/// 如果提供 `password`，会同时加密内容和文件名（`-mhe=on`）。
///
/// # Panics
///
/// 如果压缩命令执行失败或返回非零退出码，会 panic。
pub async fn compress_7z(item_path: &Path, output_path: &Path, password: Option<&str>) {
    let seven_zip_path = find_7z_executable();

    let mut args = vec![
        "a".to_string(),
        output_path.to_string_lossy().to_string(),
        item_path.to_string_lossy().to_string(),
    ];

    if let Some(pwd) = password {
        args.push(format!("-p{}", pwd));
        args.push("-mhe=on".to_string());
    }

    println!("执行压缩: {} {}", seven_zip_path.display(), args.join(" "));

    let mut child = tokio::process::Command::new(&seven_zip_path)
        .args(&args)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap_or_else(|e| panic!("执行 7z 命令失败: {}: {}", seven_zip_path.display(), e));

    let status = child.wait().await.expect("等待 7z 命令完成失败");

    if !status.success() {
        panic!("7z 压缩失败，退出码: {}", status.code().unwrap_or(-1));
    }
}
