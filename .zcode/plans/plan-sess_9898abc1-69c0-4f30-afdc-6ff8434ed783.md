# NexaShell Android 移植方案（桌面零破坏）

## 硬约束（每阶段验收必查）

1. 桌面代码**只加不改** — 除下述 5 个明确触点外不碰现有代码；移植期间顺手修桌面 bug 也禁止，单独提单独议
2. 每阶段结束跑：`pnpm lint` + `pnpm build`（vue-tsc）+ `pnpm test` + `cargo test` + 桌面冒烟
3. 唯一隐形风险点 `tauri.conf.json` 拆分 → 拆完后桌面 merge 结果必须与原文件逐字段一致

## 触点清单（允许修改的现有文件）

| 文件 | 改法 | 桌面行为 |
|---|---|---|
| `tauri.conf.json` | 拆 base + `tauri.desktop.conf.json` | 逐字段 diff 验证 |
| `src-tauri/Cargo.toml` | 加 android `[target]` 依赖段 | 不含移动代码 |
| `src-tauri/src/db/mod.rs` + `ssh/hostkey.rs` | `data_dir()` 改调新 helper（桌面分支原值） | 零变化 |
| `src-tauri/src/encryption.rs` | keyring 加 cfg 门控，desktop 原样 | 零变化 |
| `src/features/window/open-file-manager.ts` | 加 `isMobile` 分支，else 原路径 | 零变化 |

其余 Rust/前端全部是**纯新增**（cfg 分支、`mobile/` 目录）。

## Phase 0 — 基线锁定（半天）

1. 处理当前 `src-tauri/Cargo.toml` 未提交改动（确认内容后提交或还原）
2. 全量测试跑通，记录基线
3. Windows + macOS 主流程截图存档：标题栏 tabs、连接弹窗、终端、SFTP 文件管理器、设置
4. **验收**：基线入库，作为后续每阶段桌面回归对照物

## Phase 1 — Android 脚手架（1–2 天）

1. 工具链：Android Studio + SDK + NDK；`rustup target add aarch64-linux-android x86_64-linux-android`
2. `pnpm tauri android init` 生成 `src-tauri/gen/android`（Tauri v2 要求 gen 目录提交入库，含 AndroidManifest/MainActivity）
3. 配置拆分：
   - `tauri.conf.json` 瘦身为两端共享的 base（identifier/build/security）
   - 新建 `tauri.desktop.conf.json`：窗口定义（Overlay/transparent/1366×768/visible:false）、`macOSPrivateApi`、bundle（macOS signing、targets）
   - 新建 `tauri.android.conf.json`：Android 窗口与 bundle 配置
   - **验证**：merge 后桌面配置 = 原 `tauri.conf.json` 逐字段
4. `gen/android` 的 AndroidManifest 加 `INTERNET` 权限
5. **验收**：`pnpm tauri android build` 产出可安装 apk；`pnpm tauri build` 桌面行为不变；CI 绿

## Phase 2 — Rust 后端适配（2–4 天）

1. **OpenSSL 交叉编译**：android target 段加 `ssh2 = { version = "0.9", features = ["vendored-openssl"] }`（openssl-src 已内置 `android-aarch64` 配置支持）
   - 备选（若 vendored 在 Windows 交叉编译卡死）：cargo-ndk + 预编译 OpenSSL；终极备选切 `russh`（纯 Rust，无 C 链路）— 影响面大，需二次评审
2. **路径 helper（一处修根因）**：新增 `src-tauri/src/paths.rs`：`data_dir()` helper — desktop 走 `dirs::data_dir()`（原值），android 走 Tauri `app_data_dir()`；改 `db/mod.rs`、`ssh/hostkey.rs` 两个调用点
3. **keyring 替代（encryption.rs）**：cfg 门控 — desktop keyring 原样；android 用应用沙箱私有目录密钥文件（std::fs，零新依赖）
   - `# ponytail:` 注明安全天花板：沙箱文件弱于系统 Keystore，升级路径 = Android Keystore（JNI/插件，二期）
4. **本地终端裁剪**：`connect_local`/`disconnect_local` 命令注册加 `#[cfg(not(target_os = "android"))]`，Android 侧不存在
5. **验收**：`cargo check --target aarch64-linux-android` 通过；`cargo test`（desktop）全绿；Android 真机/模拟器 SSH 连接 → 交互终端跑通

## Phase 3 — 前端移动壳（3–5 天，工作量最大）

1. `platform-detection.ts` 新增 `isAndroid()`/`isMobile()` 导出（纯新增）
2. `App.vue` 顶层二选一：`isMobile ? <MobileShell/> : 现有布局原样`
3. 新增 `src/components/layout/mobile/`（参照 Termius/Termux 成熟模式）：
   - `MobileShell.vue` — 底部四 tab：主机 / 终端 / SFTP / 设置；全部复用现有 Pinia stores 与 invoke
   - `HostList.vue` — 分组卡片列表 + 搜索（数据来自现有 sessions/groups store）
   - `MobileTabBar.vue` — 顶部会话切换条
   - `ExtraKeysBar.vue` — Ctrl/Esc/Tab/方向键辅助键条，物理键盘接入自动隐藏（Termux 模式）
   - `MobileSftp.vue` — SFTP 页面化，上传/下载走 plugin-dialog（Android 系统文件选择器）
4. `open-file-manager.ts` 加 `isMobile` 分支 → 应用内浮层；else 原路径原样
5. 终端移动适配：xterm 字体基准、软键盘弹出 resize（visualViewport）、双指缩放
6. **验收**：桌面 DOM 与 Phase 0 基线一致（布局零变化）；Android 真机主流程全通（连接/终端/复制粘贴/SFTP 上传下载/设置）

## Phase 4 — 回归 + 发布链（1 天）

1. Windows + macOS 全量冒烟，对照基线截图
2. `release.yml` 加 Android matrix（apk 产物）；CI 可选加 android check job
3. 版本流程：`feat:` → minor → `Cargo.toml` version `2.5.0` → tag `v2.5.0` → push → 桌面构建照常 + Android 构建

## 明确不做（范围外）

- iOS（cfg 写法留口子，keyring/OpenSSL 不做 iOS target）
- Mosh（移动断连解法，记 backlog）
- Android 本地终端（portable-pty 不支持，且场景存疑）
- 平板/横屏深度适配（第一版竖屏优先）

## 风险表

| 风险 | 概率 | 对策 |
|---|---|---|
| openssl-sys Android 交叉构建失败 | 中 | cargo-ndk；终极备选 russh（二次评审） |
| keyring→沙箱文件安全降级 | 确定 | 一期接受并文档标注，二期 Keystore |
| xterm.js WebGL 低端 WebView 性能 | 低 | 退化 canvas renderer |
| tauri.conf 拆分引入桌面差异 | 低 | 逐字段 diff 兜底 |

总工作量约 1–2 周。工具链准备（Android SDK/NDK）在 Windows 本机一次性配置。