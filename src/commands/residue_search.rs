//! # 软件卸载残留查找工具 (residue_search)
//!
//! 扫描 Windows 系统常见的软件安装和配置文件存储位置,查找与指定软件名匹配的目录。
//!
//! ## 功能特性
//!
//! - 扫描 7 个 Windows 系统常见目录
//! - 向下递归最多 3 层
//! - 子串匹配,大小写不敏感
//! - 仅匹配目录,不匹配文件
//! - 计算目录递归总大小
//! - 输出完整路径、大小和修改时间
//! - 权限不足时自动跳过

use crate::utils::filesystem::calculate_dir_size;
use anyhow::Result;
use bytesize::ByteSize;
use chrono::{DateTime, Local};
use clap::Args;
use inquire::MultiSelect;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// 命令行参数结构体
#[derive(Args, Debug)]
#[command(name = "residue-search")]
#[command(version = "0.1.0")]
#[command(
    about = "查找软件卸载残留目录",
    long_about = "扫描 Windows 系统常见目录,查找指定软件的卸载残留目录。支持子串匹配(大小写不敏感),最多向下扫描 3 层目录。仅匹配目录,不匹配文件。"
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

/// 匹配项结构
#[derive(Debug)]
pub struct MatchedItem {
    /// 匹配目录的完整绝对路径
    pub path: PathBuf,
    /// 大小(字节),目录递归总大小
    pub size: u64,
    /// 最后修改时间
    pub modified_time: SystemTime,
}

/// 构建扫描路径列表
///
/// 根据 Windows 系统环境变量构建所有需要扫描的根目录列表。
///
/// # 返回值
///
/// 返回扫描根目录路径列表。如果某个环境变量未定义,会跳过该路径,并输出提示。
fn build_scan_roots() -> Result<Vec<PathBuf>> {
    let mut roots = Vec::new();

    // 1. C:\Program Files
    match env::var("ProgramFiles") {
        Ok(program_files) => roots.push(PathBuf::from(program_files)),
        Err(_) => println!("环境变量 ProgramFiles 未设置, 已跳过 C:\\Program Files"),
    }

    // 2. C:\Program Files (x86)
    match env::var("ProgramFiles(x86)") {
        Ok(program_files_x86) => roots.push(PathBuf::from(program_files_x86)),
        Err(_) => println!("环境变量 ProgramFiles(x86) 未设置, 已跳过 C:\\Program Files (x86)"),
    }

    // 3. C:\ProgramData
    match env::var("ProgramData") {
        Ok(program_data) => roots.push(PathBuf::from(program_data)),
        Err(_) => println!("环境变量 ProgramData 未设置, 已跳过 C:\\ProgramData"),
    }

    // 4. C:\Users\\[用户名]
    match env::var("USERPROFILE") {
        Ok(user_profile) => roots.push(PathBuf::from(user_profile)),
        Err(_) => println!("环境变量 USERPROFILE 未设置, 已跳过用户主目录"),
    }

    // 5. C:\Users\\[用户名]\\AppData\\Roaming
    match env::var("APPDATA") {
        Ok(appdata) => roots.push(PathBuf::from(appdata)),
        Err(_) => println!("环境变量 APPDATA 未设置, 已跳过 AppData\\Roaming 目录"),
    }

    // 6. C:\Users\\[用户名]\\AppData\\Local
    match env::var("LOCALAPPDATA") {
        Ok(local_appdata) => {
            let local_appdata_path = PathBuf::from(&local_appdata);
            roots.push(local_appdata_path);
        }
        Err(_) => println!("环境变量 LOCALAPPDATA 未设置, 已跳过 AppData\\Local"),
    }

    // 去重(虽然正常情况下不会有重复)
    roots.sort();
    roots.dedup();

    // 过滤出存在的路径, 同时输出不存在的路径
    let mut existing_roots = Vec::new();
    let mut missing_roots = Vec::new();

    for p in roots {
        if p.exists() {
            existing_roots.push(p);
        } else {
            missing_roots.push(p);
        }
    }

    if !missing_roots.is_empty() {
        println!("以下扫描目录不存在, 已跳过:");
        for p in &missing_roots {
            println!("  - {}", p.display());
        }
        println!();
    }

    if existing_roots.is_empty() {
        anyhow::bail!("未找到任何有效的扫描根目录,请检查系统环境变量");
    }

    Ok(existing_roots)
}

