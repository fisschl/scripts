//! # 软件卸载残留查找工具 (residue_search)
//!
//! 扫描 Windows 系统常见的软件安装和配置文件存储位置,查找与指定软件名匹配的目录和文件。
//!
//! ## 功能特性
//!
//! - 扫描 7 个 Windows 系统常见目录
//! - 向下递归最多 3 层
//! - 子串匹配,大小写不敏感
//! - 计算目录递归总大小
//! - 输出完整路径、大小和修改时间
//! - 权限不足时抛出异常

use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use clap::Args;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

/// 命令行参数结构体
#[derive(Args, Debug)]
#[command(name = "residue-search")]
#[command(version = "0.1.0")]
#[command(
    about = "查找软件卸载残留",
    long_about = "扫描 Windows 系统常见目录,查找指定软件的卸载残留文件和目录。支持子串匹配(大小写不敏感),最多向下扫描 3 层目录。"
)]
pub struct ResidueSearchArgs {
    /// 要查找的软件名称
    ///
    /// 支持子串匹配,大小写不敏感。例如输入 "chrome" 可以匹配 "Google Chrome", "ChromeSetup.exe" 等。
    #[arg(
        short = 's',
        long = "software",
        value_name = "NAME",
        help = "要查找的软件名称",
        long_help = "要查找的软件名称。支持子串匹配,大小写不敏感。例如输入 \"chrome\" 可以匹配 \"Google Chrome\", \"ChromeSetup.exe\" 等。"
    )]
    pub software_name: String,
}

/// 匹配项类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemType {
    /// 目录
    Directory,
    /// 文件
    File,
}

/// 匹配项结构
#[derive(Debug)]
pub struct MatchedItem {
    /// 匹配项的完整绝对路径
    pub path: PathBuf,
    /// 目录或文件
    pub item_type: ItemType,
    /// 大小(字节),目录为递归总大小
    pub size: u64,
    /// 最后修改时间
    pub modified_time: SystemTime,
    /// 所属的扫描根目录
    pub scan_root: PathBuf,
}

/// 构建扫描路径列表
///
/// 根据 Windows 系统环境变量构建所有需要扫描的根目录列表。
///
/// # 返回值
///
/// 返回扫描根目录路径列表。如果某个环境变量未定义,会跳过该路径。
fn build_scan_roots() -> Result<Vec<PathBuf>> {
    let mut roots = Vec::new();

    // 1. C:\Program Files
    if let Ok(program_files) = env::var("ProgramFiles") {
        roots.push(PathBuf::from(program_files));
    }

    // 2. C:\Program Files (x86)
    if let Ok(program_files_x86) = env::var("ProgramFiles(x86)") {
        roots.push(PathBuf::from(program_files_x86));
    }

    // 3. C:\ProgramData
    if let Ok(program_data) = env::var("ProgramData") {
        roots.push(PathBuf::from(program_data));
    }

    // 4. C:\Users\[用户名]
    if let Ok(user_profile) = env::var("USERPROFILE") {
        roots.push(PathBuf::from(user_profile));
    }

    // 5. C:\Users\[用户名]\AppData\Roaming
    if let Ok(appdata) = env::var("APPDATA") {
        roots.push(PathBuf::from(appdata));
    }

    // 6. C:\Users\[用户名]\AppData\Local
    if let Ok(local_appdata) = env::var("LOCALAPPDATA") {
        let local_appdata_path = PathBuf::from(&local_appdata);
        roots.push(local_appdata_path.clone());

        // 7. C:\Users\[用户名]\AppData\Local\Low
        let low_path = local_appdata_path.join("Low");
        if low_path.exists() {
            roots.push(low_path);
        }
    }

    // 去重(虽然正常情况下不会有重复)
    roots.sort();
    roots.dedup();

    // 过滤出存在的路径
    let existing_roots: Vec<PathBuf> = roots.into_iter().filter(|p| p.exists()).collect();

    if existing_roots.is_empty() {
        anyhow::bail!("未找到任何有效的扫描根目录,请检查系统环境变量");
    }

    Ok(existing_roots)
}

