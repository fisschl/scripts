# 文件工具集 (File Utils)

这是一个用 Rust 编写的实用文件处理工具集合，提供了文件压缩、复制和重命名的常用功能。

## 项目概述

本项目是一个集成了多种文件处理功能的命令行工具，使用子命令模式，包含以下主要命令：

- **compress-delete**：使用 7-Zip 压缩文件和目录，然后删除原始项目
- **file-copy-rename**：将文件从源目录复制到目标目录，使用哈希值重命名以避免重复
- **find-unused-files**：查找目录中未被引用的资源文件
- **residue-search**：查找 Windows 系统中软件卸载后的残留目录

## 安装方法

### 安装到系统

如果你想将工具安装到系统，可以使用以下命令：

```bash
cargo install --path .
```

安装后，`scripts` 命令将可在任何地方使用。

### 直接运行

通过 cargo 直接运行（推荐开发时）

```bash
cargo run -- compress-delete --directory ./projects
cargo run -- file-copy-rename --source ./photos --target ./backup --extensions jpg,png,gif
cargo run -- find-unused-files --dir ./assets --resource-extensions png,jpg --code-extensions js,ts,css
cargo run -- residue-search --interactive --software Thunder
```

### 从源码构建

```bash
# 构建项目
cargo build --release

# 构建后的可执行文件位于 target\release\scripts.exe
```

## 工具列表

### 1. compress-delete

**功能说明**：

- 将指定目录下的所有一级子目录和文件使用 7z 压缩成压缩包
- 压缩完成后自动删除对应的源文件或目录
- 智能检测 7z 安装位置

**安全特性**：

- 跳过隐藏文件/目录（以点开头）
- 跳过常见压缩格式（.zip, .7z, .rar, .tar, .gz 等）
- 压缩文件已存在时自动跳过
- 仅在压缩成功后才删除源文件/目录

**使用方法**：

```bash
# 压缩当前目录下所有项目
scripts compress-delete

# 指定工作目录
scripts compress-delete --directory ./backup

# 使用密码加密压缩
scripts compress-delete --directory ./projects --password "your_password"

# 使用短选项
scripts compress-delete -d ./projects -p "your_password"
```

**参数说明**：

- `[--directory, -d] <DIRECTORY>`: 要处理的目录路径，默认为当前目录
- `[--password, -p] <PASSWORD>`: 压缩文件密码，启用后会同时加密文件内容和文件名

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
- `[--extensions, -e] <EXTENSIONS>`: 文件扩展名（逗号分隔，不带点），默认为 `mp4,webm,m4v,avi,mkv,mov`（常见视频格式）
- `[--move, -m]`: 启用移动模式（复制后删除源文件）

### 3. find-unused-files

**功能说明**：

- 扫描指定目录中的资源文件，检查是否在代码文件中被引用
- 支持图片、样式、脚本等多种资源类型检查
- 提供三种状态判断：已使用、未使用、待确认
- 支持自动删除（谨慎使用）

**判断规则**：

1. **已使用**：找到相对路径引用（如 `img/logo.png`）
2. **未使用**：相对路径和文件名都未找到
3. **待定**：仅找到文件名但未找到相对路径

**使用方法**：

```bash
# 检查 assets 目录中的图片资源，在 src 目录中搜索引用
scripts find-unused-files --dir ./assets --resource-extensions png,jpg --code-extensions js,ts,css

# 使用短选项
scripts find-unused-files -d ./static -r "svg,gif" -c "html,vue,jsx"

# 自动删除未使用的文件（⚠️ 危险操作，请谨慎使用）
scripts find-unused-files --dir ./public --delete
```

**参数说明**：

- `[-d, --dir] <DIR>`: 要检查的目录路径
- `[-r, --resource-extensions] <EXTENSIONS>`: 资源文件扩展名，默认为 `png,jpg,jpeg,svg,gif,webp,ttf,otf,woff,woff2`
- `[-c, --code-extensions] <EXTENSIONS>`: 代码文件扩展名，默认为 `js,ts,jsx,tsx,vue,html,css,scss,sass,less`
- `[--delete]`: 自动删除未使用的文件（⚠️ 小心使用）

**⚠️ 注意事项**：

