# 工具箱 - 多功能桌面应用

一个基于 Tauri + Vue 3 + TypeScript 构建的多功能桌面工具箱应用，集成了多种实用工具。

## 🛠️ 功能特性

### 🔄 仓库镜像工具
- Git 仓库之间的完整镜像同步
- 支持克隆远程仓库到本地或另一个远程仓库
- 实时进度跟踪和状态显示
- 表单验证和错误处理

### 🔍 文件哈希工具
- 计算文件的多种哈希值（MD5、SHA1、SHA256等）
- 支持文件拖拽和选择
- 哈希结果复制和导出功能

## 🚀 技术栈

- **前端框架**: Vue 3 + TypeScript
- **UI 组件库**: Element Plus
- **图标库**: Lucide Vue Next
- **构建工具**: Vite
- **桌面运行时**: Tauri (Rust)
- **样式方案**: Tailwind CSS

## 📦 安装和运行

### 前置要求

- Node.js 18+ 和 npm/bun
- Rust 工具链 (安装 Tauri 所需)

### 开发环境

```bash
# 安装依赖
npm install

# 启动开发服务器
npm run tauri dev

# 构建生产版本
npm run tauri build
```

### 生产构建

```bash
# 构建应用
npm run tauri build

# 构建完成后，应用位于 src-tauri/target/release/ 目录
```

## 🎨 界面特色

- 现代化设计，支持深色/浅色模式
- 响应式布局，适配不同屏幕尺寸
- 流畅的动画过渡效果
- 直观的用户交互体验

## 🔧 开发指南

### 添加新工具

1. 在 `src/pages/` 目录创建新的 Vue 组件
2. 在路由配置中添加新页面
3. 在主页 `index.vue` 中添加工具卡片
4. 如需后端功能，在 `src-tauri/src/commands.rs` 中添加 Rust 命令

### 样式规范

- 使用 Tailwind CSS 进行样式开发
- 遵循 BEM 命名规范
- 确保深色模式兼容性

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

## 📄 许可证

本项目基于 MIT 许可证开源 - 查看 [LICENSE](LICENSE) 文件了解详情。

## 🙏 致谢

- [Tauri](https://tauri.app/) - 提供优秀的桌面应用框架
- [Vue.js](https://vuejs.org/) - 渐进式 JavaScript 框架
- [Element Plus](https://element-plus.org/) - 丰富的 UI 组件库
- [Tailwind CSS](https://tailwindcss.com/) - 实用的 CSS 框架
