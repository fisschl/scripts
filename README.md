# 文件工具集 (File Utils)

这是一个用 Rust 编写的实用文件处理工具集合，提供了文件压缩、复制和重命名的常用功能。

## 项目概述

本项目是一个集成了多种文件处理功能的命令行工具，使用子命令模式，包含以下主要命令：

- **compress-delete**：使用 7-Zip 压缩文件和目录，然后删除原始项目
- **file-copy-rename**：将文件从源目录复制到目标目录，使用哈希值重命名以避免重复
- **tar**：使用 tar.zst 格式压缩或解压缩文件和目录

## 前提条件

- **Rust**：确保已安装 Rust 工具链

  - 下载地址：https://rustup.rs/
  - 安装命令：`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

- **7-Zip**：用于运行 compress_delete 工具
  - 下载地址：https://www.7-zip.org/
  - 安装命令：`winget install 7zip.7zip`（Windows）

## 安装方法

### 从源码构建

```bash
# 克隆仓库
git clone <repository-url>
cd scripts

# 构建项目
cargo build --release

# 构建后的可执行文件位于 target/release/ 目录下
```

### 直接运行

```bash
# 运行压缩删除工具
cargo run -- compress-delete [参数]

# 运行文件复制重命名工具
cargo run -- file-copy-rename [参数]

# 运行 tar 归档工具
cargo run -- tar [参数]

# 查看所有可用命令
cargo run -- --help
```

## 工具列表

### 1. compress-delete

**功能说明**：

- 将指定目录下的所有一级子目录和文件使用 7z 压缩成压缩包
- 压缩完成后自动删除对应的源文件或目录
- 智能检测 7z 安装位置
- 跨平台支持（Windows、macOS、Linux）

**安全特性**：

- 跳过隐藏文件/目录（以点开头）
- 跳过开发文件和常见压缩格式
- 压缩文件已存在时自动跳过
- 仅在压缩成功后才删除源文件/目录

**使用方法**：

```bash
# 压缩当前目录下所有项目
scripts compress-delete

# 指定工作目录
scripts compress-delete --directory ./backup

# 使用短选项
scripts compress-delete -d ./projects
```

**参数说明**：

- `[--directory, -d] <DIRECTORY>`: 要处理的目录路径，默认为当前目录

### 2. file-copy-rename

**功能说明**：

- 将源目录中的特定类型文件复制到目标目录
- 使用 Blake3 哈希值和 Base58 编码重命名文件，避免重复
- 支持递归扫描子目录
- 支持文件类型过滤
- 支持复制或剪切模式

**安全特性**：

- 跳过隐藏文件/目录
- 目标文件已存在时自动跳过
- 仅在复制成功后才删除源文件（剪切模式）

**使用方法**：

```bash
# 复制默认目录的默认格式文件
scripts file-copy-rename

# 复制指定目录的图片文件
scripts file-copy-rename --source ./photos --target ./backup --extensions jpg,png,gif

# 移动视频文件
scripts file-copy-rename --source ./videos --target ./archive --extensions mp4,avi --move

# 使用短选项
scripts file-copy-rename -s ./source -t ./target -e "mp4,webm" -m
```

**参数说明**：

- `[--source, -s] <DIRECTORY>`: 源目录路径，默认为 `./source`
- `[--target, -t] <DIRECTORY>`: 目标目录路径，默认为 `./target`
- `[--extensions, -e] <EXTENSIONS>`: 文件扩展名（逗号分隔，不带点），默认为常见视频格式
- `[--move, -m]`: 启用移动模式（复制后删除源文件）

### 3. tar

**功能说明**：

- 使用 tar.zst 格式压缩文件或目录
- 从 tar.zst 归档中解压缩文件
- 支持可配置的压缩级别（1-22）
- 流式处理，内存占用低
- 自动识别压缩或解压模式

**使用方法**：

```bash
# 压缩文件或目录（输出到同级目录）
scripts tar ./my-folder
scripts tar ./my-file.txt

# 解压 tar.zst 文件（解压到归档所在目录）
scripts tar ./my-folder.tar.zst

# 指定压缩级别（1-22，默认6）
scripts tar ./my-folder --level 10
scripts tar ./my-folder -l 3
```

**参数说明**：

- `<SOURCE>`: 源路径（文件/目录或 .tar.zst 归档）
  - 当传入 .tar.zst 文件时，执行解压操作
  - 当传入文件或目录时，执行压缩操作，输出同名 .tar.zst 文件到父目录
- `[--level, -l] <LEVEL>`: 压缩级别（1-22，默认 6），仅在压缩时有效

## 技术栈

- **Rust**：高性能系统编程语言
- **clap**：命令行参数解析（支持子命令）
- **tokio**：异步运行时
- **walkdir**：目录遍历
- **blake3**：高性能哈希算法
- **bs58**：Base58 编码
- **tar**：tar 归档格式支持
- **zstd**：高效压缩算法
- **anyhow**：错误处理
- **which**：查找可执行文件
- **dirs**：目录路径处理

## 使用提示

1. **备份重要数据**：在运行删除或移动操作前，请确保已备份重要文件

2. **测试运行**：建议先在小批量文件上测试工具功能

3. **权限控制**：确保有足够的文件系统权限执行操作

4. **7-Zip 安装**：compress-delete 命令需要系统安装 7-Zip 并在 PATH 中，或在标准安装位置

## 贡献指南

欢迎提交 Issue 和 Pull Request 来改进这些工具！

## 许可证

本项目采用 MIT 许可证 - 详情请查看[LICENSE](LICENSE)文件
