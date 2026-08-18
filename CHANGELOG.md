# Changelog

版本号以 `src-tauri/Cargo.toml` 为唯一来源（`prebuild` 自动同步 `package.json`），tag 格式 `v<version>`。
变更类型遵循语义化提交：`feat:` / `fix:` / `refactor:` / `chore:` / `perf:`。

## [Unreleased]

### Fixed

- **表单连接流程丢失服务器欢迎语**：SSH 表单连接由 App.vue 先建会话、标签页约 600ms 后才挂载终端组件，此时欢迎横幅/MOTD 早已通过实时事件发射完（无监听者，丢失）；而缓冲轮询被 624758a 的 `&& created` 条件挡住（会话已存在则不轮询），导致终端打开后没有欢迎语。修复：缓冲轮询改为每次 SSH 会话挂载都执行——后端缓存是一次性 drain，重复挂载轮询只会拿到空缓存（无重放风险），表单连接则能取回丢失的横幅。注意：v1.20.0/1.20.1 中横幅「反复滚动」是 `flush_utf8` 重发 bug 掩盖了此问题，修复重发后该问题才显现
- **SSH 终端输出被无限重复发射（横幅反复滚动、输入回显变双/变多）**：`common.rs` 的 `flush_utf8` 用 `split_off` 切出不完整 UTF-8 尾部后，完整内容仍残留在 `pending` 缓冲区里从未清除，导致每次 flush 都重新发射同一份累积内容。SSH 主通道是 20ms 超时的快速空转循环，空闲时以约 30 次/秒重发横幅/MOTD/历史输出，输入回显也随之成倍出现；本地终端因读取阻塞不空转、症状不明显。修复为 flush 时用 `mem::take` 取走完整部分，`pending` 必须清空。已用独立复现程序验证（同一服务器同一循环：修复前 3 次读取发射 326 次、323 次重复；修复后 3 次读取发射 3 次、0 重复）

### Added

- 兼容导入 XTerminal 格式：首页工具栏新增「导入」按钮，粘贴 XTerminal 导出的文本即可批量创建 SSH 会话
  - 支持 `名称/地址/端口/用户/密码` 中文标签块（含全角冒号）、`host=... user=...` 英文键值对、`host[:port] | user | pass | title` 竖线格式；`#` / `//` 注释行自动忽略
  - 密码使用应用主密钥加密存储；按 (地址, 端口, 用户名) 自动去重，重复导入不会产生重复会话；解析失败的记录逐条列出
  - 新增 `import_xterminal_sessions` 后端命令（`src-tauri/src/db/import_export.rs`，含 12 个解析单测）

## [1.17.1] - 2026-08-17

### Fixed

- GUI 终端 `Ctrl+V` / `Cmd+V` 粘贴失效：xterm.js 在 Tauri WebView 里不会为 `Ctrl+V` 可靠读取剪贴板（此前按键被吞掉、仅 `Shift+Insert` 可用）。现在显式 `navigator.clipboard.readText()` 后 `terminal.paste()`，并统一 `Ctrl+C` 复制分支（选中即复制 + `clearSelection`，无选中仍作为 SIGINT 直通 shell）

## [1.17.0] - 2026-08-17

### Added

- TUI SFTP 文件浏览器：已连接会话 `ctrl+x s` 打开全屏浏览器
  - 目录浏览（目录优先、忽略 . / ..、显示大小与本地时间），回车进目录、左键/退格上一级、home 回 home、r 刷新
  - 文件下载到本地工作目录、选中文件 `u` 上传（同名路径）、`n` 内嵌输入新建目录、`x x` 双击确认删除
- 每项 SFTP 操作走独立认证连接（复用 `connect_authenticated`），与交互/监控会话隔离；异步回调刷新列表与状态

### Changed

- `ssh.rs` 新增 `SftpEntry` 与 `sftp_list_dir` / `sftp_download_file` / `sftp_upload_file` / `sftp_mkdir` / `sftp_remove`；新增 `UiEvent::Sftp` 事件路由
- 顶部菜单文案随焦点变化（回看/复制/命令/断开）

## [1.16.0] - 2026-08-17

### Added

