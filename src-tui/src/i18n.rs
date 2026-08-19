// ============================================================================
// Localization (i18n) for the NexaShell TUI
// ============================================================================
// A lightweight hand-rolled translation layer that keeps the TUI self-contained
// (no external localization crates) while giving the interface two languages:
//
//   - `Auto` : pick a language from the process locale (LANG / LC_ALL /
//              LC_MESSAGES) at startup — the "just works" default for a TUI.
//   - `En`   : force English.
//   - `Zh`   : force Simplified Chinese.
//
// Strings are addressed by a stable string key and looked up in a static
// EN/ZH dictionary. Unknown keys fall back to the English text so a missing
// entry can never render blank. Choices are deliberately kept to a tight tuple
// list so the translation table stays easy to audit and extend.

/// User-selectable language. Stored in settings; `Auto` resolves at startup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Auto,
    En,
    Zh,
}

impl Lang {
    pub const ALL: [Lang; 3] = [Lang::Auto, Lang::En, Lang::Zh];

    /// Stable id used for persistence and cycle-selection in the settings page.
    pub fn id(self) -> &'static str {
        match self {
            Lang::Auto => "auto",
            Lang::En => "en",
            Lang::Zh => "zh",
        }
    }

    pub fn from_id(id: &str) -> Lang {
        match id {
            "en" => Lang::En,
            "zh" => Lang::Zh,
            _ => Lang::Auto,
        }
    }

    /// Detect a language from the process environment (used only for Auto).
    pub fn detect() -> Lang {
        for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(v) = std::env::var(var) {
                let v = v.to_ascii_lowercase();
                if v.starts_with("zh") {
                    return Lang::Zh;
                }
                if v.starts_with("en") {
                    return Lang::En;
                }
            }
        }
        Lang::En
    }

    /// Resolve to the effective language (`Auto` -> detected).
    pub fn resolve(self) -> Lang {
        match self {
            Lang::Auto => Lang::detect(),
            l => l,
        }
    }

    /// Localized display label, used in the settings page.
    pub fn label(self, l10n: &L10n) -> &'static str {
        match self {
            Lang::Auto => l10n.tr("lang_auto"),
            Lang::En => "English",
            Lang::Zh => "中文",
        }
    }
}

/// The active localization for the UI. Thin wrapper so call sites have a clear
/// type to pass around instead of a bare `Lang`.
#[derive(Debug, Clone, Copy)]
pub struct L10n {
    lang: Lang,
}

impl L10n {
    /// Build an L10n from the stored preference; `Auto` is resolved here so the
    /// whole UI speaks one language for the session.
    pub fn new(pref: Lang) -> Self {
        Self {
            lang: pref.resolve(),
        }
    }

