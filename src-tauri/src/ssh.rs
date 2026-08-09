use crate::common::{OutputChunk, SessionId};
use serde::Serialize;
use ssh2::{HashType, OpenFlags, OpenType, PtyModes, PtyModeOpcode, ExtensiblePtyModeOpcode, Session};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tauri::{Emitter, Listener};
use thiserror::Error;
use tokio::sync::mpsc;

const MAX_CACHED_INITIAL_CHUNKS: usize = 200;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug, Error, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SshError {
    #[error("Failed to connect to {host}:{port} - {reason}")]
    ConnectionFailed {
        host: String,
        port: u16,
        reason: String,
    },

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("SSH operation failed: {0}")]
    OperationFailed(String),

    #[error("Channel error: {0}")]
    ChannelError(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("State lock poisoned: {0}")]
    LockPoisoned(String),

    #[error("Task join error: {0}")]
    TaskError(String),

    #[error("Host key verification failed for {host}: {reason}")]
    HostKeyVerificationFailed { host: String, reason: String },
}

// ============================================================================
// Constants
// ============================================================================

const SSH_BUFFER_SIZE: usize = 4096;
const INITIAL_BATCH_SIZE_THRESHOLD: usize = 200;
const INITIAL_BATCH_TIME_MS: u64 = 100;
const INITIAL_BUFFERING_TIMEOUT_MS: u64 = 2000;
const NORMAL_BATCH_SIZE_THRESHOLD: usize = 1024;
const NORMAL_BATCH_TIME_MS: u64 = 20;

// ============================================================================
// Host Key Verification (TOFU — Trust On First Use)
// ============================================================================

fn known_hosts_path() -> Result<std::path::PathBuf, String> {
    let data_dir = dirs::data_dir()
        .ok_or_else(|| "Failed to determine app data directory".to_string())?
        .join("NexaShell");
    std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
    Ok(data_dir.join("known_hosts"))
}

/// Returns the SHA256 fingerprint of the session's host key, base64-encoded
/// (without the `SHA256:` prefix to match OpenSSH's visual representation
/// minus the prefix).
fn host_key_fingerprint(sess: &Session) -> Result<String, String> {
    let key = sess
        .host_key_hash(HashType::Sha256)
        .ok_or_else(|| "No host key available".to_string())?;
    Ok(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        key,
    ))
}

/// Verify the host key for `host` using a TOFU policy.
///
/// - First connection: store the fingerprint and accept.
/// - Subsequent connections: reject if the fingerprint differs (possible MITM).
fn verify_host_key(sess: &Session, host: &str) -> Result<(), SshError> {
    let fingerprint = host_key_fingerprint(sess).map_err(|e| {
        SshError::HostKeyVerificationFailed {
            host: host.to_string(),
            reason: e,
        }
    })?;

    let path = known_hosts_path().map_err(|e| SshError::HostKeyVerificationFailed {
        host: host.to_string(),
        reason: e,
    })?;

    let mut known: HashMap<String, String> = if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(content) => content
                .lines()
                .filter_map(|line| {
                    let mut parts = line.splitn(2, ' ');
                    let h = parts.next()?;
                    let fp = parts.next()?;
                    Some((h.to_string(), fp.to_string()))
                })
                .collect(),
            Err(e) => {
                log::warn!("Failed to read known_hosts ({}): {}", path.display(), e);
                HashMap::new()
            }
        }
    } else {
        HashMap::new()
    };

    if let Some(existing) = known.get(host) {
        if existing != &fingerprint {
            return Err(SshError::HostKeyVerificationFailed {
                host: host.to_string(),
                reason: format!(
                    "Host key mismatch!\nStored: {}\nGot:    {}",
                    existing, fingerprint
                ),
            });
        }
        return Ok(());
    }

    // First time seeing this host — store and accept
    known.insert(host.to_string(), fingerprint);
    let serialized: String = known
        .iter()
        .map(|(h, fp)| format!("{} {}\n", h, fp))
        .collect();
    if let Err(e) = std::fs::write(&path, serialized) {
        log::warn!("Failed to write known_hosts: {}", e);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }

    Ok(())
}

// ============================================================================
// Data Structures
// ============================================================================

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    pub cpu_usage: f64,
    pub mem_usage: f64,
    pub mem_total: u64,
    pub mem_used: u64,
    pub mem_avail: u64,
    pub swap_usage: f64,
    pub swap_total: u64,
    pub swap_used: u64,
    pub disk_usage: f64,
    pub disk_total: u64,
    pub disk_used: u64,
    pub disk_avail: u64,
    pub net_up: f64,
    pub net_down: f64,
    pub latency: u32,
    pub load_avg: [f64; 3],
    pub uptime: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadProgress {
    pub task_id: String,
    pub session_id: String,
    pub progress: f64,
    pub uploaded_bytes: u64,
    pub total_bytes: u64,
    pub status: String,
    pub message: String,
    pub speed: f64,
    pub error: Option<String>,
}

/// Authentication credentials retained for establishing secondary sessions
/// (SFTP, monitoring, path probing) without affecting the interactive shell.
#[derive(Clone)]
struct SshAuth {
    username: String,
    password: String,
    private_key_path: Option<String>,
    key_passphrase: Option<String>,
}

