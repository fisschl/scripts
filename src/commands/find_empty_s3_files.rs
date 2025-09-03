use crate::utils::s3::{get_bucket_name, init_s3_client};
use anyhow::Result;
use console::style;

/// 执行S3空文件查找命令
///
/// # 参数
/// - `prefix`: 可选的前缀路径，用于限制搜索范围
///
/// # 返回值
/// 返回 `Result<(), anyhow::Error>`
pub async fn execute_find_empty_s3_files(prefix: Option<String>) -> Result<(), anyhow::Error> {
    println!("{}", style("开始查找S3空文件...").cyan().bold());

    println!();
    let empty_files = find_empty_files_with_progress(prefix.as_deref()).await?;
    println!();

    if empty_files.is_empty() {
        println!("{}", style("未找到空文件").yellow().bold());
    } else {
        println!(
            "{}",
            style(format!("共找到 {} 个空文件", empty_files.len()))
                .green()
                .bold()
        );
    }

    Ok(())
}

/// 在S3中查找空文件，并在发现时实时输出
///
/// # 参数
/// - `prefix`: 可选的前缀，用于过滤文件路径
///
/// # 返回值
/// 返回 `Result<Vec<String>, anyhow::Error>`，包含空文件的路径列表
async fn find_empty_files_with_progress(prefix: Option<&str>) -> Result<Vec<String>> {
    let client = init_s3_client().await;
    let bucket = get_bucket_name()?;
    let mut empty_files = Vec::new();
    let mut continuation_token = None;

    loop {
        let mut request = client.list_objects_v2().bucket(&bucket).max_keys(100);

        if let Some(prefix) = prefix {
            request = request.prefix(prefix);
        }

        if let Some(token) = &continuation_token {
            request = request.continuation_token(token);
        }

        let response = request.send().await?;
        continuation_token = response.next_continuation_token().map(|s| s.to_string());

        for object in response.contents() {
            if let Some(key) = object.key() {
                if object.size() == Some(0) {
                    empty_files.push(key.to_string());
                    println!("{}", style(key).dim());
                }
            }
        }

        if !response.is_truncated().unwrap_or(false) {
            break;
        }
    }

    Ok(empty_files)
}
