use crate::common::{OutputChunk, SessionId};
use crate::db;
use crate::db::{Group, SessionWithRelations};
use crate::ssh::{ServerStatus, SshEventSink, SshManager};
use crate::term::TerminalPane;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Frame;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

// ============================================================================
// Events & sink
// ============================================================================

pub enum UiEvent {
    Output {
        session_id: String,
        output: String,
    },
    Status {
        session_id: String,
        status: ServerStatus,
    },
    Disconnected {
        session_id: String,
        reason: String,
    },
    ConnectResult {
        session_id: String,
        result: Result<(), String>,
    },
}

/// Channel-backed SshEventSink that streams SSH events into the UI loop.
pub struct EventSink {
    tx: mpsc::UnboundedSender<UiEvent>,
}

impl EventSink {
    pub fn new(tx: mpsc::UnboundedSender<UiEvent>) -> Self {
        Self { tx }
    }
}

impl SshEventSink for EventSink {
    fn on_output(&self, session_id: &str, chunk: &OutputChunk) {
        let _ = self.tx.send(UiEvent::Output {
            session_id: session_id.to_string(),
            output: chunk.output.clone(),
        });
    }

    fn on_status(&self, session_id: &str, status: &ServerStatus) {
        let _ = self.tx.send(UiEvent::Status {
            session_id: session_id.to_string(),
            status: status.clone(),
        });
    }

    fn on_disconnected(&self, session_id: &str, reason: &str) {
        let _ = self.tx.send(UiEvent::Disconnected {
            session_id: session_id.to_string(),
            reason: reason.to_string(),
        });
    }
}

// ============================================================================
// Theme
// ============================================================================

pub struct Theme {
    pub accent: Color,
    pub fg: Color,
    pub dim: Color,
    pub border: Color,
    pub error: Color,
    pub ok: Color,
    pub warning: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            accent: Color::Cyan,
            fg: Color::Gray,
            dim: Color::DarkGray,
            border: Color::DarkGray,
            error: Color::Red,
            ok: Color::Green,
            warning: Color::Yellow,
        }
    }
}

// ============================================================================
// Dialogs
// ============================================================================

pub struct PasswordDialog {
    pub session_id: String,
    pub title: String,
    pub input: String,
    pub cursor: usize,
}

impl PasswordDialog {
    pub fn new(session_id: String, title: String) -> Self {
        Self {
            session_id,
            title,
            input: String::new(),
            cursor: 0,
        }
    }
}

pub struct FormField {
    pub label: &'static str,
    pub value: String,
    pub cursor: usize,
    pub masked: bool,
}

pub struct NewSessionForm {
    pub fields: Vec<FormField>,
    pub focus: usize,
}

impl NewSessionForm {
    pub fn new() -> Self {
        Self {
            fields: vec![
                FormField { label: "name", value: String::new(), cursor: 0, masked: false },
                FormField { label: "host", value: String::new(), cursor: 0, masked: false },
                FormField { label: "port", value: "22".to_string(), cursor: 2, masked: false },
                FormField { label: "username", value: String::new(), cursor: 0, masked: false },
                FormField { label: "password (blank = none)", value: String::new(), cursor: 0, masked: true },
                FormField { label: "private key path (blank = password auth)", value: String::new(), cursor: 0, masked: false },
                FormField { label: "key passphrase (blank = none)", value: String::new(), cursor: 0, masked: true },
            ],
            focus: 0,
        }
    }
}

pub enum Dialog {
    Password(PasswordDialog),
    NewSession(NewSessionForm),
    Help,
    Quit,
    Notice(String),
}

impl Dialog {
    pub fn title(&self) -> &str {
        match self {
            Dialog::Password(d) => &d.title,
            Dialog::NewSession(_) => "New session",
            Dialog::Help => "Help",
            Dialog::Quit => "Quit",
            Dialog::Notice(_) => "Notice",
        }
    }
}

// ============================================================================
// Command bar
// ============================================================================

#[derive(Clone)]
pub struct CommandItem {
    pub id: String,
    pub label: String,
    pub desc: String,
}

pub struct CommandBar {
    pub input: String,
    pub cursor: usize,
    pub items: Vec<CommandItem>,
    pub selected: usize,
}

impl CommandBar {
    pub fn new(items: Vec<CommandItem>) -> Self {
        Self {
            input: String::new(),
            cursor: 0,
            items,
            selected: 0,
        }
    }

    pub fn filtered(&self) -> Vec<usize> {
        let q = self.input.to_lowercase();
        self.items
            .iter()
            .enumerate()
            .filter(|(_, it)| {
                q.is_empty()
                    || it.label.to_lowercase().contains(&q)
                    || it.desc.to_lowercase().contains(&q)
            })
            .map(|(i, _)| i)
            .collect()
    }
}

// ============================================================================
// App
// ============================================================================

pub struct TerminalSession {
    pub session_id: String,
    pub server_name: String,
    pub user_host: String,
    pub pane: TerminalPane,
    pub connected: bool,
    pub disconnect_reason: Option<String>,
}

pub enum Page {
    Home,
    Terminal,
}

pub struct App {
    pub manager: Arc<SshManager>,
    pub sink: Arc<EventSink>,
    tx: mpsc::UnboundedSender<UiEvent>,
    rx: mpsc::UnboundedReceiver<UiEvent>,
    pub theme: Theme,

    pub sessions: Vec<SessionWithRelations>,
    pub groups: Vec<Group>,
    pub filter: String,
    pub filter_cursor: usize,
    pub selected: usize,

    pub page: Page,
    pub terminal: Option<TerminalSession>,
    pub status: Option<ServerStatus>,

    pub connecting: bool,
    pub dialog: Option<Dialog>,
    pub command_bar: Option<CommandBar>,

