//! # 压缩并删除工具 (compress_delete)
//!
//! 一个简洁高效的 Rust 命令行工具，用于压缩指定目录下的文件和子目录，
//! 然后删除原始文件，仅保留压缩后的 7z 文件。
//!
//! ## 功能特性
//!
//! - **简洁设计**：只处理工作目录的直接子项，不递归遍历
//! - **智能过滤**：自动跳过隐藏文件、开发文件和常见压缩文件
//! - **7-Zip 集成**：使用系统安装的 7-Zip 进行压缩，默认设置
//! - **自动清理**：压缩成功后自动删除原始项目
//! - **错误处理**：单个项目失败不影响其他项目处理
//! - **跨平台**：支持 Windows、macOS 和 Linux
//!
//! ## 使用方法
//!
//! ```bash
//! compress_delete [OPTIONS]
//! ```
//!
//! ## 参数说明
//!
//! - `[--directory, -d] <DIRECTORY>`: 要处理的目录路径，默认为当前目录
//!
//! ## 示例
//!
//! ```bash
//! # 压缩当前目录下所有项目
//! compress_delete
//!
//! # 指定工作目录
//! compress_delete --directory ./backup
//!
//! # 使用短选项
//! compress_delete -d ./projects
//! ```
//!
//! ## 工作流程
//!
//! 1. 扫描指定目录的直接子项
//! 2. 过滤隐藏文件和指定扩展名的文件
//! 3. 查找系统安装的 7-Zip 可执行文件
//! 4. 对每个项目创建 `.7z` 压缩文件
//! 5. 压缩成功后删除原始项目
//!
//! ## 注意事项
//!
//! - 此工具会永久删除原始文件，请确保有备份
//! - 需要系统安装 7-Zip 并在 PATH 中，或在标准安装位置
//! - 压缩文件与原始项目同名，扩展名为 `.7z`

use anyhow::{Context, Result};
use clap::Parser;
use dirs::home_dir;
use file_utils::utils::remove_path;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use walkdir::WalkDir;

/// 命令行参数结构体
///
/// 使用 clap 的 Derive API 自动解析命令行参数，
/// 提供类型安全和自动生成的帮助信息。
#[derive(Parser, Debug)]
#[command(name = "compress_delete")]
#[command(version = "0.1.0")]
#[command(about = "使用 7-Zip 压缩文件和目录,然后删除原始项目")]
struct Args {
    /// 要处理的工作目录路径
    ///
    /// 指定包含要压缩和删除的项目的目录。
    /// 工具只会处理该目录的直接子项,不会递归遍历。
    /// 默认为当前目录(".")。
    #[arg(short = 'd', long, default_value = ".")]
    directory: PathBuf,

    /// 压缩文件密码
    ///
    /// 为压缩文件设置密码保护。
    /// 启用后将同时加密文件内容和文件名(使用 -mhe=on 选项)。
    /// 如果不指定此参数,则不使用密码加密。
    #[arg(short = 'p', long)]
    password: Option<String>,
}

/// 查找系统中安装的 7-Zip 可执行文件
///
/// 按照优先级顺序查找 7-Zip：
/// 1. PATH 环境变量中的 `7z` 命令
/// 2. Windows 常见安装路径（Program Files 和 Program Files (x86)）
/// 3. 用户目录下的安装路径
///
/// # 返回值
///
/// * `Ok(PathBuf)` - 找到的 7z 可执行文件路径
/// * `Err(anyhow::Error)` - 未找到 7z 可执行文件
///
/// # 示例
///
/// ```rust
/// let seven_zip_path = find_7z_executable()?;
/// println!("找到 7-Zip: {}", seven_zip_path.display());
/// ```
fn find_7z_executable() -> Result<PathBuf> {
    // 首先检查 PATH 环境变量中的 7z 命令
    // 这是最常见和最方便的方式
    if which::which("7z").is_ok() {
        return Ok(PathBuf::from("7z"));
    }

    // 检查常见的 Windows 安装路径，按优先级排序
    // 7-Zip 通常安装在 Program Files 目录下
    let common_paths = vec![
        PathBuf::from(r"C:\Program Files\7-Zip\7z.exe"),
        PathBuf::from(r"C:\Program Files (x86)\7-Zip\7z.exe"),
        PathBuf::from(r"C:\7-Zip\7z.exe"),
    ];

    // 首先检查常见安装路径
    for path in &common_paths {
        if path.exists() {
            return Ok(path.clone());
        }
    }

    // 常见路径没找到，检查用户目录
    // 支持用户自定义安装位置
    if let Some(home_dir) = home_dir() {
        let user_paths = vec![
            home_dir.join("AppData\\Local\\Programs\\7-Zip\\7z.exe"),
            home_dir.join("7-Zip\\7z.exe"),
        ];

        for path in &user_paths {
            if path.exists() {
                return Ok(path.clone());
            }
        }
    }

    // 如果所有路径都未找到，返回错误并提供下载链接
    anyhow::bail!("未找到 7z 可执行文件。请从 https://www.7-zip.org/ 安装 7-Zip");
}

