# NexaShell 代码审查报告

- **审查范围**：`src/`（Vue 3 前端）+ `src-tauri/src/`（Rust 后端）；排除 `src-tui`、`node_modules`、`target`、`dist`
- **覆盖规模**：13 个模块（后端 4 / 前端 9），约 2.1 万行代码（前端约 1.5 万行 / 后端约 5.6 千行）
- **版本基线**：审查期间工作区被推进 — 起始 v1.18.0，中途更新至 v1.19.0（`eeff4d1` 新增"复制连接信息（含明文密码）"）；最终报告基于 v1.19.0 代码
- **审查方法**：13 个并行审查代理分模块审计（第一轮 workflow 输出因结果过大被截断丢失 10 个模块详情，已用 10 个独立子代理重审补齐），关键高危发现已人工对照源码复核
- **严重度定义**：critical=崩溃/数据丢失/安全漏洞；major=确定性缺陷或显著风险；minor=健壮性/边界问题；nit=风格/清理

## 统计概览（最终核定）

| 严重度 | 数量 |
|--------|------|
| Critical | 0（首轮 workflow 统计曾报 1 项，出自输出丢失的模块；对应模块重审后无代理给出 critical 分级，均按 major 处理，见下） |
| Major | 37 |
| Minor | 97 |
| Nit | 33 |
| **合计** | **167** |

> 统计口径说明：全部数据来自 13 个审查代理的最终交付（R1–R4 后端、F1–F9 前端），并已逐条落到下方模块表格。首轮 workflow 的聚合计数（1/31/100/29）与重审后的计数（0/37/97/33）存在差异：一是重审代理分级更保守（原 critical 与部分 minor 被降级/重分级），二是首轮个别模块的细节在截断中丢失、重审基于同一代码重新产出。下表以重审后的完整数据为准。

## 优先修复清单（Top 10，按业务影响排序）

1. **[安全] 明文凭据往返渲染进程**：`get_session_credentials` 解密后把密码/口令返回前端，再经 invoke 回传 `connect_ssh`；任意前端 XSS 可批量窃取全部凭据（B2）。建议后端按 session id 直连，凭据不出渲染层。
2. **[数据丢失] 会话更新静默清空分组/标签关联**：`save_session_with_credentials` 更新分支无条件 DELETE 关系、重插受 `if let Some` 保护（B2 mod.rs:501-547）。`None` 应视为"不变"。
3. **[数据丢失/安全] 会话编辑时空密码覆盖已存密文**：SSH 表单无条件提交空 password/key_passphrase（F6），与后端凭据防误清空保护（B2 mod.rs:455-489）形成前后端对抗。
4. **[数据丢失] 上传无声覆盖远端同名文件**：SFTP 上传无存在性检查，后端 `WRITE|CREATE|TRUNCATE` 直接覆盖（F7 / ssh.rs:1304-1310）。
5. **[生命周期] 停止隧道端口不释放**：`thread().unpark()` 无法中断阻塞 `accept()`，端口永久占用、重启 AddrInUse、线程+FD 泄漏（B3 tunnel.rs:182-331）。
6. **[生命周期] 断开不回收传输任务**：`disconnect_ssh` 不 join/abort 传输；取消上传在会话拆除后新开完整 SSH 连接删半成品（B1）。
7. **[可靠性] helper/transfer 会话无限阻塞超时**：对端挂死→SFTP/监控冻结、断开清理线程泄漏（B1 ssh.rs:420-426, 1703-1710）。
8. **[功能] 关闭非末尾活动标签→空白工作区**：`Math.min(index, length-2)` 指向被删标签（F1 use-tab-management.ts:275-278）。
9. **[功能/快捷键] 终端聚焦时 Ctrl+Q 退出应用 / Ctrl+D 切分屏又发 EOF**；Tab 全局被吞导致表单无法原生 Tab 导航（F2 shortcut-manager.ts）。
10. **[体验] 无用户可见连接错误与断线重连**：连接失败只记日志、断线无重连路径、本地回显造成"看似在输入"（F5）。

**跨模块关联提示**：B2 的关系清空缺陷与 SSHConnectionForm 的编辑提交流程组合后（F6）可形成真实数据丢失链；B4 的 `read_file_preview` 无校验 + B2 的明文凭据 + F2 的 logger 无脱敏，三者叠加构成"一处 XSS 即可外泄全部凭据"的攻击路径 — 建议安排一次以 XSS→凭据外泄为主线的威胁建模。

## 模块索引（正文顺序为审查完成顺序，检索优先用此索引）

| 模块 | 内容 | 核心结论 |
|------|------|----------|
| B1 | ssh.rs + ssh/hostkey.rs（SSH/SFTP 核心） | 无限阻塞超时 + 传输任务不回收 |
| B2 | db/mod.rs + import_export.rs（数据库/导入导出） | 关系静默清空 + 明文凭据回渲染层 |
| B3 | tunnel.rs + encryption.rs + terminal.rs | 隧道端口不释放 + PTY UTF-8 乱码 |
| B4 | lib.rs/system.rs/配置/CSP/capabilities | 任意文件读 IPC + 窗口通配授权 |
| F6 | ServerDashboard + SSHConnectionForm | 空密码覆盖密文 + listener 竞态 |
| F5 | RemoteConnectionView + ConnectionProgressBar | 无错误提示/重连 + 英文硬编码 |
| F8 | SettingsPanel + 多选/下拉组件 | 空下拉 Enter 崩溃 + 重复 DOM id |
| F7 | SftpBrowser + FileManagerWindow + TunnelPanel | 上传覆盖 + mkdir/rename 路径逃逸 |
| F4 | NexaShellHome + Search + CommandPalette | 命令面板错开设置 + 死事件 |
| F2 | 核心 utils（快捷键/日志/事件总线） | Ctrl+Q 退出应用 + Tab 被吞 + 日志无脱敏 |
| F3 | App.vue + layout + 通用组件 | Ctrl+T 覆盖会话 + 切分丢回滚 + KeepAlive 泄漏 |
| F1 | features + composables | 关标签空白区 + 取消复活 + 断开失败孤儿会话 |
| F9 | 样式/locales/入口/构建配置 | locale 完美对齐 + CSS 令牌旁路 |

---

## 后端模块

### B1. SSH/SFTP 核心（`src-tauri/src/ssh.rs` + `src-tauri/src/ssh/hostkey.rs`，约 2670 行）

**总体评价**：架构扎实 — 阻塞 SSH 调用全部走 `spawn_blocking`；交互 shell 与 SFTP/监控各自独立连接；主/副连接都在**认证前**做 host key TOFU 校验；Windows 盘符路径归一化有单元测试覆盖。主要风险集中在**生命周期与可靠性**：helper/transfer 会话处于 libssh2 默认无限阻塞超时，对端挂死后会拖死 SFTP/监控并泄漏线程；`disconnect_ssh` 从不 join/abort 传输任务，取消上传甚至会在会话拆除后新开一条完整 SSH 连接。未发现崩溃/数据丢失/凭据泄露类缺陷。

**优点**：主副连接均做 host-key 验证（堵住了 SFTP/监控流的 MITM 口子）；20ms 超时 + 输入后立即 flush 的 I/O 循环设计（对应最新 commit）；传输暂停/恢复/取消用 condvar + 200ms 轮询实现，cancel 优先于 pause；进度事件限流（100ms/1MB）；传输使用独立连接避免阻塞文件浏览器。

| 严重度 | 位置 | 问题 | 建议 |
|--------|------|------|------|
| **Major** | ssh.rs 420-426, 1703-1710（影响 1549-1569, 1906-2035, 1200-1204） | **helper/transfer 会话无限阻塞超时**：握手后 `set_timeout(0)`，仅主会话设 20ms。对端挂死→SFTP/monitoring/probe 永久阻塞：helper mutex 被永久占用（文件浏览器冻结）、disconnect 清理线程卡在 `blocking_lock()` 上泄漏线程+Session、传输看不到 cancel 标志永远无法终止 | helper/transfer 会话设置有界超时（如 `set_timeout(30_000)`），并用带超时的 join 替代无界清理线程 |
| **Major** | ssh.rs 1156-1208, 1270-1531, 2296-2314 | **`disconnect_ssh` 从不 join/abort 传输任务**；取消上传时在会话已拆除后仍**新开完整 SSH 连接**（`open_transfer_sftp`，line 1467）仅为了删除半成品文件（新增 TCP+握手+主机密钥验证+认证，最长 30s） | 在 `SshChannelInfo` 记录传输 JoinHandle 并在 disconnect 时 abort/await；删除半成品文件改在同一条传输连接内完成；teardown 后跳过清理 |
| Minor | ssh.rs 1466-1470（对照 1302-1316） | 取消上传用**未归一化**的原始路径删除半成品（Windows 风格 `C:\...` 输入时删不掉） | 外层捕获归一化后的路径，open 与 unlink 共用 |
| Minor | ssh.rs 1506-1510, 2289-2293 | 传输任务 panic 时 control-map 条目泄漏（清理逻辑在闭包内），后续 pause/cancel 对死控制静默失效 | 把 map 清理挪到 watcher `tokio::spawn` 的 await 之后，或加 scopeguard |
| Minor | ssh.rs 303-357 | **重复 session id 直接覆盖旧条目**：旧 listener 未 unlisten（EventId 丢失）、旧任务被 detach 而非 abort（stop_flag=false，继续收 `ssh-input-{id}` 事件）→ 双活会话同时接收每次按键 | connect 前对已存在 id 先跑 disconnect 清理，或拒绝重复 |
| Minor | ssh.rs 1144-1150 | `get_buffered_ssh_output` 用 `try_lock()`，I/O 任务恰好持锁时返回**空数组**，首屏 banner 永久丢失 | 用阻塞 lock（放 spawn_blocking）或带退避重试 |
| Minor | ssh.rs 1335, 1611-1636, 2325 | pause/cancel 的 `std::sync::Mutex` 全部 `.lock().unwrap()`，一旦被 poison 后续暂停/恢复永久 panic | 与 channels map 一致地 `map_err` 处理 poison |
| Minor | ssh.rs 20, 698-728 | 首屏输出缓存只保留 200 块且仅 2 秒，晚订阅/超量输出静默丢失 | 加大/按时间缓存，前端在 connect 前先订阅 |
| Minor | ssh.rs 2143-2148, 2254 | 下载立即 `File::create` 截断目标文件，取消时删除（覆盖已有文件时取消=原文件被毁） | 写 `.part` 临时文件，成功后再原子 rename |
| Minor | hostkey.rs 94-109 | 首连写盘失败仅记日志→TOFU 静默失效；密钥轮换/误改后无"忘记主机密钥"入口，永久拒绝连接 | 落盘失败返回错误/事件；提供重置入口 |
| Nit | ssh.rs 374-468 vs 1676-1791 | TCP 打开与认证逻辑两处重复实现，未来单边修补会漂移 | 抽取公共 helper |
| Nit | hostkey.rs 96-99 | known_hosts 用 HashMap 重写，顺序不确定且未 fsync | 追加式更新 + fsync / 原子写 |