    /// Look up a keyed string in the active language. Falls back to the English
    /// entry, then to the key itself, so a missing row can never render blank.
    pub fn tr(&self, key: &'static str) -> &'static str {
        let mut en: &'static str = key;
        let mut zh: Option<&'static str> = None;
        for (k, en_s, zh_s) in DICT {
            if *k == key {
                en = en_s;
                zh = Some(zh_s);
                break;
            }
        }
        match self.lang {
            Lang::Zh => zh.filter(|s| !s.is_empty()).unwrap_or(en),
            _ => en,
        }
    }

    /// Format a translated template by substituting positional `{}` placeholders
    /// left-to-right with the provided string slices. This exists because
    /// `format!` requires a literal format string, so runtime-translated
    /// templates are filled in here. Values needing numeric precision should be
    /// pre-formatted by the caller (e.g. `format!("{:.0}", x)`).
    pub fn fmt(&self, key: &'static str, args: &[&str]) -> String {
        let tpl = self.tr(key);
        let mut out = String::with_capacity(tpl.len() + 16);
        let mut arg_iter = args.iter();
        let mut i = 0;
        while i < tpl.len() {
            let ch = tpl[i..].chars().next().unwrap();
            if ch == '{' {
                let rest = &tpl[i + 1..];
                if let Some(rel) = rest.find('}') {
                    let inner = &rest[..rel];
                    let is_positional = inner.is_empty() || inner.starts_with(':');
                    if is_positional {
                        if let Some(a) = arg_iter.next() {
                            out.push_str(a);
                        }
                        // Skip past the entire '{…}' token.
                        i = i + 1 + rel + 1;
                        continue;
                    }
                }
            }
            out.push(ch);
            i += ch.len_utf8();
        }
        out
    }

    /// Convenience: whether the UI is in Chinese right now.
    pub fn is_zh(&self) -> bool {
        self.lang == Lang::Zh
    }
}

impl Default for L10n {
    fn default() -> Self {
        Self::new(Lang::Auto)
    }
}

#[rustfmt::skip]
static DICT: &[(&str, &str, &str)] = &[
    // ------------------------------------------------------------------
    // Home page
    // ------------------------------------------------------------------
    ("composer_title", " filter / commands ", " 过滤 / 命令 "),
    ("hint_global", " ctrl+p commands  ctrl+x leader ", " ctrl+p 命令  ctrl+x 前缀键 "),
    ("hint_home_status", " enter: connect  esc: clear  ↑/↓: select ", " enter: 连接  esc: 清空  ↑/↓: 选择 "),
    ("msg_no_match", "No sessions match the filter.", "没有会话匹配当前过滤条件。"),
    ("msg_no_sessions", "No sessions yet. Press ctrl+p and run `new`, or add sessions in the desktop app.", "还没有会话。按 ctrl+p 执行 `new`，或在桌面应用里添加会话。"),
    ("sessions_title", " Sessions ({}) ", " 会话 ({}) "),
    ("status_sessions_favs", " {} sessions · {} favorites ", " {} 个会话 · {} 个收藏 "),
    // ------------------------------------------------------------------
    // Terminal page
    // ------------------------------------------------------------------
    ("connected", "{} — connected", "{} — 已连接"),
    ("connecting", "connecting", "连接中"),
    ("connecting_to", "Connecting to {} ({})…", "正在连接 {}（{}）…"),
    ("connect_cancel", " esc: cancel ", " esc: 取消 "),
    ("disconnected", "disconnected", "已断开"),
    ("hint_copy", " copy: arrows move · enter/copy  esc: exit ", " copy: 方向键移动 · enter 复制  esc: 退出 "),
    ("hint_term_status", " ctrl+x q quit · esc: shell · ctrl+tab: switch ", " ctrl+x q 退出 · esc: 终端 · ctrl+tab: 切换 "),
    ("hint_terminal", " ctrl+tab switch · ctrl+p commands · ctrl+x c copy · ctrl+x d disconnect ", " ctrl+tab 切换 · ctrl+p 命令 · ctrl+x c 复制 · ctrl+x d 断开 "),
    ("no_active_session", "no active session", "没有活动的会话"),
    ("scrollback_lines_back", "⤒ {} lines back · ", "⤒ 已回滚 {} 行 · "),
    ("status_metrics", "{} — cpu {}%  mem {}%  lat {}ms  load {}", "{} — CPU {}%  内存 {}%  延迟 {}ms  负载 {}"),
    ("tab_down", "{}(down)", "{}(已断开)"),
    ("term_connecting", " (connecting…)", " (连接中…)"),
    // ------------------------------------------------------------------
    // Command palette
    // ------------------------------------------------------------------
    ("palette_title", " command palette ", " 命令面板 "),
    ("cmd_sessions", "sessions", "sessions"),
    ("cmd_sessions_desc", "Back to session list", "返回会话列表"),
    ("cmd_new", "new", "new"),
    ("cmd_new_desc", "Create a new session", "新建会话"),
    ("cmd_refresh", "refresh", "refresh"),
    ("cmd_refresh_desc", "Reload sessions from database", "从数据库重新载入会话"),
    ("cmd_help", "help", "help"),
    ("cmd_help_desc", "Show keyboard shortcuts", "显示键盘快捷键"),
    ("cmd_quit", "quit", "quit"),
    ("cmd_quit_desc", "Exit NexaShell TUI", "退出 NexaShell TUI"),
    ("cmd_settings", "settings", "settings"),
    ("cmd_settings_desc", "Change language, theme and scrollback", "更改语言、主题与回滚行数"),
    ("cmd_disconnect", "disconnect", "disconnect"),
    ("cmd_disconnect_desc", "Disconnect the current SSH session", "断开当前 SSH 会话"),
    ("cmd_tunnels", "tunnels", "tunnels"),
    ("cmd_tunnels_desc", "Manage port-forwarding rules for this session", "管理此会话的端口转发规则"),
    ("cmd_connect_open", " [open]", " [已开]"),
    ("cmd_snippets_header", "— snippets —", "— 片段 —"),
    ("cmd_snippets_desc", "insert into active terminal / copy to clipboard", "插入到活动终端 / 复制到剪贴板"),
    // ------------------------------------------------------------------
    // Dialogs: titles
    // ------------------------------------------------------------------
    ("title_confirm_delete", "Delete session", "删除会话"),
    ("title_help", "Help", "帮助"),
    ("title_new_session", "Session", "会话"),
    ("title_notice", "Notice", "提示"),
    ("title_quit", "Quit", "退出"),
    // ------------------------------------------------------------------
    // Dialogs: form fields
    // ------------------------------------------------------------------
    ("field_name", "name", "名称"),
    ("field_host", "host", "主机"),
    ("field_port", "port", "端口"),
    ("field_username", "username", "用户名"),
    ("field_password_none", "password (blank = none)", "密码（留空 = 无）"),
    ("field_password_keep", "password (blank = keep stored)", "密码（留空 = 保留原值）"),
    ("field_key_path", "private key path (blank = password auth)", "私钥路径（留空 = 密码认证）"),
    ("field_key_pass_none", "key passphrase (blank = none)", "密钥口令（留空 = 无）"),
    ("field_key_pass_keep", "key passphrase (blank = keep stored)", "密钥口令（留空 = 保留原值）"),
    // ------------------------------------------------------------------
    // Dialogs: hints
    // ------------------------------------------------------------------
    ("enter_cancel", " enter: submit   ", " enter: 提交   "),
    ("esc_cancel", "esc: cancel", "esc: 取消"),
    ("hint_delete", "enter: delete   esc: cancel", "enter: 删除   esc: 取消"),
    ("hint_quit", "enter: quit   esc: cancel", "enter: 退出   esc: 取消"),
    ("hint_form_nav", " tab/↑↓: navigate   ", " tab/↑↓: 切换   "),
    ("hint_form_next", "enter: next / save", "enter: 下一项 / 保存"),
    ("enter_password", "Enter password", "输入密码"),
    ("esc_dismiss", " esc: dismiss ", " esc: 关闭 "),
    // ------------------------------------------------------------------
    // Notices / runtime messages
    // ------------------------------------------------------------------
    ("confirm_delete_prompt", "Delete session \"{}\"?  ", "删除会话 \"{}\"？  "),
    ("conn_failed", "Connection failed: {}", "连接失败：{}"),
    ("confirm_delete_x", "press x again to confirm delete", "再次按 x 确认删除"),
    ("created_dir", "Created directory {}", "已创建目录 {}"),
    ("deleted_path", "Deleted {}", "已删除 {}"),
    ("downloaded", "Downloaded {} ({})", "已下载 {} ({})"),
    ("empty_directory", "(empty directory)", "（空目录）"),
    ("esc_close", " esc: close ", " esc: 关闭 "),
    ("fields_required", "name, host and username are required", "名称、主机和用户名均为必填项"),
    ("hint_start_stop", "start/stop", "启动/停止"),
    ("hint_sftp", " enter: cd/download · left: up · n:mkdir·x:del·u:upload·h:home·r:refresh · esc: close ", " enter: 进入/下载 · left: 上级 · n: 新建·x: 删除·u: 上传·h: 根目录·r: 刷新 · esc: 关闭 "),
    ("list_failed", "List failed: {}", "读取目录失败：{}"),
    ("mkdir_label", " mkdir> {}▏", " 新建目录> {}▏"),
    ("no_key_path", "No private key path set for this session", "此会话未设置私钥路径"),
    ("no_session_sftp", "Connect to a session first, then open the file browser", "请先连接会话，再打开文件浏览器"),
    ("no_session_tunnel", "No session selected — connect first, then open the tunnel panel", "未选择会话——请先连接，再打开隧道面板"),
    ("no_term_snippet_copied", "No active terminal; snippet copied to clipboard", "没有活动的终端；片段已复制到剪贴板"),
    ("op_failed", "{} failed: {}", "{} 失败：{}"),
    ("rule_auto", "auto", "自动"),
    ("rule_manual", "manual", "手动"),
    ("quit_prompt", "Quit NexaShell TUI?  ", "退出 NexaShell TUI？  "),
    ("rules_refreshed", "rules refreshed", "规则已刷新"),
    ("save_failed", "Failed to save session: {}", "保存会话失败：{}"),
    ("sftp_header", " SFTP — session {} · {}  (local: {}) ", " SFTP — 会话 {} · {}  (本地: {}) "),
    ("sftp_home", "(home)", "（主目录）"),
    ("sftp_title", " files ", " 文件 "),
    ("tunnel_no_rules", "No tunnel rules for this session yet. Add rules in the desktop app.", "此会话还没有隧道规则。请在桌面应用中添加。"),
    ("tunnel_rules_title", " tunnel rules — session {} ", " 隧道规则 — 会话 {} "),
    ("tunnel_start_failed", "Start failed: {}", "启动失败：{}"),
    ("tunnel_started", "Tunnel {}:{} -> {}:{} started", "隧道 {}:{} -> {}:{} 已启动"),
    ("tunnel_stopped", "Stopped tunnel {}:{}", "已停止隧道 {}:{}"),
    ("uploaded", "Uploaded {} to {}", "已上传 {} 到 {}"),
    // ------------------------------------------------------------------
    // Settings page
    // ------------------------------------------------------------------
    ("lang_auto", "auto (system)", "auto（跟随系统）"),
    ("settings_apply", "apply", "apply"),
    ("settings_apply_desc", "Apply and persist these settings", "应用并保存这些设置"),
    ("settings_hint", " ↑/↓: select   left/right: change   enter/s: save   esc: back ", " ↑/↓: 选择   left/right: 更改   enter/s: 保存   esc: 返回 "),
    ("settings_lang", "Language", "语言"),
    ("settings_scrollback", "Terminal scrollback (lines)", "终端回滚行数"),
    ("settings_theme", "Theme", "主题"),
    ("settings_title", " settings ", " 设置 "),
    ("settings_unsaved", "unsaved changes", "有未保存的更改"),
    ("settings_saved", "settings saved", "设置已保存"),
    // ------------------------------------------------------------------
    // Settings: theme names
    // ------------------------------------------------------------------
    // ------------------------------------------------------------------
    // Help screen
    // ------------------------------------------------------------------
    ("help_home_clear", "clear filter", "清空过滤"),
    ("help_home_conn", "connect", "连接"),
    ("help_home_filter", "filter sessions", "过滤会话"),
    ("help_home_sel", "select session", "选择会话"),
    ("help_ldr_copy_tab", "copy / switch tab", "复制 / 切换标签"),
    ("help_ldr_fav_del", "favorite / delete-disconnect", "收藏 / 删除-断开"),
    ("help_ldr_help", "help", "帮助"),
    ("help_ldr_list_refresh", "session list / refresh", "会话列表 / 刷新"),
    ("help_ldr_new_edit", "new / edit session", "新建 / 编辑会话"),
    ("help_ldr_quit", "quit", "退出"),
    ("help_ldr_settings", "settings", "设置"),
    ("help_ldr_tunnel_sftp", "tunnels / sftp", "隧道 / sftp"),
    ("help_palette", "command palette", "命令面板"),
    ("help_term_remote", "sent to remote shell", "发送到远程终端"),
    ("help_term_scroll", "scrollback / switch tab", "回滚 / 切换标签"),
    ("help_term_switch", "switch tab", "切换标签"),
    ("theme_default", "default", "默认"),
    ("theme_mono", "mono", "单色"),
    ("theme_ocean", "ocean", "海洋蓝"),
    // ------------------------------------------------------------------
    // Settings: scrollback choices
    // ------------------------------------------------------------------
    ("sb_1000", "1,000", "1,000"),
    ("sb_5000", "5,000", "5,000"),
    ("sb_10000", "10,000", "10,000"),
    ("sb_20000", "20,000", "20,000"),
];

// ----------------------------------------------------------------------------
// Help screen
// ----------------------------------------------------------------------------

/// Localized help lines for the help dialog (already resolved to the active
/// language). Each entry is a `(style, text)` pair: `style = true` renders as a
/// section header (accent/bold), `false` as a plain key/description row.
pub fn help_lines(l10n: &L10n) -> Vec<(bool, String)> {
    let (s_home, s_term, s_leader) = if l10n.is_zh() {
        ("主页", "终端页", "前缀键（按 ctrl+x，然后）")
    } else {
        ("Home page", "Terminal page", "Leader key (ctrl+x, then)")
    };
    let keys_fn = |k: &str, d: &str| (false, format!("{k:<26} {d}"));

    vec![
        (true, s_home.to_string()),
        keys_fn("↑/↓", l10n.tr("help_home_sel")),
        keys_fn("enter", l10n.tr("help_home_conn")),
        keys_fn("type", l10n.tr("help_home_filter")),
        keys_fn("esc", l10n.tr("help_home_clear")),
        keys_fn("ctrl+p", l10n.tr("help_palette")),
        (true, s_term.to_string()),
        keys_fn("any key", l10n.tr("help_term_remote")),
        keys_fn("pgup/pgdn", l10n.tr("help_term_scroll")),
        keys_fn("ctrl+tab", l10n.tr("help_term_switch")),
        (true, s_leader.to_string()),
        keys_fn("q", l10n.tr("help_ldr_quit")),
        keys_fn("h", l10n.tr("help_ldr_help")),
        keys_fn("l / r", l10n.tr("help_ldr_list_refresh")),
        keys_fn("n / e", l10n.tr("help_ldr_new_edit")),
        keys_fn("f / d", l10n.tr("help_ldr_fav_del")),
        keys_fn("o", l10n.tr("help_ldr_settings")),
        keys_fn("u / s", l10n.tr("help_ldr_tunnel_sftp")),
        keys_fn("c / t", l10n.tr("help_ldr_copy_tab")),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn en_tr_returns_english_and_falls_back_to_key() {
        let l = L10n::new(Lang::En);
        assert_eq!(l.tr("palette_title"), " command palette ");
        // Missing key falls back to the key itself, never blanks.
        assert_eq!(l.tr("no_such_key"), "no_such_key");
    }

    #[test]
    fn zh_tr_returns_chinese() {
        let l = L10n::new(Lang::Zh);
        assert_eq!(l.tr("palette_title"), " 命令面板 ");
        assert_eq!(l.tr("conn_failed"), "连接失败：{}");
    }

    #[test]
    fn auto_resolves_from_lang_env() {
        // Edition 2024 marks set_var unsafe. Save/restore the variable so this
        // test can never leak a mutation into a parallel test that (like
        // L10n::new(Auto)) reads the environment.
        let original = std::env::var("LANG").ok();
        unsafe {
            std::env::set_var("LANG", "zh_CN.UTF-8");
            assert_eq!(Lang::detect(), Lang::Zh);
            std::env::set_var("LANG", "en_US.UTF-8");
            assert_eq!(Lang::detect(), Lang::En);
        }
        match original {
            Some(v) => unsafe { std::env::set_var("LANG", v) },
            None => unsafe { std::env::remove_var("LANG") },
        }
    }

    #[test]
    fn fmt_substitutes_positional_args() {
        let l = L10n::new(Lang::En);
        assert_eq!(l.fmt("sessions_title", &["3"]), " Sessions (3) ");
        assert_eq!(
            l.fmt("tunnel_started", &["127.0.0.1", "4000", "db", "5432"]),
            "Tunnel 127.0.0.1:4000 -> db:5432 started"
        );
    }

    #[test]
    fn fmt_zh_substitutes_and_still_formats() {
        let l = L10n::new(Lang::Zh);
        assert_eq!(l.fmt("conn_failed", &["timeout"]), "连接失败：timeout");
        assert_eq!(l.fmt("created_dir", &["/tmp/x"]), "已创建目录 /tmp/x");
    }

    #[test]
    fn lang_roundtrip_via_id() {
        for lang in Lang::ALL {
            assert_eq!(Lang::from_id(lang.id()), lang);
        }
        assert_eq!(Lang::from_id("nonsense"), Lang::Auto);
    }
}
