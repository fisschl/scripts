# 文件哈希计算器 (file-hasher)

一个使用 Rust 编写的高效文件哈希计算工具，基于 Blake3 算法和 base32-crockford 编码。

## 功能特性

- 使用高性能的 Blake3 哈希算法计算文件哈希值
- 采用 base32-crockford 编码，生成易读且错误率低的哈希字符串
- 支持大文件处理（分块读取）
- 简洁的命令行界面
- 完整的单元测试覆盖

## 安装与构建

### 前置条件

- 需要安装 Rust 开发环境（[安装指南](https://www.rust-lang.org/zh-CN/tools/install)）
- Cargo 包管理工具（Rust 安装包已包含）

### 构建项目

```bash
# 克隆仓库（假设）
git clone <仓库地址>
cd scripts

# 构建项目
cargo build --release

# 构建完成后，可执行文件将位于 target/release 目录下
```

## 使用方法

### 命令行使用

```bash
# 计算单个文件的哈希值
cargo run --release -- <文件路径>

# 或使用已编译的可执行文件
./target/release/file-hasher <文件路径>
```

### 示例

```bash
# 计算 example.txt 文件的哈希值
cargo run --release -- example.txt

# 输出示例：
# 文件哈希值: 3v5zkryxenlyy6e4z75h56nbu36f6p73i6p7r5j44g25nf34y67g3a3k
```

## 作为库使用

您也可以将此项目作为库集成到其他 Rust 项目中：

```rust
use file_hasher::utils::hash::calculate_file_hash;

fn main() {
    match calculate_file_hash("path/to/file") {
        Ok(hash) => println!("文件哈希值: {}", hash),
        Err(err) => eprintln!("计算哈希值失败: {}", err),
    }
}
```

## 项目结构

```
├── .gitignore       # Git 忽略规则
├── Cargo.toml       # 项目配置和依赖管理
├── LICENSE          # 项目许可证
├── README.md        # 项目说明文档
├── src/
│   ├── lib.rs       # 库入口文件
│   ├── main.rs      # 命令行程序入口
│   └── utils/
│       ├── hash.rs  # 哈希计算实现
│       └── mod.rs   # 模块声明
└── tests/
    └── hash_test.rs # 单元测试文件
```

## 运行测试

```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test test_calculate_file_hash
```

## 技术细节

- **哈希算法**: Blake3 - 高性能加密哈希函数
- **编码方式**: base32-crockford - 专为人类可读性设计的编码方式
- **文件处理**: 分块读取，适合处理大文件
- **错误处理**: 完整的错误处理和友好的错误提示

## 许可证

查看项目中的 [LICENSE](LICENSE) 文件了解详细信息。