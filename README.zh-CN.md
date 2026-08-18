# NexaShell

[English](README.md) | **简体中文**

基于 **Rust** 与 **Vue 3** 构建的轻量级现代 SSH 客户端与终端管理器，以 [Tauri 2](https://tauri.app/) 桌面应用形式打包分发。

[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE) [![CI](https://img.shields.io/badge/CI-lint%20%2B%20typecheck%20%2B%20tests%20%2B%20clippy-blue)](.github/workflows/ci.yml)

---

## ✨ 基础功能

- **SSH 会话管理** — 标签页界面、分屏面板（每个标签页最多 3 个）、会话分组、标签与收藏
- **凭据安全存储** — 密码与私钥使用 AES-256-GCM 加密，主密钥存放于操作系统钥匙串
- **终端** — xterm.js + WebGL 低延迟渲染、主题预设（One Dark / Solarized / GitHub…）、终端内搜索、本地 PTY Shell 标签页
- **SFTP 文件管理** — 上传（拖拽、暂停/恢复）、下载、新建目录、重命名、删除，并支持每个会话独立文件窗口
- **端口转发** — 本地（`-L`）与动态 SOCKS5（`-D`）隧道，规则持久化
- **命令片段** — 可复用片段，配合快速启动命令面板
- **服务器仪表盘** — 实时监控每个会话的 CPU / 内存 / 磁盘 / 网络 / 负载
- **会话导入/导出** — 加密导出，并兼容解密旧版格式

---

## 🚀 从源码运行

### 环境要求

- [Rust](https://www.rust-lang.org/tools/install) — 最新稳定版
- [Node.js](https://nodejs.org/) — >= 18
- [pnpm](https://pnpm.sh/) — >= 9

### 步骤

```bash
# 1. 克隆仓库
git clone git@github.com:chengvar-glitch/nexashell.git
cd nexashell

# 2. 安装依赖
pnpm install

# 3. 开发模式运行（打开桌面窗口）
pnpm tauri dev

# 4. 生产构建（macOS 出 DMG）
pnpm tauri build
```

### 常用命令

| 命令 | 用途 |
|---|---|
| `pnpm dev` | Vite 开发服务器（仅 Web，端口 1420） |
| `pnpm tauri dev` | 完整桌面应用开发模式 |
| `pnpm build` | 类型检查 + 前端构建（`vue-tsc --noEmit && vite build`） |
| `pnpm lint` | ESLint（自动修复） |
| `pnpm test` | 前端单元测试（Vitest） |
| `cargo test` | Rust 单元测试（在 `src-tauri/` 下运行） |
| `pnpm tui` | 运行内置的 ratatui TUI 前端（`src-tui/`） |

---

## 📄 其他

- 发布历史：[CHANGELOG.md](./CHANGELOG.md)
- 许可证：[MIT](LICENSE)