- TUI 隧道规则面板：`ctrl+x u` 或命令面板 `tunnels` 打开，列出当前会话的持久化转发规则（方向 L/D、监听 → 目标、连接数、auto/manual），`enter` 切换启停，`r` 刷新
- 隧道运行时：每条规则独立绑定监听端口，接受连接后经独立认证连接建立 direct-tcpip 通道双向泵送；本地转发 (-L) 与动态 SOCKS5 (-D) 均支持，非阻塞 accept 保证启停即时、端口即时释放
- 断开会话时自动停掉该会话的全部隧道；退出时统一清理
- 首页会话列表为已打开会话显示 ● 标记；连接失败提示可正常关闭

### Changed

- `establish_sessions` 重构出 `connect_authenticated`（单连接认证，隧道复用保证不与交互/监控会话争用）；`SshChannelInfo` 保留连接参数供隧道使用

## [1.15.0] - 2026-08-17

### Added

- TUI 会话管理：首页选中会话 `ctrl+x e` 编辑（表单预填，密码留空则保留已存凭据）、`ctrl+x f` 切换收藏、`ctrl+x d` 删除（二次确认，同时关闭对应已开终端）
- TUI 命令片段：`ctrl+p` 命令面板列出 snippets，选中后插入当前活动终端（可再编辑）或复制到剪贴板
- 修复对话框关闭逻辑：此前 Esc/Enter 会保持对话框打开、任意其他键反而关闭；现统一为 Esc 关闭、Enter 提交、其余键忽略

### Changed

- `NewSessionForm` 增加 `editing` 状态（编辑模式标签改为「留空=保留已存凭据」），新增 `Dialog::ConfirmDelete`；`dialog_key` 重写为 keep 标志语义

## [1.14.0] - 2026-08-17

### Added

- TUI 多会话并存：同时保持多个 SSH 连接，终端页顶部 tab 条显示全部会话（断线标注 down）
- 切换：`ctrl+tab` / `ctrl+shift+tab` 循环，leader `t` 循环，`ctrl+x 1`-`9` 直达；断开会话后其余保持打开，全部断开才回首页
- 会话级状态栏（每个终端独立保存 CPU/内存/延迟/负载），状态栏显示 user@host、回看行数提示

### Changed

- `App` 从单终端重构为 `terminals: Vec<TerminalSession>` + `active_term`；事件按 session_id 路由到对应终端；退出时 `disconnect_all` 统一清理

## [1.13.0] - 2026-08-17

### Added

- TUI 终端滚动回看：`PgUp`/`PgDn`/`Home`/`End` 与鼠标滚轮在终端页面本地滚动历史输出（10k 行环形缓冲，独立于 vt100 的屏内视图；vt100 0.15 的滚动视口偏移超过一屏高度会下溢，因此历史由 TUI 自行按行捕获，滚动区以暗色渲染）
- TUI 复制模式：`ctrl+x c` 进入，方向键扩展选区，`enter`/`ctrl+c` 复制到系统剪贴板（arboard），`esc` 退出；选区在滚动视图中按可见坐标高亮
- 终端页顶部提示条显示当前回看行数与快捷键提示（`ctrl+x c` 复制）

### Changed

- `term.rs` 新增 scroll 视图（`scroll_up`/`scroll_down`/`scroll_top`/`scroll_to_bottom`/`selection_text`）并配套 8 个单元测试（滚动、钳制、跨 feed 换行捕获、视图钉住、选区提取）

## [1.12.1] - 2026-08-17

### Added

- TUI 质量门：CI 新增 `tui` 任务，对 `src-tui` 执行 `cargo check` + `test` + `clippy -D warnings`
- TUI 文档：README 新增「Terminal UI（TUI）」章节（运行方式、桌面/TUI 功能对照表），新增 `pnpm tui` 运行脚本

### Fixed

- 清理 `src-tui` 全部 46 个编译警告（未接线的共享数据库镜像 API 标注 `allow(dead_code)`，clippy 修复 5 处（collapsible-if/needless-borrow/manual-is-ascii-check））

## [1.10.2] - 2026-08-16

### Fixed

- 修复 vue-tsc 类型检查发现的 3 处真实类型错误（此前本地 bun 运行 vue-tsc 会漏报）：
  - `TunnelManagerPanel.vue`：定时刷新回调参数 `statuses` 遮蔽外层 ref，`statuses.value` 误作用于数组
  - `CommandPalette.vue`：`TabLike` 接口缺少 `id` 字段
  - `RemoteConnectionView.vue`：下载进度事件 payload 的 `status` 类型缺少 `'downloading'`，导致比较无重叠