**低置信度**：resize 回调里 `tokio::spawn`（566-571，依赖 Tauri 事件分发线程有无 runtime 上下文）；`normalize_remote_path` 折叠重复前导 `/` 可能改变 `//` 根语义；首屏缓存与实时事件都带 seq，前端去重逻辑无法从后端验证。

---

### B2. 数据库层（`src-tauri/src/db/mod.rs` + `src-tauri/src/db/import_export.rs`，约 1560 行）

**总体评价**：结构良好 — 全部参数化绑定（无 SQL 注入）；表名/列名在插值前白名单校验；PBKDF2（39 万次迭代）刻意移出共享 DB 锁；多表写入用事务；凭据防误清空保护体现了对静默数据丢失的重视。主要问题：会话更新路径对 group/tag 关系的处理与凭据保护**不对称**（调用方省略则全部清空）；导入时关系写入错误被吞掉；以及**解密后的明文凭据按设计返回给渲染进程**。

**优点**：参数绑定 + 标识符白名单，未发现注入；导入导出在锁外做加解密并发控制良好；WAL + busy_timeout + 串行单连接；`add_column_if_not_exists` 幂等。

| 严重度 | 位置 | 问题 | 建议 |
|--------|------|------|------|
| **Major** | mod.rs 501-510 | **会话更新无条件删除全部 group/tag 关联**：`DELETE FROM session_groups/tags` 总是执行，但重插受 `if let Some` 保护（529-547）。调用方省略 group_ids/tag_ids（前端空选择转 null，App.vue:535-536）即静默清空所有分组/标签 —— 与上方 455-489 行明确保护的凭据字段形成反差（该处注释正是警告同类静默清空） | `None` 视为"不变"；显式空数组 `[]` 才表示"清空" |
| **Major** | import_export.rs 205-225 | 导入对关系清理/重链全部 `let _ = tx.execute(...)` **吞掉错误**，其余语句均 `?` 回滚 — 事务可能带着半应用的关系集提交，且告知用户导入成功 | 全部改 `?` 传播，失败即整体回滚 |
| **Major** | mod.rs 553-577 | **`get_session_credentials` 把明文密码/口令返回渲染进程**，随后再经 invoke 回传 `ssh::connect_ssh`（ssh.rs:258-272）— 任意前端 XSS 可调用该命令批量窃取全部凭据。对凭据管理类应用是实打实的攻击面 | 增加"按 session id 直连"后端路径（凭据不出后端），移除渲染进程侧收发明文 |
| Minor | mod.rs 980-1001, 296-311 | 删除会话**遗留 tunnel_rules 孤儿行**（无 FK 级联，`delete_tunnel_rules_for_session` 存在但从未被调用）— 同 UUID 重建会话时过期规则静默复活 | tunnel_rules 加 `ON DELETE CASCADE`（含旧表迁移）或在 delete_session 内先删规则 |
| Minor | mod.rs 221-311 | session_groups/session_tags/tunnel_rules **全部无 FOREIGN KEY**，`foreign_keys=ON` 形同虚设；导入可写入指向不存在 group/tag 的关联 | 12 步表重建迁移补 FK，删除手写关联清理 |
| Minor | mod.rs 288-291 | 每次启动的 `last_connected_at IS NULL` 回填会把"从未连接"的会话盖上 updated_at，UI 显示伪连接时间 | 仅在加列迁移当时回填一次，或保留 NULL |
| Minor | mod.rs 219 | `tags.color` 迁移错误被 `let _` 吞掉；失败后所有 list_tags 查询运行时报 "no such column" | `?` 传播 |
| Minor | import_export.rs 191-226 | 导入用 `INSERT OR REPLACE`（无去重/冲突报告）：文件内重复 id 互相覆盖、覆盖本地同 id 会话均静默；groups/tags 用 INSERT OR IGNORE 可挂到同名不同内容的分组 | 解析期去重校验 + 向 UI 报告 导入/更新/跳过 数量 |
| Minor | import_export.rs 154-155 | 导入 payload **无大小上限**，数 GB 字符串直接进内存全量解析 + 无深度保护（本地 DoS） | 上限（如 50MB）+ 深度受限/流式解析 |
| Minor | mod.rs 708-715 | LIKE 搜索字符未转义：`%`/`_` 通配符导致错误匹配 | 转义 `\%\_\\` 并加 `ESCAPE '\'` |
| Nit | mod.rs 1271-1303 | update_tunnel_rule/update_snippet 缺空集合早退，空更新也会改写 updated_at | 与其他 edit_* 一致 |
| Nit | mod.rs 399-417 | `add_session` 是死代码（前端从未调用），且与两个插入路径重复 | 删除或合并 |

**低置信度**：导入覆盖语义（restore vs merge）需产品确认；导出密码无最小长度校验（疑在 UI 层）；`init_db` 失败仅记日志后应用继续运行（lib.rs:33），所有 DB 命令报"未初始化"。

---

### B3. 隧道 / 加密 / 本地终端（`src-tauri/src/tunnel.rs` + `src-tauri/src/encryption.rs` + `src-tauri/src/terminal.rs`，约 1150 行）

**总体评价**：明确的生命周期缺陷最突出 — **停止隧道从不终止 accept 循环也不关 listener**（端口一直占用、重启 AddrInUse、状态永远停在 "listening"；`thread::unpark()` 无法打断阻塞在 `accept()` 的线程）；`connect_local` 对同一会话重复调用会静默覆盖旧 `TerminalInfo`，孤儿化之前的子进程/PTY/listener/task。另有一个真实的输出正确性缺陷：**4096 字节读边界处拆散多字节 UTF-8 → 乱码（U+FFFD）**。加密整体扎实（随机 salt+nonce、39 万次 PBKDF2、SensitiveData 零化），但主密钥存于 static OnceCell 永不零化、明文 JSON 缓冲不零化、keychain 损坏时拒绝使用可能有效的文件兜底密钥。

| 严重度 | 位置 | 问题 | 建议 |
|--------|------|------|------|
| **Major** | tunnel.rs 182-229, 267-331 | **停止隧道端口不释放**：`TcpListener` 是 run_listener 的局部变量，stop 只置标志 + `thread().unpark()`（193），无法打断阻塞的 `listener.accept()` → 线程+FD 泄漏、`start_tunnel` 立即 `AddressInUse` 失败、状态下 "stopped"（331）不可达；会话重连全部隧道标 "failed" | 把 listener 移入共享可 Drop 的 Arc<Mutex>，停止时 drop 之；用非阻塞+poll 或真正 join |
| **Major** | terminal.rs 184-200 | **`connect_local` 静默覆盖已有会话**：第二次连接直接 insert 覆盖旧 TerminalInfo — 旧子 shell 不 kill/wait、PTY/reader/writer 不关、input/resize listener 不 unlisten、task 不 abort，孤儿 shell 活到应用退出 | 插入前先 `disconnect_local(session_id)` 或对已存在返回 Err |
| **Major** | terminal.rs 125-130 | **UTF-8 多字节序列在 4096 边界被 `from_utf8_lossy` 拆断** → 半个字符变 U+FFFD，非 ASCII 输出（中文 ls、PowerShell/CJK）永久乱码 | 跨读保留 carry buffer，只解码完整 UTF-8 序列 |
| Minor | tunnel.rs 413-418 | 客户端半关闭（写侧 EOF）时立即断连：先 `chan.eof()` 再 break，远端剩余响应数据丢失；channel 未 `close()`/`wait_close()`，SSH 侧未干净半关闭 | EOF 后继续排空 channel→client 直到 channel EOF，再 close+wait_close |
| Minor | tunnel.rs 436-524 | SOCKS5 协商失败路径**不回写 RFC1928 失败应答**（VER 05 REP 07），客户端挂到自身超时；greeting 应答无条件选 no-auth 不校验客户端是否提供 | 失败路径至少回写 {0x05,0x07}；校验方法列表含 0x00 |
| Minor | tunnel.rs 43, 267, 306-321, 454 | listen_host 用户可控：绑 0.0.0.0 时**无认证 SOCKS5/转发成为局域网任意主机的 TCP 跳板**；accept 洪泛时每连接新建线程+完整认证 SSH 连接，无并发上限 | 默认强制回环绑定并提示；加每规则并发上限 |
| Minor | encryption.rs 53-95, 109, 222, 258 | 主密钥存 **static OnceCell 生命周期=进程，无法零化**；keyring 读出的字节、文件密钥字节、明文 JSON Vec、解密明文均未零化 — 与 SensitiveData 的零化纪律不一致 | 包裹 Drop 时零化的守卫；密钥改为运行时 Arc 持有 |
| Minor | encryption.rs 58-79 | keychain 条目损坏（长度/base64 错误）直接 Err，**拒绝尝试文件兜底密钥**（仅 NoEntry 才回退）→ 明明可恢复却永久锁死凭据 | 损坏分支先尝试文件兜底，绝不覆盖 keychain |
| Minor | encryption.rs 138-143, 49-51 | Unix 兜底密钥文件 0644→后置 chmod 0600 存在竞态窗口；**Windows 上完全不设 ACL**，所谓 0600 保护名存实亡 | Unix 用 OpenOptions mode 0o600+create_new；Windows 走 DPAPI/Credential Manager |
| Minor | terminal.rs 142, 161 | shell 退出不传播：reader 结束但 writer 卡在 `recv()`、listener 仍注册、TerminalInfo 仍在 map → UI 永远显示"已连接" | reader EOF 时 drop sender/abort 输入 task，发 exit 事件并移除条目 |
| Nit | terminal.rs 251-281 | disconnect_local 用 `if let Ok` 吞 poison（与 connect_local 不一致）；先 abort 输出 task 再杀子进程，阻塞线程退出延迟 | 传播 poison；先 kill/wait 子进程再 abort task |
| Nit | tunnel.rs 540-542 | DB 端口 `as u16` 静默截断（>65535 的值绑定错误端口） | 校验范围并报错 |