    pub leader: Option<Instant>,
    pub quit: bool,

    viewport: Rect,
    last_term_size: Option<(u16, u16)>,
}

impl App {
    pub fn new(
        manager: Arc<SshManager>,
        sink: Arc<EventSink>,
        tx: mpsc::UnboundedSender<UiEvent>,
        rx: mpsc::UnboundedReceiver<UiEvent>,
    ) -> Self {
        let mut app = Self {
            manager,
            sink,
            tx,
            rx,
            theme: Theme::default(),
            sessions: Vec::new(),
            groups: Vec::new(),
            filter: String::new(),
            filter_cursor: 0,
            selected: 0,
            page: Page::Home,
            terminal: None,
            status: None,
            connecting: false,
            dialog: None,
            command_bar: None,
            leader: None,
            quit: false,
            viewport: Rect::new(0, 0, 80, 24),
            last_term_size: None,
        };
        app.refresh_sessions();
        app
    }

    pub fn refresh_sessions(&mut self) {
        self.groups = db::list_groups().unwrap_or_default();
        self.sessions = db::get_sessions_with_relations().unwrap_or_default();
        if self.selected >= self.sessions.len() {
            self.selected = 0;
        }
    }

    // ------------------------------------------------------------------
    // Event handling
    // ------------------------------------------------------------------

    pub fn handle_ui_event(&mut self, ev: UiEvent) {
        match ev {
            UiEvent::Output { session_id, output } => {
                if let Some(t) = &mut self.terminal
                    && t.session_id == session_id
                {
                    t.pane.feed(output.as_bytes());
                }
            }
            UiEvent::Status { session_id, status } => {
                if let Some(t) = &self.terminal
                    && t.session_id == session_id
                {
                    self.status = Some(status);
                }
            }
            UiEvent::Disconnected { session_id, reason } => {
                if let Some(t) = &mut self.terminal
                    && t.session_id == session_id
                {
                    t.connected = false;
                    t.disconnect_reason = Some(reason);
                }
            }
            UiEvent::ConnectResult { session_id, result } => {
                self.connecting = false;
                match result {
                    Ok(()) => {
                        let Some(sess) = self
                            .sessions
                            .iter()
                            .find(|s| s.session.id == session_id)
                            .cloned()
                        else {
                            return;
                        };
                        let _ = db::update_session_timestamp(session_id.clone());
                        let (cols, rows) = self.connect_size();
                        let mut pane = TerminalPane::new(cols, rows);
                        pane.resize(cols, rows);
                        let user_host = format!("{}@{}", sess.session.username, sess.session.addr);
                        self.terminal = Some(TerminalSession {
                            session_id: session_id.clone(),
                            server_name: sess.session.server_name.clone(),
                            user_host,
                            pane,
                            connected: true,
                            disconnect_reason: None,
                        });
                        self.status = None;
                        self.page = Page::Terminal;
                        self.last_term_size = None;
                    }
                    Err(e) => {
                        self.dialog = Some(Dialog::Notice(format!("Connection failed: {}", e)));
                    }
                }
            }
        }
    }

    pub fn on_tick(&mut self) {
        if let Some(start) = self.leader
            && start.elapsed() > Duration::from_millis(2000)
        {
            self.leader = None;
        }
    }

    pub fn drain_events(&mut self) {
        while let Ok(ev) = self.rx.try_recv() {
            self.handle_ui_event(ev);
        }
    }

