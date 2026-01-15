# NexaShell 的 Copilot 指南（中文）

此文档为在本仓库中工作的 AI 编程代理提供简洁、可执行的指导。

## 核心要点（重要内容）

- 架构：前端使用 Vue 3 + TypeScript（Vite），封装为 Tauri 桌面应用。参见 `README.md` 和 `src/main.ts`。
- 应用协调逻辑集中在 `App.vue`：全局状态（标签页、弹窗）、通过 `provide`/`inject` 的依赖注入（DI）以及快捷键注册。
- UI 的渲染根据标签对象的 `type` 字段进行分发，而不是通过字符串匹配。参见 `src/components/layout/AppContent.vue`。
- 会话管理：前端使用 Pinia（`useSessionStore`），并通过 `src/services/session-api.ts` 调用 Tauri（Rust 后端）例如 `connect_ssh`。

## 开发常用命令（工作流）

- 安装并启动开发界面：`pnpm install` 然后 `pnpm dev`（使用 Vite）。
- 构建：`pnpm build`（内部执行 `vue-tsc --noEmit && vite build`）。
- 运行 Tauri（本地原生测试）：`pnpm tauri`。
- 测试：`pnpm test`（vitest），`pnpm test:ui`（Vitest UI），`pnpm test:coverage`（覆盖率）。
- 代码风格：`pnpm lint` 与 `pnpm format`。

## 项目特有约定与模式

- 推荐使用 `provide`/`inject` 在组件间传递 API（参见 `src/types/injection-keys.ts`）。
- 使用 `Tab` 对象的 `type` 字段来决定渲染组件。AppContent 中常见模式：

```ts
// 使用 tab.type（显式联合类型）分配组件
switch (activeTab.type) {
  case 'terminal':
  case 'ssh':
    return TerminalView;
  case 'home':
  default:
    return NexaShellHome;
}
```

- 全局事件使用事件总线：`src/utils/event-bus.ts`，事件常量定义在 `src/constants/events.ts`。
- Pinia 是首选状态管理；`session-manager` 为遗留兼容层，新的代码应直接使用 `useSessionStore()`（参见 `src/stores/session.ts`）。
- Tauri 的 IPC 封装在 `src/services/session-api.ts`，添加后端 RPC 时保持一调用一方法的简单风格。

## 集成点与外部依赖

- Tauri 后端位于 `src-tauri/`，前端通过 `invoke` 调用（查找 `connect_ssh`、`disconnect_ssh`、`get_ssh_output` 等）。参见 `src/services/session-api.ts`。
- 终端渲染使用 xterm.js（参见 `src/components/terminal/TerminalView.vue`）。
- 全局快捷键使用 `@tauri-apps/plugin-global-shortcut`，在应用启动阶段注册。

## 编辑时代码示例（可遵循的范式）

- 新增一个标签类型：
  - 扩展 `src/types/tab.ts` 中的联合类型
  - 在 `AppContent` 中添加渲染分支
  - 在 `src/components/*` 下创建对应视图组件，保持单一责任

- 添加新的后端 RPC：
  - 在 `src/services/*-api.ts` 中增加一个 `invoke('new_command', {...})` 的封装函数
  - 同步在 Rust 端的 `src-tauri/src/*.rs` 中实现对应的命令处理

## 推荐优先查看的文件（快速入口）

- `README.md` — 架构概览
- `package.json` — 脚本与常用命令
- `src/main.ts` — 应用引导
- `src/App.vue` — 全局协调与注入点
- `src/stores/session.ts` — Pinia 会话逻辑示例
- `src/services/session-api.ts` — Tauri RPC 桥接示例
- `src-tauri/` — Rust 后端与 Tauri 配置

## 不应在未经人工审核下修改的内容

- 全局状态结构（如标签、会话）与注入 key：修改会影响大量组件。
- Tauri 命令名（例如 `connect_ssh`）：修改前需与 `src-tauri` 的 Rust 实现沟通。

---

翻译已完成。如需我将文档改为中英双语、补充更多代码示例（例如 `session-api` 调用示例或 `src-tauri` 的 Rust 对应），或对某部分做更详细说明，请告诉我具体位置。