- 搜索结果可能有误报，建议人工核实后再删除
- 动态引用的文件（如通过变量拼接的路径）可能无法检测到
- 建议先在不加 `--delete` 参数的情况下运行，确认结果

### 4. residue-search

**功能说明**：

- 扫描 Windows 系统常见目录，查找软件卸载残留目录
- 支持子串匹配，大小写不敏感
- 最多向下扫描 3 层目录
- 仅匹配目录，不匹配文件
- 计算目录递归总大小
- 显示修改时间
- 交互式多选删除

**扫描位置**：

- C:\Program Files
- C:\Program Files (x86)
- C:\ProgramData
- C:\Users\[用户名]
- C:\Users\[用户名]\AppData\Roaming
- C:\Users\[用户名]\AppData\Local

**使用方法**：

```bash
# 查找 Chrome 相关残留
scripts residue-search --software chrome

# 使用短选项
scripts residue-search -s "visual studio"

# 启用交互式删除模式
scripts residue-search --software chrome --interactive

# 使用短选项启用交互式删除
scripts residue-search -s chrome -i
```

**参数说明**：

- `[--software, -s] <NAME>`: 要查找的软件名称（必填）
- `[--interactive, -i]`: 启用交互式删除功能，扫描结束后可多选要删除的目录

**⚠️ 注意事项**：

- 删除操作不可逆，请谨慎确认选择的目录
- 建议在删除前备份重要数据
- 权限不足的目录会自动跳过
- 请确保匹配的目录确实是软件残留，避免误删除系统文件

## 使用提示

1. **⚠️ 备份重要数据**：在运行删除或移动操作前，请确保已备份重要文件
2. **测试运行**：建议先在小批量文件上测试工具功能
3. **权限控制**：确保有足够的文件系统权限执行操作
4. **7-Zip 安装**：compress-delete 命令需要系统安装 7-Zip 并在 PATH 中，或在标准安装位置
5. **安全删除**：工具使用系统回收站机制（trash），删除的文件可恢复，比永久删除更安全。
6. **find-unused-files 误报风险**：该工具检测结果可能有误报，删除文件前必须人工验证
7. **动态引用检测限制**：通过变量拼接或动态加载的资源路径可能无法被正确识别
8. **residue-search 风险**：虽然删除操作是移动到回收站，但在执行前仍请仔细确认匹配结果
9. **软件残留识别**：请确保匹配的目录确实是软件残留，避免误删除系统文件或其他重要数据

## 通用工具模块 (utils)

项目提供了一组可复用的通用工具函数，位于 `src/utils/` 目录下。

### 1. 文件系统操作 (`src/utils/filesystem.rs`)

#### `get_file_extension`

获取文件扩展名（小写），无扩展名时返回空字符串。

```rust
use scripts::utils::filesystem::get_file_extension;

let ext = get_file_extension(Path::new("document.PDF")); // "pdf"
```

#### `calculate_dir_size`

计算目录的实际大小（字节数），权限不足时自动跳过。

```rust
use scripts::utils::filesystem::calculate_dir_size;

let size = calculate_dir_size(Path::new("./src"));
```

### 2. 哈希计算 (`src/utils/hash.rs`)

#### `calculate_file_hash`

计算文件的 Blake3 哈希值并使用 Base58 编码，适合生成唯一文件名。

```rust
use scripts::utils::hash::calculate_file_hash;

let hash = calculate_file_hash(Path::new("./video.mp4")).await?;
```

### 3. 压缩工具 (`src/utils/compress.rs`)

#### `find_7z`

查找系统中安装的 7-Zip 可执行文件，结果会被缓存。

```rust
use scripts::utils::compress::find_7z;

let path = find_7z(); // PathBuf
```

#### `compress_7z`

使用 7-Zip 压缩文件或目录为 .7z 格式，支持密码加密。

```rust
use scripts::utils::compress::compress_7z;

// 无密码压缩
compress_7z(Path::new("./data"), Path::new("./data.7z"), None).await;

// 带密码压缩
compress_7z(Path::new("./data"), Path::new("./data.7z"), Some("password")).await;
```

## 贡献指南

欢迎提交 Issue 和 Pull Request 来改进这些工具！