    pub fn paste(&mut self, text: String) {
        if self.command_bar.is_some() || self.dialog.is_some() {
            return;
        }
        match self.page {
            Page::Home => {
                for c in text.chars() {
                    insert_char(&mut self.filter, &mut self.filter_cursor, c);
                }
            }
            Page::Terminal => {
                if let Some(t) = &self.terminal {
                    let _ = self.manager.send_ssh_input(
                        &SessionId::from(t.session_id.clone()),
                        text,
                    );
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Key handling
    // ------------------------------------------------------------------

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release {
            return;
        }

        if let Some(mut cb) = self.command_bar.take() {
            let keep = self.command_bar_key(&mut cb, key);
            if keep {
                self.command_bar = Some(cb);
            }
            return;
        }
        if self.dialog.is_some() {
            self.dialog_key(key);
            return;
        }

        let is_ctrl_x = key.modifiers.contains(KeyModifiers::CONTROL)
            && key.code == KeyCode::Char('x');
        if is_ctrl_x {
            self.leader = Some(Instant::now());
            return;
        }

        if self.leader.is_some() {
            self.leader = None;
            self.leader_key(key);
            return;
        }

        match self.page {
            Page::Home => self.home_key(key),
            Page::Terminal => self.terminal_key(key),
        }
    }

    fn leader_key(&mut self, key: KeyEvent) {
        let code = match key.code {
            KeyCode::Char(c) => c.to_ascii_lowercase(),
            KeyCode::Esc => {
                self.leader = None;
                return;
            }
            _ => return,
        };
        match code {
            'q' => self.dialog = Some(Dialog::Quit),
            'h' => self.dialog = Some(Dialog::Help),
            'p' => self.open_command_bar(),
            'n' if matches!(self.page, Page::Home) => {
                self.dialog = Some(Dialog::NewSession(NewSessionForm::new()))
            }
            'r' if matches!(self.page, Page::Home) => {
                self.refresh_sessions();
                self.leader = None;
            }
            'l' => self.go_home(),
            'd' if matches!(self.page, Page::Terminal) => {
                self.disconnect_current();
                self.go_home();
            }
            _ => {}
        }
    }

    fn home_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let code = key.code;
        match code {
            KeyCode::Char(c) if ctrl && (c == 'c' || c == 'C') => {
                self.dialog = Some(Dialog::Quit);
            }
            KeyCode::Char(c) if ctrl && c == 'p' => self.open_command_bar(),
            KeyCode::Char('a') if ctrl => self.filter_cursor = 0,
            KeyCode::Char('e') if ctrl => self.filter_cursor = self.filter.chars().count(),
            KeyCode::Char('u') if ctrl => {
                self.filter.drain(..char_boundary(&self.filter, self.filter_cursor));
                self.filter_cursor = 0;
            }
            KeyCode::Char('k') if ctrl => {
                let idx = char_boundary(&self.filter, self.filter_cursor);
                self.filter.truncate(idx);
            }
            KeyCode::Char('w') if ctrl => {
                delete_word_backward(&mut self.filter, &mut self.filter_cursor);
            }
            KeyCode::Char('/') if self.filter.is_empty() => self.open_command_bar(),
            KeyCode::Char(c) => {
                insert_char(&mut self.filter, &mut self.filter_cursor, c);
            }
            KeyCode::Backspace => delete_backward(&mut self.filter, &mut self.filter_cursor),
            KeyCode::Delete => delete_forward(&mut self.filter, &mut self.filter_cursor),
            KeyCode::Left => self.filter_cursor = self.filter_cursor.saturating_sub(1),
            KeyCode::Right => {
                self.filter_cursor = self.filter_cursor.min(self.filter.chars().count());
            }
            KeyCode::Home => self.filter_cursor = 0,
            KeyCode::End => self.filter_cursor = self.filter.chars().count(),
            KeyCode::Up => {
                let n = self.filtered_sessions().len();
                if n > 0 {
                    self.selected = (self.selected + n - 1) % n;
                }
            }
            KeyCode::Down => {
                let n = self.filtered_sessions().len();
                if n > 0 {
                    self.selected = (self.selected + 1) % n;
                }
            }
            KeyCode::Enter => {
                if let Some(idx) = self.filtered_sessions().get(self.selected).copied() {
                    let sess = self.sessions[idx].clone();
                    self.start_connect(sess);
                }
            }
            KeyCode::Esc if !self.filter.is_empty() => {
                self.filter.clear();
                self.filter_cursor = 0;
            }
            _ => {}
        }
    }

    fn terminal_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Char('p') if ctrl => self.open_command_bar(),
            _ => {
                if let (Some(seq), Some(t)) = (key_to_escape(key), &self.terminal) {
                    let _ = self
                        .manager
                        .send_ssh_input(&SessionId::from(t.session_id.clone()), seq);
                }
            }
        }
    }