/// 使用 7-Zip 压缩文件或目录
///
/// 异步执行 7-Zip 命令来压缩指定的文件或目录。
/// 使用默认压缩设置,提供良好的压缩比和速度平衡。
///
/// # 参数
///
/// * `item_path` - 要压缩的文件或目录路径
/// * `output_path` - 输出的 7z 压缩文件路径
/// * `seven_zip_path` - 7-Zip 可执行文件路径
/// * `password` - 可选的压缩文件密码
///
/// # 返回值
///
/// * `Ok(())` - 压缩成功
/// * `Err(anyhow::Error)` - 压缩失败,包含错误信息
///
/// # 7-Zip 参数说明
///
/// - `a` - 添加到压缩文件模式
/// - `-p<password>` - 设置密码
/// - `-mhe=on` - 加密文件头(文件名)
/// - 使用默认压缩级别(通常为 5)
/// - 输出格式固定为 7z
///
/// # 示例
///
/// ```rust
/// let source = Path::new("./my_project");
/// let output = Path::new("./my_project.7z");
/// let seven_zip = find_7z_executable()?;
/// compress_item(&source, &output, &seven_zip, Some("mypassword")).await?;
/// ```
async fn compress_item(
    item_path: &Path,
    output_path: &Path,
    seven_zip_path: &Path,
    password: Option<&str>,
) -> Result<()> {
    // 构建 7-Zip 命令参数
    let mut args = vec![
        "a".to_string(), // "a" 表示添加到压缩文件
        output_path.to_string_lossy().to_string(),
        item_path.to_string_lossy().to_string(),
    ];

    // 如果指定了密码,添加密码参数和文件名加密选项
    if let Some(pwd) = password {
        args.push(format!("-p{}", pwd)); // 设置密码
        args.push("-mhe=on".to_string()); // 加密文件头(文件名)
    }

    println!("执行压缩: {} {}", seven_zip_path.display(), args.join(" "));

    // 执行 7-Zip 命令并等待完成
    let output = tokio::process::Command::new(seven_zip_path)
        .args(&args)
        .stdout(Stdio::piped()) // 捕获标准输出
        .stderr(Stdio::piped()) // 捕获标准错误
        .output()
        .await
        .with_context(|| format!("执行 7z 命令失败: {}", seven_zip_path.display()))?;

    // 检查退出码，如果不成功则返回错误
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("7z 压缩失败: {}", stderr);
    }

    Ok(())
}

/// 收集要处理的项目
///
/// 扫描工作目录的直接子项，应用过滤规则后返回符合条件的文件和目录列表。
/// 只处理顶层项目，不递归遍历子目录。
///
/// # 过滤规则
///
/// 1. 跳过工作目录本身
/// 2. 跳过隐藏文件和目录（以 `.` 开头）
/// 3. 跳过指定扩展名的文件（不带点格式）：
///    - **开发文件**: `ts`, `mjs`, `rs`, `exe`
///    - **常见压缩**: `7z`, `zip`, `rar`, `tar`, `gz`
///    - **Java 文件**: `jar`, `war`, `ear`
///
/// # 参数
///
/// * `work_directory` - 要扫描的工作目录路径
///
/// # 返回值
///
/// * `Ok(Vec<PathBuf>)` - 符合条件的文件和目录路径列表
/// * `Err(anyhow::Error)` - 扫描过程中的错误
///
/// # 示例
///
/// ```rust
/// let work_dir = Path::new("./projects");
/// let items = collect_items(work_dir)?;
/// println!("找到 {} 个项目", items.len());
/// ```
fn collect_items(work_directory: &Path) -> Result<Vec<PathBuf>> {
    // 定义要跳过的文件扩展名
    let skip_extensions = [
        "ts", "mjs", "rs", "exe", "7z", "zip", "rar", "tar", "gz", "jar", "war", "ear",
    ];

    // 使用函数式编程风格收集符合条件的项目
    let items: Vec<PathBuf> = WalkDir::new(work_directory)
        .max_depth(1) // 只处理直接子项，不递归
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();

            // 跳过工作目录本身和隐藏文件/目录
            e.path() != work_directory && !name.starts_with('.')
        })
        .filter_map(Result::ok) // 忽略遍历错误
        .map(|entry| entry.path().to_path_buf())
        .filter(|path| {
            // 跳过特定扩展名的文件（不带点，小写）
            if let Some(ext) = path
                .extension()
                .and_then(|s| s.to_str())
                .map(|s| s.to_lowercase())
            {
                !skip_extensions.contains(&ext.as_str())
            } else {
                true // 没有扩展名的文件不跳过
            }
        })
        .collect();

    Ok(items)
}

