# NexaShell 分屏 (Split Panes) + 多窗口 (Multiple Windows) 可行性研究

> 日期: 2026-08-03 · 基准版本: v1.1.4 · 结论: **两者均高可行性，分屏后端零改动，多窗口小量 Rust + 权限改动**

---

## ⏱ 实施状态更新（2026-08-16 · 当前基准 v1.9.7）

| 能力 | 研究结论 | 实施状态 |
|---|---|---|
| 分屏 (横/竖/递归/拖拽) | ✅ 高可行 | ✅ **已实现**（v1.2.0 起）：⌘D / ⇧⌘D 拆分、拖拽调宽、右键拆分；每标签页上限 3 面板；SSH pane 复用源会话凭据（非响应式缓存）；v1.2.1–v1.2.3 修复布局塌陷/欢迎语重复/凭据缓存泄漏等 |
| 多窗口 A (独立实例) | ✅ 高可行 | ✅ **已实现**（v1.7.0 前）：SFTP 文件管理器抽取为独立窗口（`filemanager.html` 独立入口），`capabilities/default.json` 已含 `*` 窗口白名单 |
| 多窗口 B (标签分离, iTerm2 式) | ✅ 高可行 | ⬜ 未实现 |
| 多窗口 C (跨窗口会话镜像/接管) | ⚠️ 中 | ⬜ 未实现（暂缓） |

**与当前代码的差异点**（下文"现状架构梳理"基于 v1.1.4，部分已过时）：
- 窗口已从 `transparent: true` 改为 **`transparent: false`** + `backgroundColor:#1c1c1e`（v1.9.2/v1.9.3 为修底部白边），`clip-path` 圆角方案已废弃
- 分屏由 `useTabManagement()` 的 `splitTree` 递归结构 + `SplitRenderer.ts` / `PaneContainer.vue` 渲染，`RemoteConnectionView.vue` 已按 pane 实例化
- `sessionId !== tabId`：分屏后一个标签页含多个 pane（各自独立 session），`tabToSessionMap` 不再是严格 1:1

---

## 1. 结论摘要

| 能力 | 可行性 | 后端改动 | 前端改动 | 预估工期 |
|---|---|---|---|---|
| 分屏 (横/竖/递归/拖拽) | ✅ 高 | **0** | 解耦 tabId/sessionId + 渲染层重构 | 1–2 天 |
| 多窗口 A (独立实例) | ✅ 高 | ~80 行 + capabilities | ~50 行 | 0.5–1 天 |
| 多窗口 B (标签分离, iTerm2 式) | ✅ 高 | ~30 行 | ~150 行 + 路由 | +1–2 天 |
| 多窗口 C (跨窗口会话镜像/接管) | ⚠️ 中 | ~40 行 | ~100 行 | +1 天 |

**核心结论**: 现有架构对这两个特性**异常友好**——后端 SshManager/TerminalManager 以 `SessionId` 为 key、完全窗口无关，事件按 `{sessionId}` 隔离广播；分屏本质是"同一窗口内渲染多个 RemoteConnectionView 实例"，多窗口本质是"每个窗口加载同一前端入口、共享同一全局后端状态"。

---

## 2. 现状架构梳理（关键事实）

### 2.1 窗口
- `tauri.conf.json` 声明**单窗口** `main`：1366×768、`transparent: true`、`titleBarStyle: Overlay`、`macOSPrivateApi: true`
- `lib.rs` setup 里有一段 macOS cocoa 代码（透明背景/阴影/隐藏标题）**只作用于 `main` 窗口** —— 多窗口需泛化
- 窗口控制命令 `toggle_maximize/minimize_window/close_window` 接收 `Window` 参数（当前窗口），天然支持多窗口 ✅

### 2.2 标签与会话（前端）
- `Tab = { id, label, type: 'home'|'terminal'|'ssh', closable }`，由 `useTabManagement()`（模块级 ref）管理
- **关键耦合**: `sessionId === tabId`（App.vue `handleSSHConnect` 用同一 UUID 创建会话和标签；RemoteConnectionView 内部 `createLocalSession/SSHSession` 也把 sessionId 当 tabId 传）
- Pinia session store: `sessions: Record<sessionId, SessionState>` + `tabToSessionMap: Record<tabId, sessionId>`（**1:1 映射**）
- `AppContent.vue` 只渲染**一个**当前组件：`<component :is="currentComponent" :key="activeTabId">` 包在 `KeepAlive :max="10"` 里 —— **这是分屏的唯一结构性阻碍**

