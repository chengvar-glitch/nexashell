use crate::common::{OutputChunk, SessionId};
use crate::db::TunnelRuleRow;
use serde::Serialize;
use ssh2::{
    ExtensiblePtyModeOpcode, PtyModeOpcode, PtyModes, Session,
};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use thiserror::Error;
use tokio::sync::mpsc;

use crate::hostkey::verify_host_key;

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
// Event Sink — replaces the Tauri event bus so the core SSH lifecycle is
// frontend-agnostic. The TUI wires a channel-backed sink; the desktop app
// wires its AppHandle emitter.
// ============================================================================

pub trait SshEventSink: Send + Sync {
    fn on_output(&self, session_id: &str, chunk: &OutputChunk);
    fn on_status(&self, session_id: &str, status: &ServerStatus);
    fn on_disconnected(&self, session_id: &str, reason: &str);
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

/// Authentication credentials retained for the interactive shell and the
/// monitoring helper connection.
#[derive(Clone)]
pub(crate) struct SshAuth {
    pub(crate) username: String,
    pub(crate) password: String,
    pub(crate) private_key_path: Option<String>,
    pub(crate) key_passphrase: Option<String>,
}

pub struct SshChannelInfo {
    pub handle: Option<tokio::task::JoinHandle<()>>,
    pub status_handle: Option<tokio::task::JoinHandle<()>>,
    pub input_sender: mpsc::UnboundedSender<String>,
    pub stop_flag: Arc<AtomicBool>,

    /// Main interactive channel, kept so resize requests can reach it. It is
    /// also held by the I/O task (which owns the actual read/write loop).
    pub channel: Arc<tokio::sync::Mutex<ssh2::Channel>>,

    /// Auxiliary blocking session used for server-status monitoring. It runs
    /// on its own TCP connection and stays in blocking mode.
    pub helper_sess: Arc<tokio::sync::Mutex<Session>>,

    /// Connection details retained so tunnel rules can open their own fresh
    /// authenticated connections to the same host.
    pub addr: String,
    pub host_for_err: String,
    pub port: u16,
    pub auth: crate::ssh::SshAuth,
}

/// Runtime state of one running tunnel rule (local or dynamic forwarding).
pub struct TunnelState {
    pub session_id: String,
    pub stop_flag: Arc<AtomicBool>,
    pub handle: Option<std::thread::JoinHandle<()>>,
    pub accepted: Arc<std::sync::atomic::AtomicU32>,
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
    tunnels: Arc<RwLock<HashMap<String, TunnelState>>>,
}

impl SshManager {
    #[allow(clippy::too_many_arguments)]
    pub async fn connect_ssh(
        &self,
        sink: Arc<dyn SshEventSink>,
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
            private_key_path,
            key_passphrase,
        };
        let addr_c = addr.clone();
        let host_c = host_for_err.clone();
        let auth_c = auth.clone();

        // 1. Establish both sessions (interactive + helper) on separate TCP
        //    connections. Blocking work runs on spawn_blocking.
        let connect_res = tokio::task::spawn_blocking(move || {
            Self::establish_sessions(&addr_c, &host_c, port, &auth_c, cols, rows)
        })
        .await
        .map_err(|e| SshError::TaskError(e.to_string()))?;

        let (main_sess, main_channel, helper_sess) = connect_res?;

        // 2. Setup communication channels
        let (input_sender, input_receiver) = mpsc::unbounded_channel::<String>();
        let stop_flag = Arc::new(AtomicBool::new(false));
        let refresh_interval = Arc::new(AtomicU64::new(3000));

        let channel_arc = Arc::new(tokio::sync::Mutex::new(main_channel));
        let sess_arc = Arc::new(tokio::sync::Mutex::new(main_sess));
        let helper_arc = Arc::new(tokio::sync::Mutex::new(helper_sess));

        let handle = Self::spawn_io_task(
            channel_arc.clone(),
            sess_arc.clone(),
            stop_flag.clone(),
            input_receiver,
            sink.clone(),
            session_id.clone(),
        );

