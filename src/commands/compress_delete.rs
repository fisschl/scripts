//! # 压缩并删除工具 (compress_delete)
//!
//! 一个简洁高效的 Rust 命令行工具，用于压缩指定目录下的文件和子目录，
//! 然后删除原始文件，仅保留压缩后的 7z 文件。

use crate::utils::filesystem::{get_file_extension, remove_path};
use anyhow::{Context, Result};
use clap::Args;
use dirs::home_dir;
use std::path::{Path, PathBuf};
use std::process::Stdio;

/// 命令行参数结构体
///
/// 使用 clap 的 Args API 自动解析命令行参数，
/// 提供类型安全和自动生成的帮助信息。
#[derive(Args, Debug)]
#[command(name = "compress_delete")]
#[command(version = "0.1.0")]
#[command(
    about = "使用 7-Zip 压缩文件和目录,然后删除原始项目",
    long_about = "将工作目录的直接子项压缩为 .7z 并删除原始文件。\n仅处理首层文件/目录（不递归），输出文件与原项同名，扩展名为 .7z。可选设置密码加密内容与文件名。"
)]
pub struct CompressDeleteArgs {
    /// 要处理的工作目录路径
    ///
    /// 指定包含要压缩和删除的项目的目录。
    /// 工具只会处理该目录的直接子项,不会递归遍历。
    /// 默认为当前目录(".")。
    #[arg(
        short = 'd',
        long,
        default_value = ".",
        value_name = "DIRECTORY",
        help = "工作目录路径",
        long_help = "仅处理该目录的直接子项（不递归）。默认当前目录 (.)。"
    )]
    pub directory: PathBuf,

    /// 压缩文件密码
    ///
    /// 为压缩文件设置密码保护。
    /// 启用后将同时加密文件内容和文件名(使用 -mhe=on 选项)。
    /// 如果不指定此参数,则不使用密码加密。
    #[arg(
        short = 'p',
        long,
        value_name = "PASSWORD",
        help = "压缩文件密码",
        long_help = "启用后同时加密文件内容和文件名（-mhe=on）。不指定则不加密。"
    )]
    pub password: Option<String>,
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
pub fn find_7z_executable() -> Result<PathBuf> {
    // 首先检查 PATH 环境变量中的 7z 命令
    // 这是最常见和最方便的方式
    if which::which("7z").is_ok() {
        return Ok(PathBuf::from("7z"));
    }

    // 检查常见的 Windows 安装路径，按优先级排序
    // 7-Zip 通常安装在 Program Files 目录下
    let common_paths = vec![
        PathBuf::from("C:\\Program Files\\7-Zip\\7z.exe"),
        PathBuf::from("C:\\Program Files (x86)\\7-Zip\\7z.exe"),
        PathBuf::from("C:\\7-Zip\\7z.exe"),
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
pub async fn compress_item(
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
    let mut child = tokio::process::Command::new(seven_zip_path)
        .args(&args)
        .stdout(Stdio::inherit()) // 流式输出到终端
        .stderr(Stdio::inherit()) // 流式输出到终端
        .spawn()
        .with_context(|| format!("执行 7z 命令失败: {}", seven_zip_path.display()))?;

    let status = child.wait().await.with_context(|| "等待 7z 命令完成失败")?;

    // 检查退出码，如果不成功则返回错误
    if !status.success() {
        anyhow::bail!("7z 压缩失败，退出码: {}", status.code().unwrap_or(-1));
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
pub fn collect_items(work_directory: &Path) -> Result<Vec<PathBuf>> {
    // 定义要跳过的文件扩展名
    let skip_extensions = [
        "ts", "mjs", "rs", "exe", "7z", "zip", "rar", "tar", "gz", "jar", "war", "ear",
    ];

    // 使用 std::fs::read_dir 读取目录项，只遍历首层
    let items: Vec<PathBuf> = std::fs::read_dir(work_directory)
        .with_context(|| format!("无法读取目录: {}", work_directory.display()))?
        .filter_map(|entry| entry.ok()) // 忽略读取错误的项
        .map(|entry| entry.path())
        .filter(|path| {
            // 获取文件名
            let file_name = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name,
                None => return false,
            };

            // 跳过隐藏文件/目录
            if file_name.starts_with('.') {
                return false;
            }

            // 跳过特定扩展名的文件（不带点，小写）
            let ext = get_file_extension(path);
            if !ext.is_empty() && skip_extensions.contains(&ext.as_str()) {
                false
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
pub async fn process_item(
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

/// 命令执行函数
///
/// 负责协调整个压缩和删除流程：
/// 1. 验证工作目录
/// 2. 收集要处理的项目
/// 3. 查找 7-Zip 可执行文件
/// 4. 逐个处理项目
/// 5. 输出处理结果
///
/// # 参数
///
/// * `args` - 命令行参数
///
/// # 返回值
///
/// * `Ok(())` - 程序成功执行
/// * `Err(anyhow::Error)` - 程序执行失败
pub async fn run(args: CompressDeleteArgs) -> anyhow::Result<()> {
    // 获取工作目录路径并转换为绝对路径
    let work_directory = args
        .directory
        .canonicalize()
        .with_context(|| format!("无法访问工作目录: {}", args.directory.display()))?;

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

    // 逐个处理项目，遇到失败直接返回错误
    for item in items {
        process_item(
            &item,
            &work_directory,
            &seven_zip_path,
            args.password.as_deref(),
        )
        .await
        .with_context(|| format!("处理 {} 失败", item.display()))?;
    }

    // 显示完成信息
    println!("操作成功完成！");
    Ok(())
}