pub struct SshChannelInfo {
    pub handle: Option<tokio::task::JoinHandle<()>>,
    pub status_handle: Option<tokio::task::JoinHandle<()>>,
    pub input_sender: mpsc::UnboundedSender<String>,
    pub stop_flag: Arc<AtomicBool>,
    /// Monotonically increasing sequence number for output chunks.
    /// Kept for future replay/sync features.
    #[allow(dead_code)]
    pub next_seq: Arc<AtomicU64>,
    pub initial_outputs: Arc<tokio::sync::Mutex<Vec<OutputChunk>>>,
    pub refresh_interval: Arc<AtomicU64>,

    /// Main interactive session (non-blocking, used by the I/O task only).
    #[allow(dead_code)]
    pub sess_arc: Arc<tokio::sync::Mutex<Session>>,

    /// Auxiliary blocking session used for SFTP, monitoring, and path probing.
    /// This runs on its own TCP connection and stays in blocking mode,
    /// eliminating the blocking-mode race that previously froze terminals
    /// during file uploads.
    pub helper_sess: Arc<tokio::sync::Mutex<Session>>,

    pub input_listener_id: Option<tauri::EventId>,
    pub resize_listener_id: Option<tauri::EventId>,
    pub app_handle: Option<tauri::AppHandle>,
}

fn build_pty_modes() -> PtyModes {
    let mut modes = PtyModes::new();

    modes.set_character(PtyModeOpcode::VINTR, Some(3u8 as char));
    modes.set_character(PtyModeOpcode::VQUIT, Some(28u8 as char));
    modes.set_character(PtyModeOpcode::VERASE, Some(127u8 as char));
    modes.set_character(PtyModeOpcode::VKILL, Some(21u8 as char));
    modes.set_character(PtyModeOpcode::VEOF, Some(4u8 as char));
    modes.set_character(PtyModeOpcode::VEOL, None);
    modes.set_character(PtyModeOpcode::VEOL2, None);
    modes.set_character(PtyModeOpcode::VSTART, None);
    modes.set_character(PtyModeOpcode::VSTOP, None);
    modes.set_character(PtyModeOpcode::VSUSP, Some(26u8 as char));
    modes.set_character(PtyModeOpcode::VDSUSP, None);
    modes.set_character(PtyModeOpcode::VREPRINT, Some(18u8 as char));
    modes.set_character(PtyModeOpcode::VWERASE, Some(23u8 as char));
    modes.set_character(PtyModeOpcode::VLNEXT, Some(22u8 as char));

    modes.set_boolean(PtyModeOpcode::IGNPAR, false);
    modes.set_boolean(PtyModeOpcode::PARMRK, false);
    modes.set_boolean(PtyModeOpcode::INPCK, false);
    modes.set_boolean(PtyModeOpcode::ISTRIP, false);
    modes.set_boolean(PtyModeOpcode::INLCR, false);
    modes.set_boolean(PtyModeOpcode::IGNCR, false);
    modes.set_boolean(PtyModeOpcode::ICRNL, true);
    modes.set_boolean(PtyModeOpcode::IUCLC, false);

    modes.set_boolean(PtyModeOpcode::IXON, false);
    modes.set_boolean(PtyModeOpcode::IXANY, false);
    modes.set_boolean(PtyModeOpcode::IXOFF, false);
    modes.set_boolean(PtyModeOpcode::IMAXBEL, false);

    const IUTF8: u8 = 42;
    modes.set_boolean(ExtensiblePtyModeOpcode::Extended(IUTF8), true);

    modes.set_boolean(PtyModeOpcode::ISIG, true);
    modes.set_boolean(PtyModeOpcode::ECHO, false);
    modes.set_boolean(PtyModeOpcode::ICANON, false);
    modes.set_boolean(PtyModeOpcode::ECHOE, true);
    modes.set_boolean(PtyModeOpcode::ECHOK, true);
    modes.set_boolean(PtyModeOpcode::ECHONL, false);

    modes.set_boolean(PtyModeOpcode::OPOST, true);
    modes.set_boolean(PtyModeOpcode::ONLCR, true);

    modes.set_u32(PtyModeOpcode::TTY_OP_ISPEED, 38400);
    modes.set_u32(PtyModeOpcode::TTY_OP_OSPEED, 38400);

    modes
}

#[derive(Default)]
pub struct SshManager {
    channels: Arc<RwLock<HashMap<SessionId, SshChannelInfo>>>,
}