    /// Returns `false` when the palette should close after this key.
    fn command_bar_key(&mut self, cb: &mut CommandBar, key: KeyEvent) -> bool {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Esc => return false,
            KeyCode::Char(c) if ctrl && c == 'p' => return false,
            KeyCode::Char('a') if ctrl => cb.cursor = 0,
            KeyCode::Char('e') if ctrl => cb.cursor = cb.input.chars().count(),
            KeyCode::Char('u') if ctrl => {
                cb.input.drain(..char_boundary(&cb.input, cb.cursor));
                cb.cursor = 0;
            }
            KeyCode::Char('k') if ctrl => {
                let idx = char_boundary(&cb.input, cb.cursor);
                cb.input.truncate(idx);
            }
            KeyCode::Char('w') if ctrl => {
                delete_word_backward(&mut cb.input, &mut cb.cursor);
            }
            KeyCode::Char(c) => insert_char(&mut cb.input, &mut cb.cursor, c),
            KeyCode::Backspace => delete_backward(&mut cb.input, &mut cb.cursor),
            KeyCode::Delete => delete_forward(&mut cb.input, &mut cb.cursor),
            KeyCode::Left => cb.cursor = cb.cursor.saturating_sub(1),
            KeyCode::Right => cb.cursor = cb.cursor.min(cb.input.chars().count()),
            KeyCode::Home => cb.cursor = 0,
            KeyCode::End => cb.cursor = cb.input.chars().count(),
            KeyCode::Up => {
                let f = cb.filtered();
                if !f.is_empty() {
                    cb.selected = (cb.selected + f.len() - 1) % f.len();
                }
            }
            KeyCode::Down => {
                let f = cb.filtered();
                if !f.is_empty() {
                    cb.selected = (cb.selected + 1) % f.len();
                }
            }
            KeyCode::Enter => {
                let f = cb.filtered();
                if let Some(&idx) = f.get(cb.selected) {
                    let item = cb.items[idx].clone();
                    self.run_command(item);
                }
                return false;
            }
            _ => {}
        }
        true
    }

    fn dialog_key(&mut self, key: KeyEvent) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        let dlg = self.dialog.take();
        let mut dlg = match dlg {
            Some(d) => d,
            None => return,
        };

        match &mut dlg {
            Dialog::Password(p) => match key.code {
                KeyCode::Esc => {}
                KeyCode::Enter => {
                    let password = std::mem::take(&mut p.input);
                    p.cursor = 0;
                    let sid = p.session_id.clone();
                    self.dialog = None;
                    if let Some(sess) = self
                        .sessions
                        .iter()
                        .find(|s| s.session.id == sid)
                        .cloned()
                    {
                        self.do_connect(sess, Some(password), None);
                    }
                    return;
                }
                KeyCode::Char(c) if ctrl && c == 'p' => {}
                KeyCode::Char(c) => insert_char(&mut p.input, &mut p.cursor, c),
                KeyCode::Backspace => delete_backward(&mut p.input, &mut p.cursor),
                KeyCode::Delete => delete_forward(&mut p.input, &mut p.cursor),
                KeyCode::Left => p.cursor = p.cursor.saturating_sub(1),
                KeyCode::Right => p.cursor = p.cursor.min(p.input.chars().count()),
                KeyCode::Home => p.cursor = 0,
                KeyCode::End => p.cursor = p.input.chars().count(),
                _ => {}
            },
            Dialog::NewSession(f) => match key.code {
                KeyCode::Esc => {}
                KeyCode::Tab => {
                    f.focus = (f.focus + 1) % f.fields.len();
                }
                KeyCode::Down => {
                    f.focus = (f.focus + 1) % f.fields.len();
                }
                KeyCode::Up => {
                    f.focus = (f.focus + f.fields.len() - 1) % f.fields.len();
                }
                KeyCode::Enter => {
                    let last = f.focus == f.fields.len() - 1;
                    if last {
                        let form = std::mem::replace(f, NewSessionForm::new());
                        self.save_session_form(form);
                        return;
                    }
                    f.focus += 1;
                }
                KeyCode::Char(c) => {
                    let field = &mut f.fields[f.focus];
                    insert_char(&mut field.value, &mut field.cursor, c);
                }
                KeyCode::Backspace => {
                    let field = &mut f.fields[f.focus];
                    delete_backward(&mut field.value, &mut field.cursor);
                }
                KeyCode::Delete => {
                    let field = &mut f.fields[f.focus];
                    delete_forward(&mut field.value, &mut field.cursor);
                }
                KeyCode::Left => {
                    let field = &mut f.fields[f.focus];
                    field.cursor = field.cursor.saturating_sub(1);
                }
                KeyCode::Right => {
                    let field = &mut f.fields[f.focus];
                    field.cursor = field.cursor.min(field.value.chars().count());
                }
                KeyCode::Home => {
                    let field = &mut f.fields[f.focus];
                    field.cursor = 0;
                }
                KeyCode::End => {
                    let field = &mut f.fields[f.focus];
                    field.cursor = field.value.chars().count();
                }
                _ => {}
            },
            Dialog::Quit => match key.code {
                KeyCode::Enter => self.quit = true,
                KeyCode::Esc => {}
                _ => return,
            },
            Dialog::Notice(_) => match key.code {
                KeyCode::Esc | KeyCode::Enter => {}
                _ => return,
            },
            Dialog::Help => match key.code {
                KeyCode::Esc | KeyCode::Enter => {}
                _ => return,
            },
        }
        self.dialog = Some(dlg);
    }

    // ------------------------------------------------------------------
    // Actions
    // ------------------------------------------------------------------

    pub fn open_command_bar(&mut self) {
        let mut items = vec![
            CommandItem {
                id: "sessions".into(),
                label: "sessions".into(),
                desc: "Back to session list".into(),
            },
            CommandItem {
                id: "new".into(),
                label: "new".into(),
                desc: "Create a new session".into(),
            },
            CommandItem {
                id: "refresh".into(),
                label: "refresh".into(),
                desc: "Reload sessions from database".into(),
            },
            CommandItem {
                id: "help".into(),
                label: "help".into(),
                desc: "Show keyboard shortcuts".into(),
            },
            CommandItem {
                id: "quit".into(),
                label: "quit".into(),
                desc: "Exit NexaShell TUI".into(),
            },
        ];
        if matches!(self.page, Page::Terminal) {
            items.insert(
                0,
                CommandItem {
                    id: "disconnect".into(),
                    label: "disconnect".into(),
                    desc: "Disconnect the current SSH session".into(),
                },
            );
        }
        for s in &self.sessions {
            let sess = &s.session;
            items.push(CommandItem {
                id: format!("connect:{}", sess.id),
                label: format!("connect · {} ({})", sess.server_name, sess.addr),
                desc: format!("{}@{}", sess.username, sess.addr),
            });
        }
        self.command_bar = Some(CommandBar::new(items));
    }

    fn run_command(&mut self, item: CommandItem) {
        if let Some(sid) = item.id.strip_prefix("connect:") {
            if let Some(sess) = self.sessions.iter().find(|s| s.session.id == sid).cloned() {
                self.start_connect(sess);
            }
            return;
        }
        match item.id.as_str() {
            "sessions" => self.go_home(),
            "new" => self.dialog = Some(Dialog::NewSession(NewSessionForm::new())),
            "refresh" => self.refresh_sessions(),
            "help" => self.dialog = Some(Dialog::Help),
            "quit" => self.dialog = Some(Dialog::Quit),
            "disconnect" => {
                self.disconnect_current();
                self.go_home();
            }
            _ => {}
        }
    }

    fn go_home(&mut self) {
        self.page = Page::Home;
        self.status = None;
        self.last_term_size = None;
    }

    fn disconnect_current(&mut self) {
        if let Some(t) = &self.terminal {
            let _ = self
                .manager
                .disconnect_ssh(&SessionId::from(t.session_id.clone()));
        }
        self.terminal = None;
    }

    pub fn shutdown(&mut self) {
        self.disconnect_current();
    }

    fn connect_size(&self) -> (u16, u16) {
        let cols = self.viewport.width.clamp(20, 400);
        let rows = self.viewport.height.clamp(5, 200);
        (cols, rows)
    }

    fn start_connect(&mut self, session: SessionWithRelations) {
        if self.connecting {
            return;
        }
        let sid = session.session.id.clone();
        let (_, password, key_passphrase) =
            db::get_session_credentials(sid.clone()).unwrap_or((sid.clone(), None, None));

        match session.session.auth_type.as_str() {
            "password" => {
                if let Some(p) = password {
                    self.do_connect(session, Some(p), None);
                } else {
                    self.dialog = Some(Dialog::Password(PasswordDialog::new(
                        sid,
                        "Enter password".to_string(),
                    )));
                }
            }
            "key" => {
                if let Some(kp) = session.session.private_key_path.clone() {
                    let _ = kp;
                    self.do_connect(session, None, key_passphrase);
                } else {
                    self.dialog =
                        Some(Dialog::Notice("No private key path set for this session".into()));
                }
            }
            _ => self.do_connect(session, password, key_passphrase),
        }
    }

    fn do_connect(
        &mut self,
        session: SessionWithRelations,
        password: Option<String>,
        key_passphrase: Option<String>,
    ) {
        if self.connecting {
            return;
        }
        let sess = session.session;
        self.connecting = true;
        self.status = None;

        let (cols, rows) = self.connect_size();
        let manager = self.manager.clone();
        let sink = self.sink.clone();
        let tx = self.tx.clone();
        let sid = sess.id.clone();

        tokio::spawn(async move {
            let result = manager
                .connect_ssh(
                    sink,
                    SessionId::from(sid.clone()),
                    sess.addr,
                    sess.port as u16,
                    sess.username,
                    password.unwrap_or_default(),
                    sess.private_key_path,
                    key_passphrase,
                    cols as u32,
                    rows as u32,
                )
                .await
                .map_err(|e| e.to_string());
            let _ = tx.send(UiEvent::ConnectResult {
                session_id: sid,
                result,
            });
        });
    }

    fn save_session_form(&mut self, form: NewSessionForm) {
        let values: Vec<String> = form.fields.into_iter().map(|f| f.value).collect();
        let name = values[0].trim().to_string();
        let host = values[1].trim().to_string();
        let port: i64 = values[2].trim().parse().unwrap_or(22);
        let username = values[3].trim().to_string();
        let password = values[4].trim().to_string();
        let key_path = values[5].trim().to_string();
        let key_passphrase = values[6].trim().to_string();

        if name.is_empty() || host.is_empty() || username.is_empty() {
            self.dialog = Some(Dialog::Notice(
                "name, host and username are required".into(),
            ));
            return;
        }

        let auth_type = if key_path.is_empty() { "password" } else { "key" };
        let private_key_path = if key_path.is_empty() { None } else { Some(key_path) };
        let password = if password.is_empty() { None } else { Some(password) };
        let key_passphrase = if key_passphrase.is_empty() {
            None
        } else {
            Some(key_passphrase)
        };

        let res = db::save_session_with_credentials(
            None,
            host,
            port,
            name,
            username,
            auth_type.to_string(),
            private_key_path,
            password,
            key_passphrase,
            None,
            None,
            None,
        );
        match res {
            Ok(_) => {
                self.refresh_sessions();
            }
            Err(e) => {
                self.dialog = Some(Dialog::Notice(format!("Failed to save session: {}", e)));
            }
        }
    }

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    fn filtered_sessions(&self) -> Vec<usize> {
        let q = self.filter.to_lowercase();
        self.sessions
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                let sess = &s.session;
                q.is_empty()
                    || sess.server_name.to_lowercase().contains(&q)
                    || sess.addr.to_lowercase().contains(&q)
                    || sess.username.to_lowercase().contains(&q)
                    || s.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .map(|(i, _)| i)
            .collect()
    }

    // ------------------------------------------------------------------
    // Rendering
    // ------------------------------------------------------------------

    pub fn draw(&mut self, frame: &mut Frame) {
        self.viewport = frame.area();
        match self.page {
            Page::Home => self.draw_home(frame),
            Page::Terminal => self.draw_terminal(frame),
        }
        if let Some(cb) = &self.command_bar {
            Self::draw_command_bar(frame, cb, &self.theme);
        }
        if let Some(d) = &self.dialog {
            Self::draw_dialog(frame, d, &self.theme);
        }
    }

    fn draw_home(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let theme = &self.theme;
        let buf = frame.buffer_mut();

        // Top bar
        buf.set_string(
            area.x,
            area.y,
            " NexaShell TUI ",
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        );
        let hint = " ctrl+p commands  ctrl+x leader ";
        buf.set_string(
            area.x + area.width.saturating_sub(hint.len() as u16),
            area.y,
            hint,
            Style::default().fg(theme.dim),
        );

        let list_area = Rect::new(
            area.x,
            area.y + 1,
            area.width,
            area.height.saturating_sub(6),
        );

        let filtered = self.filtered_sessions();
        if filtered.is_empty() {
            let msg = if self.sessions.is_empty() {
                "No sessions yet. Press ctrl+p and run `new`, or add sessions in the desktop app."
            } else {
                "No sessions match the filter."
            };
            buf.set_string(
                area.x + 2,
                area.y + 2,
                msg,
                Style::default().fg(theme.dim),
            );
        } else {
            let items: Vec<ListItem> = filtered
                .iter()
                .enumerate()
                .map(|(i, &idx)| {
                    let s = &self.sessions[idx];
                    let sess = &s.session;
                    let star = if sess.is_favorite { " ★" } else { "" };
                    let tags: String = s
                        .tags
                        .iter()
                        .map(|t| format!(" [{}]", t))
                        .collect::<Vec<_>>()
                        .join("");
                    let text = format!(
                        " {}  {}  {}@{}{}{}{}",
                        if i == self.selected { "▶" } else { " " },
                        sess.server_name,
                        sess.username,
                        sess.addr,
                        star,
                        tags,
                        ""
                    );
                    ListItem::new(Line::from(Span::raw(text)))
                })
                .collect();

            let mut state = ListState::default();
            state.select(Some(self.selected.min(filtered.len().saturating_sub(1))));
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(theme.border))
                        .title(Span::styled(
                            format!(" Sessions ({}) ", filtered.len()),
                            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
                        )),
                )
                .highlight_style(
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                );
            frame.render_stateful_widget(list, list_area, &mut state);
        }

        // Composer
        let composer_area = Rect::new(area.x, area.y + area.height - 4, area.width, 3);
        self.draw_composer(frame, composer_area);

        // Status bar
        let status_area = Rect::new(area.x, area.y + area.height - 1, area.width, 1);
        self.draw_home_status(frame, status_area);
    }

    fn draw_composer(&self, frame: &mut Frame, area: Rect) {
        let theme = &self.theme;
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .title(Span::styled(
                " filter / commands ",
                Style::default().fg(theme.dim),
            ));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let text = format!("> {}", self.filter);
        let p = Paragraph::new(Line::from(Span::raw(text)))
            .style(Style::default().fg(theme.fg))
            .wrap(Wrap { trim: false });
        frame.render_widget(p, inner);

        // Cursor: offset by the "> " prefix + character offset (approximate
        // wide-char aware position).
        let prefix = 2usize;
        let offset = self.filter[..char_boundary(&self.filter, self.filter_cursor)]
            .chars()
            .map(char_width)
            .sum::<usize>();
        let x = inner.x + (prefix + offset) as u16;
        frame.set_cursor_position((x.min(inner.x + inner.width.saturating_sub(1)), inner.y));
    }

    fn draw_home_status(&self, frame: &mut Frame, area: Rect) {
        let theme = &self.theme;
        let buf = frame.buffer_mut();
        let favs = self
            .sessions
            .iter()
            .filter(|s| s.session.is_favorite)
            .count();
        let text = format!(
            " {} sessions · {} favorites ",
            self.sessions.len(),
            favs
        );
        buf.set_string(
            area.x,
            area.y,
            &text,
            Style::default().fg(theme.dim).bg(Color::Black),
        );
        let hint = " enter: connect  esc: clear  ↑/↓: select ";
        buf.set_string(
            area.x + area.width.saturating_sub(hint.len() as u16),
            area.y,
            hint,
            Style::default().fg(theme.dim),
        );
    }

    fn draw_terminal(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let theme = &self.theme;
        let (term_area, status_area) = {
            let rows = Layout::vertical([
                Constraint::Length(1),
                Constraint::Min(3),
                Constraint::Length(1),
            ])
            .split(area);
            (rows[1], rows[2])
        };

        // Top bar
        {
            let buf = frame.buffer_mut();
            let mut line = format!(
                " {}  {} ",
                if let Some(t) = &self.terminal {
                    &t.server_name
                } else {
                    "connecting..."
                },
                if let Some(t) = &self.terminal {
                    &t.user_host
                } else {
                    ""
                }
            );
            if self.connecting {
                line.push_str("  (connecting…)");
            }
            buf.set_string(
                area.x,
                area.y,
                &line,
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
            );
            let hint = " ctrl+p commands · ctrl+x d disconnect ";
            buf.set_string(
                area.x + area.width.saturating_sub(hint.len() as u16),
                area.y,
                hint,
                Style::default().fg(theme.dim),
            );
        }

        if let Some(t) = &mut self.terminal {
            // Sync emulator + remote PTY with the widget size.
            let (w, h) = (term_area.width, term_area.height);
            if self.last_term_size != Some((w, h)) {
                let (pw, ph) = self.last_term_size.unwrap_or((0, 0));
                if (pw, ph) != (w, h) {
                    t.pane.resize(w, h);
                    let _ = self.manager.resize_ssh(
                        &SessionId::from(t.session_id.clone()),
                        w as u32,
                        h as u32,
                    );
                }
                self.last_term_size = Some((w, h));
            }
            t.pane.render(term_area, frame.buffer_mut(), true, t.connected);
        }

        // Status bar
        self.draw_term_status(frame, status_area);
    }

    fn draw_term_status(&self, frame: &mut Frame, area: Rect) {
        let theme = &self.theme;
        let buf = frame.buffer_mut();

        let (dot, dot_color, label) = match (&self.terminal, &self.status, self.connecting) {
            (Some(t), _, _) if !t.connected => (
                "●",
                theme.error,
                t.disconnect_reason
                    .clone()
                    .unwrap_or_else(|| "disconnected".into()),
            ),
            (_, _, true) => ("●", theme.warning, "connecting".into()),
            (Some(_), Some(s), _) => (
                "●",
                theme.ok,
                format!(
                    "cpu {:.0}%  mem {:.0}%  lat {}ms  load {:.2}",
                    s.cpu_usage,
                    s.mem_usage,
                    s.latency,
                    s.load_avg[0]
                ),
            ),
            (Some(_), None, _) => ("●", theme.ok, "connected".into()),
            _ => ("○", theme.dim, "no session".into()),
        };

        let text = format!(" {} {} ", dot, label);
        buf.set_string(area.x, area.y, &text, Style::default().fg(dot_color));
        let hint = " ctrl+x q quit · esc: shell ";
        buf.set_string(
            area.x + area.width.saturating_sub(hint.len() as u16),
            area.y,
            hint,
            Style::default().fg(theme.dim),
        );
    }

    // ------------------------------------------------------------------
    // Overlays
    // ------------------------------------------------------------------

    fn popup_area(area: Rect, pct_x: u16, pct_y: u16) -> Rect {
        let v = Layout::vertical([
            Constraint::Percentage((100 - pct_y) / 2),
            Constraint::Percentage(pct_y),
            Constraint::Percentage((100 - pct_y) / 2),
        ])
        .split(area);
        let h = Layout::horizontal([
            Constraint::Percentage((100 - pct_x) / 2),
            Constraint::Percentage(pct_x),
            Constraint::Percentage((100 - pct_x) / 2),
        ])
        .split(v[1]);
        h[1]
    }

    fn draw_command_bar(frame: &mut Frame, cb: &CommandBar, theme: &Theme) {
        let area = Self::popup_area(frame.area(), 60, 60);
        frame.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent))
            .title(Span::styled(
                " command palette ",
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
            ));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Input line
        frame.render_widget(
            Paragraph::new(Line::from(Span::raw(format!("> {}", cb.input))))
                .style(Style::default().fg(theme.fg)),
            Rect::new(inner.x, inner.y, inner.width, 1),
        );
        let offset = cb.input[..char_boundary(&cb.input, cb.cursor)]
            .chars()
            .map(char_width)
            .sum::<usize>();
        frame.set_cursor_position((
            (inner.x + 2 + offset as u16).min(inner.x + inner.width.saturating_sub(1)),
            inner.y,
        ));

        // Items
        let filtered = cb.filtered();
        let list_area = Rect::new(inner.x, inner.y + 2, inner.width, inner.height.saturating_sub(2));
        let items: Vec<ListItem> = filtered
            .iter()
            .enumerate()
            .map(|(i, &idx)| {
                let it = &cb.items[idx];
                let text = format!(
                    " {}  {:<24} {}",
                    if i == cb.selected { "▶" } else { " " },
                    it.label,
                    it.desc
                );
                ListItem::new(Line::from(Span::raw(text)))
            })
            .collect();
        let mut state = ListState::default();
        let sel = if filtered.is_empty() {
            None
        } else {
            Some(cb.selected.min(filtered.len() - 1))
        };
        state.select(sel);
        frame.render_stateful_widget(
            List::new(items).highlight_style(
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            list_area,
            &mut state,
        );
    }

    fn draw_dialog(frame: &mut Frame, dlg: &Dialog, theme: &Theme) {
        let area = match dlg {
            Dialog::NewSession(_) => Self::popup_area(frame.area(), 65, 70),
            _ => Self::popup_area(frame.area(), 55, 40),
        };
        frame.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.accent))
            .title(Span::styled(
                format!(" {} ", dlg.title()),
                Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
            ));
        let inner = block.inner(area);
        frame.render_widget(block, area);

        match dlg {
            Dialog::Password(p) => {
                let masked: String = p.input.chars().map(|_| '•').collect();
                let line = format!("> {}", masked);
                frame.render_widget(
                    Paragraph::new(Line::from(Span::raw(line)))
                        .style(Style::default().fg(theme.fg)),
                    Rect::new(inner.x, inner.y, inner.width, 1),
                );
                let offset = p.input[..char_boundary(&p.input, p.cursor)]
                    .chars()
                    .count();
                frame.set_cursor_position((
                    (inner.x + 2 + offset as u16).min(inner.x + inner.width.saturating_sub(1)),
                    inner.y,
                ));
                frame.render_widget(
                    Paragraph::new(Line::from(vec![
                        Span::styled(
                            " enter: submit   ",
                            Style::default().fg(theme.dim),
                        ),
                        Span::styled("esc: cancel", Style::default().fg(theme.dim)),
                    ])),
                    Rect::new(inner.x, inner.y + 2, inner.width, 1),
                );
            }
            Dialog::NewSession(f) => {
                for (i, field) in f.fields.iter().enumerate() {
                    let y = inner.y + i as u16 * 2;
                    if y + 1 >= inner.y + inner.height {
                        break;
                    }
                    let focused = i == f.focus;
                    let label_style = if focused {
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(theme.dim)
                    };
                    let buf = frame.buffer_mut();
                    buf.set_string(inner.x, y, format!(" {}", field.label), label_style);
                    let value: String = if field.masked {
                        field.value.chars().map(|_| '•').collect()
                    } else {
                        field.value.clone()
                    };
                    buf.set_string(
                        inner.x + 2,
                        y + 1,
                        format!(" {}", value),
                        Style::default()
                            .fg(theme.fg)
                            .bg(if focused { Color::DarkGray } else { Color::Black }),
                    );
                    if focused {
                        let offset = field.value[..char_boundary(&field.value, field.cursor)]
                            .chars()
                            .map(char_width)
                            .sum::<usize>();
                        frame.set_cursor_position((
                            (inner.x + 3 + offset as u16).min(inner.x + inner.width.saturating_sub(1)),
                            y + 1,
                        ));
                    }
                }
                let hint_y = inner.y + inner.height.saturating_sub(2);
                frame.render_widget(
                    Paragraph::new(Line::from(vec![
                        Span::styled(
                            " tab/↑↓: navigate   ",
                            Style::default().fg(theme.dim),
                        ),
                        Span::styled("enter: next / save", Style::default().fg(theme.dim)),
                    ])),
                    Rect::new(inner.x, hint_y, inner.width, 1),
                );
            }
            Dialog::Help => {
                let lines = [
                    "Home page",
                    "  ↑/↓         select session",
                    "  enter       connect",
                    "  type        filter sessions",
                    "  esc         clear filter",
                    "  ctrl+p      command palette",
                    "Terminal page",
                    "  any key     sent to remote shell",
                    "  ctrl+p      command palette",
                    "Leader key (ctrl+x, then)",
                    "  q           quit     h  help",
                    "  l           sessions d  disconnect",
                    "  n           new      r  refresh",
                ];
                for (i, l) in lines.iter().enumerate() {
                    let y = inner.y + i as u16;
                    if y >= inner.y + inner.height {
                        break;
                    }
                    let style = if l.starts_with("  ") {
                        Style::default().fg(theme.fg)
                    } else {
                        Style::default()
                            .fg(theme.accent)
                            .add_modifier(Modifier::BOLD)
                    };
                    frame.buffer_mut().set_string(inner.x, y, l, style);
                }
            }
            Dialog::Quit => {
                frame.render_widget(
                    Paragraph::new(Line::from(vec![
                        Span::raw("Quit NexaShell TUI?  "),
                        Span::styled(
                            "enter: quit   esc: cancel",
                            Style::default().fg(theme.dim),
                        ),
                    ]))
                    .style(Style::default().fg(theme.fg)),
                    Rect::new(inner.x, inner.y, inner.width, 1),
                );
            }
            Dialog::Notice(msg) => {
                let p = Paragraph::new(Line::from(Span::raw(msg.clone())))
                    .style(Style::default().fg(theme.fg))
                    .wrap(Wrap { trim: true });
                frame.render_widget(p, Rect::new(inner.x, inner.y, inner.width, inner.height - 2));
                frame.render_widget(
                    Paragraph::new(Line::from(Span::styled(
                        " esc: dismiss ",
                        Style::default().fg(theme.dim),
                    ))),
                    Rect::new(inner.x, inner.y + inner.height - 2, inner.width, 1),
                );
            }
        }
    }
}