**低置信度**：拒绝文件兜底密钥可能是"绝不覆盖"的刻意策略；每连接线程克隆含明文密码的 `SshAuth`（与既有设计一致，内存卫生问题）；pump 中 2ms WouldBlock 休眠属微小延迟权衡。

**低置信度**：encryption.rs 拒绝文件兜底密钥可能是"绝不覆盖"的刻意策略；每连接线程克隆含明文密码的 `SshAuth`（内存卫生问题）；pump 中 2ms WouldBlock 休眠属微小延迟权衡。

---

### B4. 系统入口与安全配置（`lib.rs` + `system.rs` + `common.rs` + `main.rs` + `build.rs` + `Cargo.toml` + `tauri.conf.json` + `capabilities/default.json`）

**总体评价**：入口结构清晰 — main.rs 仅 4 行引导、invoke_handler 显式注册、版本单一来源且同步正确（Cargo.toml = package.json = 1.19.0，conf 用 `version: null` 自动派生）、生产 CSP 良好、macOS cocoa 块正确限定在主线程、ExitRequested 已接 SSH/终端清理。真正的三个问题：**两个无校验的任意文件读取 IPC 命令**（`read_file_preview`/`get_file_size`，文档注释谎称有目录限制）、**capability 对全部窗口通配授权**、**退出清理遗漏 TunnelManager 且在主线程做阻塞清理**。

**优点**：版本单一来源符合 AGENTS.md；CSP `script-src 'self'` 无 unsafe-eval、`object-src 'none'`、`base-uri 'self'`，devCsp 单独放宽不泄漏到 release；`read_file_preview` 已限 1024 字节上限；cocoa unsafe 代码 cfg-gated 且限定主线程；`try_state` 取管理器无 panic；无 `withGlobalTauri`。

| 严重度 | 位置 | 问题 | 建议 |
|--------|------|------|------|
| **Major** | system.rs 58-81 | **`read_file_preview` 零路径校验**：文档声称"限于常见用户目录下"，代码直接 `Path::new(&path)+is_file()+File::open`，无白名单/canonicalize/符号链接解析。Tauri v2 应用自定义 invoke 命令不受 capability ACL 限制，任意 XSS/未来调用方可外泄 `~/.ssh/id_*`、`.env` 等文件前 1KB。当前前端无调用方（死代码）但注释是陷阱 | 删除命令，或实现 canonicalize + 基目录前缀白名单校验 + TOCTOU 复检，并修正注释 |
| **Major** | capabilities/default.json 5, 17-19 | **`"windows": ["main", "*"]` 通配符过度授权**：所有窗口（含运行时创建的 file-manager-* 子窗口，open-file-manager.ts:44）获得全套 core:window/window 创建销毁权限；被攻破的子窗口可再开/关窗口 | 用 glob 标签精确作用域（如 `file-manager-*`），并按子窗口最小权限拆分 capability 文件 |
| Minor | lib.rs 152-161 | ExitRequested **遗漏 TunnelManager** 清理，且 `disconnect_all()` 在主线程同步执行网络/PTY 拆除（对端无响应时拖住退出）；`RunEvent::Exit` 未兜底 | 补 TunnelManager stop_all；清理限时或挪到后台任务；补 Exit 兜底 |
| Minor | lib.rs 32-35 | `db::init_db()` 失败仅写日志，setup 继续返回 Ok → 应用"带病运行"，每个操作都在报"未初始化" | setup 返回 Err 提示启动失败，或发 in-app 错误事件 |
| Minor | system.rs 84-87 | `get_file_size` 对目录也成功（metadata 不过滤）、同样零校验、返回 `serde_json::Value` 包装 | 同 read_file_preview 修复 + is_file 检查 + 返回类型化 u64 |
| Minor | system.rs 64, 73-81 | async 命令里直接做阻塞文件 I/O 占用 tokio 工作线程；is_file→open 有 TOCTOU 窗口 | 去掉 async 或包 spawn_blocking；对 canonicalize 后路径复检 |
| Minor | Cargo.toml 6 | `edition = 2024` 无 rust-toolchain.toml 固定，老工具链报"requires rustc 1.85" | 加 rust-toolchain.toml |
| Minor | lib.rs 152-161 | **主窗口关闭而子文件管理器窗口仍开时应用不退出**：Tauri 仅在最后窗口关闭时退出，若 main 先关，ExitRequested 不触发，SSH/隧道资源悬挂且无可见 UI | WindowEvent 跟踪窗口生命周期：main 关闭时先关子窗口或确认 |
| Nit | lib.rs 45, 56-62 | macOS 阴影设置自相矛盾（set_shadow(true) 后 setHasShadow_(NO) 又 (YES)），且错误全部 `let _` 吞掉 | 保留有效调用并注释动机；记录错误 |
| Nit | system.rs 36-56 | toggle_maximize/minimize_window/close_window 声明 async 却无 await | 去掉 async |

**低置信度**：Tauri v2 自定义命令不受 ACL 限制 → 任意 XSS 可直接调 `get_session_credentials` 拿解密凭据（模块级威胁建模建议）；`file-manager-*` glob 语义需对照 tauri 2.11 文档确认；NO→YES 阴影对可能是有意的重绘 workaround；主窗口先关的退出行为需在 macOS 实测。

---

## 前端模块

### F6. 服务器仪表盘与 SSH 连接表单（`ServerDashboard.vue` + `SSHConnectionForm.vue`，约 1600 行）

**总体评价**：仪表盘是纯事件驱动（自身无轮询），正常路径监听回收正确，远端数据全部走 Vue 文本插值（无 v-html，无 XSS）；表单是干净的 v-model + 校验封装，事件总线清理对称，零 i18n 违规。主要风险集中在异步 listener 生命周期竞态、编辑时空密码覆盖已存密文凭据、以及取消时回滚会删除已被已保存会话引用的分组/标签。

**优点**：`listen()` 的 UnlistenFn 在 onUnmounted 正确注销；CLOSE_DIALOG 事件总线全程对称注册/移除；所有远端数据用文本插值渲染；密码输入 type=password + 显式显隐切换 + aria-label；历史缓冲有界（MAX_HISTORY=60 + shift）；传输暂停/恢复/取消 emit/v-model 契约清晰。

| 严重度 | 位置 | 问题 | 建议 |
|--------|------|------|------|
| **Major** | ServerDashboard.vue 255-290 | **异步 listen() 竞态泄漏重复 listener**：`await listen(...)` 后才给模块级 `unlisten` 赋值；sessionId 切换/提前卸载时 unlisten 仍为 null → 清理不执行，两次并发 setupListener 都解析后二次覆盖 unlisten，旧会话 listener 永久存活（历史重复追加、图表 2×MAX_HISTORY、卸载后仍在处理事件）；listen() 失败是未处理的 promise rejection | 代数/销毁标志保护：resolve 后 `if (disposed || gen !== currentGen) { u(); return; }`，unmounted 时 gen++；await 包 try/catch |
| **Major** | SSHConnectionForm.vue 493-525, 331-333, 364-369 | **编辑时空密码静默覆盖已存凭据**：onSubmit/onSaveOnly 原样展开 formData，password/key_passphrase 恒存在（默认 ''）；watch 只真值拷贝 → 编辑未重输密码的会话时提交空串，若父组件无条件写入则 AES-GCM 密文被替换为空（"空密码抹掉已存凭据"数据丢失） | 编辑模式剥离空密码字段（后端保留原密文），提供显式"清除已存凭据"操作；父组件仅在非空时更新密文 |
| **Major** | SSHConnectionForm.vue 432-440, 547-569 | **取消回滚删除已保存的分组/标签**：新建的 group/tag 记录在新建列表，取消时一律硬删，但成功提交后 `newlyCreated*` 从未清空 → 弹窗在保存后仍驻留（save-only 保持打开）时取消，会删掉已被会话引用的元数据 | 保存成功后清空新建列表；回滚职责上移父组件按持久化结果决定 |
| Minor | SSHConnectionForm.vue 60-67, 473-476 | 端口清空显示校验错误而非默认 22（`''` 过 `!== null` 检查、`'' < 1` 触发报错；类型 number|null 与实际可能 '' 不符） | 校验前归一化 ''→null |
| Minor | ServerDashboard.vue 257-294 | 面板隐藏时 listener/图表计算仍全量运行（is-hidden 纯 CSS）；无客户端节流 | 按 show 门控订阅与重算；加最小间隔节流 |
| Minor | SSHConnectionForm.vue 99-149 | 凭据输入框缺 autocomplete/spellcheck 属性，WebView 密码管理器可能已编辑会话时自动填充/留存密码 | 密码域 autocomplete="new-password"、用户名字段 off/username、spellcheck=false |
| Nit | SSHConnectionForm.vue 274, 310, 780-790 | 死代码：errorMessage prop 声明未用、.form-general-error CSS 未用 | 删除或真正渲染 |
| Nit | SSHConnectionForm.vue 487-537 | onSubmit/onSaveOnly 重复实现提交数据构建，修补凭据逻辑需两处同步 | 抽取 buildSubmitData() |
| Nit | SSHConnectionForm.vue 583-625 | 自定义 Tab 处理硬编码元素列表（漏 save-only 按钮与多选组件）并循环聚焦（焦点陷阱），破坏原生 tab 序与读屏器 | 移除或基于实时 focusable 查询重建 |
| Nit | ServerDashboard.vue 170-178 | 图表 x 步长按 MAX_HISTORY 计算，样本少时全部挤在右缘 | 按实际长度算 step |

**低置信度**：latestStatus.loadAvg[0..2] 无长度保护（366-372，后端空数组时渲染抛错）；deep watch 可能在用户输入时被父组件 mutation 覆盖（345-373）；"空密码覆盖"实际影响取决于父组件 save 处理器是否无条件写；切换事件/受控模式时旧 listener 未注销。

---

### F5. 远程连接主视图（`RemoteConnectionView.vue`（1991 行）+ `ConnectionProgressBar.vue`（689 行））

**总体评价**：异常自律的 xterm 宿主 — listener 清理、KeepAlive 激活处理、实时流与缓冲 banner 之间的 seq 去重都处理得仔细，无 v-html/XSS 面。主要缺口在 UX/错误处理（连接失败与断线后状态只记日志、组件内无重连路径）、跨卸载/会话切换的异步任务缺少取消/代数保护、以及大量硬编码英文用户可见字符串（外加一处中文注释）违反 i18n 约定。未发现 critical 级问题。

