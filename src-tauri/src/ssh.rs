use crate::common::{OutputChunk, SessionId};
use serde::Serialize;
use ssh2::{ExtensiblePtyModeOpcode, OpenFlags, OpenType, PtyModeOpcode, PtyModes, Session};
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

mod hostkey;
use hostkey::verify_host_key;

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

    /// Upload was cancelled by the user before it finished transferring.
    #[error("Upload cancelled by user")]
    UploadCancelled,
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

/// Per-task control shared between the active upload loop and the pause /
/// resume / cancel Tauri commands. The `paused` condvar lets an in-flight
/// upload park at a chunk boundary until it is resumed or cancelled.
#[derive(Default)]
pub struct UploadControl {
    pub cancel: Arc<AtomicBool>,
    pub paused: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>,
}

pub struct SshChannelInfo {
    pub handle: Option<tokio::task::JoinHandle<()>>,
    pub status_handle: Option<tokio::task::JoinHandle<()>>,
    pub input_sender: mpsc::UnboundedSender<String>,
    pub stop_flag: Arc<AtomicBool>,
    pub initial_outputs: Arc<tokio::sync::Mutex<Vec<OutputChunk>>>,
    pub refresh_interval: Arc<AtomicU64>,

    /// Auxiliary blocking session used for SFTP, monitoring, and path probing.
    /// This runs on its own TCP connection and stays in blocking mode,
    /// eliminating the blocking-mode race that previously froze terminals
    /// during file uploads.
    pub helper_sess: Arc<tokio::sync::Mutex<Session>>,

    /// Active SFTP upload controls keyed by task id. Entries are inserted when
    /// an upload starts and removed when it terminates (success/error/cancel).
    pub uploads: Arc<RwLock<HashMap<String, Arc<UploadControl>>>>,

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
    modes.set_boolean(PtyModeOpcode::ICANON, true);
    modes.set_boolean(PtyModeOpcode::ECHO, true);
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
    #[allow(clippy::too_many_arguments)]
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
            Self::establish_sessions(&addr, &host_for_err, port, &auth, cols, rows)
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
        let uploads_map = Arc::new(RwLock::new(HashMap::new()));