/// 扫描目录查找匹配项
///
/// 使用递归深度优先搜索,向下最多扫描指定层数,查找匹配软件名的目录。
///
/// # 参数
///
/// * `root` - 扫描根目录
/// * `software_name_lower` - 软件名的小写形式(用于匹配)
/// * `max_depth` - 最大递归深度(从根目录开始计数,根目录为第0层)
///
/// # 返回值
///
/// 返回匹配的目录列表,每个匹配项包含路径、大小和修改时间。
fn scan_directory(
    root: &Path,
    software_name_lower: &str,
    max_depth: usize,
) -> Result<Vec<MatchedItem>> {
    fn scan_recursive(
        current_path: &Path,
        depth: usize,
        max_depth: usize,
        software_name_lower: &str,
        matched_map: &mut HashMap<PathBuf, MatchedItem>,
    ) {
        // 读取当前目录的所有子项
        let entries = match fs::read_dir(current_path) {
            Ok(entries) => entries,
            Err(_) => {
                // 权限不足或其他错误时跳过
                return;
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let entry_path = entry.path();

            // 提取文件名
            let file_name = match entry_path.file_name() {
                Some(name) => name.to_string_lossy().to_lowercase(),
                None => continue,
            };

            // 判断是否为目录
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            // 如果是文件,直接跳过
            if !metadata.is_dir() {
                continue;
            }

            // 检查目录名是否匹配软件名
            if file_name.contains(software_name_lower) {
                // 检查路径是否已经在哈希表中
                if matched_map.contains_key(&entry_path) {
                    // 已经添加过,跳过
                    continue;
                }

                // 获取修改时间,失败则跳过该项
                let modified_time = match metadata.modified() {
                    Ok(time) => time,
                    Err(_) => continue,
                };

                // 计算目录大小
                let size = calculate_dir_size(&entry_path);

                let item = MatchedItem {
                    path: entry_path.clone(),
                    size,
                    modified_time,
                };

                // 直接存储键值对
                matched_map.insert(entry_path.clone(), item);

                continue;
            }

            // 如果深度已达到最大值,停止递归
            if depth >= max_depth {
                continue;
            }

            // 目录未匹配,继续递归遍历
            scan_recursive(
                &entry_path,
                depth + 1,
                max_depth,
                software_name_lower,
                matched_map,
            );
        }
    }

    let mut matched_map = HashMap::new();

    // 从根目录开始,深度为 0
    scan_recursive(root, 0, max_depth, software_name_lower, &mut matched_map);

    // 将 HashMap 转换为 Vec
    Ok(matched_map.into_values().collect())
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

    // 扫描所有根目录, 结果依次累加
    let mut all_matched_items: Vec<MatchedItem> = Vec::new();

    for root in &scan_roots {
        let mut matches = scan_directory(root, &software_name_lower, 3)?;
        all_matched_items.append(&mut matches);
    }

    // 输出匹配结果
    println!("{} 匹配结果 {}", "=".repeat(20), "=".repeat(20));
    println!();

    if all_matched_items.is_empty() {
        println!("未找到匹配的目录");
    } else {
        for item in &all_matched_items {
            println!("  {}", item.path.display());
            println!("    大小: {}", ByteSize(item.size));
            let datetime: DateTime<Local> = item.modified_time.into();
            println!("    修改时间: {}", datetime.format("%Y-%m-%d %H:%M:%S"));
            println!();
        }
    }

    // 统计结果
    println!("{} 统计结果 {}", "=".repeat(20), "=".repeat(20));

    let total_size: u64 = all_matched_items.iter().map(|item| item.size).sum();
    let total_count = all_matched_items.len();

    println!("匹配的目录: {} 个", total_count);
    println!("总大小: {}", ByteSize(total_size));

    // 提供所有匹配目录供交互式选择
    if all_matched_items.is_empty() {
        println!("\n没有匹配的目录可供删除");
        return Ok(());
    }

    // 构建选项列表 - 纯路径字符串
    let options: Vec<String> = all_matched_items
        .iter()
        .map(|item| item.path.display().to_string())
        .collect();

    // 使用 MultiSelect 让用户选择要删除的目录
    println!();
    let selected = match MultiSelect::new("请选择要删除的目录", options).prompt() {
        Ok(selected) => selected,
        Err(_) => {
            println!("操作已取消");
            return Ok(());
        }
    };

    if selected.is_empty() {
        println!("未选择任何项,操作已取消");
        return Ok(());
    }

    // 将选中的路径字符串转换为 PathBuf
    let selected_paths: Vec<PathBuf> = selected.iter().map(|s| PathBuf::from(s)).collect();

    // 确认删除
    println!("\n即将删除以下 {} 个目录:", selected_paths.len());
    for path in &selected_paths {
        println!("  {}", path.display());
    }
    println!();

    // 执行删除
    let mut success_count = 0;
    let mut fail_count = 0;

    for path in selected_paths {
        let result = fs::remove_dir_all(&path);

        match result {
            Ok(_) => {
                println!("✓ 成功删除: {}", path.display());
                success_count += 1;
            }
            Err(e) => {
                println!("✗ 删除失败: {} - {}", path.display(), e);
                fail_count += 1;
            }
        }
    }

    println!();
    println!(
        "删除完成: 成功 {} 个, 失败 {} 个",
        success_count, fail_count
    );

    Ok(())
}