/// 扫描目录查找匹配项
///
/// 使用栈模拟深度优先搜索,向下最多扫描 3 层,查找匹配软件名的目录和文件。
///
/// # 参数
///
/// * `root` - 扫描根目录
/// * `software_name_lower` - 软件名的小写形式(用于匹配)
/// * `max_depth` - 最大递归深度(从根目录开始计数,根目录为第0层)
///
/// # 返回值
///
/// 返回匹配项路径列表(不包含大小和修改时间信息)
fn scan_directory(
    root: &Path,
    software_name_lower: &str,
    _max_depth: usize,
) -> Result<Vec<(PathBuf, ItemType)>> {
    let mut matched_items = Vec::new();

    // 使用栈模拟 DFS: (路径, 深度)
    let mut stack: Vec<(PathBuf, usize)> = vec![(root.to_path_buf(), 0)];

    while let Some((current_path, depth)) = stack.pop() {
        // 读取当前目录的所有子项
        let entries = match fs::read_dir(&current_path) {
            Ok(entries) => entries,
            Err(e) => {
                // 权限不足时抛出异常
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    anyhow::bail!(
                        "无法访问目录(权限不足): {}\n错误信息: {}\n提示: 请使用管理员权限运行此工具",
                        current_path.display(),
                        e
                    );
                }
                // 其他错误跳过
                continue;
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::PermissionDenied {
                        anyhow::bail!(
                            "无法访问目录项(权限不足): {}\n错误信息: {}\n提示: 请使用管理员权限运行此工具",
                            current_path.display(),
                            e
                        );
                    }
                    continue;
                }
            };

            let entry_path = entry.path();

            // 提取文件名
            let file_name = match entry_path.file_name() {
                Some(name) => name.to_string_lossy().to_lowercase(),
                None => continue,
            };

            // 判断是目录还是文件
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::PermissionDenied {
                        anyhow::bail!(
                            "无法访问文件元数据(权限不足): {}\n错误信息: {}\n提示: 请使用管理员权限运行此工具",
                            entry_path.display(),
                            e
                        );
                    }
                    continue;
                }
            };

            let is_dir = metadata.is_dir();
            let item_type = if is_dir {
                ItemType::Directory
            } else {
                ItemType::File
            };

            // 检查是否匹配软件名
            if file_name.contains(software_name_lower) {
                matched_items.push((entry_path.clone(), item_type));
            }

            // 如果是目录且深度未达到最大值,压入栈继续遍历
            // 深度 0, 1, 2 可以继续向下(对应第 1, 2, 3 层)
            if is_dir && depth < 3 {
                stack.push((entry_path, depth + 1));
            }
        }
    }

    Ok(matched_items)
}

/// 计算文件或目录的大小
///
/// 对于文件,直接返回文件大小。
/// 对于目录,递归遍历所有文件并累加大小。
///
/// # 参数
///
/// * `path` - 文件或目录路径
///
/// # 返回值
///
/// 返回大小(字节数)
fn calculate_size(path: &Path) -> Result<u64> {
    let metadata = fs::metadata(path).with_context(|| {
        format!(
            "无法读取文件元数据(权限不足): {}\n提示: 请使用管理员权限运行此工具",
            path.display()
        )
    })?;

    if metadata.is_file() {
        Ok(metadata.len())
    } else if metadata.is_dir() {
        let mut total_size = 0u64;

        for entry in WalkDir::new(path).into_iter() {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    // 权限不足时抛出异常
                    if let Some(io_error) = e.io_error() {
                        if io_error.kind() == std::io::ErrorKind::PermissionDenied {
                            anyhow::bail!(
                                "无法访问目录(权限不足): {}\n错误信息: {}\n提示: 请使用管理员权限运行此工具",
                                path.display(),
                                io_error
                            );
                        }
                    }
                    continue;
                }
            };

            if entry.file_type().is_file() {
                let file_metadata = match entry.metadata() {
                    Ok(m) => m,
                    Err(_e) => {
                        // 权限不足时抛出异常
                        anyhow::bail!(
                            "无法读取文件元数据(权限不足): {}\n提示: 请使用管理员权限运行此工具",
                            entry.path().display()
                        );
                    }
                };
                total_size += file_metadata.len();
            }
        }

        Ok(total_size)
    } else {
        Ok(0)
    }
}

