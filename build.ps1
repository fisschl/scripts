# 设置全局错误处理为停止模式
$ErrorActionPreference = "Stop"

bun run build

Remove-Item -Path "dist\*" -Recurse -Force

Copy-Item -Path "src-tauri\target\release\bundle\msi\*.msi" -Destination "dist\" -Force
