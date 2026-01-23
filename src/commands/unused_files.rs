//! # 未使用文件查找工具 (unused_files)
//!
//! 扫描指定目录中的文件，检查是否在搜索目录中被引用使用。
//! 判断规则：
//! 1. 以相对路径（不带前导斜杠）在文件内容中搜索，找到则认为**已使用**
//! 2. 若未找到相对路径，再以文件名搜索，未找到则认为**未使用**
//! 3. 其他情况标记为**待定**

use anyhow::{Context, Result};
use clap::Args;
use grep_regex::RegexMatcherBuilder;
use grep_searcher::SearcherBuilder;
use grep_searcher::sinks::UTF8;
use ignore::WalkBuilder;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use trash;
use walkdir::WalkDir;

/// 文件使用状态
#[derive(Debug, PartialEq, Eq)]
pub enum FileStatus {
    /// 确定已使用（找到相对路径引用）
    Used,
    /// 确定未使用（相对路径和文件名都未找到）
    Unused,
    /// 待定（找到文件名但未找到相对路径）
    Uncertain,
}

/// 命令行参数结构体
#[derive(Args, Debug)]
#[command(name = "unused_files")]
#[command(version = "0.1.0")]
#[command(
    about = "查找目录中未被使用的文件",
    long_about = "扫描目录中的资源文件，检查是否在代码文件中被引用。判断规则：1. 以相对路径（不带前导斜杠）在代码文件内容中搜索，找到则认为已使用；2. 若未找到相对路径，再以文件名搜索，未找到则认为未使用；3. 其他情况（仅找到文件名）标记为待定。"
)]
pub struct UnusedFilesArgs {
    /// 要检查的目录路径
    ///
    /// 在该目录中查找资源文件，并在代码文件中搜索引用。
    #[arg(
        short = 'd',
        long,
        value_name = "DIR",
        help = "要检查的目录",
        long_help = "要检查的目录路径，工具会扫描该目录中的资源文件并在代码文件中查找引用"
    )]
    pub dir: PathBuf,

    /// 资源文件扩展名
    ///
    /// 指定要检查的资源文件扩展名，多个扩展名用逗号分隔。
    /// 例如：png,jpg,svg
    #[arg(
        short = 'r',
        long = "resource-extensions",
        default_value = "png,jpg,jpeg,svg,gif,webp,ttf,otf,woff,woff2",
        value_name = "EXTENSIONS",
        help = "资源文件扩展名列表",
        long_help = "要检查的资源文件扩展名，逗号分隔，不带点，大小写不敏感。例如：png,jpg,svg"
    )]
    pub resource_extensions: String,

    /// 代码文件扩展名
    ///
    /// 指定要在其中搜索引用的代码文件扩展名，多个扩展名用逗号分隔。
    /// 例如：js,ts,jsx,tsx,css,html
    #[arg(
        short = 'c',
        long = "code-extensions",
        default_value = "js,ts,jsx,tsx,vue,html,css,scss,sass,less",
        value_name = "EXTENSIONS",
        help = "代码文件扩展名列表",
        long_help = "要在其中搜索引用的代码文件扩展名，逗号分隔，不带点，大小写不敏感。例如：js,ts,css"
    )]
    pub code_extensions: String,

    /// 自动删除未使用的文件
    #[arg(
        long = "delete",
        help = "自动删除未使用的文件",
        long_help = "开启后会自动删除未使用的文件，默认关闭。请谨慎使用此选项！"
    )]
    pub delete: bool,
}

/// 获取文件相对于基础目录的相对路径（不带前导斜杠）
///
/// # 参数
///
/// * `file_path` - 文件的绝对路径
/// * `base_dir` - 基础目录路径
///
/// # 返回值
///
/// 返回相对路径字符串，使用正斜杠分隔符
fn get_relative_path(file_path: &Path, base_dir: &Path) -> Result<String> {
    let relative = file_path
        .strip_prefix(base_dir)
        .with_context(|| format!("无法获取相对路径: {}", file_path.display()))?;

    // 转换为字符串，并使用正斜杠
    let path_str = relative
        .to_str()
        .context("路径包含无效的 UTF-8 字符")?
        .replace('\\', "/");

    Ok(path_str)
}

