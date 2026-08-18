# macOS 签名与公证（解决 DMG「已损坏」提示）

## 问题背景

GitHub Actions 自动构建的 DMG 下载后，macOS 提示 **「已损坏，无法打开。你应该将它移到废纸篓」**，而 Windows 安装包正常。

原因：CI 构建的 arm64 应用**未经过代码签名**，浏览器下载时会自动给文件打上 `com.apple.quarantine`（隔离）属性，Gatekeeper 校验失败后把应用判定为「损坏」。本地构建能跑是因为没有隔离属性、且 arm64 链接器默认做了 ad-hoc 签名。

> 官方文档原文：*Code signing is required on macOS … to prevent a warning that your application is broken and can not be started, when downloaded from the browser.*
> https://v2.tauri.app/distribute/sign/macos/

## 两种解决方案

| 方案 | 成本 | 用户打开体验 |
|---|---|---|
| **ad-hoc 签名**（仓库已默认启用） | 免费 | 提示「无法验证开发者」，右键「打开」或 系统设置 → 隐私与安全性 →「仍要打开」即可 |
| **Developer ID 签名 + 公证**（推荐） | Apple Developer 会员（$99/年） | 全新安装，双击即开，无任何提示 |

---

## 方案 A：ad-hoc 签名（已默认生效）

`src-tauri/tauri.conf.json` 中已配置：

```json
"macOS": {
  "signingIdentity": "-"
}
```

- CI 未配置 `APPLE_*` secrets 时自动使用 ad-hoc 签名，应用带有效签名，**不会**再报「已损坏」。
- 局限：ad-hoc 签名不受信任链认可，Gatekeeper 仍要求用户在 系统设置 → 隐私与安全性 中手动放行（「仍要打开」）。
- 对应官方文档 Ad-Hoc Signing 一节（`signingIdentity: "-"`），tauri 会跳过 DMG 自签名，避免 DMG 损坏（tauri-apps/tauri#12288）。

**已下载的旧 DMG 手动修复**（当前 v1.20.2 及更早版本）：

```bash
# 方式一：对已安装的应用移除隔离属性
xattr -cr "/Applications/NexaShell.app"

# 方式二（等价）：移除所有隔离属性
sudo xattr -rd com.apple.quarantine "/Applications/NexaShell.app"
```

---

## 方案 B：Developer ID 签名 + 公证（全新安装无提示）

### 1. 前提

- 付费的 **Apple Developer Program** 账号（免费账号**无法公证**，公证后才有「已验证」状态）。
- 一台 Mac（导出证书、执行本地验证用）。

### 2. 创建 Developer ID Application 证书

**方式一（Xcode，推荐）**：Xcode → Settings → Accounts → 登录 Apple ID → Manage Certificates → `+` → **Developer ID Application**。

**方式二（开发者网站）**：https://developer.apple.com/account/resources/certificates → 新建证书 → 类型选 **Developer ID Application** → 按提示上传 CSR（钥匙串访问 → 证书助理 → 从证书颁发机构请求证书）。

### 3. 导出 .p12 并转 base64

钥匙串访问 → 登录 → 我的证书 → 找到刚创建的证书（含私钥）→ 右键导出为 `certificate.p12`（设置导出密码）：

```bash
# base64 编码，内容填入 GitHub Secret APPLE_CERTIFICATE
base64 -i certificate.p12
# 导出密码填入 APPLE_CERTIFICATE_PASSWORD
```

### 4. 准备公证凭据（二选一）

**方式一：App Store Connect API Key（推荐，可在 CI 长期使用）**

1. https://appstoreconnect.apple.com/access/users → Keys（+）→ 创建 App Store Connect API Key（勾选访问权限，用于公证任意一项即可）。
2. 下载 `.p8` 私钥文件（仅下载一次），记下 **Key ID** 与 **Issuer ID**。
3. 在 CI 中导出 `APPLE_API_KEY`（Key ID）、`APPLE_API_ISSUER`（Issuer ID），并把 `.p8` 文件放到构建目录 `./private_keys/AuthKey_<KeyID>.p8`（或设置 `APPLE_API_KEY_PATH` 指向它）。
   > 注意：本仓库 release.yml 默认使用 Apple ID 方式（`APPLE_ID`/`APPLE_PASSWORD`/`APPLE_TEAM_ID`），如改用 API Key 需同步调整工作流。

**方式二：Apple ID + 专用密码（本仓库 release.yml 默认方式）**

1. https://account.apple.com → 登录与安全 → App 专用密码 → 生成一个专用密码。
2. 邮箱填入 `APPLE_ID`，专用密码填入 `APPLE_PASSWORD`。

### 5. 获取 Team ID 与签名身份

- **Team ID**：https://developer.apple.com/account → Membership 页展示（10 位字母数字）。
- **签名身份**（可选，一般可直接用 `"Developer ID Application: 你的名字 (TEAMID)"`）：

```bash
security find-identity -v -p codesigning
```

### 6. 配置 GitHub Secrets

仓库 Settings → Secrets and variables → Actions → New repository secret：

| Secret | 内容 |
|---|---|
| `APPLE_CERTIFICATE` | `certificate.p12` 的 base64 |
| `APPLE_CERTIFICATE_PASSWORD` | p12 导出密码 |
| `APPLE_SIGNING_IDENTITY` | 签名身份名，如 `Developer ID Application: xxx (TEAMID)` |
| `APPLE_ID` | 开发者账号邮箱 |
| `APPLE_PASSWORD` | App 专用密码 |
| `APPLE_TEAM_ID` | Team ID |

> **重要**：以上 6 个 secret 要么全部配置、要么全部不配置。工作流以 `APPLE_SIGNING_IDENTITY` 是否存在为开关——未配置时自动回退 ad-hoc 签名，不会构建失败。

### 7. 触发构建验证

推送 tag（如 `git tag v1.20.3 && git push origin main --tags`），在 Actions 查看 macOS job 日志：
- 出现 **Import Apple Developer Certificate** 步骤 → 签名已启用；
- tauri 输出包含 `Signing with identity` 与 `notarizing` / `stapled` → 签名 + 公证成功。

### 8. 本地验证（可选）

```bash
# 签名信息（应为 Developer ID Application）
codesign -dv --verbose=4 "path/to/NexaShell.app"
# Gatekeeper 评估（应为 Developer ID 验证通过）
spctl -a -vv --type execute "path/to/NexaShell.app"
# 公证票据
xcrun stapler validate "path/to/NexaShell.app"
```

---

## 常见问题

**Q：配置了 secrets 但构建报错「no identity found」？**
p12 导出的证书类型不对——必须是 **Developer ID Application**（不是 Apple Development / Distribution）。重新创建并导出。

**Q：公证很慢/失败？**
首次公证可能需数分钟到数小时；`APPLE_ID` 必须配 App 专用密码而非账号登录密码。报错详情在 tauri 日志的 notarization 段。

**Q：不想每次发版都等公证？**
可在 tauri-action `args` 追加 `--skip-stapling`（首次验证用），正式发布去掉。