impl SshManager {
    pub async fn connect_ssh(
        &self,
        app_handle: Option<tauri::AppHandle>,
        session_id: SessionId,
        ip: String,
        port: u16,
        username: String,
        password: String,
        private_key_path: Option<String>,
        key_passphrase: Option<String>,
        cols: u32,
        rows: u32,
    ) -> Result<(), SshError> {
        let channels_arc = Arc::clone(&self.channels);

        let addr = format!("{}:{}", ip, port);
        let host_for_err = addr.clone();
        let auth = SshAuth {
            username: username.clone(),
            password,
            private_key_path: private_key_path.clone(),
            key_passphrase,
        };

        // 1. Establish both sessions (interactive + helper) on separate TCP
        //    connections. Blocking work runs on spawn_blocking.
        let connect_res = tokio::task::spawn_blocking(move || {
            Self::establish_sessions(
                &addr,
                &host_for_err,
                port,
                &auth,
                cols,
                rows,
            )
        })
        .await
        .map_err(|e| SshError::TaskError(e.to_string()))?;

        let (main_sess, main_channel, helper_sess) = connect_res?;

        // 2. Setup communication channels
        let (input_sender, input_receiver) = mpsc::unbounded_channel::<String>();
        let stop_flag = Arc::new(AtomicBool::new(false));
        let next_seq = Arc::new(AtomicU64::new(1));
        let initial_outputs = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let refresh_interval = Arc::new(AtomicU64::new(3000));

        let channel_arc = Arc::new(tokio::sync::Mutex::new(main_channel));
        let sess_arc = Arc::new(tokio::sync::Mutex::new(main_sess));
        let helper_arc = Arc::new(tokio::sync::Mutex::new(helper_sess));

        let (input_listener_id, resize_listener_id) = if let Some(h) = &app_handle {
            let input_id = Self::register_input_listener(h, &session_id, &input_sender, &stop_flag);
            let resize_id = Self::register_resize_listener(h, &session_id, &channel_arc, &stop_flag);
            (Some(input_id), Some(resize_id))
        } else {
            (None, None)
        };

        let handle = Self::spawn_io_task(
            channel_arc,
            sess_arc.clone(),
            stop_flag.clone(),
            next_seq.clone(),
            initial_outputs.clone(),
            input_receiver,
            app_handle.clone(),
            session_id.clone(),
        );

        let status_handle = Self::spawn_monitoring_task(
            app_handle.clone(),
            session_id.clone(),
            helper_arc.clone(),
            stop_flag.clone(),
            refresh_interval.clone(),
        );

        {
            let mut channels = channels_arc
                .write()
                .map_err(|e| SshError::LockPoisoned(e.to_string()))?;
            channels.insert(
                session_id,
                SshChannelInfo {
                    handle: Some(handle),
                    status_handle: Some(status_handle),
                    input_sender,
                    stop_flag,
                    next_seq,
                    initial_outputs,
                    refresh_interval,
                    sess_arc,
                    helper_sess: helper_arc,
                    input_listener_id,
                    resize_listener_id,
                    app_handle: app_handle.clone(),
                },
            );
        }

        Ok(())
    }

    /// Open, handshake, verify host key, and authenticate two independent SSH
    /// sessions over separate TCP connections. Returns `(main, channel, helper)`.
    fn establish_sessions(
        addr: &str,
        host_for_err: &str,
        port: u16,
        auth: &SshAuth,
        cols: u32,
        rows: u32,
    ) -> Result<(Session, ssh2::Channel, Session), SshError> {
        use std::net::ToSocketAddrs;

        let open_tcp = || -> Result<TcpStream, SshError> {
            let socket_addr = addr
                .to_socket_addrs()
                .map_err(|e| SshError::ConnectionFailed {
                    host: host_for_err.to_string(),
                    port,
                    reason: format!("Failed to resolve address: {}", e),
                })?
                .next()
                .ok_or_else(|| SshError::ConnectionFailed {
                    host: host_for_err.to_string(),
                    port,
                    reason: "No addresses found".to_string(),
                })?;

            let tcp = TcpStream::connect_timeout(&socket_addr, Duration::from_secs(30)).map_err(
                |e| SshError::ConnectionFailed {
                    host: host_for_err.to_string(),
                    port,
                    reason: e.to_string(),
                },
            )?;

            // TCP read timeout so a hung server cannot block handshake forever.
            let _ = tcp.set_read_timeout(Some(Duration::from_secs(60)));

            Ok(tcp)
        };

        let make_session = |tcp: TcpStream| -> Result<Session, SshError> {
            let mut sess = Session::new()
                .map_err(|e| SshError::OperationFailed(format!("Failed to create session: {}", e)))?;
            sess.set_tcp_stream(tcp);
            sess.handshake()
                .map_err(|e| SshError::OperationFailed(format!("Handshake failed: {}", e)))?;
            // libssh2 keepalive every 30 seconds
            sess.set_keepalive(true, 30);
            Ok(sess)
        };

        let authenticate = |sess: &Session| -> Result<(), SshError> {
            let mut authenticated = false;
            if let Some(ref key_path) = auth.private_key_path {
                let path = Path::new(key_path);
                if path.exists() {
                    let key_result = if let Some(ref passphrase) = auth.key_passphrase {
                        sess.userauth_pubkey_file(
                            &auth.username,
                            None,
                            path,
                            Some(passphrase),
                        )
                    } else {
                        sess.userauth_pubkey_file(&auth.username, None, path, None)
                    };
                    match key_result {
                        Ok(()) => authenticated = sess.authenticated(),
                        Err(e) => {
                            log::warn!(
                                "Public key auth failed for '{}': {}; trying password",
                                auth.username,
                                e
                            );
                        }
                    }
                } else {
                    log::warn!("Private key file not found: {}", key_path);
                }
            }

            if !authenticated {
                sess.userauth_password(&auth.username, &auth.password)
                    .map_err(|e| {
                        SshError::AuthenticationFailed(format!(
                            "Authentication failed: {}",
                            e
                        ))
                    })?;
            }

            if !sess.authenticated() {
                return Err(SshError::AuthenticationFailed(
                    "Authentication failed".to_string(),
                ));
            }
            Ok(())
        };

        // Main session
        let main_tcp = open_tcp()?;
        let main_sess = make_session(main_tcp)?;
        verify_host_key(&main_sess, host_for_err)?;
        authenticate(&main_sess)?;

        let mut channel = main_sess
            .channel_session()
            .map_err(|e| SshError::ChannelError(format!("Create channel failed: {}", e)))?;
        let pty_modes = build_pty_modes();
        channel
            .request_pty("xterm-256color", Some(pty_modes), Some((cols, rows, 0, 0)))
            .map_err(|e| SshError::ChannelError(format!("Failed to request PTY: {}", e)))?;
        channel
            .shell()
            .map_err(|e| SshError::ChannelError(format!("Failed to start shell: {}", e)))?;

        // Main session is non-blocking so the I/O task never stalls.
        main_sess.set_blocking(false);

        // Helper session (blocking, for SFTP/monitoring/probing)
        let helper_tcp = open_tcp()?;
        let helper_sess = make_session(helper_tcp)?;
        // Verify the host key on the helper connection too: each connection is
        // resolved independently (DNS rotation could pin them to different
        // endpoints), so skipping verification here would let an attacker MITM
        // SFTP uploads / monitoring / probing traffic.
        verify_host_key(&helper_sess, host_for_err)?;
        authenticate(&helper_sess)?;
        // helper_sess stays in blocking mode (default)

        Ok((main_sess, channel, helper_sess))
    }