- 发布构建仅保留 macOS（Apple Silicon，aarch64）与 Windows（NSIS），去掉 Linux 与 macOS Intel（universal）

## [1.10.1] - 2026-08-16


### Fixed

- 修复 cargo clippy `collapsible-if` 告警（`db/mod.rs` 隧道规则方向校验），使 CI 的 `clippy -D warnings` 通过

## [1.10.0] - 2026-08-16

### Added

- 设置页「关于」区域改版：定位描述改为「面向个人开发者的轻量级 SSH 会话管理工具」，并新增主要特性列表（多会话/分屏、SFTP、本地终端、端口转发、命令片段、主题、安全、导入导出），中英双语

## [1.9.7] - 2026-08-16

### Fixed

- 主界面右键菜单：全局拦截浏览器默认菜单（Inspect Element/Reload 等），收窄放行列表（仅链接/输入框/文本域/终端等保留原生菜单）
- 文件管理器窗口右键弹出浏览器菜单：全局拦截 `contextmenu`

> 注：`v1.9.5` / `v1.9.6` 未单独打 tag，以下变更合并计入 v1.9.7：
> - 终端显示与分屏修复：底部白/黑边跟随主题、8px 内边距、拆分上限 3、第三面板无输入输出

## [1.9.4] - 2026-08-16

### Fixed

- 底部白边：全量删除各组件为“圆角窗口”添加的 `border-radius`

## [1.9.3] - 2026-08-16

### Fixed

- 窗口底部宽白边：关闭窗口透明与 `clip-path`，窗口不透明、底色与 UI 一致

## [1.9.2] - 2026-08-16

### Fixed

- 窗口底部白边：主窗口设置 `backgroundColor:#1c1c1e`，裁剪圆角 transparent 穿透

## [1.9.1] - 2026-08-16

### Fixed

- 终端边框/外框：去掉外层 10px padding 与硬编码深色背景，终端铺满整页

## [1.9.0] - 2026-08-16

### Added

- **终端主题系统**：One Dark / Modern / Solarized / GitHub 四套预设，默认跟随系统明暗模式

## [1.8.1] - 2026-08-16

### Fixed

- 设置页点击抖动：覆盖全局 `.panel` 的 `:active` 缩放与 hover 动效 backdrop-blur 抖动

## [1.8.0] - 2026-08-16

### Added

- **SSH 端口转发**：Local（`-L`）与 Dynamic SOCKS5（`-D`）转发，规则随会话持久化并在连接时自动启动
- **命令片段库 + 命令面板**：可复用的命令片段，⌘K 快速唤起
- **多强调色主题**

## [1.7.1] - 2026-08-16

### Performance

- 降低面板隐藏时的闲置 CPU 占用与重绘

## [1.7.0] - 2026-08-16

### Added

- **独立 SFTP 文件管理器窗口**：SFTP 文件浏览器抽取为独立窗口（⌘O），支持目录浏览/文件列表定位当前目录/独立传输会话
- 主窗口 UI 打磨为原生桌面质感（2.11 版本线对齐）

### Fixed

- 文件列表 hover 抖动；文件管理器窗口主题同步

## [1.6.0] - 2026-08-15

### Added

- 已保存连接下拉
- SFTP 进度节流
- 补 UploadControl 测试

### Fixed

- 进度条显示与取消逻辑；上传断连清理

## [1.5.0] - 2026-08-15

### Added

- **上传前确认弹窗**：拖放文件先展示解析后的目标路径，确认后再传输
- **暂停 / 继续 / 取消**：后端 per-task `UploadControl`，暂停驻留块边界、断点续传、取消删除半截远端文件
- 任务队列增加暂停/继续/取消按钮与 `paused` / `cancelled` 状态

## [1.4.1] - 2026-08-15

### Fixed

- SSH 敲命令即断连：改用 blocking + `set_timeout` 模式绕开 libssh2 non-blocking EAGAIN 问题；TCP 层 `SO_KEEPALIVE` 抗云安全组空闲断连；前端监听断开事件提示

## [1.4.0] - 2026-08-13