        let status_handle = Self::spawn_monitoring_task(
            sink,
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
                    channel: channel_arc,
                    helper_sess: helper_arc,
                    addr,
                    host_for_err,
                    port,
                    auth,
                },
            );
        }

        Ok(())
    }

    /// Open a single fully-authenticated SSH connection over a new TCP
    /// connection (blocking). Used by tunnel forwarding so tunnelled traffic
    /// never contends with the live interactive/helper sessions.
    pub(crate) fn connect_authenticated(
        addr: &str,
        host_for_err: &str,
        port: u16,
        auth: &SshAuth,
    ) -> Result<Session, SshError> {
        let open_tcp = || -> Result<TcpStream, SshError> {
            let socket_addr = addr.to_socket_addrs().map_err(|e| {
                SshError::ConnectionFailed {
                    host: host_for_err.to_string(),
                    port,
                    reason: format!("Failed to resolve address: {}", e),
                }
            })?.next().ok_or_else(|| SshError::ConnectionFailed {
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

            // Enable OS-level TCP keepalive so idle connections stay alive
            // through cloud security groups / NAT.
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
            sess.set_timeout(60_000);
            sess.set_tcp_stream(tcp);
            sess.handshake()
                .map_err(|e| SshError::OperationFailed(format!("Handshake failed: {}", e)))?;
            sess.set_timeout(0);
            Ok(sess)
        };

        let tcp = open_tcp()?;
        let sess = make_session(tcp)?;
        verify_host_key(&sess, host_for_err)?;
        Self::authenticate_session(&sess, auth)?;
        Ok(sess)
    }

    fn authenticate_session(sess: &Session, auth: &SshAuth) -> Result<(), SshError> {
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
        // Main session
        let main_sess = Self::connect_authenticated(addr, host_for_err, port, auth)?;

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

        // Helper session (blocking, for monitoring)
        // Verify the host key on the helper connection too: each connection is
        // resolved independently (DNS rotation could pin them to different
        // endpoints), so skipping verification here would let an attacker MITM
        // monitoring traffic.
        let helper_sess = Self::connect_authenticated(addr, host_for_err, port, auth)?;
        // helper_sess stays in blocking mode (default)

        Ok((main_sess, channel, helper_sess))
    }

    #[allow(clippy::too_many_arguments)]
    fn spawn_io_task(
        channel_arc: Arc<tokio::sync::Mutex<ssh2::Channel>>,
        sess_arc: Arc<tokio::sync::Mutex<Session>>,
        stop_flag: Arc<AtomicBool>,
        mut input_receiver: mpsc::UnboundedReceiver<String>,
        sink: Arc<dyn SshEventSink>,
        session_id: SessionId,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut buffer = [0u8; SSH_BUFFER_SIZE];
            let mut pending_output = String::new();
            let mut last_emit = std::time::Instant::now();
            let mut seen_first_output = false;
            let initial_buffering_start = std::time::Instant::now();
            let mut in_initial_buffering = true;

            loop {
                if stop_flag.load(Ordering::SeqCst) {
                    break;
                }

                // Process user input FIRST for low-latency IME response.
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
                        sink.on_disconnected(&session_id.0, &e);
                        stop_flag.store(true, Ordering::SeqCst);
                        break;
                    }
                    None => {
                        // No data (timed out). The blocking read already paced
                        // us; loop again to drain input.
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
                    let chunk = OutputChunk::new(1, std::mem::take(&mut pending_output));
                    sink.on_output(&session_id.0, &chunk);
                    last_emit = std::time::Instant::now();
                    seen_first_output = true;
                }
            }
        })
    }

    fn spawn_monitoring_task(
        sink: Arc<dyn SshEventSink>,
        session_id: SessionId,
        helper_sess: Arc<tokio::sync::Mutex<Session>>,
        stop_flag: Arc<AtomicBool>,
        refresh_interval: Arc<AtomicU64>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut last_net_read: Option<(f64, f64, std::time::Instant)> = None;
            let mut last_cpu_read: Option<(u64, u64)> = None;
            // Cache the remote OS once so non-Linux hosts don't re-run the
            // Linux-only metrics pipeline (awk/free/df/proc) every poll.
            let platform_cached: Arc<std::sync::OnceLock<bool>> =
                Arc::new(std::sync::OnceLock::new());

            loop {
                if stop_flag.load(Ordering::SeqCst) {
                    break;
                }

                let start_time = std::time::Instant::now();
                let helper = helper_sess.clone();
                let platform_cached = platform_cached.clone();

                let status_res = tokio::task::spawn_blocking(move || {
                    let sess = helper.blocking_lock();

                    let is_linux = *platform_cached.get_or_init(|| {
                        Self::is_linux_host_blocking(&sess).unwrap_or(true)
                    });

                    if !is_linux {
                        // Non-Linux host: metrics pipeline is Linux-specific.
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

                    sink.on_status(&session_id.0, &status);
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

    pub fn resize_ssh(
        &self,
        session_id: &SessionId,
        cols: u32,
        rows: u32,
    ) -> Result<(), SshError> {
        let channels = self
            .channels
            .read()
            .map_err(|e| SshError::LockPoisoned(e.to_string()))?;

        if let Some(channel_info) = channels.get(session_id) {
            let task_channel = channel_info.channel.clone();
            // Detached resize task: the channel lock is async so it cannot run
            // synchronously here. Dropping the JoinHandle detaches (does NOT
            // abort) the task.
            #[allow(clippy::let_underscore_future)]
            let _ = tokio::spawn(async move {
                let mut ch = task_channel.lock().await;
                if let Err(e) = ch.request_pty_size(cols, rows, None, None) {
                    log::warn!("Resize failed: {}", e);
                }
            });
            Ok(())
        } else {
            Err(SshError::SessionNotFound(session_id.0.to_string()))
        }
    }

    pub fn disconnect_ssh(&self, session_id: &SessionId) -> Result<(), SshError> {
        if let Ok(mut channels) = self.channels.write()
            && let Some(mut info) = channels.remove(session_id)
        {
            info.stop_flag.store(true, Ordering::SeqCst);

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
        // Tunnels of this session cannot outlive it.
        self.stop_tunnels_for_session(session_id);
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
        if let Ok(mut tunnels) = self.tunnels.write() {
            for (_, state) in tunnels.drain() {
                state.stop_flag.store(true, Ordering::SeqCst);
            }
        }
    }

    // ==========================================================================
    // Tunnel forwarding (local -L and dynamic SOCKS5 -D)
    // ==========================================================================

    /// Return connection details retained for a live session so tunnel rules
    /// can open their own authenticated connections on the same host.
    fn session_connection(
        &self,
        session_id: &SessionId,
    ) -> Result<(String, String, u16, SshAuth), SshError> {
        let channels = self
            .channels
            .read()
            .map_err(|e| SshError::LockPoisoned(e.to_string()))?;
        let info = channels
            .get(session_id)
            .ok_or_else(|| SshError::SessionNotFound(session_id.as_ref().to_string()))?;
        Ok((
            info.addr.clone(),
            info.host_for_err.clone(),
            info.port,
            info.auth.clone(),
        ))
    }

    /// Start a persisted tunnel rule: binds the local listener and forwards
    /// accepted connections over fresh authenticated SSH channels.
    pub fn start_tunnel_rule(&self, rule: &TunnelRuleRow) -> Result<(), SshError> {
        if self.tunnel_running(&rule.id) {
            return Err(SshError::OperationFailed(
                "Tunnel rule already running".to_string(),
            ));
        }
        let (conn_addr, host_for_err, conn_port, auth) = self
            .session_connection(&SessionId::from(rule.session_id.clone()))?;

        let listener = TcpListener::bind((rule.listen_host.as_str(), rule.listen_port as u16))
            .map_err(|e| {
                SshError::OperationFailed(format!(
                    "Failed to bind {}:{}: {}",
                    rule.listen_host, rule.listen_port, e
                ))
            })?;

        let stop_flag = Arc::new(AtomicBool::new(false));
        let accepted = Arc::new(AtomicU32::new(0));
        let rule_for_thread = rule.clone();
        let stop = stop_flag.clone();
        let accepted_conn = accepted.clone();
        let handle = std::thread::Builder::new()
            .name(format!("nexashell-tui-tunnel-{}", rule.id))
            .spawn(move || {
                run_tunnel_listener(
                    &rule_for_thread,
                    listener,
                    &conn_addr,
                    &host_for_err,
                    conn_port,
                    &auth,
                    stop,
                    accepted_conn,
                );
            })
            .map_err(|e| SshError::OperationFailed(format!("Failed to spawn tunnel: {}", e)))?;

        self.tunnels
            .write()
            .map_err(|e| SshError::LockPoisoned(e.to_string()))?
            .insert(
                rule.id.clone(),
                TunnelState {
                    session_id: rule.session_id.clone(),
                    stop_flag,
                    handle: Some(handle),
                    accepted,
                },
            );
        log::info!(
            "Tunnel {} listening on {}:{} -> {}:{}",
            rule.id,
            rule.listen_host,
            rule.listen_port,
            rule.target_host,
            rule.target_port
        );
        Ok(())
    }

    pub fn stop_tunnel(&self, rule_id: &str) {
        let mut tunnels = match self.tunnels.write() {
            Ok(g) => g,
            Err(e) => {
                log::error!("Tunnel state lock poisoned: {}", e);
                return;
            }
        };
        if let Some(state) = tunnels.remove(rule_id) {
            state.stop_flag.store(true, Ordering::SeqCst);
            if let Some(handle) = state.handle {
                let _ = handle.join();
            }
            log::info!("Stopped tunnel rule {}", rule_id);
        }
    }

    pub fn tunnel_running(&self, rule_id: &str) -> bool {
        if let Ok(tunnels) = self.tunnels.read() {
            tunnels.contains_key(rule_id)
        } else {
            false
        }
    }

    pub fn tunnel_accepted(&self, rule_id: &str) -> u32 {
        if let Ok(tunnels) = self.tunnels.read()
            && let Some(state) = tunnels.get(rule_id)
        {
            state.accepted.load(Ordering::SeqCst)
        } else {
            0
        }
    }

    fn stop_tunnels_for_session(&self, session_id: &SessionId) {
        let ids: Vec<String> = if let Ok(tunnels) = self.tunnels.read() {
            tunnels
                .iter()
                .filter(|(_, s)| s.session_id.as_str() == session_id.0.as_str())
                .map(|(id, _)| id.clone())
                .collect()
        } else {
            Vec::new()
        };
        for id in ids {
            self.stop_tunnel(&id);
        }
    }
}

/// Accept loop for a running tunnel: accepts local connections and hands each
/// one to a per-connection thread that pumps bytes over a direct-tcpip channel.
/// The listener is non-blocking so the loop can observe the stop flag between
/// connection attempts and terminate promptly (and free the bound port).
#[allow(clippy::too_many_arguments)]
fn run_tunnel_listener(
    rule: &TunnelRuleRow,
    listener: TcpListener,
    conn_addr: &str,
    host_for_err: &str,
    conn_port: u16,
    auth: &SshAuth,
    stop: Arc<AtomicBool>,
    accepted: Arc<AtomicU32>,
) {
    let _ = listener.set_nonblocking(true);
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, peer)) => {
                if stop.load(Ordering::SeqCst) {
                    break;
                }
                accepted.fetch_add(1, Ordering::SeqCst);
                let (a, h, p) = (conn_addr.to_string(), host_for_err.to_string(), conn_port);
                let au = auth.clone();
                let tstop = stop.clone();
                let rule = rule.clone();
                let _ = std::thread::Builder::new()
                    .name(format!("nexashell-tui-tunnel-conn-{}", rule.id))
                    .spawn(move || {
                        let _ = handle_tunnel_connection(
                            &rule,
                            &a,
                            &h,
                            p,
                            &au,
                            stream,
                            peer,
                            tstop.as_ref(),
                        );
                    });
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                if !stop.load(Ordering::SeqCst) {
                    log::debug!("Tunnel {} accept error: {}", rule.id, e);
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_tunnel_connection(
    rule: &TunnelRuleRow,
    addr: &str,
    host_for_err: &str,
    port: u16,
    auth: &SshAuth,
    mut client: TcpStream,
    peer: SocketAddr,
    stop: &AtomicBool,
) -> Result<(), String> {
    let _ = client.set_nodelay(true);

    // Dynamic SOCKS tunnels negotiate the destination first.
    let (remote_host, remote_port) = if rule.direction == "dynamic" {
        socks5_negotiate(&mut client)?
    } else {
        (rule.target_host.clone(), rule.target_port as u16)
    };

    let sess = SshManager::connect_authenticated(addr, host_for_err, port, auth)
        .map_err(|e| format!("SSH connect failed: {}", e))?;
    // Non-blocking so the single-threaded pump below can service both
    // directions without deadlocking on a blocking read.
    sess.set_blocking(false);

    let client_ip = peer.ip().to_string();
    let mut chan = sess
        .channel_direct_tcpip(
            &remote_host,
            remote_port,
            Some((client_ip.as_str(), peer.port())),
        )
        .map_err(|e| {
            format!(
                "Failed to open direct-tcpip channel to {}:{}: {}",
                remote_host, remote_port, e
            )
        })?;

    pump_bidirectional(&mut client, &mut chan, stop);
    Ok(())
}

/// Bidirectionally copy bytes between a local client stream and an SSH channel.
/// Ownership: both live on this one thread, and the channel is non-blocking so
/// a single-threaded poll loop can service both directions without deadlocking.
fn pump_bidirectional(client: &mut TcpStream, chan: &mut ssh2::Channel, stop: &AtomicBool) {
    let mut chan_buf = [0u8; 16384];
    let mut client_buf = [0u8; 16384];
    let _ = client.set_read_timeout(Some(Duration::from_millis(40)));

    loop {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        // SSH channel -> client
        match chan.read(&mut chan_buf) {
            Ok(0) => break,
            Ok(n) => {
                if client.write_all(&chan_buf[..n]).is_err() {
                    break;
                }
                if client.flush().is_err() {
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => break,
        }
        if stop.load(Ordering::SeqCst) {
            break;
        }
        // client -> SSH channel
        match client.read(&mut client_buf) {
            Ok(0) => {
                let _ = chan.eof();
                break;
            }
            Ok(n) => {
                if chan.write_all(&client_buf[..n]).is_err() {
                    break;
                }
                let _ = chan.flush();
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(2));
            }
            Err(_) => break,
        }
    }
}

/// Perform a SOCKS5 handshake on `stream` and return the requested destination
/// (host + port). Only supports the no-auth + CONNECT subset of RFC 1928.
fn socks5_negotiate(stream: &mut TcpStream) -> Result<(String, u16), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(|e| e.to_string())?;
    let mut buf = [0u8; 512];
    let n = stream.read(&mut buf).map_err(|e| e.to_string())?;
    if n < 3 || buf[0] != 0x05 {
        return Err("Not a SOCKS5 request".to_string());
    }
    // Greeting: 05 NMETHODS METHODS... -> reply 05 00 (no auth).
    let nmethods = buf[1] as usize;
    if n < 2 + nmethods {
        return Err("Truncated SOCKS5 greeting".to_string());
    }
    stream.write_all(&[0x05, 0x00]).map_err(|e| e.to_string())?;
    stream.flush().map_err(|e| e.to_string())?;

    // CONNECT request: 05 01 00 ATYP DST.ADDR DST.PORT
    let mut n = 0usize;
    while n < 4 {
        let r = stream.read(&mut buf[n..]).map_err(|e| e.to_string())?;
        if r == 0 {
            return Err("Connection closed during SOCKS5 request".to_string());
        }
        n += r;
    }
    if buf[0] != 0x05 || buf[1] != 0x01 || buf[2] != 0x00 {
        return Err("Unsupported SOCKS5 request".to_string());
    }
    let host = match buf[3] {
        0x01 => {
            while n < 4 + 4 {
                let r = stream.read(&mut buf[n..]).map_err(|e| e.to_string())?;
                if r == 0 {
                    return Err("Connection closed during SOCKS5 request".to_string());
                }
                n += r;
            }
            format!("{}.{}.{}.{}", buf[4], buf[5], buf[6], buf[7])
        }
        0x03 => {
            if n < 5 {
                return Err("Truncated SOCKS5 domain".to_string());
            }
            let len = buf[4] as usize;
            while n < 5 + len {
                let r = stream.read(&mut buf[n..]).map_err(|e| e.to_string())?;
                if r == 0 {
                    return Err("Connection closed during SOCKS5 request".to_string());
                }
                n += r;
            }
            String::from_utf8_lossy(&buf[5..5 + len]).to_string()
        }
        _ => return Err("Unsupported SOCKS5 address type".to_string()),
    };
    while n < 4 + 4 + 2 {
        let r = stream.read(&mut buf[n..]).map_err(|e| e.to_string())?;
        if r == 0 {
            return Err("Connection closed during SOCKS5 request".to_string());
        }
        n += r;
    }
    let port = u16::from_be_bytes([buf[4 + 4 + 2 - 2], buf[4 + 4 + 2 - 1]]);
    stream
        .write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .map_err(|e| e.to_string())?;
    stream.flush().map_err(|e| e.to_string())?;
    Ok((host, port))
}