    fn register_input_listener(
        app_handle: &tauri::AppHandle,
        session_id: &SessionId,
        input_sender: &mpsc::UnboundedSender<String>,
        stop_flag: &Arc<AtomicBool>,
    ) -> tauri::EventId {
        let event_name = format!("ssh-input-{}", session_id.0);
        let input_tx = input_sender.clone();
        let task_stop = stop_flag.clone();

        app_handle.listen(&event_name, move |event: tauri::Event| {
            if task_stop.load(Ordering::SeqCst) {
                return;
            }

            #[derive(serde::Deserialize)]
            struct InputPayload {
                input: String,
            }

            if let Ok(payload) = serde_json::from_str::<InputPayload>(event.payload()) {
                let _ = input_tx.send(payload.input);
            }
        })
    }

    fn register_resize_listener(
        app_handle: &tauri::AppHandle,
        session_id: &SessionId,
        channel_arc: &Arc<tokio::sync::Mutex<ssh2::Channel>>,
        stop_flag: &Arc<AtomicBool>,
    ) -> tauri::EventId {
        let resize_event_name = format!("ssh-resize-{}", session_id.0);
        let task_channel = channel_arc.clone();
        let task_stop = stop_flag.clone();

        app_handle.listen(&resize_event_name, move |event: tauri::Event| {
            if task_stop.load(Ordering::SeqCst) {
                return;
            }

            #[derive(serde::Deserialize)]
            struct ResizePayload {
                cols: u32,
                rows: u32,
            }

            if let Ok(payload) = serde_json::from_str::<ResizePayload>(event.payload()) {
                let task_channel_clone = task_channel.clone();
                let _ = tokio::spawn(async move {
                    let mut ch = task_channel_clone.lock().await;
                    if let Err(e) = ch.request_pty_size(payload.cols, payload.rows, None, None) {
                        log::warn!("Resize failed: {}", e);
                    }
                });
            }
        })
    }

