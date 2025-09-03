//! S3配置模块
//!
//! 该模块负责S3客户端的配置和初始化。

use aws_sdk_s3::Client;

/// 异步初始化 S3 客户端
///
/// # 环境变量要求
///
/// ## 必需环境变量：
/// - `AWS_ACCESS_KEY_ID` - AWS访问密钥ID
/// - `AWS_SECRET_ACCESS_KEY` - AWS秘密访问密钥  
/// - `AWS_REGION` - AWS区域，如 `us-east-1`、`cn-north-1`
/// - `S3_BUCKET` - S3存储桶名称（必填）
///
/// ## 可选环境变量：
/// - `AWS_ENDPOINT_URL` - S3兼容服务端点（阿里云OSS等）
///
/// # 示例配置
///
/// ## 阿里云OSS：
/// ```bash
/// export AWS_ACCESS_KEY_ID=your-access-key-id
/// export AWS_SECRET_ACCESS_KEY=your-access-key-secret
/// export AWS_REGION=cn-hangzhou
/// export AWS_ENDPOINT_URL=https://oss-cn-hangzhou.aliyuncs.com
/// export S3_BUCKET=my-bucket-name
/// ```
pub async fn init_s3_client() -> Client {
    let config = aws_config::load_from_env().await;
    Client::new(&config)
}

/// 获取全局 S3 存储桶名称
///
/// # 注意
/// 需要确保 `S3_BUCKET` 环境变量已正确设置，否则会panic
///
/// # Panics
/// 如果 `S3_BUCKET` 环境变量未设置，此函数会panic
pub fn get_bucket_name() -> Result<String, std::env::VarError> {
    std::env::var("S3_BUCKET")
}