/// 在文件中搜索文本模式（使用 grep-searcher）
///
/// # 参数
///
/// * `searcher` - 可复用的搜索器实例
/// * `file_path` - 要搜索的文件路径
/// * `pattern` - 要搜索的文本（会被转义为字面量）
///
/// # 返回值
///
/// * `Ok(true)` - 找到匹配
/// * `Ok(false)` - 未找到匹配
/// * `Err` - 读取文件或匹配时出错
fn search_in_file(
    searcher: &mut grep_searcher::Searcher,
    file_path: &Path,
    pattern: &str,
) -> Result<bool> {
    // 创建字面量匹配器（转义特殊字符）
    let matcher = RegexMatcherBuilder::new()
        .build(&regex::escape(pattern))
        .context("创建匹配器失败")?;

    // 用于记录是否找到匹配
    let mut found = false;

    // 执行搜索
    searcher.search_path(
        &matcher,
        file_path,
        UTF8(|_lnum, _line| {
            found = true;
            Ok(false) // 找到一个匹配就停止搜索
        }),
    )?;

    Ok(found)
}

/// 收集目录中的所有代码文件路径
///
/// # 参数
///
/// * `search_dir` - 要搜索的目录路径
/// * `code_extensions` - 代码文件扩展名集合
///
/// # 返回值
///
/// 返回代码文件路径的向量
fn collect_code_files(
    search_dir: &Path,
    code_extensions: &HashSet<String>,
) -> Result<Vec<PathBuf>> {
    let mut code_files = Vec::new();

    // 使用 ignore 库来遵循 .gitignore 规则
    let walker = WalkBuilder::new(search_dir)
        .git_ignore(true) // 遵循 .gitignore
        .git_exclude(true) // 遵循 .git/info/exclude
        .build();

    for entry in walker {
        let entry = entry.context("遍历目录时出错")?;
        let path = entry.path();

        // 只处理文件
        if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            continue;
        }

        // 只收集指定扩展名的代码文件
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            if code_extensions.contains(&ext_str) {
                code_files.push(path.to_path_buf());
            }
        }
    }

    Ok(code_files)
}

/// 在预收集的代码文件中搜索文本模式
///
/// # 参数
///
/// * `searcher` - 可复用的搜索器实例
/// * `code_files` - 预收集的代码文件路径
/// * `pattern` - 要搜索的文本（会被转义为字面量）
///
/// # 返回值
///
/// * `Ok(true)` - 在至少一个文件中找到匹配
/// * `Ok(false)` - 在所有文件中都未找到匹配
fn search_in_code_files(
    searcher: &mut grep_searcher::Searcher,
    code_files: &[PathBuf],
    pattern: &str,
) -> Result<bool> {
    for path in code_files {
        // 在文件中搜索
        match search_in_file(searcher, path, pattern) {
            Ok(true) => return Ok(true), // 找到匹配，立即返回
            Ok(false) => continue,       // 未找到，继续下一个文件
            Err(_) => continue,          // 搜索出错，跳过该文件
        }
    }

    Ok(false)
}

/// 检查文件的使用状态
///
/// # 参数
///
/// * `searcher` - 可复用的搜索器实例
/// * `file_path` - 要检查的文件路径
/// * `base_dir` - 文件所在的基础目录
/// * `code_files` - 预收集的代码文件路径
///
/// # 返回值
///
/// 返回文件的使用状态
fn check_file_status(
    searcher: &mut grep_searcher::Searcher,
    file_path: &Path,
    base_dir: &Path,
    code_files: &[PathBuf],
) -> Result<FileStatus> {
    // 获取相对路径
    let relative_path = get_relative_path(file_path, base_dir)?;

    // 获取文件名
    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .context("无效的文件名")?;

    // 第一步：搜索相对路径
    if search_in_code_files(searcher, code_files, &relative_path)? {
        return Ok(FileStatus::Used);
    }

    // 第二步：搜索文件名
    if search_in_code_files(searcher, code_files, file_name)? {
        return Ok(FileStatus::Uncertain);
    }

    // 两者都未找到
    Ok(FileStatus::Unused)
}