**优点**：所有 Tauri 事件 listener 登记在模块级并在 cleanupResources 中 await 注销，onActivated/onDeactivated 正确限定 resize 与状态轮询范围；seq 去重防止 MOTD 双写；无 v-html；上传前确认对话框 + 路径归一化 + 乐观任务更新；状态历史封顶 60 + 图表仅在可见时重建 + 自适应 700ms/3000ms 刷新 + WebGL 渲染。

| 严重度 | 位置 | 问题 | 建议 |
|--------|------|------|------|
| **Major** | RemoteConnectionView.vue 690-693, 818-820（模板 1418-1578） | **连接失败被吞掉，无用户可见错误、无重试**：connectSession 抛错但 connectToSession 只 logger.error；模板无错误横幅 → 密码错/拒连/超时只留下空白终端，且 onData 本地回显分支（1407-1410）让用户"看似在输入" | 本地错误 ref 渲染横幅 + 重试按钮；连接失败/未连接时抑制本地回显 |
| **Major** | RemoteConnectionView.vue 632-694, 766-781, 824-831 | **自发断线后无重连路径**：ssh-disconnected 只标记状态并提示"重开标签页"，但 connectSession 仅在 `sessionStore.hasSession()==false` 时创建会话（640）；同一 id 的 "disconnected" 会话重开标签页不会重建 SSH 通道，按键继续走 isDead 本地回显（1396-1410）。780 行注释声称可重连，但模块内无对应代码路径 | 提供强制重建会话的重试/重连动作（force 标志或先移除存储条目），或标签层先清理死会话再挂载 |
| **Major** | RemoteConnectionView.vue 571-572, 684-688, 985-991 | **缓冲输出轮询可写入已 dispose 的终端且卸载后仍运行**：cleanupResources 调 terminal.dispose() 但**从不置 terminal=null**，572 行 `!terminal` 守卫失效；connectSession 循环在卸载/切会话后仍最多轮询 14×200ms≈2.8s，对已销毁 xterm 调 write 抛异常（仅被外层 catch 顺带吞掉） | dispose 后置 null；代数计数器在卸载/切会话时自增，轮询与 listener 检查之 |
| Minor | RemoteConnectionView.vue 1391-1411 | **按键级异步输入**：每次 onData await emit + 响应式 getSession 查询；快速输入/粘贴/IME 时并发 emit 无顺序保障，后端可能收到乱序输入块 | FIFO 队列串行化（fire-and-forget 或链式）；连接期缓存 session 引用；拆除期跳过 emit |
| Minor | RemoteConnectionView.vue 700-704, 1362-1385, 919-937 | **resize 处理**：window resize 的 fit() 无 try/catch（ResizeObserver 分支有）；每次尺寸变化同步 emit ssh-resize；onActivated 重复 refit+emit；隐藏 KeepAlive 标签 0×0 时会 emits 0 行列的伪造 resize | fit 包 try/catch、rAF/定时器节流、0 或未变时不 emit、激活时仅在尺寸真变化时 refit |
| Minor | RemoteConnectionView.vue 51-72, 1039, 934 | **statusUnlisten 双重注册竞态**：setupStatusListener 在 initialize 与 onActivated 并发调用，`await listen()` 后才赋值 → 重叠调用注册两个 listener 但只存最后一个，前一个泄漏且状态事件双份 | 记录 in-flight 注册 promise，前一个 settle 后再注册，期间先注销旧 listener |
| Minor | RemoteConnectionView.vue 824-831, 1037-1039, 934 | **sessionId 变化不重绑状态监听**：watcher 重注册 output/disconnect 监听却从不调 setupStatusListener（只在 initialize/onActivated 跑）→ 复用组件实例换会话时仍听 `ssh-status-{旧id}`，新会话仪表盘数据过期 | sessionId watcher 内补 setupStatusListener() |
| Minor | RemoteConnectionView.vue 486-514 | **cancelUploadTask 先删后查**：491 行 removeUploadTask 后 499 行 `find` 恒 undefined，回填的错误条目丢失原文件名/方向（错误文案通用化，下载取消路由也受影响） | 先 `const previous = find(...)` 再乐观删除，回填用捕获值 |
| Minor | RemoteConnectionView.vue 151, 379, 408, 428, 1502 + ConnectionProgressBar.vue 全组件 | **硬编码英文用户可见字符串绕过 vue-i18n**：'Preparing...'、'Failed to start: ...'、'Close'、'Total time:'、Retry/Close/Cancel、默认 steps、'Connection Failed' 等都是字面量，中文 locale 下始终英文 | 全部走 t() 键并扩展 en/zh 词条 |
| Minor | RemoteConnectionView.vue 1683 | **前端代码出现中文注释**（CSS 注释），违反仓库 no-Chinese 约定（唯一一处） | 翻译为英文注释 |
| Nit | ConnectionProgressBar.vue 174, 297-357, 359-366, 412-414, 593-601 | `:class` 恒等式三元、.progress-container 重复声明、未引用的 @keyframes/.spinner-ring/.success-icon-wrapper | 清理冗余与死 CSS |
| Nit | RemoteConnectionView.vue 302-337 | handleFileDrop 注释声称"立即显示仪表盘并切到上传页"，实际只开确认浮层（showDashboard 在 confirmUploads 才置位） | 按注释实现或改注释 |

**低置信度**：seq 去重假设后端严格按序且每次连接重置计数器（585-589、729-731 用 `lastSeq = payload.seq` 而非 max，乱序/未重置会丢/重放内容 — 需对后端验证）；initialize 与早卸载竞态时 1042-1115 的 drop 监听捕获旧 sessionId 且 1117 行提前 return 后监听已注册（无终端创建）；0×0 隐藏标签 fit 出 0 行列 resize 后端是否容忍。

---

### F8. 设置面板与多选组件（`SettingsPanel.vue` + `MultiSelect.vue` + `DropdownMenu.vue` + `Groups/Tags/MetadataMultiSelect.vue`，约 2200 行）

**总体评价**：结构总体良好 — 持久化闭环（主题/强调色走 themeManager、终端走 Pinia store）经核实正确、所有插值均被 Vue 转义、外链带 `rel="noopener noreferrer"`。最严重缺陷是 **MultiSelect.vue 在空下拉菜单下可用键盘触发崩溃**（模零运算把 highlightedIndex 变 NaN，Enter 解引用 undefined）。次要有：更新检查可能在 getVersion 解析前拿默认版本 '0.1.0' 对比；DropdownMenu 用 setTimeout(0) 装 document 点击监听，可越过关闭/卸载存活并泄漏；多选组件不转发 itemType 导致所有实例渲染相同 DOM id，破坏 label 关联。

**优点**：主题/强调色持久化带校验与 'auto' 回退；无 XSS（动态字符串全部 mustache 转义）；prop/emit 类型化良好，wrapper 正确转发 model 与新增项事件，MetadataMultiSelect 把后端错误抛给 MultiSelect 日志路径；DropdownMenu/MultiSelect 卸载时清定时器，SettingsPanel 断开 IntersectionObserver；新建 group/tag 后发 GROUPS_UPDATED/TAGS_UPDATED 事件刷新依赖列表。

| 严重度 | 位置 | 问题 | 建议 |
|--------|------|------|------|
| **Major** | MultiSelect.vue 288-317 | **空下拉菜单可键盘触发崩溃**：navigationItems 为空时 ArrowDown/Up 执行 `(highlightedIndex+1) % 0` = NaN；Enter 时 `NaN !== -1` 为真 → `navigationItems[NaN]` 为 undefined → `item.id` 抛 TypeError（首次使用、空分组/标签列表的用户可稳定复现）；此后 Arrow 键一直 NaN，直到鼠标事件重置 | 仅当 `len > 0` 时才循环取模；Enter 分支加 `navigationItems.length > 0` 短路 |
| Minor | SettingsPanel.vue 355, 403-416, 606-610 | **更新检查可能拿默认版本 '0.1.0' 对比**：appVersion 初始 '0.1.0'，onMounted 异步 getVersion 才替换；在解析完成前/无 Tauri 的 dev 构建中，checkForUpdates('0.1.0') 几乎必报"有更新"并提供误下载；About 区也闪现 0.1.0 | versionReady 标志门控检查按钮；catch 时禁用并告警 |
| Minor | DropdownMenu.vue 173-184, 201-207 | **click-outside 监听 setTimeout(0) 后装**：若 timeout 触发前 visible 已关闭/卸载，else 分支/unmount 移除的是"从未添加"的监听，随后 timeout 装上永不移除的 document 监听 → 每次点击发多余 update:visible=false 事件 + 闭包泄漏 | 保存 timeout id，else/unmount 一并清理；timeout 内先查 `props.visible` |
| Minor | MultiSelect.vue 198-209, 260-276 | **一次性 internalUpdate 标志可吞掉合法外部更新**：父组件不回显 emit 值（归一化/防抖）时标志残留，下一次真实外部变更被 `if (internalUpdate)` 分支丢弃 → selectedItems 与 v-model 脱同步（当前仅因 SSHConnectionForm 同步回显才正常） | 改基于值的同步（相等则跳过赋值），或 emit 后同步清标志 |
| Minor | SettingsPanel.vue 4-14 | **设置弹窗无 dialog 语义/焦点管理**：纯 div、无 role/aria-modal/aria-labelledby、无焦点陷阱/恢复，键盘可 tab 到遮罩后 | 补 role=dialog + aria-modal + 打开聚焦 + Tab 陷阱 + 关闭恢复焦点 |
| Minor | MultiSelect.vue 39；三个 wrapper 都不转发 | **重复 DOM id**：itemType 从不被调用方传入，所有实例都渲染 id="item-select"（SSHConnectionForm 同页两个多选 → 两个重复 id，label 全部关联第一个，点击"分组"可能聚焦到"标签"输入框） | 用 Vue 3.5 useId() 或 label 包裹 input，去掉 for/id 耦合 |
| Minor | SettingsPanel.vue 494-553 | **scroll-spy 可能高亮错区 + 关闭时不拆除 observer**：rootMargin 窄带可同时包含两节，回调按观察顺序 last-wins 可能选中下部；面板关闭时 observer 仍观察已脱离元素 | 取最靠上的相交节（比较 boundingClientRect().top）；visible=false 分支也 disconnect |
| Minor | SettingsPanel.vue 357-416 | **更新检查失败全部折叠为一条通用消息**：离线/403 限流/JSON 解析失败不分；且错误/最新时 latestVersion/releaseUrl 残留旧值 | updater.ts 区分 403/429 reason；非 available 时重置 |
| Minor | MetadataMultiSelect.vue 57-91 | **后端建项失败对用户静默**：invoke('add_group'/'add_tag') 失败只被 MultiSelect 记日志，下拉保持打开，用户不知未创建未选中 | 增加 on-create-error 通道/内联错误提示 |
| Nit | DropdownMenu.vue 30, 37 | trigger prop 是死代码（从未被读，无调用方传） | 删除或真正实现 |
| Nit | SettingsPanel.vue 918-921, 923-928 等 | 多条死 CSS（空规则/.setting-checkbox/.about-info strong 无匹配元素） | 删除 |
| Nit | SettingsPanel.vue 2, 418-426 | **Teleport 分支实际是死路径**：唯一调用方 App.vue 传 `:use-teleport="false"`，contentRef 分支从未进入 | 删除 prop 与分支或注明消费者 |
| Nit | MultiSelect.vue 232-241 | navigationItems 双重类型转换写法绕弯（隐藏了对象字面量按 T 类型）；下拉无 listbox 语义（无 aria-expanded/role=option） | 简化为普通 computed；补 combobox/listbox ARIA |