/// 格式化文件大小为人类可读格式
///
/// 自动选择合适的单位(B/KB/MB/GB/TB)。
///
/// # 参数
///
/// * `size` - 字节数
///
/// # 返回值
///
/// 返回格式化的大小字符串,例如 "1.5 GB"
fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if size >= TB {
        format!("{:.1} TB", size as f64 / TB as f64)
    } else if size >= GB {
        format!("{:.1} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1} MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1} KB", size as f64 / KB as f64)
    } else {
        format!("{} B", size)
    }
}

/// 格式化时间为指定格式
///
/// 格式: YYYY-MM-DD HH:MM:SS
///
/// # 参数
///
/// * `time` - 系统时间
///
/// # 返回值
///
/// 返回格式化的时间字符串
fn format_time(time: SystemTime) -> String {
    let datetime: DateTime<Local> = time.into();
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 命令执行函数
pub async fn run(args: ResidueSearchArgs) -> Result<()> {
    // 验证软件名参数
    let software_name = args.software_name.trim();
    if software_name.is_empty() {
        anyhow::bail!("软件名不能为空或仅包含空白字符");
    }

    let software_name_lower = software_name.to_lowercase();

    // 显示工具信息头部
    println!(
        "{}  软件卸载残留查找工具  {}",
        "=".repeat(15),
        "=".repeat(15)
    );
    println!("查询软件: {}", software_name);
    println!();

    // 构建扫描路径列表
    let scan_roots = build_scan_roots()?;

    // 显示扫描位置
    println!("扫描位置:");
    for root in &scan_roots {
        println!("  - {}", root.display());
    }
    println!();

    println!("正在扫描,请稍候...");
    println!();

    // 扫描所有根目录
    let mut all_matched_items: Vec<MatchedItem> = Vec::new();

    for root in &scan_roots {
        let matches = scan_directory(root, &software_name_lower, 3)?;

        for (path, item_type) in matches {
            // 获取修改时间
            let modified_time = match fs::metadata(&path) {
                Ok(metadata) => match metadata.modified() {
                    Ok(time) => time,
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::PermissionDenied {
                            anyhow::bail!(
                                "无法读取文件修改时间(权限不足): {}\n提示: 请使用管理员权限运行此工具",
                                path.display()
                            );
                        }
                        continue;
                    }
                },
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::PermissionDenied {
                        anyhow::bail!(
                            "无法读取文件元数据(权限不足): {}\n提示: 请使用管理员权限运行此工具",
                            path.display()
                        );
                    }
                    continue;
                }
            };

            // 计算大小
            let size = calculate_size(&path)?;

            all_matched_items.push(MatchedItem {
                path,
                item_type,
                size,
                modified_time,
                scan_root: root.clone(),
            });
        }
    }

    // 按扫描根目录分组
    let mut grouped_items: HashMap<PathBuf, Vec<&MatchedItem>> = HashMap::new();
    for item in &all_matched_items {
        grouped_items
            .entry(item.scan_root.clone())
            .or_insert_with(Vec::new)
            .push(item);
    }

    // 输出匹配结果
    println!("{} 匹配结果 {}", "=".repeat(20), "=".repeat(20));
    println!();

    if all_matched_items.is_empty() {
        println!("未找到匹配的文件或目录");
    } else {
        // 按扫描根目录顺序输出
        for root in &scan_roots {
            if let Some(items) = grouped_items.get(root) {
                if !items.is_empty() {
                    println!("[{}]", root.display());

                    for item in items {
                        let type_label = match item.item_type {
                            ItemType::Directory => "[目录]",
                            ItemType::File => "[文件]",
                        };

                        println!("  {} {}", type_label, item.path.display());
                        println!("         大小: {}", format_size(item.size));
                        println!("         修改时间: {}", format_time(item.modified_time));
                        println!();
                    }
                }
            }
        }
    }

    // 统计信息
    println!("{} 统计结果 {}", "=".repeat(20), "=".repeat(20));

    let dir_count = all_matched_items
        .iter()
        .filter(|item| item.item_type == ItemType::Directory)
        .count();

    let file_count = all_matched_items
        .iter()
        .filter(|item| item.item_type == ItemType::File)
        .count();

    let total_size: u64 = all_matched_items.iter().map(|item| item.size).sum();

    println!("匹配的目录: {} 个", dir_count);
    println!("匹配的文件: {} 个", file_count);
    println!("总计: {} 项", all_matched_items.len());
    println!("总大小: {}", format_size(total_size));

    Ok(())
}