// ============================================================================
// Input helpers
// ============================================================================

fn char_boundary(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

fn char_width(c: char) -> usize {
    unicode_width::UnicodeWidthChar::width(c).unwrap_or(0)
}

fn insert_char(s: &mut String, cursor: &mut usize, c: char) {
    let idx = char_boundary(s, *cursor);
    s.insert(idx, c);
    *cursor += 1;
}

fn delete_backward(s: &mut String, cursor: &mut usize) {
    if *cursor == 0 {
        return;
    }
    let idx = char_boundary(s, *cursor);
    let prev = char_boundary(s, *cursor - 1);
    s.drain(prev..idx);
    *cursor -= 1;
}

fn delete_forward(s: &mut String, cursor: &mut usize) {
    let idx = char_boundary(s, *cursor);
    let next = char_boundary(s, *cursor + 1);
    if idx < s.len() {
        s.drain(idx..next);
    }
}

fn delete_word_backward(s: &mut String, cursor: &mut usize) {
    let idx = char_boundary(s, *cursor);
    if idx == 0 {
        return;
    }
    // Skip trailing whitespace, then the word itself.
    let mut chars: Vec<char> = s[..idx].chars().collect();
    while let Some(&c) = chars.last() {
        if c.is_whitespace() {
            chars.pop();
        } else {
            break;
        }
    }
    while let Some(&c) = chars.last() {
        if !c.is_whitespace() {
            chars.pop();
        } else {
            break;
        }
    }
    let new_idx = chars.len();
    let orig = char_boundary(s, *cursor);
    s.drain(new_idx..orig);
    *cursor = new_idx;
}

// ============================================================================
// Key → terminal escape sequence
// ============================================================================

fn mod_param(key: &KeyEvent) -> u8 {
    let mut m = 1;
    if key.modifiers.contains(KeyModifiers::SHIFT) {
        m += 1;
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        m += 2;
    }
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        m += 4;
    }
    m
}