### 2.3 终端渲染
- `RemoteConnectionView.vue`（1605 行）按 `props.sessionId` 实例化，**每个实例自带**：xterm + FitAddon + WebglAddon + SearchAddon + ResizeObserver + 拖拽上传 + ServerDashboard + macOS IME 修复 —— 天然支持"同一页面多实例并存"，分屏无需改它
- 连接参数恢复路径已存在：`connectSession()` 先查 `sessionStore.hasSession(id)`，有则跳过创建、直接靠事件重放恢复 —— 切标签/切 pane 无缝

### 2.4 后端（Rust）
- `SshManager.sessions: Arc<RwLock<HashMap<SessionId, SshSession>>>`、`TerminalManager.channels` 同构 —— **全局、无窗口/标签概念**
- 事件模型：
  - 输出: `app_handle.emit("ssh-output-{id}")` / `ssh-status-{id}`（**全局广播**，任意窗口可监听）
  - 输入: Rust 侧 `app_handle.listen("ssh-input-{id}")` / `ssh-resize-{id}`（**全局监听**）
  - 上传进度: `ssh-upload-progress-{id}`，前端已按 `payload.sessionId === props.sessionId` 过滤
- 会话清理: `disconnect_ssh/disconnect_local` 按 sessionId；`RunEvent::ExitRequested` 时 `disconnect_all()`

### 2.5 权限 (capabilities)
- `capabilities/default.json`: `"windows": ["main"]` 白名单 —— **新窗口 label 不在白名单内则拿不到任何 core 权限（事件监听、窗口控制全部失效），多窗口必须改这里**

### 2.6 快捷键现状
Cmd+Q / Cmd+, / Cmd+Shift+T / Cmd+T / Cmd+P / Cmd+W / Esc —— **与 Cmd+D / Cmd+Shift+D（分屏惯用键）无冲突** ✅

---

## 3. 分屏 (Split Panes) — 详细方案

### 3.1 可行性论证
后端零改动成立的前提：
1. 任意多个 sessionId 可同时在线（HashMap 无单例限制）
2. 每个 pane 用**独立 sessionId**，事件 `ssh-output-{paneId}` 天然隔离，无串扰
3. RemoteConnectionView 已是纯 props.sessionId 驱动，多实例渲染互不干扰

### 3.2 需要的前端改动
1. **解耦 tabId / sessionId**（最核心）
   - `Tab` 增加 `panes: Pane[]`；`Pane = { paneId, sessionId, type, size?, orientation? }`
   - 会话 store 的 `tabToSessionMap` 改为 `paneToSessionMap`（或 tab → sessionIds[]），关闭 tab 时**循环断开所有 pane**（现在 `disconnectByTabId` 只断 1 个）
   - RemoteConnectionView 创建会话时 tabId 参数传 paneId（小改 2 处）
2. **AppContent 渲染层重构**
   - 活动 tab 渲染一个 `PaneContainer`（递归 split 树：`{ direction: 'horizontal'|'vertical', children: [pane|split] }`）
   - 每个叶子 pane 渲染一个 `<RemoteConnectionView :session-id="pane.sessionId">`，全部**同时可见**
   - KeepAlive 从包组件改为包 PaneContainer（`max` 上调到 16 或按 pane 数量动态）
3. **交互**
   - 拆分入口: 快捷键 Cmd+D（竖分）/ Cmd+Shift+D（横分）+ 标签右键菜单"水平/垂直拆分" + 标签页 `+` 菜单
   - "拆分当前 pane"（同服务器）: 从 `session.connectionParams` + `get_session_credentials` 取凭据，用新 sessionId 重连（凭据获取链路已存在）
   - 焦点导航: 点击聚焦；可选 Cmd+方向键 / Cmd+[ ] 切换 pane
   - 拖拽分栏条: 自研 ~100 行（pointer events + flex-basis）或引入 `splitpanes` 库（项目依赖精简，建议自研）
   - 关闭语义: Cmd+W 先关聚焦 pane，最后一个 pane 再关 tab（浏览器一致）
4. **不依赖第三方库**，纯 Vue 组合式实现

### 3.3 性能与风险
- 每 pane = 1 个 xterm 实例 + 1 个 WebGL context；建议**并发 pane 上限 8–12**；WebglAddon 创建失败时回退 DomAddon（xterm 原生支持）
- ServerDashboard / 拖拽上传 / IME 修复均为 per-instance，无需改动
- 无后端改动 → 无数据一致性风险

---

## 4. 多窗口 — 详细方案

### 4.1 三种层次
| 层次 | 描述 | 复杂度 | 适用 |
|---|---|---|---|
| **A. 独立实例** | 新窗口 = 完整 App UI（自己的标签栏/首页），各自连各自的服务器 | 低 | v1 首选 |
| **B. 标签分离** | 把当前标签"移出"到新窗口（iTerm2/浏览器式） | 中 | v2 |
| **C. 镜像/接管** | 任意窗口查看/接管任意运行中会话 | 中高 | 暂缓 |