**低置信度**：scroll-spy 双节相交时 last-wins 是否必然错选（回调顺序无保证）；blur 后 120ms 内重新聚焦时关闭定时器竞态；releaseUrl 锚点 href 若被 GitHub API 响应污染可注入非 https 协议（当下可信源）；v-if 首次挂载 visible=true 的启动路径未触发 initialSection 逻辑（App 恒 false）；divider item 若无稳定 key 产生 Vue key 警告；`/welcome-image.png` 打包路径是否 404 未验证。审查重点项说明：这些组件**没有** check-all/全选复选框实现（任务清单中的"check-all/indeterminate"在该文件集中不存在）。

---

### F7. SFTP 浏览器 / 独立文件管理器窗口 / 隧道面板（`SftpBrowser.vue` + `FileManagerWindow.vue` + `TunnelManagerPanel.vue`，约 2200 行）

**总体评价**：结构总体良好 — listener 全部清理、传输队列与会话事件接线正确、跨窗口通过窗口标签（`file-manager-{sid}`）隔离会话、i18n 合规干净。主要问题集中在**静默数据丢失路径**（上传无存在性检查直接覆盖远端文件；删除确认文案承诺递归删除而后端只能删空目录）、mkdir/rename **缺少路径段净化**（`..`/分隔符可逃逸当前目录）、隧道面板无 host/port 冲突校验，以及若干状态竞态（错误横幅不消失、列表在上传开始时而非完成时刷新、乐观 cancel/pause 与后端脱同步）。未发现 critical 与凭据泄露。