/// 命令执行函数
pub async fn run(args: UnusedFilesArgs) -> Result<()> {
    // 验证目录是否存在
    if !args.dir.exists() {
        anyhow::bail!("目录不存在: {}", args.dir.display());
    }

    // 显示程序信息
    println!(
        "{}  未使用文件查找工具 {}",
        "=".repeat(15),
        "=".repeat(15)
    );
    println!("目录: {}", args.dir.display());
    println!();

    // 解析资源文件扩展名参数
    let resource_extensions: HashSet<String> = args
        .resource_extensions
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    if resource_extensions.is_empty() {
        anyhow::bail!("资源文件扩展名列表不能为空");
    }

    // 解析代码文件扩展名参数
    let code_extensions: HashSet<String> = args
        .code_extensions
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    if code_extensions.is_empty() {
        anyhow::bail!("代码文件扩展名列表不能为空");
    }

    println!(
        "资源文件扩展名: {}",
        resource_extensions
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "代码文件扩展名: {}",
        code_extensions
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!();

    // 收集要检查的资源文件
    let files_to_check: Vec<PathBuf> = WalkDir::new(&args.dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            if let Some(ext) = entry.path().extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                resource_extensions.contains(&ext_str)
            } else {
                false
            }
        })
        .map(|entry| entry.path().to_path_buf())
        .collect();

    if files_to_check.is_empty() {
        println!("未找到匹配的资源文件");
        return Ok(());
    }

    println!("找到 {} 个资源文件需要检查\n", files_to_check.len());

    // 预收集所有代码文件（只收集一次）
    println!("正在收集代码文件...");
    let code_files = collect_code_files(&args.dir, &code_extensions).context("收集代码文件失败")?;

    println!("找到 {} 个代码文件\n", code_files.len());

    // 创建可复用的搜索器实例（只创建一次）
    let mut searcher = SearcherBuilder::new().build();

    // 统计计数器和路径列表
    let mut used_count = 0;
    let mut unused_files: Vec<String> = Vec::new();
    let mut uncertain_files: Vec<String> = Vec::new();

    // 检查每个文件
    for file_path in files_to_check {
        let relative_path = get_relative_path(&file_path, &args.dir)
            .with_context(|| format!("获取相对路径失败: {}", file_path.display()))?;

        let status = check_file_status(&mut searcher, &file_path, &args.dir, &code_files)
            .with_context(|| format!("检查文件失败: {}", file_path.display()))?;

        match status {
            FileStatus::Used => {
                used_count += 1;
            }
            FileStatus::Unused => {
                unused_files.push(relative_path);
            }
            FileStatus::Uncertain => {
                uncertain_files.push(relative_path);
            }
        }
    }

    // 输出未使用的文件
    if !unused_files.is_empty() {
        println!("{} 未使用的文件 {}", "=".repeat(20), "=".repeat(20));
        for file in &unused_files {
            println!("{}", file);
        }
        println!();
    }

    // 输出待定的文件
    if !uncertain_files.is_empty() {
        println!("{} 待定的文件 {}", "=".repeat(20), "=".repeat(20));
        for file in &uncertain_files {
            println!("{}", file);
        }
        println!();
    }

    // 显示统计信息
    println!("{} 统计结果 {}", "=".repeat(20), "=".repeat(20));
    println!("已使用: {}", used_count);
    println!("未使用: {}", unused_files.len());
    println!("待定: {}", uncertain_files.len());
    println!(
        "总计: {}",
        used_count + unused_files.len() + uncertain_files.len()
    );

    // 如果开启了删除选项，删除未使用的文件
    if args.delete && !unused_files.is_empty() {
        println!();
        println!("{} 删除未使用的文件 {}", "=".repeat(18), "=".repeat(18));

        for relative_path in &unused_files {
            let file_path = args.dir.join(relative_path);
            trash::delete(&file_path)
                .with_context(|| format!("无法将文件移动到回收站: {}", relative_path))?;
            println!("✓ 已移动到回收站: {}", relative_path);
        }

        println!();
        println!("成功删除: {}", unused_files.len());
    }

    Ok(())
}
