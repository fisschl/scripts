# 安装 file-utils 工具到全局环境

# 设置错误处理策略
# Stop: 遇到任何错误时停止执行
$ErrorActionPreference = "Stop"

# 启用严格模式 v3.0
# 会检测未初始化的变量、不可访问的属性、无效的参数等
Set-StrictMode -Version 3.0

# 确保脚本在项目根目录执行
if (-not (Test-Path "Cargo.toml")) {
    Write-Error "错误：请在项目根目录执行此脚本"
    exit 1
}

# 检查 Cargo 是否已安装
if (-not (Get-Command "cargo" -ErrorAction SilentlyContinue)) {
    Write-Error "错误：未找到 cargo 命令，请先安装 Rust 工具链"
    Write-Error "可以从 https://rustup.rs/ 下载安装"
    exit 1
}

Write-Host "开始安装 file-utils 工具..." -ForegroundColor Green

# 安装到全局
Write-Host "正在编译并安装..." -ForegroundColor Cyan
cargo install --path .

# 检查命令执行结果（对于外部命令，$ErrorActionPreference 不会自动捕获退出码）
if ($LASTEXITCODE -ne 0) {
    Write-Error "安装失败，退出码: $LASTEXITCODE"
}

Write-Host ""
Write-Host "✅ 安装成功!" -ForegroundColor Green
Write-Host ""

# 验证安装
Write-Host "验证安装..." -ForegroundColor Cyan
scripts --version

if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ 验证成功!" -ForegroundColor Green
} else {
    Write-Warning "⚠️  验证失败，请检查 PATH 环境变量是否包含 Cargo 二进制目录"
}

Write-Host ""
Write-Host "📖 使用方法: scripts --help" -ForegroundColor Cyan
Write-Host ""
Write-Host "🔧 卸载方法: cargo uninstall scripts" -ForegroundColor Magenta

Write-Host "安装完成!" -ForegroundColor Green