    fn spawn_io_task(
        channel_arc: Arc<tokio::sync::Mutex<ssh2::Channel>>,
        sess_arc: Arc<tokio::sync::Mutex<Session>>,
        stop_flag: Arc<AtomicBool>,
        next_seq: Arc<AtomicU64>,
        initial_outputs: Arc<tokio::sync::Mutex<Vec<OutputChunk>>>,
        mut input_receiver: mpsc::UnboundedReceiver<String>,
        app_handle: Option<tauri::AppHandle>,
        session_id: SessionId,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut buffer = [0u8; SSH_BUFFER_SIZE];
            let mut pending_output = String::new();
            let mut last_emit = std::time::Instant::now();
            let mut seen_first_output = false;
            let initial_buffering_start = std::time::Instant::now();
            let mut in_initial_buffering = true;
            let mut idle_reads = 0u32;

            loop {
                if stop_flag.load(Ordering::SeqCst) {
                    break;
                }

                // Process user input FIRST for low-latency IME response.
                // The main session is non-blocking, so a full channel/TCP
                // buffer surfaces as WouldBlock — retry with backoff instead of
                // dropping the remainder of the input.
                while let Ok(input) = input_receiver.try_recv() {
                    let mut sess_lock = sess_arc.lock().await;
                    let mut ch = channel_arc.lock().await;
                    let bytes = input.as_bytes();
                    let mut written = 0usize;
                    let mut backoff = Duration::from_millis(1);

                    while written < bytes.len() {
                        if stop_flag.load(Ordering::SeqCst) {
                            break;
                        }
                        match ch.write(&bytes[written..]) {
                            Ok(n) => {
                                written += n;
                                backoff = Duration::from_millis(1);
                            }
                            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                                // Buffer full — release the locks while waiting
                                // so reads/resizes can proceed, then retry.
                                drop(ch);
                                drop(sess_lock);
                                tokio::time::sleep(backoff).await;
                                sess_lock = sess_arc.lock().await;
                                ch = channel_arc.lock().await;
                                if backoff < Duration::from_millis(50) {
                                    backoff *= 2;
                                }
                            }
                            Err(e) => {
                                log::error!("[SSH I/O] write failed: {}", e);
                                break;
                            }
                        }
                    }

                    if let Err(e) = ch.flush() {
                        if e.kind() != std::io::ErrorKind::WouldBlock {
                            log::error!("[SSH I/O] flush failed: {}", e);
                        }
                    }
                }

                let read_result = {
                    let _sess_lock = sess_arc.lock().await;
                    let mut ch = channel_arc.lock().await;
                    match ch.read(&mut buffer) {
                        Ok(0) => Some(Err("Connection closed by remote host".to_string())),
                        Ok(n) => Some(Ok(n)),
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => None,
                        Err(e) => Some(Err(format!("Read error: {}", e))),
                    }
                };

                match read_result {
                    Some(Ok(n)) => {
                        idle_reads = 0;
                        pending_output.push_str(&String::from_utf8_lossy(&buffer[..n]));
                    }
                    Some(Err(e)) => {
                        log::warn!("[SSH {}] I/O loop ending: {}", session_id.0, e);
                        if let Some(h) = &app_handle {
                            let _ = h.emit(
                                &format!("ssh-disconnected-{}", session_id.0),
                                serde_json::json!({ "reason": e }),
                            );
                        }
                        stop_flag.store(true, Ordering::SeqCst);
                        break;
                    }
                    None => {
                        idle_reads += 1;
                        let sleep_ms = if idle_reads > 5 { 10 } else { 1 };
                        tokio::time::sleep(Duration::from_millis(sleep_ms)).await;
                    }
                }

                let in_initial = in_initial_buffering
                    && initial_buffering_start.elapsed()
                        > Duration::from_millis(INITIAL_BUFFERING_TIMEOUT_MS);

                if in_initial {
                    in_initial_buffering = false;
                }

                let (size_threshold, time_threshold_ms) =
                    if in_initial_buffering && !seen_first_output {
                        (INITIAL_BATCH_SIZE_THRESHOLD, INITIAL_BATCH_TIME_MS)
                    } else {
                        (NORMAL_BATCH_SIZE_THRESHOLD, NORMAL_BATCH_TIME_MS)
                    };

                if (!pending_output.is_empty() && in_initial)
                    || (!pending_output.is_empty()
                        && (pending_output.len() > size_threshold
                            || last_emit.elapsed() > Duration::from_millis(time_threshold_ms)))
                {
                    let seq = next_seq.fetch_add(1, Ordering::SeqCst);
                    let output = std::mem::take(&mut pending_output);
                    let chunk = OutputChunk::new(seq, output);

                    if in_initial_buffering {
                        let mut cache = initial_outputs.lock().await;
                        if cache.len() < MAX_CACHED_INITIAL_CHUNKS {
                            cache.push(chunk.clone());
                        }
                    }

                    if let Some(h) = &app_handle {
                        let _ = h.emit(&format!("ssh-output-{}", session_id.0), &chunk);
                    }

                    last_emit = std::time::Instant::now();
                    seen_first_output = true;
                }
            }
        })
    }

    fn spawn_monitoring_task(
        app_handle: Option<tauri::AppHandle>,
        session_id: SessionId,
        helper_sess: Arc<tokio::sync::Mutex<Session>>,
        stop_flag: Arc<AtomicBool>,
        refresh_interval: Arc<AtomicU64>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut last_net_read: Option<(f64, f64, std::time::Instant)> = None;
            let mut last_cpu_read: Option<(u64, u64)> = None;

            loop {
                if stop_flag.load(Ordering::SeqCst) {
                    break;
                }

                let start_time = std::time::Instant::now();
                let helper = helper_sess.clone();

                // All blocking SSH operations run on spawn_blocking so we never
                // stall a tokio worker thread.
                let status_res = tokio::task::spawn_blocking(move || {
                    let sess = helper.blocking_lock();
                    let mut channel = sess.channel_session().map_err(|e| {
                        SshError::ChannelError(format!("Create status channel failed: {}", e))
                    })?;
                    Self::fetch_server_status_blocking(&mut channel, last_cpu_read)
                })
                .await;

                let latency = start_time.elapsed().as_millis() as u32;
                let status_res = match status_res {
                    Ok(inner) => inner,
                    Err(e) => {
                        log::debug!("[monitor] join error: {}", e);
                        tokio::time::sleep(Duration::from_millis(
                            refresh_interval.load(Ordering::SeqCst),
                        ))
                        .await;
                        continue;
                    }
                };

                if let Ok((mut status, current_cpu_raw)) = status_res {
                    let now = std::time::Instant::now();
                    status.latency = latency;
                    last_cpu_read = Some(current_cpu_raw);

                    let current_rx = status.net_down;
                    let current_tx = status.net_up;

                    if let Some((prev_rx, prev_tx, prev_time)) = last_net_read {
                        let duration = now.duration_since(prev_time).as_secs_f64();
                        if duration > 0.0 {
                            let rx_diff = if current_rx >= prev_rx {
                                current_rx - prev_rx
                            } else {
                                0.0
                            };
                            let tx_diff = if current_tx >= prev_tx {
                                current_tx - prev_tx
                            } else {
                                0.0
                            };
                            status.net_down = rx_diff / duration;
                            status.net_up = tx_diff / duration;
                        }
                    } else {
                        status.net_down = 0.0;
                        status.net_up = 0.0;
                    }

                    last_net_read = Some((current_rx, current_tx, now));

                    if let Some(h) = &app_handle {
                        let _ = h.emit(&format!("ssh-status-{}", session_id.0), &status);
                    }
                }

                let interval = refresh_interval.load(Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(interval)).await;
            }
        })
    }

    /// Fetches server metrics using a single blocking exec channel. The caller
    /// must hold the helper session lock and run this on a blocking thread.
    fn fetch_server_status_blocking(
        channel: &mut ssh2::Channel,
        last_cpu: Option<(u64, u64)>,
    ) -> Result<(ServerStatus, (u64, u64)), SshError> {
        // Linux metrics command. We probe uname first; for non-Linux hosts we
        // still attempt the Linux command but degrade gracefully rather than
        // failing with a cryptic parse error.
        let cmd = "LC_ALL=C awk '/^cpu / {print $2+$3+$4+$5+$6+$7+$8, $5}' /proc/stat 2>/dev/null || echo '0 0'; \
                   LC_ALL=C free -b 2>/dev/null | awk '/Mem:/{print $2,$3,$7} /Swap:/{print $2,$3}' || echo '0 0 0\n0 0'; \
                   LC_ALL=C df -PB1 -x tmpfs -x devtmpfs -x overlay 2>/dev/null | awk 'NR>1 && !seen[$1]++ {t+=$2;u+=$3;a+=$4} END{print t,u,a}' || echo '0 0 0'; \
                   LC_ALL=C cat /proc/net/dev 2>/dev/null | awk 'NR>2 && $1!=\"lo:\"{rx+=$2; tx+=$10} END{print rx+0,tx+0}' || echo '0 0'; \
                   LC_ALL=C cat /proc/loadavg 2>/dev/null | awk '{print $1,$2,$3}' || echo '0 0 0'; \
                   LC_ALL=C uptime -p 2>/dev/null || echo 'up unknown'";

        channel
            .exec(cmd)
            .map_err(|e| SshError::ChannelError(format!("exec failed: {}", e)))?;

        let mut output = String::new();
        channel
            .read_to_string(&mut output)
            .map_err(|e| SshError::OperationFailed(format!("read failed: {}", e)))?;
        let _ = channel.wait_close();

        let lines: Vec<&str> = output
            .lines()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if lines.len() < 6 {
            return Err(SshError::OperationFailed(format!(
                "Unsupported platform or invalid status output (lines: {})",
                lines.len()
            )));
        }

        let cpu_parts: Vec<u64> = lines[0]
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        let (current_cpu_total, current_cpu_idle) = if cpu_parts.len() == 2 {
            (cpu_parts[0], cpu_parts[1])
        } else {
            (0, 0)
        };

        let cpu_usage = if let Some((prev_total, prev_idle)) = last_cpu {
            let diff_total = current_cpu_total.saturating_sub(prev_total);
            let diff_idle = current_cpu_idle.saturating_sub(prev_idle);
            if diff_total > 0 {
                (100.0 * (1.0 - (diff_idle as f64 / diff_total as f64))).clamp(0.0, 100.0)
            } else {
                0.0
            }
        } else {
            0.0
        };

        let mem_parts: Vec<u64> = lines[1]
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        let (mem_total, mem_used, mem_avail) = if mem_parts.len() >= 2 {
            let total = mem_parts[0];
            let used = mem_parts[1];
            let avail = if mem_parts.len() >= 3 && mem_parts[2] > 0 {
                mem_parts[2]
            } else {
                total.saturating_sub(used)
            };
            (total, used, avail)
        } else {
            (1, 0, 1)
        };
        let mem_usage =
            (100.0 * (1.0 - (mem_avail as f64 / mem_total as f64))).clamp(0.0, 100.0);

        let swap_parts: Vec<u64> = lines[2]
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        let (swap_total, swap_used) = if swap_parts.len() == 2 {
            (swap_parts[0], swap_parts[1])
        } else {
            (0, 0)
        };
        let swap_usage = if swap_total > 0 {
            ((swap_used as f64 / swap_total as f64) * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };

        let disk_parts: Vec<u64> = lines[3]
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        let (disk_total, disk_used, disk_avail, disk_usage) = if disk_parts.len() >= 3 {
            let total = disk_parts[0];
            let used = disk_parts[1];
            let avail = disk_parts[2];
            let usage = if (used + avail) > 0 {
                (used as f64 / (used + avail) as f64) * 100.0
            } else {
                0.0
            };
            (total, used, avail, usage.clamp(0.0, 100.0))
        } else {
            (0, 0, 0, 0.0)
        };

        let net_parts: Vec<f64> = lines[4]
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        let (net_down_raw, net_up_raw) = if net_parts.len() == 2 {
            (net_parts[0], net_parts[1])
        } else {
            (0.0, 0.0)
        };

        let load_parts: Vec<f64> = lines[5]
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        let load_avg = if load_parts.len() == 3 {
            [load_parts[0], load_parts[1], load_parts[2]]
        } else {
            [0.0, 0.0, 0.0]
        };

        let uptime = lines.get(6).map(|s| s.to_string()).unwrap_or_default();

        Ok((
            ServerStatus {
                cpu_usage,
                mem_usage,
                mem_total,
                mem_used,
                mem_avail,
                swap_usage,
                swap_total,
                swap_used,
                disk_usage,
                disk_total,
                disk_used,
                disk_avail,
                net_down: net_down_raw,
                net_up: net_up_raw,
                latency: 0,
                load_avg,
                uptime,
            },
            (current_cpu_total, current_cpu_idle),
        ))
    }

    pub fn send_ssh_input(&self, session_id: &SessionId, input: String) -> Result<(), SshError> {
        let channels = self
            .channels
            .read()
            .map_err(|e| SshError::LockPoisoned(e.to_string()))?;

        if let Some(channel_info) = channels.get(session_id) {
            channel_info
                .input_sender
                .send(input)
                .map_err(|_| SshError::ChannelError("Failed to send input".to_string()))
        } else {
            Err(SshError::SessionNotFound(session_id.0.to_string()))
        }
    }

    pub fn get_buffered_ssh_output(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<OutputChunk>, SshError> {
        let channels = self
            .channels
            .read()
            .map_err(|e| SshError::LockPoisoned(e.to_string()))?;

        if let Some(channel_info) = channels.get(session_id) {
            let outputs = channel_info
                .initial_outputs
                .try_lock()
                .map(|g| g.clone())
                .unwrap_or_default();
            Ok(outputs)
        } else {
            Err(SshError::SessionNotFound(session_id.0.to_string()))
        }
    }

    pub fn disconnect_ssh(&self, session_id: &SessionId) -> Result<(), SshError> {
        if let Ok(mut channels) = self.channels.write() {
            if let Some(mut info) = channels.remove(session_id) {
                info.stop_flag.store(true, Ordering::SeqCst);

                if let Some(ref app_handle) = info.app_handle {
                    if let Some(input_id) = info.input_listener_id.take() {
                        app_handle.unlisten(input_id);
                    }
                    if let Some(resize_id) = info.resize_listener_id.take() {
                        app_handle.unlisten(resize_id);
                    }
                }

                if let Some(handle) = info.handle.take() {
                    handle.abort();
                }
                if let Some(status_handle) = info.status_handle.take() {
                    status_handle.abort();
                }

                // Best-effort graceful shutdown of helper session. The main
                // session's channel is dropped when the JoinHandle aborts.
                let helper = info.helper_sess.clone();
                std::thread::spawn(move || {
                    let sess = helper.blocking_lock();
                    sess.disconnect(None, "User disconnect", None).ok();
                });
            }
        }
        log::info!("Disconnected SSH session: {}", session_id.0);
        Ok(())
    }

    pub fn disconnect_all(&self) {
        let session_ids: Vec<SessionId> = if let Ok(channels) = self.channels.read() {
            channels.keys().cloned().collect()
        } else {
            Vec::new()
        };

        for session_id in session_ids {
            let _ = self.disconnect_ssh(&session_id);
        }
    }

    /// Uploads a file via SFTP using the dedicated helper session.
    ///
    /// The SFTP subsystem and remote file are opened **once** before the chunk
    /// loop, which is dramatically faster than re-opening on every 512KB
    /// chunk. Because the helper session runs in blocking mode on its own
    /// TCP connection, there is no race with the interactive I/O task.
    pub fn upload_file_sftp(
        &self,
        app_handle: tauri::AppHandle,
        session_id: SessionId,
        task_id: String,
        local_path: String,
        remote_path: String,
    ) -> Result<(), SshError> {
        let (helper_sess, stop_flag) = {
            let channels = self
                .channels
                .read()
                .map_err(|e| SshError::LockPoisoned(e.to_string()))?;
            let info = channels
                .get(&session_id)
                .ok_or_else(|| SshError::SessionNotFound(session_id.as_ref().to_string()))?;
            (info.helper_sess.clone(), info.stop_flag.clone())
        };

        let watcher_app = app_handle.clone();
        let watcher_sid = session_id.as_ref().to_string();
        let watcher_task_id = task_id.clone();

        let upload_handle = tokio::task::spawn_blocking(move || {
            let sid = session_id.as_ref().to_string();
            let upload_start = std::time::Instant::now();
            let event_name = format!("ssh-upload-progress-{}", sid);

            let result: Result<u64, SshError> = (|| {
                let mut local_file = std::fs::File::open(&local_path).map_err(|e| {
                    SshError::OperationFailed(format!(
                        "Failed to open local file {}: {}",
                        local_path, e
                    ))
                })?;

                let total_bytes = local_file.metadata().map(|m| m.len()).unwrap_or(0);
                let mut buffer = [0u8; 1024 * 512];
                let mut total_written: u64 = 0;

                // Open SFTP subsystem and remote file ONCE.
                let sess = helper_sess.blocking_lock();
                let sftp = sess.sftp().map_err(|e| {
                    SshError::OperationFailed(format!("Failed to start SFTP: {}", e))
                })?;

                let mut remote_file = sftp
                    .open_mode(
                        Path::new(&remote_path),
                        OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNCATE,
                        0o644,
                        OpenType::File,
                    )
                    .map_err(|e| {
                        SshError::OperationFailed(format!(
                            "Failed to open remote file {}: {}",
                            remote_path, e
                        ))
                    })?;

                loop {
                    if stop_flag.load(Ordering::SeqCst) {
                        return Err(SshError::OperationFailed(
                            "Upload aborted: session disconnected".to_string(),
                        ));
                    }

                    let n = local_file.read(&mut buffer).map_err(|e| {
                        SshError::OperationFailed(format!("Read local file failed: {}", e))
                    })?;
                    if n == 0 {
                        break;
                    }

                    remote_file.write_all(&buffer[..n]).map_err(|e| {
                        SshError::OperationFailed(format!("Failed to write remote file: {}", e))
                    })?;
                    total_written += n as u64;

                    let elapsed = upload_start.elapsed().as_secs_f64();
                    let speed = if elapsed > 0.0 {
                        total_written as f64 / elapsed
                    } else {
                        0.0
                    };
                    let progress = if total_bytes > 0 {
                        (total_written as f64 / total_bytes as f64) * 100.0
                    } else {
                        0.0
                    };

                    let _ = app_handle.emit(
                        &event_name,
                        UploadProgress {
                            task_id: task_id.clone(),
                            session_id: sid.clone(),
                            progress,
                            uploaded_bytes: total_written,
                            total_bytes,
                            status: "uploading".to_string(),
                            message: format!("Uploading... ({:.1} MB/s)", speed / 1024.0 / 1024.0),
                            speed,
                            error: None,
                        },
                    );
                }

                remote_file
                    .flush()
                    .map_err(|e| SshError::OperationFailed(format!("Flush failed: {}", e)))?;
                drop(remote_file);
                drop(sftp);
                drop(sess);

                Ok(total_bytes)
            })();

            match result {
                Ok(total_bytes) => {
                    let elapsed = upload_start.elapsed().as_secs_f64();
                    let speed = if elapsed > 0.0 {
                        total_bytes as f64 / elapsed
                    } else {
                        0.0
                    };
                    let _ = app_handle.emit(
                        &event_name,
                        UploadProgress {
                            task_id,
                            session_id: sid,
                            progress: 100.0,
                            uploaded_bytes: total_bytes,
                            total_bytes,
                            status: "success".to_string(),
                            message: "Upload completed successfully".to_string(),
                            speed,
                            error: None,
                        },
                    );
                }
                Err(e) => {
                    log::error!("[SFTP {}] upload failed: {}", sid, e);
                    let _ = app_handle.emit(
                        &event_name,
                        UploadProgress {
                            task_id,
                            session_id: sid,
                            progress: 0.0,
                            uploaded_bytes: 0,
                            total_bytes: 0,
                            status: "error".to_string(),
                            message: format!("Upload failed: {}", e),
                            speed: 0.0,
                            error: Some(e.to_string()),
                        },
                    );
                }
            }
        });

        tokio::spawn(async move {
            if let Err(join_err) = upload_handle.await {
                log::error!("[SFTP upload] task panicked: {}", join_err);
                let _ = watcher_app.emit(
                    &format!("ssh-upload-progress-{}", watcher_sid),
                    UploadProgress {
                        task_id: watcher_task_id,
                        session_id: watcher_sid,
                        progress: 0.0,
                        uploaded_bytes: 0,
                        total_bytes: 0,
                        status: "error".to_string(),
                        message: "Upload aborted: internal task failure".to_string(),
                        speed: 0.0,
                        error: Some(join_err.to_string()),
                    },
                );
            }
        });

        Ok(())
    }

    /// Probes the remote user's home/current directory using the helper session.
    pub async fn probe_remote_path(&self, session_id: &SessionId) -> Result<String, SshError> {
        let helper_sess = {
            let channels = self
                .channels
                .read()
                .map_err(|e| SshError::LockPoisoned(e.to_string()))?;
            let info = channels
                .get(session_id)
                .ok_or_else(|| SshError::SessionNotFound(session_id.as_ref().to_string()))?;
            info.helper_sess.clone()
        };

        tokio::task::spawn_blocking(move || {
            let sess = helper_sess.blocking_lock();
            let mut channel = sess.channel_session().map_err(|e| {
                SshError::ChannelError(format!("Failed to create probe channel: {}", e))
            })?;

            channel
                .exec("pwd")
                .map_err(|e| SshError::OperationFailed(e.to_string()))?;

            let mut output = String::new();
            channel
                .read_to_string(&mut output)
                .map_err(|e| SshError::OperationFailed(e.to_string()))?;
            let _ = channel.wait_close();

            Ok(output.trim().to_string())
        })
        .await
        .map_err(|e| SshError::TaskError(e.to_string()))?
    }

    pub fn set_refresh_rate(
        &self,
        session_id: &SessionId,
        interval_ms: u64,
    ) -> Result<(), SshError> {
        let channels = self
            .channels
            .read()
            .map_err(|e| SshError::LockPoisoned(e.to_string()))?;
        let info = channels
            .get(session_id)
            .ok_or_else(|| SshError::SessionNotFound(session_id.as_ref().to_string()))?;
        info.refresh_interval.store(interval_ms, Ordering::SeqCst);
        Ok(())
    }
}

// ============================================================================
// Tauri Command Handlers
// ============================================================================

#[tauri::command]
#[allow(non_snake_case)]
pub async fn connect_ssh(
    state: tauri::State<'_, SshManager>,
    app_handle: tauri::AppHandle,
    sessionId: String,
    ip: String,
    port: u16,
    username: String,
    password: String,
    privateKeyPath: Option<String>,
    keyPassphrase: Option<String>,
    cols: u32,
    rows: u32,
) -> Result<(), SshError> {
    state
        .connect_ssh(
            Some(app_handle),
            SessionId::from(sessionId),
            ip,
            port,
            username,
            password,
            privateKeyPath,
            keyPassphrase,
            cols,
            rows,
        )
        .await
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn get_buffered_ssh_output(
    state: tauri::State<'_, SshManager>,
    sessionId: String,
) -> Result<Vec<OutputChunk>, SshError> {
    state.get_buffered_ssh_output(&SessionId::from(sessionId))
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn disconnect_ssh(
    state: tauri::State<'_, SshManager>,
    sessionId: String,
) -> Result<(), SshError> {
    state.disconnect_ssh(&SessionId::from(sessionId))
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn send_ssh_input(
    state: tauri::State<'_, SshManager>,
    sessionId: String,
    input: String,
) -> Result<(), SshError> {
    state.send_ssh_input(&SessionId::from(sessionId), input)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn set_ssh_status_refresh_rate(
    state: tauri::State<'_, SshManager>,
    sessionId: String,
    intervalMs: u64,
) -> Result<(), SshError> {
    state.set_refresh_rate(&SessionId::from(sessionId), intervalMs)
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn upload_file_sftp(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, SshManager>,
    sessionId: String,
    taskId: String,
    localPath: String,
    remotePath: String,
) -> Result<(), SshError> {
    state.upload_file_sftp(
        app_handle,
        SessionId::from(sessionId),
        taskId,
        localPath,
        remotePath,
    )
}

#[tauri::command]
#[allow(non_snake_case)]
pub async fn probe_remote_path(
    state: tauri::State<'_, SshManager>,
    sessionId: String,
) -> Result<String, SshError> {
    state
        .probe_remote_path(&SessionId::from(sessionId))
        .await
}
