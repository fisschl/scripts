# 工具箱 - 多功能桌面应用

一个基于 Tauri + Vue 3 + TypeScript 构建的多功能桌面工具箱应用，集成了多种实用工具。

## 🛠️ 功能特性

### 📁 文件管理工具
- 本地文件浏览和管理
- 文件操作功能
- 简洁直观的用户界面

### ☁️ S3 同步工具
- AWS S3 实例配置管理
- 本地与云端文件同步
- 文件路径更新和管理
- 并行处理文件和目录

### 🔍 文件哈希工具
- 计算文件的 Blake3 哈希值
- 通过文件选择器选择文件
- 显示哈希计算结果

## 🚀 技术栈

- **前端框架**: Vue 3 + TypeScript
- **UI 组件库**: Element Plus
- **图标库**: Lucide Vue Next
- **构建工具**: Vite
- **桌面运行时**: Tauri (Rust)
- **样式方案**: Tailwind CSS

## 📦 安装和运行

### 开发环境

```bash
bun install

# 启动开发服务器
bun run tauri dev

# 构建生产版本
bun run tauri build
```

### 生产构建

```bash
# 构建应用
npm run tauri build

# 构建完成后，应用位于 src-tauri/target/release/ 目录
```

## 🔧 开发指南

### 项目结构

- `src/` - 前端源代码
  - `pages/` - 各个工具页面
    - `file-manager/` - 文件管理功能
    - `s3-sync/` - S3 同步功能
    - `index.vue` - 主页
  - `components/` - 可复用组件
  - `utils/` - 工具函数
- `src-tauri/` - Tauri 后端代码
  - `src/commands/` - Rust 命令实现

### 添加新功能

1. 在 `src/pages/` 目录创建新的 Vue 组件或目录
2. 在主页 `index.vue` 中添加对应功能的入口
3. 如需后端功能，在 `src-tauri/src/commands/` 目录创建相应的 Rust 模块
4. 在 `src-tauri/src/commands.rs` 中注册新命令

### 开发规范

- 使用 TypeScript 进行类型安全的开发
- 使用 Tailwind CSS 进行样式开发
- 确保代码符合 ESLint 和 Prettier 规范
- 对关键功能添加适当的注释

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

## 📄 许可证

本项目基于 MIT 许可证开源 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 🙏 致谢

- [Tauri](https://tauri.app/) - 提供优秀的桌面应用框架
- [Vue.js](https://vuejs.org/) - 渐进式 JavaScript 框架
- [Element Plus](https://element-plus.org/) - 丰富的 UI 组件库
- [Tailwind CSS](https://tailwindcss.com/) - 实用的 CSS 框架