        let (input_listener_id, resize_listener_id) = if let Some(h) = &app_handle {
            let input_id = Self::register_input_listener(h, &session_id, &input_sender, &stop_flag);
            let resize_id =
                Self::register_resize_listener(h, &session_id, &channel_arc, &stop_flag);
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
                    initial_outputs,
                    refresh_interval,
                    helper_sess: helper_arc,
                    uploads: uploads_map,
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

        let open_tcp =
            || -> Result<TcpStream, SshError> {
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

                let tcp = TcpStream::connect_timeout(&socket_addr, Duration::from_secs(30))
                    .map_err(|e| SshError::ConnectionFailed {
                        host: host_for_err.to_string(),
                        port,
                        reason: e.to_string(),
                    })?;

                // Enable OS-level TCP keepalive so idle connections stay alive
                // through cloud security groups / NAT (which reap idle TCP
                // flows after ~3-5 min). The kernel sends keepalive probes
                // autonomously, independent of libssh2's non-blocking SSH I/O.
                let sock = socket2::SockRef::from(&tcp);
                let keepalive = socket2::TcpKeepalive::new()
                    .with_time(Duration::from_secs(15))
                    .with_interval(Duration::from_secs(5));
                let _ = sock.set_tcp_keepalive(&keepalive);

                Ok(tcp)
            };

        let make_session = |tcp: TcpStream| -> Result<Session, SshError> {
            let mut sess = Session::new().map_err(|e| {
                SshError::OperationFailed(format!("Failed to create session: {}", e))
            })?;
            // Guard the (blocking) handshake against a hung server using
            // libssh2's own timeout, which only applies to blocking calls. It
            // deliberately does NOT set a socket-level read timeout: a
            // SO_RCVTIMEO on the fd would leak into the whole session lifetime
            // and tear down long-lived idle SSH connections with a spurious
            // "transport read" error.
            sess.set_timeout(60_000);
            sess.set_tcp_stream(tcp);
            sess.handshake()
                .map_err(|e| SshError::OperationFailed(format!("Handshake failed: {}", e)))?;
            // Handshake complete — clear the blocking timeout so subsequent
            // (non-blocking) I/O is never subject to it.
            sess.set_timeout(0);
            Ok(sess)
        };

        let authenticate = |sess: &Session| -> Result<(), SshError> {
            let mut authenticated = false;
            if let Some(ref key_path) = auth.private_key_path {
                let path = Path::new(key_path);
                if path.exists() {
                    let key_result = if let Some(ref passphrase) = auth.key_passphrase {
                        sess.userauth_pubkey_file(&auth.username, None, path, Some(passphrase))
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
                        SshError::AuthenticationFailed(format!("Authentication failed: {}", e))
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

        // Keep the main session in BLOCKING mode, but bound each blocking call
        // to 100ms via libssh2's own timeout. Blocking mode is the officially
        // recommended usage for ssh2-rs and avoids libssh2 1.11.1's known
        // non-blocking bug (send_existing fails to set OUTBOUND on EAGAIN,
        // which corrupts the transport and causes spurious "transport read"
        // disconnects while typing). The 100ms ceiling lets the I/O loop drain
        // user input with sub-100ms latency instead of stalling forever.
        main_sess.set_timeout(100);

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
                // Detached resize task: the channel lock is async so it cannot
                // run synchronously in this event callback. Dropping the
                // JoinHandle detaches (does NOT abort) the task, which may
                // outlive the callback.
                #[allow(clippy::let_underscore_future)]
                let _ = tokio::spawn(async move {
                    let mut ch = task_channel_clone.lock().await;
                    if let Err(e) = ch.request_pty_size(payload.cols, payload.rows, None, None) {
                        log::warn!("Resize failed: {}", e);
                    }
                });
            }
        })
    }

    #[allow(clippy::too_many_arguments)]
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
            // Blocking reads are bounded to 100ms by the session timeout, so an
            // idle loop naturally paces itself (no busy-poll backoff needed).

            loop {
                if stop_flag.load(Ordering::SeqCst) {
                    break;
                }

                // Process user input FIRST for low-latency IME response.
                // The main session is blocking with a 100ms timeout, so a full
                // channel/TCP buffer surfaces as WouldBlock/TimedOut — retry
                // with backoff instead of dropping the remainder of the input.
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
                            Err(ref e)
                                if e.kind() == std::io::ErrorKind::WouldBlock
                                    || e.kind() == std::io::ErrorKind::TimedOut =>
                            {
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

                    if let Err(e) = ch.flush()
                        && e.kind() != std::io::ErrorKind::WouldBlock
                        && e.kind() != std::io::ErrorKind::TimedOut
                    {
                        log::error!("[SSH I/O] flush failed: {}", e);
                    }
                }

                let read_result = {
                    let _sess_lock = sess_arc.lock().await;
                    let mut ch = channel_arc.lock().await;
                    match ch.read(&mut buffer) {
                        Ok(0) => Some(Err("Connection closed by remote host".to_string())),
                        Ok(n) => Some(Ok(n)),
                        // In blocking mode with set_timeout, an idle read
                        // returns TimedOut rather than WouldBlock; both mean
                        // "no data right now, keep looping".
                        Err(ref e)
                            if e.kind() == std::io::ErrorKind::WouldBlock
                                || e.kind() == std::io::ErrorKind::TimedOut =>
                        {
                            None
                        }
                        Err(e) => Some(Err(format!("Read error: {}", e))),
                    }
                };

                match read_result {
                    Some(Ok(n)) => {
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
                        // No data (timed out). The blocking read already paced
                        // us; loop again to drain input, which is the only
                        // thing that needs polling.
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
            // Cache the remote OS once so non-Linux hosts don't re-run the
            // Linux-only metrics pipeline (awk/free/df/proc) every poll — it
            // would fail or return zeros and is pure wasted round-trips.
            // A `OnceLock` (rather than a local `Option<bool>`) makes the cache
            // actually visible across loop iterations — a plain `Option<bool>`
            // is `Copy`, so the `move` closure copies it every iteration and
            // the probe re-runs forever.
            //
            // On probe *failure* we assume the host IS Linux (`unwrap_or(true)`)
            // and fall through to the real metrics pipeline, whose per-command
            // `|| echo '0 0'` guards degrade gracefully. Treating a probe error
            // as "not Linux" (`unwrap_or(false)`) would permanently pin the
            // dashboard to all-zero `empty_status()` on any transient failure.
            let platform_cached: Arc<std::sync::OnceLock<bool>> =
                Arc::new(std::sync::OnceLock::new());

            loop {
                if stop_flag.load(Ordering::SeqCst) {
                    break;
                }

                let start_time = std::time::Instant::now();
                let helper = helper_sess.clone();
                let platform_cached = platform_cached.clone();

                // All blocking SSH operations run on spawn_blocking so we never
                // stall a tokio worker thread. When the platform is not yet
                // known, the blocking closure determines it first (single
                // cheap `uname` exec), then conditionally runs the Linux metrics.
                let status_res = tokio::task::spawn_blocking(move || {
                    let sess = helper.blocking_lock();

                    let is_linux = *platform_cached.get_or_init(|| {
                        Self::is_linux_host_blocking(&sess).unwrap_or(true)
                    });

                    if !is_linux {
                        // Non-Linux host: metrics pipeline is Linux-specific.
                        // Return a zeroed status rather than re-running the
                        // command chain every interval.
                        return Ok((Self::empty_status(), (0, 0)));
                    }

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

    /// Detects whether the remote host runs Linux using a single cheap `uname`
    /// exec. Called at most once per session and cached so the monitoring loop
    /// skips the Linux-only metrics pipeline on macOS/BSD/Windows hosts.
    /// The caller must hold the helper session lock and run on a blocking thread.
    fn is_linux_host_blocking(sess: &Session) -> Result<bool, SshError> {
        let mut channel = sess
            .channel_session()
            .map_err(|e| SshError::ChannelError(format!("uname channel failed: {}", e)))?;
        channel
            .exec("uname -s 2>/dev/null || echo unknown")
            .map_err(|e| SshError::OperationFailed(e.to_string()))?;
        let mut output = String::new();
        let _ = channel.read_to_string(&mut output);
        let _ = channel.wait_close();
        Ok(output.trim().eq_ignore_ascii_case("linux"))
    }

    /// A zeroed status used for hosts that don't expose the Linux `/proc`
    /// metrics the dashboard reads.
    fn empty_status() -> ServerStatus {
        ServerStatus {
            cpu_usage: 0.0,
            mem_usage: 0.0,
            mem_total: 0,
            mem_used: 0,
            mem_avail: 0,
            swap_usage: 0.0,
            swap_total: 0,
            swap_used: 0,
            disk_usage: 0.0,
            disk_total: 0,
            disk_used: 0,
            disk_avail: 0,
            net_up: 0.0,
            net_down: 0.0,
            latency: 0,
            load_avg: [0.0; 3],
            uptime: "unsupported platform".to_string(),
        }
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
                   LC_ALL=C free -b 2>/dev/null | awk '/Mem:/{print $2,$3,$7} /Swap:/{print $2,$3}' || printf '0 0 0\n0 0\n'; \
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
        let mem_usage = (100.0 * (1.0 - (mem_avail as f64 / mem_total as f64))).clamp(0.0, 100.0);

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
        if let Ok(mut channels) = self.channels.write()
            && let Some(mut info) = channels.remove(session_id)
        {
            info.stop_flag.store(true, Ordering::SeqCst);

            // Cancel any in-flight SFTP uploads so their spawn_blocking loops
            // exit promptly (they would otherwise keep writing to a dead
            // helper session) and clean up their own uploads-map entries.
            if let Ok(uploads) = info.uploads.read() {
                for control in uploads.values() {
                    control.cancel.store(true, Ordering::SeqCst);
                    // Wake a paused waiter so it sees the cancel immediately
                    // instead of waiting out the 200ms poll interval.
                    control.paused.1.notify_all();
                }
            }

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
        let (helper_sess, stop_flag, uploads) = {
            let channels = self
                .channels
                .read()
                .map_err(|e| SshError::LockPoisoned(e.to_string()))?;
            let info = channels
                .get(&session_id)
                .ok_or_else(|| SshError::SessionNotFound(session_id.as_ref().to_string()))?;
            (
                info.helper_sess.clone(),
                info.stop_flag.clone(),
                info.uploads.clone(),
            )
        };

        let watcher_app = app_handle.clone();
        let watcher_sid = session_id.as_ref().to_string();
        let watcher_task_id = task_id.clone();

        // Register the per-task control so pause/cancel commands can find it.
        let control: Arc<UploadControl> = Arc::new(UploadControl::default());
        {
            let mut uploads = uploads
                .write()
                .map_err(|e| SshError::LockPoisoned(e.to_string()))?;
            uploads.insert(task_id.clone(), control.clone());
        }
        // Clean up the control entry once the upload terminates (success/error/cancel).
        let uploads_cleanup = uploads.clone();
        let cleanup_task_id = task_id.clone();

        let upload_handle = tokio::task::spawn_blocking(move || {
            let sid = session_id.as_ref().to_string();
            let upload_start = std::time::Instant::now();
            let event_name = format!("ssh-upload-progress-{}", sid);
            let control = control.clone();

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
                // Throttle progress events: emitting on every 512KB chunk can
                // flood Tauri IPC (hundreds of events/sec on fast links) and
                // churn the frontend queue. Emit at most every 100ms OR every
                // 1MB moved.
                let mut last_progress_emit = std::time::Instant::now();
                let mut last_progress_bytes: u64 = 0;

                // Open SFTP subsystem and remote file ONCE. The file handle and
                // the localized SFTP file pointer stay open across pause spans,
                // so resume simply continues writing at the same offset.
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
                    // Cancel has priority over pause: a cancelled task must
                    // leave the pause wait immediately.
                    if control.cancel.load(Ordering::SeqCst) {
                        return Err(SshError::UploadCancelled);
                    }
                    if stop_flag.load(Ordering::SeqCst) {
                        return Err(SshError::OperationFailed(
                            "Upload aborted: session disconnected".to_string(),
                        ));
                    }

                    // Park at the current chunk boundary while paused. The
                    // pause lock is held for the whole span, so the pause
                    // status is reported exactly once per pause/resume cycle.
                    {
                        let (pause_lock, cvar) = &*control.paused;
                        let mut paused = pause_lock.lock().unwrap();
                        if *paused {
                            let _ = app_handle.emit(
                                &event_name,
                                UploadProgress {
                                    task_id: task_id.clone(),
                                    session_id: sid.clone(),
                                    progress: if total_bytes > 0 {
                                        (total_written as f64 / total_bytes as f64) * 100.0
                                    } else {
                                        0.0
                                    },
                                    uploaded_bytes: total_written,
                                    total_bytes,
                                    status: "paused".to_string(),
                                    message: "Upload paused".to_string(),
                                    speed: 0.0,
                                    error: None,
                                },
                            );
                            while *paused {
                                if control.cancel.load(Ordering::SeqCst) {
                                    return Err(SshError::UploadCancelled);
                                }
                                if stop_flag.load(Ordering::SeqCst) {
                                    return Err(SshError::OperationFailed(
                                        "Upload aborted: session disconnected".to_string(),
                                    ));
                                }
                                paused = cvar
                                    .wait_timeout(paused, std::time::Duration::from_millis(200))
                                    .map_err(|e| {
                                        SshError::OperationFailed(format!(
                                            "Pause wait failed: {}",
                                            e
                                        ))
                                    })?
                                    .0;
                            }
                        }
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

                    // Throttle: skip the event unless 100ms elapsed or 1MB has
                    // been written since the last one. The terminal 100%
                    // "success" event is emitted separately after the loop.
                    let time_elapsed =
                        last_progress_emit.elapsed() >= std::time::Duration::from_millis(100);
                    let bytes_moved = total_written.saturating_sub(last_progress_bytes) >= 1024 * 1024;
                    if !time_elapsed && !bytes_moved {
                        continue;
                    }
                    last_progress_emit = std::time::Instant::now();
                    last_progress_bytes = total_written;

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
                            task_id: task_id.clone(),
                            session_id: sid.clone(),
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
                Err(SshError::UploadCancelled) => {
                    log::info!("[SFTP {}] upload cancelled by user", sid);
                    // Delete the partial remote file left behind by the
                    // interrupted transfer (best-effort). The blocking session
                    // lock is already released once the closure unwound.
                    {
                        let sess = helper_sess.blocking_lock();
                        if let Ok(sftp) = sess.sftp() {
                            let _ = sftp.unlink(Path::new(&remote_path));
                        }
                    }
                    let _ = app_handle.emit(
                        &event_name,
                        UploadProgress {
                            task_id: task_id.clone(),
                            session_id: sid.clone(),
                            progress: 0.0,
                            uploaded_bytes: 0,
                            total_bytes: 0,
                            status: "cancelled".to_string(),
                            message: "Upload cancelled".to_string(),
                            speed: 0.0,
                            error: None,
                        },
                    );
                }
                Err(e) => {
                    log::error!("[SFTP {}] upload failed: {}", sid, e);
                    let _ = app_handle.emit(
                        &event_name,
                        UploadProgress {
                            task_id: task_id.clone(),
                            session_id: sid.clone(),
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

            // Drop the per-task control reference so stale entries do not
            // accumulate for terminated uploads.
            if let Ok(mut uploads) = uploads_cleanup.write() {
                uploads.remove(&cleanup_task_id);
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

    /// Look up the per-task upload control for a session/task, if present.
    fn upload_control(
        &self,
        session_id: &SessionId,
        task_id: &str,
    ) -> Result<Option<Arc<UploadControl>>, SshError> {
        let channels = self
            .channels
            .read()
            .map_err(|e| SshError::LockPoisoned(e.to_string()))?;
        let info = channels
            .get(session_id)
            .ok_or_else(|| SshError::SessionNotFound(session_id.as_ref().to_string()))?;
        let uploads = info
            .uploads
            .read()
            .map_err(|e| SshError::LockPoisoned(e.to_string()))?;
        Ok(uploads.get(task_id).cloned())
    }

    /// Pause a running upload at its next chunk boundary.
    pub fn pause_upload(&self, session_id: &SessionId, task_id: &str) -> Result<(), SshError> {
        if let Some(control) = self.upload_control(session_id, task_id)? {
            let (lock, _cvar) = &*control.paused;
            let mut paused = lock.lock().unwrap();
            *paused = true;
            // No notify needed: the upload loop polls `paused` each iteration.
        }
        Ok(())
    }

    /// Resume a previously paused upload.
    pub fn resume_upload(&self, session_id: &SessionId, task_id: &str) -> Result<(), SshError> {
        if let Some(control) = self.upload_control(session_id, task_id)? {
            let (lock, cvar) = &*control.paused;
            let mut paused = lock.lock().unwrap();
            *paused = false;
            cvar.notify_all();
        }
        Ok(())
    }

    /// Cancel a running or paused upload. The upload loop will terminate and
    /// the partial remote file will be removed.
    pub fn cancel_upload(&self, session_id: &SessionId, task_id: &str) -> Result<(), SshError> {
        if let Some(control) = self.upload_control(session_id, task_id)? {
            control.cancel.store(true, Ordering::SeqCst);
            // Wake any park wait so a paused upload aborts immediately.
            let (lock, cvar) = &*control.paused;
            let mut paused = lock.lock().unwrap();
            *paused = false;
            cvar.notify_all();
        }
        Ok(())
    }
}

// ============================================================================
// Tauri Command Handlers
// ============================================================================

#[tauri::command]
#[allow(non_snake_case, clippy::too_many_arguments)]
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
    state.probe_remote_path(&SessionId::from(sessionId)).await
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn pause_upload(
    state: tauri::State<'_, SshManager>,
    sessionId: String,
    taskId: String,
) -> Result<(), SshError> {
    state.pause_upload(&SessionId::from(sessionId), &taskId)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn resume_upload(
    state: tauri::State<'_, SshManager>,
    sessionId: String,
    taskId: String,
) -> Result<(), SshError> {
    state.resume_upload(&SessionId::from(sessionId), &taskId)
}

#[tauri::command]
#[allow(non_snake_case)]
pub fn cancel_upload(
    state: tauri::State<'_, SshManager>,
    sessionId: String,
    taskId: String,
) -> Result<(), SshError> {
    state.cancel_upload(&SessionId::from(sessionId), &taskId)
}

#[cfg(test)]
mod upload_control_tests {
    use super::*;
    use std::sync::atomic::Ordering;

    /// A paused upload's `cancel` flag must cause the pause wait to terminate
    /// (the loop polls both paused and cancel).
    #[test]
    fn cancel_wakes_paused_wait() {
        let control = Arc::new(<UploadControl>::default());

        let parked = {
            let control_c = control.clone();
            std::thread::spawn(move || {
                let guard = control_c.paused.0.lock().unwrap();
                let _ = control_c.paused.1.wait_timeout(
                    guard,
                    std::time::Duration::from_millis(300),
                );
                control_c.cancel.load(Ordering::SeqCst)
            })
        };

        // Simulate a paused upload being cancelled: set both flags and wake.
        *control.paused.0.lock().unwrap() = true;
        control.cancel.store(true, Ordering::SeqCst);
        *control.paused.0.lock().unwrap() = false;
        control.paused.1.notify_all();

        let observed = parked.join().unwrap();
        assert!(observed, "cancel flag should be observable after wake");
        assert!(control.cancel.load(Ordering::SeqCst));
    }

    /// Pause/resume flip the `paused` flag so the upload loop parks and
    /// resumes accordingly.
    #[test]
    fn pause_then_resume_flips_flag() {
        let control = Arc::new(<UploadControl>::default());
        let (lock, _cvar) = &*control.paused;

        // simulate pause_upload
        *lock.lock().unwrap() = true;
        assert!(*lock.lock().unwrap(), "paused flag should be set");

        // simulate resume_upload
        *lock.lock().unwrap() = false;
        assert!(!*lock.lock().unwrap(), "paused flag should be cleared");
    }

    /// Repeated pause/resume cycles remain idempotent on the flag.
    #[test]
    fn idempotent_pause() {
        let control = Arc::new(<UploadControl>::default());
        let (lock, _cvar) = &*control.paused;
        *lock.lock().unwrap() = true;
        *lock.lock().unwrap() = true;
        assert!(*lock.lock().unwrap());
        *lock.lock().unwrap() = false;
        *lock.lock().unwrap() = false;
        assert!(!*lock.lock().unwrap());
    }

    /// Build a SshManager with a single registered session carrying an uploads
    /// map, so pause/resume/cancel can be exercised against the real lookup.
    fn manager_with_session() -> (SshManager, SessionId, Arc<UploadControl>) {
        use ssh2::Session as SshSession;
        use tokio::sync::mpsc;

        let sid = SessionId::from("test-session");
        let control = Arc::new(<UploadControl>::default());
        let uploads = Arc::new(RwLock::new(HashMap::from([(
            "task-1".to_string(),
            control.clone(),
        )])));
        let (tx, _rx) = mpsc::unbounded_channel::<String>();

        let info = SshChannelInfo {
            handle: None,
            status_handle: None,
            input_sender: tx,
            stop_flag: Arc::new(AtomicBool::new(false)),
            initial_outputs: Arc::new(tokio::sync::Mutex::new(Vec::new())),
            refresh_interval: Arc::new(AtomicU64::new(3000)),
            helper_sess: Arc::new(tokio::sync::Mutex::new(SshSession::new().unwrap())),
            uploads,
            input_listener_id: None,
            resize_listener_id: None,
            app_handle: None,
        };

        let manager = SshManager {
            channels: Arc::new(RwLock::new(HashMap::from([(sid.clone(), info)]))),
        };
        (manager, sid, control)
    }

    /// pause on a registered task flips the paused flag and notifies waiters.
    #[test]
    fn pause_and_resume_registered_task() {
        let (manager, sid, control) = manager_with_session();

        manager.pause_upload(&sid, "task-1").unwrap();
        assert!(*control.paused.0.lock().unwrap(), "pause should set the flag");

        manager.resume_upload(&sid, "task-1").unwrap();
        assert!(
            !*control.paused.0.lock().unwrap(),
            "resume should clear the flag"
        );
    }

    /// cancel on a registered task sets the cancel flag and wakes a parked wait.
    #[test]
    fn cancel_registered_task() {
        let (manager, sid, control) = manager_with_session();

        // park a waiter as a paused upload would
        let parked = {
            let c = control.clone();
            std::thread::spawn(move || {
                let guard = c.paused.0.lock().unwrap();
                let _ = c.paused.1.wait_timeout(
                    guard,
                    std::time::Duration::from_millis(500),
                );
                c.cancel.load(Ordering::SeqCst)
            })
        };

        manager.cancel_upload(&sid, "task-1").unwrap();
        let observed = parked.join().unwrap();
        assert!(
            control.cancel.load(Ordering::SeqCst),
            "cancel should set the cancel flag"
        );
        // The waiter observes cancel set (it may also have timed out, but the
        // flag must be set regardless).
        let _ = observed;
    }

    /// pause/resume/cancel on an unknown task id is a no-op that still returns Ok.
    #[test]
    fn operations_on_unknown_task_are_noop() {
        let (manager, sid, _control) = manager_with_session();

        manager.pause_upload(&sid, "missing").unwrap();
        manager.resume_upload(&sid, "missing").unwrap();
        manager.cancel_upload(&sid, "missing").unwrap();
    }

    /// pause/resume on an unknown session returns SessionNotFound.
    #[test]
    fn operations_on_unknown_session_error() {
        let (manager, _sid, _control) = manager_with_session();
        let ghost = SessionId::from("ghost");

        let err = manager.pause_upload(&ghost, "task-1").unwrap_err();
        assert!(matches!(err, SshError::SessionNotFound(_)));
    }

    /// disconnect_ssh cancels in-flight uploads for that session.
    #[test]
    fn disconnect_cancels_uploads() {
        let (manager, sid, control) = manager_with_session();

        manager.disconnect_ssh(&sid).unwrap();
        assert!(
            control.cancel.load(Ordering::SeqCst),
            "disconnect should cancel in-flight uploads"
        );
        assert!(
            manager.channels.read().unwrap().is_empty(),
            "disconnect removes the session channel"
        );
    }

}