**优点**：listener 全部用 UnlistenFn 存储并在 onUnmounted 可靠清理；会话隔离经窗口标签正确实现；列表导航用单调请求序号 + 15s watchdog 防旧请求覆盖新状态；路径归一化（`/C:/`、`C:\`）前后端一致；全部 UI 文案走 vue-i18n 且键存在；隧道面板关闭/卸载时停 2s 轮询，删除规则先停后删。

| 严重度 | 位置 | 问题 | 建议 |
|--------|------|------|------|
| **Major** | FileManagerWindow.vue 133-145（后端 ssh.rs 1304-1310） | **上传无声覆盖已存在远端文件**：目录列表有删除等破坏性操作确认，但上传对同名文件不检查不确认，后端 `WRITE\|CREATE\|TRUNCATE` 直接替换 | 上传前远端 stat 或后端返回 exists → confirm-overwrite 对话框；或任务级 overwrite 开关 |
| **Major** | SftpBrowser.vue 112-122, 160-170（use-sftp.ts 106-112；后端 ssh.rs 2024-2026） | **mkdir/rename 名称不净化**：`../x`、`/abs/path` 直接拼路径，重命名可跨目录移动、mkdir 可在当前目录外建目录，操作静默成功在用户看不到的地方 | 拒绝含 `/`、`\`、`..` 段、空名的输入，本地化内联报错 |
| **Major** | SftpBrowser.vue 147-153（en.ts 245；后端 ssh.rs 1962-1968） | **删除确认承诺递归删除，后端只能 rmdir 空目录**：删非空目录必失败且报裸后端错误，用户被承诺"及其内容"却永远失败 | 后端实现递归删除（遍历 unlink/rmdir），或限制仅空目录可删并改文案 |
| **Major** | TunnelManagerPanel.vue 301-356 | **无监听冲突校验 + 自动启动失败被吞**：新增规则只校验端口范围，不查同 host:port 冲突（重复规则持久化后两个开关操作同一端口）；add 后自动启动失败仅 logger.warn；listenHost 未 trim（'   ' 为真值绕过 127.0.0.1 兜底） | 新增前比对现有规则拒绝重复；启动失败回显到表单错误；trim 并校验 host |
| Minor | FileManagerWindow.vue 123-145 | **目录列表在上传开始时刷新而非完成时**：startUpload 仅等任务 spawn，大文件传输完成前刷新看不到最终文件，后续无补刷 | 监听任务 success 状态再刷新 |
| Minor | SftpBrowser.vue 95, 112-122, 147-170 | **actionError 永不清理**：任何一次失败的 mkdir/rename/delete 后红色横幅永久显示，后续成功操作也不消失，还遮蔽 sftp.error | 每个操作开头清空 + 导航成功时清空 |
| Minor | FileManagerWindow.vue 77-79, 379-443 | **断线时在途传输永不转错误态**：ssh-disconnected 只隐藏布局，传输任务卡在 uploading/downloading 永久转圈且无法取消（cancel 对死会话失败 → 乐观移除又被 unknown-task 分支复活） | 断开时把所有非终态任务标 error 并触发刷新 |
| Minor | FileManagerWindow.vue 434-441（use-transfer-queue.ts 189-202） | **cancel 即使后端失败也移除条目**：UI 消失而后端仍在写，迟到的进度事件又复活为 uploading | 仅在后端确认成功时移除（或本地标 cancelled 等终态确认） |
| Minor | SftpBrowser.vue 185-187（use-sftp.ts 145-149） | **15s 列表 watchdog 在 dispose 后不取消**：关窗后 setTimeout 闭包继续改死组件的 loading/entries 最多 15s | dispose 里 clearTimeout |
| Minor | SftpBrowser.vue 106-110 | **目录符号链接无法进入且无提示**：isDir=false 被 enterPath 直接跳过（isSymlink 无处理），链接行全部是死行 | isSymlink 时也尝试 navigate，失败再报错 |
| Minor | SftpBrowser.vue 177-183 | **死代码：probePlatform() 每次挂载多一次 IPC，platform 从未被消费** | 删除或真正使用 |
| Minor | FileManagerWindow.vue 133-145, 185-196 | **上传目标路径在事件时抓取，可与在途导航竞态**：拖放/选择发生在导航完成前会上传到旧目录 | 等 pending 导航完成再取 base，或 loading 时禁用上传 |
| Minor | TunnelManagerPanel.vue 396-406 | visible watcher 无 immediate：挂载即可见时列表为空且无轮询 | 加 immediate: true 或 onMounted 兜底 |
| Minor | TunnelManagerPanel.vue 262-266, 301-346 | **后端隧道错误被静默转成"已停止"**：API 层错误返回 []，无法启动的规则（权限/无效 host/防火墙）显示为普通 Stopped，用户反复点启动 | 错误状态进 statuses，start/stop 失败回显到规则 statusError |
| Nit | FileManagerWindow.vue 390-392 | 'Transfer' 字面量兜底 + 种子消息英文 + task.message 从不渲染 | t() 兜底；渲染或删除 message |

**低置信度**：失败的 cancel 后迟到的进度事件是否真复活任务取决于后端是否继续发事件（需集成测试）；Tauri 事件默认 app-wide — 主窗口启动的传输也会出现在文件管理器窗口队列（需确认是否预期）；符号链接目录 readdir 在部分服务器报 SSH_FX_NO_SUCH_FILE（建议修复需错误回退）；'starting' 状态在 2s 轮询间隔内短暂显示 Stopped 徽标；SftpBrowser 的 sessionIdRef 一旦初始化不再同步（当前单消费者无碍）。

---

### F4. 首页仪表盘 / 搜索 / 命令面板（`NexaShellHome.vue` + `SearchBox.vue` + `SearchDropdown.vue` + `CommandPalette.vue`，约 3000 行）

**总体评价**：整体干净 — 无 v-html/XSS 面、事件监听与 onUnmounted 配对正确、computed 过滤整洁、零中文违规。主要问题集中在搜索下拉的**动作正确性**（'Command Palette' 结果打开的是设置；四个广告动作派发的 CustomEvent 无人监听）和删除当前选中分组/标签后的**陈旧 UI/过滤视图破损**，此外还有一批键盘与异步竞态边界（陈旧 activeIndex、选中后查询词不清空、打开时会话抓取竞态）。

**优点**：四个文件全部无 v-html；NexaShellHome 3 个 eventBus 处理器注册/移除配对；SearchDropdown 移除 document click/resize 监听与复制 toast 定时器；所有 $t 键在 en/zh 双 locale 存在；复制连接信息在凭据无法解密时优雅降级且从不打印密码；会话删除走本地化 ConfirmDialog。

| 严重度 | 位置 | 问题 | 建议 |
|--------|------|------|------|
| **Major** | SearchDropdown.vue 232-240 | **'Command Palette' 搜索结果打开的是设置页**：item id 'command-palette' 派发 OPEN_SETTINGS（shortcuts 节），而真实调色板用 APP_EVENTS.COMMAND_PALETTE（shortcut-manager.ts:278-288） | 改为 emit(APP_EVENTS.COMMAND_PALETTE) |
| **Major** | SearchDropdown.vue 229, 242-266 | **四个搜索动作派发无人监听的 CustomEvent（静默空操作）**：'app:open-help'、'app:open-file-manager'、'app:open-web-terminal'、'app:open-code-editor' 全仓无 addEventListener 订阅；选择后关下拉、什么都不发生 | 接真实处理器（如 openFileManagerWindow）或删除未实现项 |
| **Major** | NexaShellHome.vue 600-615, 873-891 | **删除分组/标签不重载会话**：本地更新 groups/tags 但不 loadSessions() 也不发更新事件；若删除的正是当前选中项，filteredSessions 的 `find` 失败走兜底标题（'Group'/'Tag'）→ **静默展示全部会话**，行内还残留已删名称 | 选中项被删时重置 selected* 与 activeView；补 loadSessions() 与更新事件 |
| **Major** | NexaShellHome.vue 873-891（CommandPalette.vue 276-283） | **分组/标签删除无任何确认**：与 会话删除走 ConfirmDialog（1139-1163）不同，分组/标签一键级联删除所有会话的关联（后端级联注释），snippet 删除同样无确认 | 复用 ConfirmDialog，文案点名资源 |
| **Major** | SearchDropdown.vue 124-130, 269-365, 367-410 | **查询变化不重置 activeIndex，Enter 可能执行错误项**：filteredItems 每次按键重算但 activeIndex 仅在 visible watch 初始化/方向键推进；列表收缩/重排后高亮落在错误行上（CommandPalette 有 `@input="highlightedIndex = 0"`，SearchDropdown 没有）；越界时 Enter 静默空操作 | watch(searchQuery) 重置为首个可选索引；handleKeyDown 内 clamp |
| Minor | SearchDropdown.vue 123-129, 172-173, 428-435 | **query 清理写入本地死状态**：WindowTitleBar 总传 `:search-query`，computed 恒解析到父值，localSearchQuery 是死代码；selectItem 里 `searchQuery.value=''` 是空操作 → 选中后输入框保留旧词，重开下拉直接显示旧结果 | emits 增加 update:searchQuery 把清空事件传回父组件 |
| Minor | SearchDropdown.vue 135-142, 158-176 | **会话抓取竞态 + 加载前算好的 activeIndex**：每次打开 fire-and-forget 无序号保护，快速开合时旧响应覆盖新响应；nextTick 固定的 activeIndex 在会话行追加后错位 | 单调 token 丢弃过期结果；会话赋值后再算 activeIndex |
| Minor | SearchDropdown.vue 275-286 | **最近会话排序在 WebKit 上失效**：SQLite `CURRENT_TIMESTAMP`（'YYYY-MM-DD HH:MM:SS'）Chromium 能 parse，WKWebView 返回 Invalid Date → 排序退化为不稳定插入序（NexaShellHome 590-597 有 normalize 而这里没有） | 抽公共 normalize（空格→T + 尾 Z）两处共用 |
| Minor | NexaShellHome.vue 256-261, 761-772 | **空状态混淆"无会话"与"当前视图无匹配"**，且首屏加载时闪现（无 loading 标志） | loading ref + 区分两类空态 |
| Minor | NexaShellHome.vue 176-205, 977-1000 | **标签重命名编辑器无 blur 处理**：点了别处编辑态悬挂，未按确认的修改静默丢弃（分组输入有 @blur 保存，标签没有） | 补 @blur 保存或明确取消 |
| Minor | NexaShellHome.vue 269-274, 518, 749-758 | **表头全选复选框从不反映选中态；selectedSessionIds 切换视图/删除后从不清理**（跨视图泄漏） | 绑定全选 computed（checked/indeterminate）；视图切换/删除时重置 |
| Minor | CommandPalette.vue 226-237 | **无活动 SSH 会话时 snippet 执行静默空操作**：无 toast 且面板保持打开；activeSessionHint 只在打开时刷新，面板开着切标签页时页脚提示与实际执行目标不一致 | 失败时向用户提示或禁用 snippet；聚焦/标签变化时刷新提示 |
| Minor | SearchDropdown.vue 414-426 | scrollToActiveItem 用 `item.scrollIntoView` 滚动整个文档（teleport 到 body 的下拉），方向键导航时背景页面跳动 | 计算容器 rect 改 container.scrollTop |
| Nit | NexaShellHome.vue 58, 66, 74, 150, 158, 571, 574, 398 | 硬编码英文占位/标题/兜底 + `common.more` 无 locale 键（永远回退 'More'） | 补 locale 键 |
| Nit | NexaShellHome.vue 293-295, 818-821 | handleConnect 是空操作；拖拽把手是纯装饰（无重排逻辑）——暗示了不存在的功能 | 实现行点击选择或删注释；隐藏拖拽把手 |
| Nit | CommandPalette.vue 285-296 | visible watch 无 immediate，首次挂载即可见时初始化不运行 | 加 immediate |

**低置信度**：activeSshSessionId 把 panes 最后一格当活动格（167-173），若 panes 非按激活排序会瞄错目标；groupCounts/tagCounts 按 name 做键，若后端允许同名分组计数合并出错；组重命名 blur 即保存而新增输入 blur 丢弃 — 意图可能不一致；分组计数仅 >0 显示与会话计数恒显示不一致（可能是有意）。

---

### F2. 核心工具层（`core/utils/*` + `core/constants/*` + `core/config/*` + `core/types/*` + `core/i18n/index.ts` + 两个入口，约 2000 行）

**总体评价**：整体结构良好 — 事件总线带真正的防泄漏设计（跟踪包装、按处理器退订、重注册移除旧窗口监听）、主题持久化防御式编程、更新器/版本模块设计用心。两个高影响缺陷在**全局快捷键管理器**：终端聚焦时全局组合键仍触发（Ctrl+Q 退出应用、Ctrl+D 切分屏同时又发 EOF）；Tab 被全局 preventDefault（搜索下拉之外全部吞掉），破坏原生表单/键盘导航。此外 **logger 原样存储/导出 `data` 无脱敏**，任何记录连接载荷的调用方都是凭据泄露风险点。

**优点**：事件总线防泄漏（event-bus.ts:42-71）；主题持久化正确遵循 OS prefers-color-scheme 仅限 auto 模式；更新器处理 GitHub draft release 有 tags 兜底；terminal-input-fix 严格限定 Mac WebKit 并返回 disposer；入口极简；CJK 扫描仅命中允许的 locale 文件（零 i18n 违规）。

| 严重度 | 位置 | 问题 | 建议 |
|--------|------|------|------|
| **Major** | shortcut-manager.ts 95-118（配合 App.vue 172-181、RemoteConnectionView.vue 1274-1299） | **终端聚焦时全局快捷键仍触发**：xterm 辅助元素是真实 TEXTAREA，isGlobalShortcut 分支命中；xterm 的 key handler 有意放行纯 Ctrl+Q/Ctrl+D 但事件仍冒泡到 window 监听 → **Ctrl+Q 既发 XOFF 又退出整个应用，Ctrl+D 既发 EOF 又切分屏**，作者"纯 Ctrl+D 必须透传给 shell"的意图被全局 manager 击穿 | 终端容器加焦点标志（data-terminal-focused）或 `event.defaultPrevented && closest('.xterm')` 时跳过快捷键分发 |
| **Major** | shortcut-manager.ts 100-129 | **Tab 全局 preventDefault（搜索下拉之外全部）**：与注释"允许搜索区 Tab"正相反，设置面板与所有表单的原生 Tab 导航全部失灵（SSH 表单靠自己的 handleTabKey 补偿，其余无） | 仅在有实际动作时拦截 Tab，其余直接 return 不 preventDefault |
| **Major** | logger.ts 102-133, 197-220, 308-350 | **logger 原样存储/导出 `data` 无脱敏**：历史可经 getAllHistory/exportAllAsJSON/CSV/downloadLogs/`window.__LOGGER_MANAGER__` 取走/下载/检索；生产环境 INFO+ 也打印到 console；CSV 转义不完整且公式前导单元格（=,+,-,@）未防护 | writeLog 增加敏感键（password/passphrase/secret/token/privateKey/authorization/cookie）脱敏；CSV 转义 + 公式前缀单引号 |
| Minor | logger-devtools.ts 61-70 | `__logger__.search()` 对无 data 的日志抛 TypeError（JSON.stringify(undefined) 后 .toLowerCase()） | `(entry.data === undefined ? '' : JSON.stringify(entry.data)).toLowerCase()` |
| Minor | event-bus.ts 10, 44-52 + main.ts | **异步订阅者 rejection 无人处理**：wrappedHandler 的 try/catch 只覆盖同步抛错；main.ts 未装 unhandledrejection/unhandlederror 全局处理器，生产环境静默丢失 | Promise.resolve(...).catch(console.error)；main.ts 注册全局 handler |
| Minor | updater.ts 44-51, 79-93 | **更新检查无超时、tags 兜底不处理分页**：离线/慢网络下 'check for updates' 永久转圈；tags 默认第一页 100 条且 GitHub 不保证版本排序 | AbortController ~10s 超时；显式 per_page 或处理分页 |
| Minor | version.ts 50-56 | **非数字预发布标识符比较相等**：`1.0.0-beta.1` vs `1.0.0-alpha.1` 返回 0（semver 应 beta>alpha） | 数值部分平手时按段比较标识符（数字按数值、字母按字典序）再落最终版规则 |
| Minor | time-utils.ts 36-51 | **相对时间硬编码英文**（'Just now'/'5m ago'），zh 用户看到英文片段；ServerDashboard 用 t('dashboard.ago') 而 NexaShellHome 渲染此处输出 — 两处行为不一致 | 接受 t 函数或返回结构化 {value, unit} |
| Minor | app-utils.ts 23-28 | **废弃 shim 用同步名字导出异步函数**：`isMacOSBrowser = isMacOSImpl` 实为返回 Promise 的 safeInvoke 版本，`if (isMacOSBrowser())` 恒真（死文件但下个消费者是地雷） | 删除或改导同步窗口检测版 |
| Minor | theme-manager.ts 122-144 | **'theme-changed' 事件名不在 APP_EVENTS 且载荷形状不一致**：setTheme 发 {theme}，setAccent 发 {theme, accent}，监听方无法依赖 accent 在场 | 加 THEME_CHANGED 常量，统一发 {theme, accent} |
| Minor | logger.ts 267-274 | **updateConfig 重建所有 ModuleLogger 清空各模块历史**：devtools setLevel/setFilters 后 getAllHistory 空空如也，像神秘数据丢失 | 原地改共享 config 引用 |
| Nit | menu.ts 5-8 | 菜单项硬编码 'Cmd+X' 在 Windows 原样显示（实际绑定是 Ctrl+），formatShortcut 存在未用；formatShortcut 双修饰键时重复 push 'Ctrl' | 用 formatShortcut 派生；去重修饰键 |
| Nit | platform-detection.ts 46-48, 63-65 | **在 typeof window 守卫之前就解引用 window**，守卫形同虚设（非 window 环境直接 ReferenceError） | 调整顺序 |
| Nit | i18n/index.ts 80-86 | **顶层 initLocale() fire-and-forget**：zh 用户可能先见英文闪烁；动态导入失败成未处理 rejection | 导出 initI18n() 供 main.ts await 后挂载 |

**低置信度**：xterm 是否对每个写入的控件键真正 preventDefaults（冲突确定，但推荐排除信号需运行验证）；SettingsPanel 是否依赖原生 Tab（若是，Tab 缺陷在其上立即可见）；`consoleMethod(formattedMsg)` 未绑定 console 在少数引擎抛 Illegal invocation（现代浏览器容忍）；formatRelativeTime 对服务器时钟超前显示 'Just now'（仅展示）。

---

### F3. 应用外壳与布局（`App.vue` + `layout/*` + `TabItem.vue` + `WelcomeScreen.vue` + `ShortcutHint.vue` + `ConfirmDialog.vue`，约 3000 行）

**总体评价**：外壳结构良好 — 清理纪律强（App.vue/AppTabs/WindowTitleBar/SplitRenderer/WelcomeScreen 卸载时全部移除定时器/监听/事件总线）、事件名与 events.ts 零拼写错误、无 v-html、无凭据日志。主要问题集中在连接/标签/布局编排的三个正确性缺陷：**Ctrl+T "新建 SSH"路径绕过表单状态重置包装器**（取消编辑后可能静默覆盖已保存会话）、**切分窗格必然卸载源窗格 RemoteConnectionView**（终端回滚丢失，与代码自身注释矛盾）、**KeepAlive 缓存从不逐出已关闭标签的 xterm 组件**。另有若干 i18n/可访问性缺口与死代码。

**优点**：App.vue onBeforeUnmount 堪称范本（注销全部快捷键/事件总线/三组定时器/contextmenu 监听 + 会话清理）；事件名与 APP_EVENTS 全部一致；连接日志不含密码（只记 host/port/name）；SplitRenderer 正确配对 pointermove/pointerup 并在卸载时清除；PaneContainer 通过注入回调提交拖拽尺寸保持树状态一致；use-tab-management 有 closeTab/closePane 重入保护；RemoteConnectionView KeepAlive 感知（onActivated/onDeactivated 重连状态监听与 resize）。

| 严重度 | 位置 | 问题 | 建议 |
|--------|------|------|------|
| **Major** | App.vue 117-126, 218-221, 362-368, 760-765 | **Ctrl+T 打开 SSH 表单绕过状态重置**：OPEN_SSH_FORM_KEY 包装器会重置 sshErrorMessage/isConnecting/sshFormMode/editingSessionId/savedSSHFormData，但事件总线 handler（218-221）直接调裸 openSSHForm 跳过重置。取消编辑后 Ctrl+T 重开仍处 edit 模式且 editingSessionId 残留 → 用户保存时覆盖之前编辑的会话（AppTabs 注入的 openSSHForm 用了包装器，仅事件路径损坏） | 抽取 resetSSHFormState() 两个路径共用 |
| **Major** | SplitRenderer.ts 105-144, 146-204（PaneContainer.vue 33-36） | **切分窗格必然卸载源窗格 RemoteConnectionView**：PaneContainer 注释称"切分/折叠不卸载、xterm 历史存活"，但 pane→split 时根 div 原地 patch、子 vnode 类型变化（RemoteConnectionView → div/bar）→ 旧组件卸载 terminal.dispose()、同 session 重新挂载，只重放后端短欢迎缓冲，**全部回滚历史丢失** | 单窗格也走递归路径：稳定 key 的容器内嵌 keyed 内层 SplitRenderer，切分只变子节点；加测试验证 |
| **Major** | AppContent.vue 63-72 | **KeepAlive 缓存从不逐出已关闭标签**：`<KeepAlive :max="16">` 关标签时没有任何驱逐（closeTab 只断会话），关闭标签的 PaneContainer→RemoteConnectionView（含 xterm 缓冲/内存/DOM）以 deactivated 状态存活到缓存满 16 或应用退出；长会话终端内存永久滞留 | 订阅标签移除事件并 bumped cache generation（或 include 列表移除），让关闭标签的 vnode 卸载 |
| Minor | ShortcutHint.vue 82-96 | **工具提示永久不可见**：.shortcut-hint 恒 opacity:0，全仓无 hover/:focus 规则让其现形 — 三个按钮的提示是死 UI | 补 hover/focus-within 显隐规则 |
| Minor | AppTabs.vue 195-197, 340, 350 | **死监听：APP_EVENTS.NEW_TAB 全仓从未派发**，handleNewTabShortcut 不可达 | 删除或接线 |
| Minor | AppTabs.vue 398, 402, 412-413 / TabItem.vue 92 / WindowTitleBar.vue 235, 243, 277 | **硬编码英文用户可见字符串（i18n 违规）**：'More options'/'Window Actions'/'Close tab'/Minimize/Maximize/Close aria-label，这些文件不在豁免清单 | 补 locale 键走 t() |
| Minor | ConfirmDialog.vue 45-70 | **默认文案硬编码英文且组件从不 useI18n**：title/confirmText/cancelText 默认 'Confirm'/'Cancel'；当前调用方都传了翻译串，但未来漏传参的调用方会在 zh 环境显示英文 | 组件内用 t('common.confirm'/'common.cancel') 作默认 |
| Minor | ConfirmDialog.vue 1-40, 77-83 | **对话框 a11y 缺口**：无焦点陷阱/无 Escape 关闭/无焦点恢复/无 aria-modal/aria-describedby；遮罩点击直接关闭破坏性确认有疑问 | visible=true 时保存 activeElement 并聚焦面板；Tab 陷阱；Escape→cancel；恢复焦点 |
| Minor | App.vue 382-384 | **卸载时会话清理 fire-and-forget**：onBeforeUnmount 不 await，关窗口时 webview 可能先于 invoke 链拆除，后端收不到清理请求 | 先 await 清理再请求关窗，或后端 drop 时兜底杀会话 |
| Minor | AppTabs.vue 253-260 | **updated_at 日期解析脆弱**：`replace(' ','T')+'Z'` 只认 SQLite 格式，格式一变排序即错（无测试兜底） | 防御性解析 + 针对真实格式的单元测试 |
| Nit | App.vue 446-450 | 1s 间隔内"connecting 超 30s"分支只算条件不做事（死代码） | 删除或实现客户端超时提示 |
| Nit | WelcomeScreen.vue 16-18 | logoSrc 是恒返回常量的无意义 computed；顺带验证 /welcome-image.png 是否打进生产包 | 改普通 const；确认资源打包 |

**低置信度**：WindowTitleBar 的 __unlistenResize 在 await 后赋值，卸载打断会泄漏监听（当前一次性挂载不可达，HMR 可能暴露）；toggleSavedConnections 双击竞态；SSH 表单经进度条 close/retry 关闭时同样不重置 sshFormMode（143-157 同族问题，仅当后续 OPEN_SSH_FORM 事件到达才暴露）；shortcut-manager 把 Ctrl/Cmd+W 当全局快捷键，SSH 表单密码框里按 Ctrl+W 会关当前标签（需验证表单是否抑制全局快捷键）；SplitRenderer sizes watcher getter 每次求值返回新数组，node 变化即误触发（当前无害）。

---

### F1. Features 与 Composables（`features/*` + `composables/*`，约 2300 行）

**总体评价**：整体扎实 — 凭据缓存刻意放在响应式状态之外、SFTP 导航用序号防陈旧响应、tab 关闭/切分有重入保护。但有三个确定的用户可见缺陷：**关闭非最后一个活动标签后 activeTabId 悬空（空白工作区）**、**取消传输后后端自己的 'cancelled' 事件把行重新加回来**、**disconnectSession 在后端清理失败时也删除本地状态（孤儿活会话）**。次要问题集中在 listener/队列生命周期竞态、读 API 吞错误、死导出与未校验设置。

**优点**：明文密码不进响应式 Pinia 状态且在断开/重置时清除；SFTP 列表单调序号 + watchdog；closePane/closeTab 重入保护防双击双断开；Windows 盘符路径归一化有单元测试；API 包装全走共享 logger 且从不记录明文密码（只记布尔）；SFTP 与传输队列都有 dispose/unlisten 并接 onUnmounted。

| 严重度 | 位置 | 问题 | 建议 |
|--------|------|------|------|
| **Major** | use-tab-management.ts 275-278 | **关闭非最后一个活动标签后 activeTabId 悬空**：`Math.min(index, length-2)` 对非末尾标签返回 index 自身 → 活动标签指向即将被 splice 掉的标签，AppContent 渲染空 → **空白工作区**（仅关最后一个标签正确）；关 home（index 0）同样错 | `nextIndex = index < length-1 ? index+1 : index-1`，并同步重derive activePaneId |
| **Major** | use-transfer-queue.ts 189-202（后端 ssh.rs 1472-1485, 2263） | **取消后后端 'cancelled' 事件把任务复活**：cancel 乐观移除，但后端广播终态 'cancelled'，applyProgress 把未知 taskId 当跨窗口任务重建 → 被取消的行重新出现在队列 | 本地记录 cancelledByUs 集合跳过重加；或不做乐观移除改为置 cancelled 等后端终态确认 |
| **Major** | session/store.ts 220-242 | **disconnectSession 后端失败也清本地状态**：disconnectSSH/disconnectLocal/stopSessionTunnels 失败仅记日志，状态/映射/凭据无条件删除 → 前端以为断开而 Rust 侧会话/隧道仍活，cleanupAllSessions 永不再重试，孤儿连接泄漏到进程退出；同标签重连还会叠第二条连接 | 后端失败时保留记录（标 error）仅成功才删；至少重试一次并向 UI 暴露 |
| Minor | use-transfer-queue.ts 168-187 | **pause()/resume() 忽略任务方向**：下载任务也调 pause_upload，后端无 pause_download → 静默 no-op 而 UI 翻到 'paused'，下一个进度事件又翻回；resume 硬编码 'uploading' | 按 direction 分支（下载禁用/抛错或实现后端 pause_download），resume 状态按方向设置 |
| Minor | use-sftp.ts 133-171, 320-325 | **dispose() 不取消在途 invoke 与 watchdog 定时器**：卸载后 15s 定时器仍改 loading；无 disposed 标志，慢响应在 dispose 后仍写 entries（requestSeq 只防导航竞态不防卸载重会话） | dispose 置 disposed 标志、clearTimeout、await 后 bail |
| Minor | use-sftp.ts 122-179 | **stack 无界增长且永不消费；loadedPath 死状态；navigate() push 的是 go() 已改过的目标路径**（假设的未来 back 按钮会导航到当前目录）；头部注释引用不存在的 use-sftp-task-queue | 删除 stack/loadedPath 或实现真正的 back()；修注释 |
| Minor | session/api.ts 92-171、tunnel/api.ts 64-108、snippet/api.ts 27-35 | **读 API 吞错返回空默认值**：listSessions 失败渲染成空列表，UI 无法区分"无会话"与"后端坏了"；getBufferedSSHOutput 失败使重连后首屏静默空白 | 非空失败重抛或返回判别联合 {ok,data}\|{ok:false,error}，空结果才回退 [] |
| Minor | session/store.ts 163-204 | **createLocalSession 无重复保护**（createSSHSession 有）：同 tab 重复建本地会话静默覆盖映射，旧会话不 disconnect 直接丢 → 后端 PTY 泄漏且 UI 无法再断开 | 同样 early return 或先断开旧的 |
| Minor | open-file-manager.ts 43-61 | **异步建窗失败仍返回 true**：`win.once('tauri://error')` 只 await 注册，创建失败仅记日志且已返回 true，UI 以为窗口已开 | 用 Promise 等 created/error 实际结果（带超时）或创建后 getByLabel 校验 |
| Minor | use-transfer-queue.ts 94-105, 210-219 | **setupListeners 重复注册泄漏旧监听对；dispose() fire-and-forget unlisten**（未 await 未 catch） | 重复调用先 unlisten 再注册；dispose 内 await/.catch |
| Minor | settings/store.ts 58-78, 82 | **setter 零校验 + 返回响应式 terminal 对象可绕过持久化**：setFontSize(0/NaN/1e6) 直接持久化；组件可直接 `settingsStore.terminal.fontSize = 999` 绕过 setter（不持久化） | setter 内 clamp（如 6-72）；返回 readonly(terminal) |
| Nit | use-sftp.ts 280-358 | 死导出 onDownloadProgress/useSftpBrowseState（全仓无引用）；注释引用缺失模块 | 删除或接线 |
| Nit | tabs/constants.ts 11-15、use-transfer-queue.ts 110/142 | TAB_LABEL_PREFIX 死代码；弃用 substr；'Not connected'/'Preparing upload...' 等硬编码英文绕过 i18n | 删除/改 slice/走 t() |

**低置信度**：use-remote-path 的 `line.startsWith('/')` 在 ANSI 剥离前判断，pwd 输出前导转义序列时永不命中；cleanLine 只剥 SGR 不剥其他控制序列；normalizeRemotePath('/C:/')→'/C:'（尾部斜杠丢失，仅根路径场景与 SFTP 表单 '/C:/' 不一致）；`cd ../..` 的正则只捕获第一个 `..`，多级相对 cd 欠修正一级；createSSHSession 重复 id 静默 return 不抛错（调用方 await 会当成成功）；settings store 返回响应式 terminal 对象被直接改写风险（当前 grep 显示 setter 均被使用，风险低）。

---

### F9. 样式 / 语言包 / 入口 / 构建配置（`styles/*` + `locales/*` + `index.html` + `filemanager.html` + barrels + `vite.config.ts` + `tsconfig*` + `eslint.config.js` + `scripts/sync-version.mjs` + `.env.example`，约 1200 行）

**总体评价**：两个头号检查项健康 — **locale 奇偶校验精确**（en/zh 各 295 个叶子键，自动化 diff 零缺失、零 `%{placeholder}` 漂移）；**多页 Vite 构建正确配对 filemanager.html ↔ filemanager-main.ts**（运行时 WebviewWindow 创建，未在 tauri.conf.json 声明）。主要弱点是 **CSS 令牌纪律** — `server-dashboard.css` 完全绕开设计令牌体系（40+ 硬编码颜色，还引用两个从未定义的自定义属性）；以及**自定义 no-Chinese ESLint 规则存在盲区**（实测对 Vue 模板文本/静态属性和全部 CSS 不生效）。

**优点**：locale 奇偶完美（含嵌套 connection.step.*）；入口 HTML 配对正确，tauri.conf.json 只声明 main 窗口与运行时创建一致；no-Chinese 规则存在且豁免清单正确（实测对 TS 字符串/模板/注释生效）；feature barrels 全部重导出真实模块且无 default export 丢失；design-system.css 令牌层组织良好（色彩优先级正确），common.css 一致消费令牌。

| 严重度 | 位置 | 问题 | 建议 |
|--------|------|------|------|
| **Major** | eslint.config.js 104-161 | **no-Chinese 规则不看 Vue 模板文本/静态属性/CSS**：规则只访问 AST Literal/TemplateElement/注释；实测 `<span>你好</span>` 与 `title="你好"` 零报错，CSS 无解析器完全不检查 — 最常见的硬编码面（模板标记）与全部样式逃脱，error 级别半装饰性 | 用 vue-eslint-parser 的 defineTemplateBodyVisitor 扩展 VText/VLiteral/VAttribute；CSS 加 stylelint 或处理器 |
| **Major** | styles/components/server-dashboard.css 全文 | **仪表盘绕过设计令牌：40+ 硬编码颜色 + 从未定义的 `--primary-rgb`/`--primary-color`**（386/407 行引用）→ 上传徽标/文件图标恒回退硬编码蓝，用户设置强调色对这里无效果；布局锁死深色玻璃风，浅色模式不生效 | 颜色映射到令牌（color-text-* / color-primary / color-danger），alpha 用 color-mix；定义或删除 --primary-rgb |
| Minor | server-dashboard.css 154-160 vs 955-961 等 4 对 | **类重复定义且冲突**：.disk-info-list/.metric-icon/.metric-value/.toggle-handle:hover 各自定义两次，后者级联覆盖前者，前者是死规则 | 合并成单条规则；加 stylelint no-duplicate-selectors |
| Minor | design-system.css 107-320 | **暗色与 4 组强调色盘在 auto(@media) 与手动(.theme-dark) 两分支整段重复**（8 段近似复制），改色必须两处同步 | 单一来源 + 嵌套选择器/预处理器混入生成两个选择器 |
| Minor | common.css 156, 610, 694, 705 | 硬编码 rgba(10,132,255,.3)/#ff5f57/#ff4757 而存在 --color-primary/--color-macos-close/--color-danger 令牌；错误色 #ff4757 既非 light 也非 dark 的 danger 值 | 换令牌；如需保留 #ff4757 提升为 --color-danger-soft |
| Minor | design-system.css 87-95 | **非豁免文件出现中文注释与字体名 '微软雅黑'**（CSS 未过 ESLint 所以静默通过） | 注释译英；字体名注明功能必需并加英文注释 |
| Minor | session/store.ts 3 | **store 从自己 feature 的 barrel 导入 sessionApi**：`@/features/session` → index.ts 再 `export * from './store'`（本模块）— 依赖 barrel 求值顺序，未来重排即 TDZ 循环错误 | 直接 `from './api'` |
| Minor | composables/index.ts 5-8 | barrel 遗漏 use-remote-path（虽是活跃 composable，RemoteConnectionView 直接绕过 barrel 导入） | 补 `export * from './use-remote-path'` |
| Minor | .env.example 4-8, 14 + env.ts | **文档化的 VITE_PORT/VITE_APP_VERSION 旋钮未接线**：Vite 端口硬编码 1420 不读 VITE_PORT；VITE_APP_VERSION 所谓"默认取 Cargo 版本"的机制不存在（sync-version.mjs 只写 package.json，env.ts 兜底 '0.0.0'；About 面板靠 Tauri getVersion() 才躲过此坑） | 让 sync-version 生成 env 模块，或删掉误导注释与兜底声明 |
| Minor | scripts/sync-version.mjs 4-13 | **版本同步脆弱且失败静默**：正则假设 `[package] version` 字面量，`version.workspace = true` 布局会无消息 exit 1 中断每次 pnpm build 的 prebuild；JSON.parse 无保护 | try/catch + 明确报错；支持 workspace 版本；包裹 JSON.parse |
| Minor | app-utils.ts 1-28 | **废弃 shim 零消费者**但仍拖入 features/window、features/tabs、event-bus 等依赖；`--color-macos-minimize` 令牌从未被引用 | 删除 app-utils.ts；移除未用令牌 |
| Nit | index.html 7 | 残留脚手架标题 'Tauri + Vue + Typescript App'（filemanager.html 已正确） | 改 NexaShell |
| Nit | vite.config.ts 19, 24-27, 49-58 | build.target 'es2021' vs tsconfig ES2020 不齐；**vite.config.ts 既被 ESLint 排除又从不被类型检查**（build 只跑 src 的 vue-tsc，tsconfig.node.json 需 tsc -b 才查，而构建脚本从不调） | 对齐目标版本；把 tsconfig.node.json 纳入 type-check |
| Nit | common.css 570-576 | .modal-header 里 margin-bottom 被随后的 margin 简写覆盖（死声明） | 删除 |

**低置信度**：filemanager-main.ts 是否重复 main.ts 引导逻辑未逐一比对；tsconfig `types: ["vitest/globals"]` 抑制其他环境 @types 未发现实际依赖者；@types/node ^26 vs typescript 5.6 的组合因配置文件从不类型检查而被掩盖；--color-macos-minimize 是否在 Rust 侧或非 .vue 资源中消费未知；eslint .vue 解析块对 `<script lang="js">` 的边界情况未实测。

---

## 附：标注说明

- 各模块"低置信度"条目为审查代理未能在静态阅读中证实、需要运行验证或产品确认的疑点。
- 部分行号引自审查当时的源码快照（v1.19.0），修复前请以当前工作区为准复核。
- 首轮 workflow 曾报 1 条 critical（模块结果因输出截断丢失，未能在本报告中直接引用原文）；对应模块经独立子代理重审后均未达 critical 分级，最重分级为 major。若需确认该条原始表述，可重跑对应模块审查。
- 审查期间仓库被外部推进至 v1.19.0（`eeff4d1` 复制连接信息功能）；本报告审查对象为推进后的代码。