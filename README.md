# NexaShell

**English** | [简体中文](README.zh-CN.md)

A lightweight, modern SSH client and terminal manager. Built with **Rust** and **Vue 3**, packaged as a [Tauri 2](https://tauri.app/) desktop application.

[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE) [![CI](https://img.shields.io/badge/CI-lint%20%2B%20typecheck%20%2B%20tests%20%2B%20clippy-blue)](.github/workflows/ci.yml)

---

## ✨ Features

- **SSH session management** — tabbed interface, split panes (up to 3 per tab), session grouping, tags and favorites
- **Secure credential storage** — passwords and private keys encrypted with AES-256-GCM, master key kept in the OS keychain
- **Terminal** — low-latency rendering powered by xterm.js + WebGL, theme presets (One Dark / Solarized / GitHub…), in-terminal search, and local PTY-backed shell tabs
- **SFTP file manager** — upload (drag & drop, pause/resume), download, mkdir, rename and delete, plus a standalone per-session file window
- **Port forwarding** — local (`-L`) and dynamic SOCKS5 (`-D`) tunnels with persisted rules
- **Command snippets** — reusable snippets with a quick-launch command palette
- **Server dashboard** — live CPU / memory / disk / network / load monitoring for each session
- **Session import/export** — encrypted export with backward-compatible decryption of legacy formats

---

## 🚀 Run from Source

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) — latest stable
- [Node.js](https://nodejs.org/) — >= 18
- [pnpm](https://pnpm.sh/) — >= 9

### Steps

```bash
# 1. Clone the repository
git clone git@github.com:chengvar-glitch/nexashell.git
cd nexashell

# 2. Install dependencies
pnpm install

# 3. Run in development mode (opens the desktop window)
pnpm tauri dev

# 4. Build for production (DMG on macOS)
pnpm tauri build
```

### Common commands

| Command | Purpose |
|---|---|
| `pnpm dev` | Vite dev server (web only, port 1420) |
| `pnpm tauri dev` | Full desktop app dev mode |
| `pnpm build` | Type-check + frontend build (`vue-tsc --noEmit && vite build`) |
| `pnpm lint` | ESLint with auto-fix |
| `pnpm test` | Frontend unit tests (Vitest) |
| `cargo test` | Rust unit tests (run from `src-tauri/`) |
| `pnpm tui` | Run the bundled ratatui TUI frontend (`src-tui/`, under development) |

---

## 📄 More

- Release history: [CHANGELOG.md](./CHANGELOG.md)
- License: [MIT](LICENSE)