### Refactored / Optimized

- 路径解析 OSC 优先
- `known_hosts` 并发处理
- DB SQL 样板抽取（`build_update`）
- SSH 空闲退避
- 监控缓存
- 终端输出批处理
- 拆分大文件（ssh.rs / db）
- 补齐测试；接入 CI/CD；Rust edition 2024

## [1.3.4] - 2026-08-09

### Fixed

- SSH 登录欢迎横幅丢失：会话先于终端挂载时回补初始输出缓冲

## [1.3.3] - 2026-08-09

### Fixed

- SSH 登录后无输出且无法输入：恢复 PTY 规范模式与回显（ECHO/ICANON）

## [1.3.2] - 2026-08-09

### Fixed（安全审查 P0 项）

- RemoteConnectionView 生命周期钩子清理全部失效
- helper 会话补充主机密钥验证
- 非阻塞写背压重试
- 钥匙串损坏拒绝重建（防凭据孤儿化）
- 编辑会话不清空凭据
- 导入导出持锁；PBKDF2 移出阻塞路径
- 连接取消、定时器分池、eslint 全局 ignore、devCsp、依赖清理

## [1.3.1] - 2026-08-09

### Fixed

- 连接成功后表单未关闭：`finally` 误清成功路径的关闭定时器

## [1.3.0] - 2026-08-09

### Added（大版本安全加固）

- **主机密钥验证**（`known_hosts`，防 MITM）
- **密钥库存储**：OS keychain 为主、文件兜底
- **随机盐导出**：加密导入/导出
- **SFTP 独立会话**
- DB 事务与 WAL、CSP、日志系统
- 前端 20 余项泄漏与类型修复

## [1.2.3] - 2026-08-07

### Fixed

- 分屏拖拽重归一化
- 监听器竞态
- 凭据缓存泄漏
- SFTP 路径穿越
- 审查 17+4 项修复

## [1.2.2] - 2026-08-03

### Fixed

- 分屏欢迎语重复显示（回补 seq 去重）
- 拆分误断原 pane 会话（会话生命周期归标签层）
- 单 pane 统一渲染路径，保留 xterm 历史

## [1.2.1] - 2026-08-03

### Fixed

- 分屏布局高度塌陷（scoped CSS 失效）
- 拆分新 pane 无欢迎语（改为完整连接流程）
- 凭据不入响应式状态

## [1.2.0] - 2026-08-03

### Added

- **分屏 v1**：⌘D / ⇧⌘D 拆分、拖拽调宽、右键拆分、多 pane 会话管理

## [1.1.4] - 2026-08-01

### Fixed

- SSH 监控超时、事件泄漏、静默错误等 8 处修复

## [1.1.3] - 2026-07-28

### Refactored

- 统一版本号管理，`Cargo.toml` 为唯一来源

## [1.1.2] - 2026-07-28

### Chore

- 包管理器从 pnpm 迁移至 bun

## [1.1.1] - 2026-07-27

### Fixed

- macOS WebKit 中文 IME 符号输入 xterm 终端

## [1.1.0] - 2026-07-26

### Added / Improved

- 首页会话加载消除 N+1 查询
- 清理生产环境 console 日志

## [1.0.3] - 2026-07-24

### Fixed

- 对齐 Tauri JS/Rust 版本依赖（pin ~2.9.x），升级 uuid/@types/node
- SSH 空闲 CPU 优化
- 移除重复 resize 监听

## [1.0.2] - 2026-07-17

### Fixed

- 语言切换失效：动态导入未解构 `.default` 导致 `mergeDefaults` 合并错误

## [1.0.1] - 2026-07-17

### Fixed（首轮发布修复）

- 严重内存泄漏：无界 channel 泄漏、快捷键冲突、PBKDF2 阻塞主线程、DB 连接泄漏、N+1 查询、语言切换失效
- 配套性能优化：DB 单例、加密缓存、SSH 零拷贝 I/O、前端 `shallowRef`、i18n 懒加载

## [1.0.0] - 2026-07 (Initial)

- 首个可用版本：SSH 连接/会话管理、xterm.js 终端、SFTP 上传、服务器监控、中英双语 i18n
- 前端安全重构与 UI 微交互打磨（骨架屏/焦点环/Spring 过渡）
