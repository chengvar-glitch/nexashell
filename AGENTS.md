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

### 0. 对话风格 (Dialog Style)
- 与用户对话时使用 `/caveman` 技能（caveman 模式），默认 `full` 等级。
- 回复极简，去掉填充/客套/废话，保留全部技术实质。
- 代码、commit 类型（feat/fix/…）、API 名称、错误字符串原样保留，不翻译、不缩写。
- 用与用户相同的语言回复；压缩风格而非压缩语言。
- 涉及安全警告、不可逆操作确认、多步骤顺序、或将引入歧义时，自动退出 caveman 恢复完整表达。

### 1. 工程模式 (Engineering Style) — Ponytail 全程生效
- 写代码、重构、修复、评审全程启用 `/ponytail` 技能，默认 `full`，每响应生效，不回退到过度设计。
- 梯子优先：YAGNI（不需要的别造）→ 已有代码复用 → stdlib → 平台原生 → 已装依赖 → 一行搞定 → 最小可运行。
- Bug 修根因不修症状：先 grep 所有调用点，再改共享函数；一处守卫优于每个调用方各补一层。
- 不加未要求的抽象（单实现接口、单产品工厂、恒不变配置）。删除优于新增，无聊优于聪明，最少文件、最短 diff。
- 代码先行，解释最多三行；说明比代码长就删。模式：`[code] → skipped: [X], add when [Y].`
- 简化掉真实天花板（全局锁、O(n²) 扫描、朴素启发式）时用 `ponytail:` 注释标注并给升级路径。
- 非平凡逻辑（分支/循环/解析器/金钱或安全路径）留一个可运行自检：`assert` 的 `demo()`/`__main__` 或一个小 `test_*.py`，不用框架。平凡一行不需测试。
- 永不简化：信任边界的输入校验、防数据丢失的错误处理、安全措施、无障碍基础、明确要求的完整版。

### 2. 全权托管 (Full Agent Delegation)
- Agent 拥有项目全部决策权，用户只提需求、审查结果。
- Agent 直接修改代码、运行命令、提交推送，无需逐事请示。
- 用户指出的问题，Agent 自主排查并修复。

### 3. 版本即 Push (Push = Version)
- 每次 `git push` 都是一个版本，commit message 即为版本日志。
- 语义化提交：`feat:` / `fix:` / `refactor:` / `chore:` 前缀标明类型。
- 版本号格式：从 `v1.0.0` 递增，每次 push 前检查是否需要 tag。
  - `fix:` → patch 版本 (+1)
  - `feat:` → minor 版本 (+1)
  - 破坏性变更 → major 版本 (+1)
- Push 前必须确保代码通过 `pnpm lint`。

### 4. 构建交付 (Build for MacBook)
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
- `edition = "2024"` requires **Rust >= 1.85** (stable since Feb 2025). CI
  pins `dtolnay/rust-toolchain@stable`; local builds need a 2025+ toolchain.
- VSCode extensions: Vue (Volar), Tauri, rust-analyzer.
