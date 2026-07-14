# NexaShell — Agent Guide

## Stack

- **Frontend**: Vue 3 (Composition API, `<script setup>`), TypeScript, Pinia, vue-i18n, xterm.js
- **Backend**: Rust (Tauri v2), ssh2, rusqlite, AES-GCM encryption
- **Build**: Vite 6, pnpm, tauri-cli
- **Package manager**: pnpm (>=8, enforced via `.npmrc`)

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
- **No tests exist** — setup file and all tests were deleted. Do not attempt to run `pnpm test`.
- No CI/CD configured.

## Environment variables

- `TAURI_DEV_HOST` — HMR in Tauri dev mode
- `VITE_APP_VERSION` — override app version
- `VITE_DEBUG` — enable debug mode

## Gotchas

- `pnpm test` script is **not defined** in `package.json` despite being listed in README.
- `.npmrc` sets `engine-strict=true` — must use pnpm, not npm/yarn.
- Rust deps in `src-tauri/Cargo.toml`, JS deps in root `package.json`.
- VSCode extensions: Vue (Volar), Tauri, rust-analyzer.
- Empty placeholder directories: `src/core/utils/tab/`, `src/core/utils/window/`.