### 4.2 关键架构利好
- 后端**全局无窗口概念** → 多窗口天然共享 SshManager/TerminalManager，会话不重复
- 事件**全局广播** → 窗口 B 只要 `listen("ssh-output-{id}")` 就能看到窗口 A 的会话输出（B/C 的基础）
- `localStorage` 跨窗口共享 → 设置/主题自动同步（副作用: 欢迎页标记、设置 store 共享，符合预期）
- 每个窗口是独立 JS 上下文 → 独立 Pinia/tab 状态，互不干扰

### 4.3 需要的改动（层次 A）
1. **Rust**
   - 新增 `create_window` command（`tauri::WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))`，复用 main 的窗口配置）
   - SshSession 增加 `owner_window: String` 字段；`RunEvent::WindowEvent { label, event: Destroyed }` 时断开该窗口拥有的所有会话（**App.vue 的 onBeforeUnmount → cleanupAllSessions 在 webview 销毁时不可靠，必须走 Rust 侧**）
   - macOS cocoa 透明/阴影/隐藏标题样式**泛化**：提取为函数，对每个新建窗口应用（现在只处理 main）
2. **capabilities/default.json**
   - `windows` 白名单加入新窗口 label（如 `["main", "window-*"]` 或新增 capability block）—— **不改则新窗口事件/窗口控制全失效（大坑）**
3. **前端**
   - "新窗口"按钮/菜单项 + 快捷键（如 Cmd+Shift+N）
   - 窗口标题按内容更新（可选）

### 4.4 层次 B（标签分离）额外改动
- 新窗口加载 `index.html?detached=<sessionId>`（query/hash 路由），main.ts/App.vue 检测后渲染**终端专用精简入口**（无标签栏/首页）
- 分离语义: 原窗口删除 tab 但不断开会话（所有权移交），新窗口 `sessionStore` 注册 stub 会话（status=connected）后直接 `listen("ssh-output-{id}")` 重放
- 关闭分离窗口 → 断开该会话（Rust Destroyed 清理按 owner 处理）
- 需要 `core:webview:allow-create-webview-window` 权限或走 Rust 命令创建（建议后者，避免权限面扩大）

### 4.5 风险
- macOS `transparent + Overlay` 多窗口: 每个窗口的圆角/阴影/标题栏需单独配置与验证（macOSPrivateApi 泛化后即可）
- 输入事件为**全局监听**: 同一会话被两个窗口同时输入才有竞争 —— 层次 A/B 不会发生（会话唯一归属），层次 C 需加锁或禁止镜像输入
- 窗口销毁时 Rust 侧清理是**必须项**，否则会话泄漏（ssh 连接挂着不释放）

---

## 5. 建议落地顺序

1. **分屏 v1**（横/竖拆分 + 拖拽调宽 + Cmd+D/Shift+D + 关闭语义）—— 价值最高、后端零改动、风险最低
2. **多窗口 A**（独立实例 + Rust 清理 + capabilities + macOS 样式泛化）
3. **多窗口 B**（标签分离）—— 视用户反馈决定
4. **多窗口 C**（镜像/接管）—— 暂缓，需输入竞争方案

分屏与多窗口**可独立交付**；组合后体验对齐 iTerm2（每窗口可多分屏、tab 可分离成窗口）。

---

## 6. 涉及文件清单

| 文件 | 改动 | 用途 |
|---|---|---|
| `src/composables/use-tab-management.ts` | 重构 | Tab 增加 panes，closeTab 批量断开 |
| `src/components/layout/AppContent.vue` | 重构 | PaneContainer 渲染 |
| `src/features/tabs/types.ts` | 扩展 | Pane/SplitTree 类型 |
| `src/components/connections/RemoteConnectionView.vue` | 小改 | 创建会话时 tabId 传 paneId |
| `src/features/session/store.ts` | 小改 | pane→session 映射、批量断开 |
| `src/features/window/operations.ts` | 新增 | createWindow 调用 |
| `src/core/utils/shortcut-manager.ts` | 新增 | Cmd+D / Cmd+Shift+D / Cmd+Shift+N |
| `src-tauri/src/lib.rs` | 小改 | create_window、Destroyed 清理、样式泛化 |
| `src-tauri/src/ssh.rs` / `terminal.rs` | 小改 | session owner_window 字段 |
| `src-tauri/capabilities/default.json` | 必改 | 新窗口 label 白名单 |
| `src/main.ts` / `App.vue` | 小改（层次 B） | detached 路由入口 |

---

*研究基于 v1.1.4 源码静态分析，未改动任何代码；实施状态更新于 2026-08-16（v1.9.7）。*
