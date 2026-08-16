# NexaShell — Agent Guide

## Stack

- **Frontend**: Vue 3 (Composition API, `<script setup>`), TypeScript, Pinia, vue-i18n, xterm.js
- **Backend**: Rust (Tauri v2), ssh2, rusqlite, AES-GCM encryption
- **Build**: Vite 6, pnpm, tauri-cli
- **Package manager**: pnpm (>=9)

## Commands

| Command | Purpose |
|---------|---------|
| `pnpm dev` | Vite dev server (web-only, port 1420) |
| `pnpm tauri dev` | Full native app dev mode |
| `pnpm build` | `vue-tsc --noEmit && vite build` |
| `pnpm lint` | ESLint: `eslint src --ext .vue,.js,.ts,.jsx,.tsx --fix` |
| `pnpm format` | Prettier (semi, singleQuote, trailingComma es5, arrowParens avoid, 80 width) |
| `pnpm tauri build` | Production desktop bundle |

## Architecture

```
src/           # Vue 3 frontend
  main.ts      # Entrypoint (Vue + Pinia + i18n setup)
  features/    # Feature-based modules (session, tabs, settings, window)
  core/        # Config, constants, i18n, types, utils
  composables/ # Reusable logic (use-modal, use-tab-management)
src-tauri/     # Rust backend
  src/lib.rs   # Tauri builder, invoke handlers, plugins
  src/ssh.rs   # SSH connection lifecycle, SFTP, server status (~1250 lines)
  src/db.rs    # SQLite (sessions, groups, tags, encryption) (~1200 lines)
  src/terminal.rs # Local PTY via portable-pty
```

IPC: `invoke` for request/response, Tauri `events` for streaming output (e.g. `ssh-output-{id}`).

## Conventions

- **No Chinese characters in code** — custom ESLint rule enforces i18n usage. Exceptions: locale files, `SettingsPanel.vue`, `WelcomeScreen.vue`.
- Path alias `@/*` → `./src/*`.
- Feature-based folders under `src/features/` with barrel `index.ts`.
- CSS custom properties for design tokens in `design-system.css`.
- `vue/multi-word-component-names` is disabled.
- `macOSPrivateApi: true` in Tauri config for overlay title bar.
- `APP_EVENTS` constants for event bus (`CustomEvent`/`dispatchEvent`).

## Testing

- Vitest configured in `vite.config.ts` (happy-dom, globals: true, v8 coverage).
- Frontend: `pnpm test` (Vitest), unit tests co-located as `*.test.ts`.
- Rust: `cargo test` (crypto round-trips, legacy export compat, SQL builder).
- CI/CD: `.github/workflows/ci.yml` (lint + type-check + frontend test + cargo build).

## Environment variables

- `TAURI_DEV_HOST` — HMR in Tauri dev mode
- `VITE_APP_VERSION` — override app version
- `VITE_DEBUG` — enable debug mode

## Workflow — Agent-Managed Development

### 1. 全权托管 (Full Agent Delegation)
- Agent 拥有项目全部决策权，用户只提需求、审查结果。
- Agent 直接修改代码、运行命令、提交推送，无需逐事请示。
- 用户指出的问题，Agent 自主排查并修复。

### 2. 版本即 Push (Push = Version)
- 每次 `git push` 都是一个版本，commit message 即为版本日志。
- 语义化提交：`feat:` / `fix:` / `refactor:` / `chore:` 前缀标明类型。
- 版本号格式：从 `v1.0.0` 递增，每次 push 前检查是否需要 tag。
  - `fix:` → patch 版本 (+1)
  - `feat:` → minor 版本 (+1)
  - 破坏性变更 → major 版本 (+1)
- Push 前必须确保代码通过 `pnpm lint`。

### 3. 构建交付 (Build for MacBook)
- 每次 push **后**，自动执行：
  ```bash
  pnpm tauri build
  ```
- 产物路径：`src-tauri/target/release/bundle/dmg/nexashell_<version>_aarch64.dmg`
- 构建失败则 Agent 必须修复后再 push。

### 提交-构建 标准流程

```bash
# 1. 修改版本号（仅需改一处）
#    编辑 src-tauri/Cargo.toml 中的 version 字段
#    prebuild 会自动同步到 package.json
#    tauri.conf.json 的 version 设为 null，自动从 Cargo.toml 读取

# 2. 提交
git add -A
git commit -m "type: 描述"

# 3. 检查版本并打 tag
git tag v<new-version>
git push origin main --tags

# 4. 构建 MacBook 版本
pnpm tauri build
```

## Gotchas

- Package manager is pnpm (>=9).
- Rust deps in `src-tauri/Cargo.toml`, JS deps in root `package.json`.
- VSCode extensions: Vue (Volar), Tauri, rust-analyzer.