fn csi_final(final_byte: char, key: &KeyEvent) -> String {
    let m = mod_param(key);
    if m == 1 {
        format!("\x1b[{}", final_byte)
    } else {
        format!("\x1b[1;{}{}", m, final_byte)
    }
}

fn csi_tilde(n: u8, key: &KeyEvent) -> String {
    let m = mod_param(key);
    if m == 1 {
        format!("\x1b[{}~", n)
    } else {
        format!("\x1b[{};{}~", n, m)
    }
}

pub fn key_to_escape(key: KeyEvent) -> Option<String> {
    let m = key.modifiers;
    let has_alt = m.contains(KeyModifiers::ALT);
    let has_ctrl = m.contains(KeyModifiers::CONTROL);
    let has_shift = m.contains(KeyModifiers::SHIFT);

    match key.code {
        KeyCode::Char(c) => {
            if has_ctrl {
                if c == ' ' {
                    return Some("\x00".into());
                }
                let lower = c.to_ascii_lowercase();
                if lower.is_ascii_lowercase() {
                    return Some(((lower as u8 - b'a' + 1) as char).to_string());
                }
                return match c {
                    '[' => Some("\x1b".into()),
                    '\\' => Some("\x1c".into()),
                    ']' => Some("\x1d".into()),
                    '^' => Some("\x1e".into()),
                    '_' => Some("\x1f".into()),
                    _ => None,
                };
            }
            if has_alt {
                return Some(format!("\x1b{}", c));
            }
            Some(c.to_string())
        }
        KeyCode::Enter => Some("\r".into()),
        KeyCode::Backspace => Some("\x7f".into()),
        KeyCode::Tab => Some(if has_shift { "\x1b[Z".into() } else { "\t".into() }),
        KeyCode::Esc => Some("\x1b".into()),
        KeyCode::Left => Some(csi_final('D', &key)),
        KeyCode::Right => Some(csi_final('C', &key)),
        KeyCode::Up => Some(csi_final('A', &key)),
        KeyCode::Down => Some(csi_final('B', &key)),
        KeyCode::Home => Some(csi_final('H', &key)),
        KeyCode::End => Some(csi_final('F', &key)),
        KeyCode::PageUp => Some(csi_tilde(5, &key)),
        KeyCode::PageDown => Some(csi_tilde(6, &key)),
        KeyCode::Insert => Some(csi_tilde(2, &key)),
        KeyCode::Delete => Some(csi_tilde(3, &key)),
        KeyCode::F(n) => {
            let m = mod_param(&key);
            if (1..=4).contains(&n) {
                let letter = match n {
                    1 => 'P',
                    2 => 'Q',
                    3 => 'R',
                    _ => 'S',
                };
                if m == 1 {
                    Some(format!("\x1bO{}", letter))
                } else {
                    Some(format!("\x1b[1;{}{}", m, letter))
                }
            } else {
                let tilde = match n {
                    5 => 15,
                    6 => 17,
                    7 => 18,
                    8 => 19,
                    9 => 20,
                    10 => 21,
                    11 => 23,
                    _ => 24,
                };
                Some(csi_tilde(tilde, &key))
            }
        }
        KeyCode::Null => None,
        _ => None,
    }
}