/// 处理单个项目
///
/// 对单个文件或目录执行完整的压缩和删除流程:
/// 1. 生成同名的 .7z 压缩文件路径
/// 2. 检查压缩文件是否已存在,存在则跳过
/// 3. 使用 7-Zip 压缩项目
/// 4. 压缩成功后删除原始项目
///
/// # 参数
///
/// * `item_path` - 要处理的文件或目录路径
/// * `work_directory` - 工作目录路径(用于存放压缩文件)
/// * `seven_zip_path` - 7-Zip 可执行文件路径
/// * `password` - 可选的压缩文件密码
///
/// # 返回值
///
/// * `Ok(())` - 处理成功
/// * `Err(anyhow::Error)` - 处理失败,包含详细错误信息
///
/// # 安全性
///
/// 此函数会永久删除原始文件,只有在压缩成功后才会执行删除操作。
///
/// # 示例
///
/// ```rust
/// let project = Path::new("./my_project");
/// let work_dir = Path::new("./backup");
/// let seven_zip = find_7z_executable()?;
/// process_item(project, work_dir, &seven_zip, Some("password")).await?;
/// ```
async fn process_item(
    item_path: &Path,
    work_directory: &Path,
    seven_zip_path: &Path,
    password: Option<&str>,
) -> Result<()> {
    // 提取项目名称用于显示和生成输出文件名
    let item_name = item_path
        .file_name()
        .and_then(|n| n.to_str())
        .context("无效的项目名称")?;

    println!("处理: {}", item_name);

    // 生成输出路径，压缩文件与原始项目同名，扩展名为 .7z
    let output_path = work_directory.join(format!("{}.7z", item_name));

    // 检查压缩文件是否已存在，避免重复处理
    if output_path.exists() {
        println!(
            "压缩文件已存在: {}",
            output_path.file_name().unwrap().to_string_lossy()
        );
        return Ok(());
    }

    // 使用 7-Zip 压缩项目
    compress_item(item_path, &output_path, seven_zip_path, password).await?;

    // 根据是否使用密码显示不同的提示信息
    if password.is_some() {
        println!(
            "压缩完成(已加密): {} -> {}",
            item_name,
            output_path.file_name().unwrap().to_string_lossy()
        );
    } else {
        println!(
            "压缩完成: {} -> {}",
            item_name,
            output_path.file_name().unwrap().to_string_lossy()
        );
    }

    // 压缩成功后删除原始项目
    remove_path(item_path).await?;
    println!("删除原始项目: {}", item_name);

    Ok(())
}

/// 主函数
///
/// 程序入口点，负责协调整个压缩和删除流程：
/// 1. 解析命令行参数
/// 2. 验证工作目录
/// 3. 收集要处理的项目
/// 4. 查找 7-Zip 可执行文件
/// 5. 逐个处理项目
/// 6. 输出处理结果
///
/// # 错误处理
///
/// - 工作目录不存在：程序退出并显示错误
/// - 找不到 7-Zip：程序退出并提示安装
/// - 单个项目处理失败：记录错误但继续处理其他项目
///
/// # 返回值
///
/// * `Ok(())` - 程序成功执行
/// * `Err(anyhow::Error)` - 程序执行失败
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 解析命令行参数，clap 会自动生成帮助信息
    let args = Args::parse();

    // 获取工作目录路径
    let work_directory = args.directory;

    // 验证工作目录是否存在
    if !work_directory.exists() {
        anyhow::bail!("工作目录不存在: {}", work_directory.display());
    }

    // 显示程序标题和工作目录信息
    println!("{} 压缩并删除工具 {}", "=".repeat(15), "=".repeat(15));
    println!("工作目录: {}", work_directory.display());

    // 显示密码设置状态
    if args.password.is_some() {
        println!("加密模式: 已启用(加密文件内容和文件名)");
    } else {
        println!("加密模式: 未启用");
    }
    println!();

    // 收集要处理的项目（应用过滤规则）
    let items = collect_items(&work_directory)?;

    // 如果没有找到项目，直接返回
    if items.is_empty() {
        println!("没有找到要处理的项目");
        return Ok(());
    }

    println!("找到 {} 个项目要处理\n", items.len());

    // 查找系统安装的 7-Zip 可执行文件
    let seven_zip_path = find_7z_executable().context("找不到 7z 可执行文件")?;

    // 逐个处理项目,单个失败不影响其他项目
    for item in items {
        if let Err(e) = process_item(
            &item,
            &work_directory,
            &seven_zip_path,
            args.password.as_deref(),
        )
        .await
        {
            println!("处理 {} 失败: {}", item.display(), e);
        }
    }

    // 显示完成信息
    println!("操作成功完成！");
    Ok(())
}
