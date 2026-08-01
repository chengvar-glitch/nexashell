use crate::common::{OutputChunk, SessionId};
use serde::Serialize;
use ssh2::{Session, OpenFlags, OpenType, PtyModes, PtyModeOpcode, ExtensiblePtyModeOpcode};
use std::collections::HashMap;
use std::io::{Read, Write, Seek, SeekFrom};
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

/// Custom error type for SSH operations with detailed context
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
}

// ============================================================================
// Constants for SSH I/O optimization
// ============================================================================

/// Buffer size for SSH channel reads (4KB - optimal for terminal I/O)
const SSH_BUFFER_SIZE: usize = 4096;

/// Initial batch threshold (welcome banner, login prompts)
/// Increased initial time to ensure welcome banner is fully received
const INITIAL_BATCH_SIZE_THRESHOLD: usize = 200;
const INITIAL_BATCH_TIME_MS: u64 = 100; // Increased from 5ms to 100ms

/// Timeout for initial buffering phase (after connection established)
/// After this time, stop buffering initial output
const INITIAL_BUFFERING_TIMEOUT_MS: u64 = 2000; // 2 seconds to capture all initial output

/// Normal operation batch threshold
const NORMAL_BATCH_SIZE_THRESHOLD: usize = 1024;
const NORMAL_BATCH_TIME_MS: u64 = 20;

// ============================================================================
// Data Structures
// ============================================================================

/// SSH connection configuration
#[derive(Debug, Clone)]
pub struct SshSession {
    #[allow(dead_code)]
    pub ip: String,
    #[allow(dead_code)]
    pub port: u16,
    #[allow(dead_code)]
    pub username: String,
}

/// Server performance metrics
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

/// Represents the progress of an SFTP file upload
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

/// Contains state and communication handles for an active SSH channel
pub struct SshChannelInfo {
    /// Handle to the background tokio task processing the SSH data
    pub handle: Option<tokio::task::JoinHandle<()>>,

    /// Handle to the background monitoring task
    pub status_handle: Option<tokio::task::JoinHandle<()>>,

    pub input_sender: mpsc::UnboundedSender<String>,

    /// Atomic flag to signal the background task to terminate
    pub stop_flag: Arc<AtomicBool>,

    /// Monotonically increasing sequence number for output chunks
    #[allow(dead_code)]
    pub next_seq: Arc<AtomicU64>,

    /// Cached initial output (welcome banner) for late-joining clients
    pub initial_outputs: Arc<tokio::sync::Mutex<Vec<OutputChunk>>>,

    /// Refresh interval for monitoring task (in milliseconds)
    pub refresh_interval: Arc<AtomicU64>,

    /// Session handle for opening new channels
    pub sess_arc: Arc<tokio::sync::Mutex<Session>>,

    /// Event handler IDs for cleanup on disconnect
    pub input_listener_id: Option<tauri::EventId>,
    pub resize_listener_id: Option<tauri::EventId>,

    /// App handle for unlisten calls during cleanup
    pub app_handle: Option<tauri::AppHandle>,
}

fn build_pty_modes() -> PtyModes {
    let mut modes = PtyModes::new();

    // Character mappings (standard control characters)
    modes.set_character(PtyModeOpcode::VINTR, Some('\x03' as u8 as char));  // Ctrl+C
    modes.set_character(PtyModeOpcode::VQUIT, Some('\x1c' as u8 as char));  // Ctrl+\
    modes.set_character(PtyModeOpcode::VERASE, Some('\x7f' as u8 as char)); // Backspace
    modes.set_character(PtyModeOpcode::VKILL, Some('\x15' as u8 as char));  // Ctrl+U
    modes.set_character(PtyModeOpcode::VEOF, Some('\x04' as u8 as char));   // Ctrl+D
    modes.set_character(PtyModeOpcode::VEOL, None);   // disabled
    modes.set_character(PtyModeOpcode::VEOL2, None);  // disabled
    modes.set_character(PtyModeOpcode::VSTART, None); // disabled (flow control off)
    modes.set_character(PtyModeOpcode::VSTOP, None);  // disabled (flow control off)
    modes.set_character(PtyModeOpcode::VSUSP, Some('\x1a' as u8 as char));  // Ctrl+Z
    modes.set_character(PtyModeOpcode::VDSUSP, None); // disabled
    modes.set_character(PtyModeOpcode::VREPRINT, Some('\x12' as u8 as char)); // Ctrl+R
    modes.set_character(PtyModeOpcode::VWERASE, Some('\x17' as u8 as char));  // Ctrl+W
    modes.set_character(PtyModeOpcode::VLNEXT, Some('\x16' as u8 as char));   // Ctrl+V

    // Input modes — disable output processing on the server side
    modes.set_boolean(PtyModeOpcode::IGNPAR, false);
    modes.set_boolean(PtyModeOpcode::PARMRK, false);
    modes.set_boolean(PtyModeOpcode::INPCK, false);
    modes.set_boolean(PtyModeOpcode::ISTRIP, false);
    modes.set_boolean(PtyModeOpcode::INLCR, false);
    modes.set_boolean(PtyModeOpcode::IGNCR, false);
    modes.set_boolean(PtyModeOpcode::ICRNL, true);
    modes.set_boolean(PtyModeOpcode::IUCLC, false);

    // CRITICAL: disable XON/XOFF software flow control
    modes.set_boolean(PtyModeOpcode::IXON, false);
    modes.set_boolean(PtyModeOpcode::IXANY, false);
    modes.set_boolean(PtyModeOpcode::IXOFF, false);

    modes.set_boolean(PtyModeOpcode::IMAXBEL, false);

    // IUTF8 = 42 (not in PtyModeOpcode enum, use Extended)
    const IUTF8: u8 = 42;
    modes.set_boolean(ExtensiblePtyModeOpcode::Extended(IUTF8), true);

    // Local modes — shell-appropriate defaults (vim/tools will override via tcsetattr)
    modes.set_boolean(PtyModeOpcode::ISIG, true);
    modes.set_boolean(PtyModeOpcode::ICANON, true);
    modes.set_boolean(PtyModeOpcode::ECHO, true);
    modes.set_boolean(PtyModeOpcode::ECHOE, true);
    modes.set_boolean(PtyModeOpcode::ECHOK, true);
    modes.set_boolean(PtyModeOpcode::ECHONL, false);

    // Output modes — ONLCR must be true for normal terminal display
    modes.set_boolean(PtyModeOpcode::OPOST, true);
    modes.set_boolean(PtyModeOpcode::ONLCR, true);

    // Baud rates
    modes.set_u32(PtyModeOpcode::TTY_OP_ISPEED, 38400);
    modes.set_u32(PtyModeOpcode::TTY_OP_OSPEED, 38400);

    modes
}

/// Global manager for coordinating SSH sessions and channels
///
/// This manager coordinates all active SSH connections and provides
/// the primary interface for SSH operations. Uses RwLock for better
/// performance when reads outnumber writes (typical in SSH usage).
#[derive(Default)]
pub struct SshManager {
    sessions: Arc<RwLock<HashMap<SessionId, SshSession>>>,
    channels: Arc<RwLock<HashMap<SessionId, SshChannelInfo>>>,
}

impl SshManager {
    /// Creates a new SSH manager instance
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Establishes a new SSH connection and spawns the I/O handler task
    ///
    /// # Arguments
    /// * `app_handle` - Tauri app handle for event emission
    /// * `session_id` - Unique identifier for this session
    /// * `ip` - SSH server IP address
    /// * `port` - SSH server port
    /// * `username` - SSH username
    /// * `password` - SSH password
    /// * `cols` - Terminal columns
    /// * `rows` - Terminal rows
    ///
    /// # Returns
    /// `Ok(())` on success, `Err(SshError)` with detailed error context on failure
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
        let sessions_arc = Arc::clone(&self.sessions);
        let channels_arc = Arc::clone(&self.channels);

        let addr = format!("{}:{}", ip, port);
        let host_for_err = addr.clone();
        let username_for_spawn = username.clone();
        let password_for_spawn = password;
        let private_key_path_for_spawn = private_key_path;
        let key_passphrase_for_spawn = key_passphrase;

        // 1. Establish connection and authenticate (blocking part in separate thread)
        let connection_res = tokio::task::spawn_blocking(move || {
            use std::net::ToSocketAddrs;
            let socket_addr = addr
                .to_socket_addrs()
                .map_err(|e| SshError::ConnectionFailed {
                    host: host_for_err.clone(),
                    port,
                    reason: format!("Failed to resolve address: {}", e),
                })?
                .next()
                .ok_or_else(|| SshError::ConnectionFailed {
                    host: host_for_err.clone(),
                    port,
                    reason: "No addresses found".to_string(),
                })?;

            let tcp =
                TcpStream::connect_timeout(&socket_addr, Duration::from_secs(30)).map_err(|e| {
                    SshError::ConnectionFailed {
                        host: host_for_err.clone(),
                        port,
                        reason: e.to_string(),
                    }
                })?;

            let mut sess = Session::new().map_err(|e| {
                SshError::OperationFailed(format!("Failed to create session: {}", e))
            })?;
            sess.set_tcp_stream(tcp);
            sess.handshake()
                .map_err(|e| SshError::OperationFailed(format!("Handshake failed: {}", e)))?;

            // Try public key authentication first if a key path is provided
            let mut authenticated = false;
            if let Some(ref key_path) = private_key_path_for_spawn {
                let path = Path::new(key_path);
                if path.exists() {
                    let key_result = if let Some(ref passphrase) = key_passphrase_for_spawn {
                        sess.userauth_pubkey_file(
                            &username_for_spawn,
                            None,
                            path,
                            Some(passphrase),
                        )
                    } else {
                        sess.userauth_pubkey_file(
                            &username_for_spawn,
                            None,
                            path,
                            None,
                        )
                    };
                    authenticated = key_result.is_ok() && sess.authenticated();
                } else {
                    eprintln!("Warning: private key file not found: {}", key_path);
                }
            }

            // Fallback to password authentication if key auth did not succeed
            if !authenticated {
                if private_key_path_for_spawn.is_some() {
                    eprintln!(
                        "Warning: Public key authentication failed for '{}', falling back to password",
                        username_for_spawn
                    );
                }
                sess.userauth_password(&username_for_spawn, &password_for_spawn)
                    .map_err(|_| SshError::AuthenticationFailed("Invalid credentials".to_string()))?;
            }

            if !sess.authenticated() {
                return Err(SshError::AuthenticationFailed(
                    "Authentication failed".to_string(),
                ));
            }

            let mut channel = sess
                .channel_session()
                .map_err(|e| SshError::ChannelError(format!("Create channel failed: {}", e)))?;

            let pty_modes = build_pty_modes();
            channel
                .request_pty("xterm-256color", Some(pty_modes), Some((cols, rows, 0, 0)))
                .map_err(|e| SshError::ChannelError(format!("Failed to request PTY: {}", e)))?;

            channel
                .shell()
                .map_err(|e| SshError::ChannelError(format!("Failed to start shell: {}", e)))?;

            // Set non-blocking mode for async I/O
            sess.set_blocking(false);

            Ok((sess, channel))
        })
        .await;

        let (sess, channel) = match connection_res {
            Ok(Ok(val)) => val,
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(SshError::TaskError(e.to_string())),
        };

        // 2. Setup communication channels
        let (input_sender, input_receiver) = mpsc::unbounded_channel::<String>();
        let stop_flag = Arc::new(AtomicBool::new(false));
        let next_seq = Arc::new(AtomicU64::new(1));
        let initial_outputs = Arc::new(tokio::sync::Mutex::new(Vec::new()));
        let refresh_interval = Arc::new(AtomicU64::new(3000)); // Default to idle: 3s

        let channel_arc = Arc::new(tokio::sync::Mutex::new(channel));
        let sess_arc = Arc::new(tokio::sync::Mutex::new(sess));

        // 3. Register event listeners for user input and resize
        let (input_listener_id, resize_listener_id) = if let Some(h) = &app_handle {
            let input_id = Self::register_input_listener(h, &session_id, &input_sender, &stop_flag);
            let resize_id = Self::register_resize_listener(h, &session_id, &channel_arc, &stop_flag);
            (Some(input_id), Some(resize_id))
        } else {
            (None, None)
        };

        // 4. Spawn I/O task
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

        // 5. Spawn monitoring task
        let status_handle = Self::spawn_monitoring_task(
            app_handle.clone(),
            session_id.clone(),
            sess_arc.clone(),
            stop_flag.clone(),
            refresh_interval.clone(),
        );

        // 6. Save session state
        {
            let mut sessions = sessions_arc
                .write()
                .map_err(|e| SshError::LockPoisoned(e.to_string()))?;
            sessions.insert(session_id.clone(), SshSession { ip, port, username });

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
                    input_listener_id,
                    resize_listener_id,
                    app_handle: app_handle.clone(),
                },
            );
        }

        Ok(())
    }

    /// Registers event listener for user input (keyboard).
    /// Returns the event handler ID for later cleanup.
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

    /// Registers event listener for terminal resize events.
    /// Returns the event handler ID for later cleanup.
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
                    let _ = ch.request_pty_size(payload.cols, payload.rows, None, None);
                });
            }
        })
    }

    /// Spawns the background I/O task that processes SSH input/output
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

                // Process user input FIRST for low-latency IME response
                while let Ok(input) = input_receiver.try_recv() {
                    let _sess_lock = sess_arc.lock().await;
                    let mut ch = channel_arc.lock().await;
                    if let Err(e) = ch.write_all(input.as_bytes()).and_then(|_| ch.flush()) {
                        eprintln!("[SSH I/O] write_all/flush failed: {}", e);
                    }
                }

                // Attempt non-blocking read from SSH channel
                // We lock the session to ensure thread safety with monitoring task
                let read_result = {
                    let _sess_lock = sess_arc.lock().await;
                    let mut ch = channel_arc.lock().await;
                    match ch.read(&mut buffer) {
                        Ok(0) => Some(Err("Connection closed")),
                        Ok(n) => Some(Ok(n)),
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => None,
                        Err(_) => Some(Err("Read error")),
                    }
                };

                match read_result {
                    Some(Ok(n)) => {
                        idle_reads = 0;
                        pending_output.push_str(&String::from_utf8_lossy(&buffer[..n]));
                    }
                    Some(Err(_)) => {
                        stop_flag.store(true, Ordering::SeqCst);
                        break;
                    }
                    None => {
                        idle_reads += 1;
                        // Sleep progressively to reduce CPU usage when idle:
                        //   first few idle cycles: sleep 1ms
                        //   sustained idle: sleep 10ms
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

                // Flush initial buffering or batch emit
                if (!pending_output.is_empty() && in_initial)
                    || (!pending_output.is_empty()
                        && (pending_output.len() > size_threshold
                            || last_emit.elapsed() > Duration::from_millis(time_threshold_ms)))
                {
                    let seq = next_seq.fetch_add(1, Ordering::SeqCst);
                    let output = std::mem::take(&mut pending_output);
                    let chunk = OutputChunk::new(seq, output);

                    // Cache initial outputs (capped)
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

    /// Spawns the background monitoring task for server metrics
    fn spawn_monitoring_task(
        app_handle: Option<tauri::AppHandle>,
        session_id: SessionId,
        sess_arc: Arc<tokio::sync::Mutex<Session>>,
        stop_flag: Arc<AtomicBool>,
        refresh_interval: Arc<AtomicU64>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            // Initial readings for delta calculation (rx, tx, time)
            let mut last_net_read: Option<(f64, f64, std::time::Instant)> = None;
            let mut last_cpu_read: Option<(u64, u64)> = None; // (total, idle)

            loop {
                if stop_flag.load(Ordering::SeqCst) {
                    break;
                }

                let start_time = std::time::Instant::now();
                let channel = {
                    let sess = sess_arc.lock().await;
                    sess.channel_session()
                        .map_err(|e| SshError::ChannelError(e.to_string()))
                };
                let status_res = match channel {
                    Ok(mut ch) => {
                        Self::fetch_server_status_from_channel(&mut ch, last_cpu_read).await
                    }
                    Err(e) => Err(e),
                };
                let latency = start_time.elapsed().as_millis() as u32;

                if let Ok((mut status, current_cpu_raw)) = status_res {
                    let now = std::time::Instant::now();
                    status.latency = latency;
                    last_cpu_read = Some(current_cpu_raw);

                    // Calculate network speed
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

    /// Fetches server performance metrics via a short-lived SSH channel (no session lock held)
    async fn fetch_server_status_from_channel(
        channel: &mut ssh2::Channel,
        last_cpu: Option<(u64, u64)>,
    ) -> Result<(ServerStatus, (u64, u64)), SshError> {

        // Use more robust commands that work on various Linux environments
        // 1. CPU: /proc/stat
        // 2. Mem & Swap: free
        // 3. Disk: df (total, used, percentage)
        // 4. Net: /proc/net/dev
        // 5. LoadAvg: /proc/loadavg
        // 6. Uptime: uptime -p or /proc/uptime
        let cmd = "LC_ALL=C awk '/^cpu / {print $2+$3+$4+$5+$6+$7+$8, $5}' /proc/stat 2>/dev/null || echo '0 0'; \
                   LC_ALL=C free -b 2>/dev/null | awk '/Mem:/{print $2,$3,$7} /Swap:/{print $2,$3}' || echo '0 0 0\n0 0'; \
                   LC_ALL=C df -PB1 -x tmpfs -x devtmpfs -x overlay 2>/dev/null | awk 'NR>1 && !seen[$1]++ {t+=$2;u+=$3;a+=$4} END{print t,u,a}' || echo '0 0 0'; \
                   LC_ALL=C cat /proc/net/dev 2>/dev/null | awk 'NR>2 && $1!=\"lo:\"{rx+=$2; tx+=$10} END{print rx+0,tx+0}' || echo '0 0'; \
                   LC_ALL=C cat /proc/loadavg 2>/dev/null | awk '{print $1,$2,$3}' || echo '0 0 0'; \
                   LC_ALL=C uptime -p 2>/dev/null || echo 'up unknown'";

        // Retry exec with bounded attempts and backoff (max 10 retries, 50ms interval)
        const MAX_EXEC_RETRIES: u8 = 10;
        const EXEC_RETRY_DELAY_MS: u64 = 50;
        let mut exec_retries: u8 = 0;
        loop {
            match channel.exec(cmd) {
                Ok(_) => break,
                Err(ref e) if e.code() == ssh2::ErrorCode::Session(-37) => {
                    exec_retries += 1;
                    if exec_retries > MAX_EXEC_RETRIES {
                        return Err(SshError::ChannelError(format!(
                            "exec retry limit ({}) exceeded", MAX_EXEC_RETRIES
                        )));
                    }
                    tokio::time::sleep(Duration::from_millis(EXEC_RETRY_DELAY_MS)).await;
                }
                Err(e) => return Err(SshError::ChannelError(e.to_string())),
            }
        }

        // Read output with timeout on inactivity (5s between successful reads)
        const READ_INACTIVITY_TIMEOUT: Duration = Duration::from_secs(5);
        let mut last_progress = std::time::Instant::now();
        let mut output = String::new();
        loop {
            if last_progress.elapsed() > READ_INACTIVITY_TIMEOUT {
                return Err(SshError::OperationFailed(
                    "Status read timeout after 5s of inactivity".to_string()
                ));
            }
            let mut buf = [0u8; 1024];
            match channel.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    output.push_str(&String::from_utf8_lossy(&buf[..n]));
                    last_progress = std::time::Instant::now();
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
                Err(e) => return Err(SshError::OperationFailed(e.to_string())),
            }
        }

        let lines: Vec<&str> = output
            .lines()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if lines.len() < 6 {
            return Err(SshError::OperationFailed(format!(
                "Invalid status output format (lines: {})",
                lines.len()
            )));
        }

        // Parse CPU
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

        // Parse Memory & Swap
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
        // Use available memory for more accurate usage percentage if possible
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

        // Parse Disk
        let disk_parts: Vec<u64> = lines[3]
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        let (disk_total, disk_used, disk_avail, disk_usage) = if disk_parts.len() >= 3 {
            let total = disk_parts[0];
            let used = disk_parts[1];
            let avail = disk_parts[2];
            // Match Linux 'df' usage calculation: Used / (Used + Available)
            let usage = if (used + avail) > 0 {
                (used as f64 / (used + avail) as f64) * 100.0
            } else {
                0.0
            };
            (total, used, avail, usage.clamp(0.0, 100.0))
        } else {
            (0, 0, 0, 0.0)
        };

        // Parse Network Raw
        let net_parts: Vec<f64> = lines[4]
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        let (net_down_raw, net_up_raw) = if net_parts.len() == 2 {
            (net_parts[0], net_parts[1])
        } else {
            (0.0, 0.0)
        };

        // Parse Load Avg
        let load_parts: Vec<f64> = lines[5]
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        let load_avg = if load_parts.len() == 3 {
            [load_parts[0], load_parts[1], load_parts[2]]
        } else {
            [0.0, 0.0, 0.0]
        };

        // Parse Uptime
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

    /// Sends user input to a specific SSH session
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

    /// Retrieves cached initial output (welcome banner, login prompts) for a session
    ///
    /// Useful for clients that connect after the session has started.
    /// Uses `try_lock` to avoid blocking the runtime; returns an empty Vec
    /// if the lock is held (which is safe — a late joiner just sees no history).
    pub fn get_buffered_ssh_output(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<OutputChunk>, SshError> {
        let channels = self
            .channels
            .read()
            .map_err(|e| SshError::LockPoisoned(e.to_string()))?;

        if let Some(channel_info) = channels.get(session_id) {
            let outputs = channel_info.initial_outputs.try_lock().map(|g| g.clone()).unwrap_or_default();
            Ok(outputs)
        } else {
            Err(SshError::SessionNotFound(session_id.0.to_string()))
        }
    }

    /// Disconnects a specific SSH session and cleans up resources
    pub fn disconnect_ssh(&self, session_id: &SessionId) -> Result<(), SshError> {
        // Remove from channels and clean up task
        if let Ok(mut channels) = self.channels.write() {
            if let Some(mut info) = channels.remove(session_id) {
                info.stop_flag.store(true, Ordering::SeqCst);

                // Unregister event listeners to prevent leaks
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
            }
        }

        // Remove from sessions
        if let Ok(mut sessions) = self.sessions.write() {
            sessions.remove(session_id);
        }
        println!("Disconnected SSH session: {}", session_id.0);
        Ok(())
    }

    /// Disconnects all active SSH sessions
    pub fn disconnect_all(&self) {
        // Collect all session IDs first to avoid holding locks
        let session_ids: Vec<SessionId> = if let Ok(channels) = self.channels.read() {
            channels.keys().cloned().collect()
        } else {
            Vec::new()
        };

        for session_id in session_ids {
            let _ = self.disconnect_ssh(&session_id);
            println!("Disconnected SSH session: {}", session_id.0);
        }
    }

    /// Checks if a session exists
    #[allow(dead_code)]
    pub fn has_session(&self, session_id: &SessionId) -> bool {
        if let Ok(sessions) = self.sessions.read() {
            sessions.contains_key(session_id)
        } else {
            false
        }
    }

    /// Uploads a file via SFTP to the specified remote path.
    /// This implementation runs in the background and emits progress events.
    /// It uses chunked uploading and releases the session lock between chunks
    /// to ensure the terminal remains responsive.
    pub fn upload_file_sftp(
        &self,
        app_handle: tauri::AppHandle,
        session_id: SessionId,
        task_id: String,
        local_path: String,
        remote_path: String,
    ) -> Result<(), SshError> {
        let (sess_arc, stop_flag) = {
            let channels = self
                .channels
                .read()
                .map_err(|e| SshError::LockPoisoned(e.to_string()))?;
            let info = channels
                .get(&session_id)
                .ok_or_else(|| SshError::SessionNotFound(session_id.as_ref().to_string()))?;
            (info.sess_arc.clone(), info.stop_flag.clone())
        };

        // Clone handles for the panic watcher BEFORE they're moved into spawn_blocking
        let watcher_app = app_handle.clone();
        let watcher_sid = session_id.as_ref().to_string();
        let watcher_event_name = format!("ssh-upload-progress-{}", watcher_sid);
        let watcher_task_id = task_id.clone();

        // Perform the upload in a blocking tokio task to avoid blocking the runtime
        let upload_handle = tokio::task::spawn_blocking(move || {
            let sid = session_id.as_ref().to_string();
            let upload_start = std::time::Instant::now();
            let upload_event_name = format!("ssh-upload-progress-{}", sid);
            
            let result: Result<u64, SshError> = (|| {
                let mut local_file = std::fs::File::open(&local_path).map_err(|e| {
                    SshError::OperationFailed(format!("Failed to open local file {}: {}", local_path, e))
                })?;

                let total_bytes = local_file.metadata().map(|m| m.len()).unwrap_or(0);
                
                // 512KB chunks provide a good balance between throughput and terminal responsiveness
                let mut buffer = [0u8; 1024 * 512];
                let mut total_written: u64 = 0;
                let mut is_first_chunk = true;

                loop {
                    // Abort upload if session is being disconnected
                    if stop_flag.load(Ordering::SeqCst) {
                        return Err(SshError::OperationFailed(
                            "Upload aborted: session disconnected".to_string()
                        ));
                    }
                    // 1. Read a chunk from the local file
                    let n = local_file.read(&mut buffer).map_err(|e| {
                        SshError::OperationFailed(format!("Read local file failed: {}", e))
                    })?;
                    
                    if n == 0 {
                        break;
                    }

                    // 2. Acquire the session lock for this chunk
                    let sess = sess_arc.blocking_lock();
                    
                    // Temporarily set to blocking for synchronous SFTP operations
                    sess.set_blocking(true);

                    let chunk_res = (|| {
                        let sftp = sess.sftp().map_err(|e| {
                            SshError::OperationFailed(format!("Failed to start SFTP: {}", e))
                        })?;

                        let flags = if is_first_chunk {
                            OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNCATE
                        } else {
                            OpenFlags::WRITE
                        };

                        let mut remote_file = sftp.open_mode(
                            std::path::Path::new(&remote_path),
                            flags,
                            0o644,
                            OpenType::File
                        ).map_err(|e| {
                            SshError::OperationFailed(format!("Failed to open remote file {}: {}", remote_path, e))
                        })?;

                        if !is_first_chunk {
                            remote_file.seek(SeekFrom::Start(total_written)).map_err(|e| {
                                SshError::OperationFailed(format!("Failed to seek remote file: {}", e))
                            })?;
                        }

                        remote_file.write_all(&buffer[..n]).map_err(|e| {
                            SshError::OperationFailed(format!("Failed to write to remote file: {}", e))
                        })?;

                        remote_file.flush().map_err(|e| {
                            SshError::OperationFailed(format!("Failed to flush remote file: {}", e))
                        })?;

                        Ok(())
                    })();

                    // 3. CRITICAL: Restore non-blocking mode and release the lock
                    sess.set_blocking(false);
                    drop(sess);

                    // Check for errors after releasing the lock
                    chunk_res?;
                    
                    total_written += n as u64;
                    is_first_chunk = false;

                    // Calculate progress and speed
                    let elapsed = upload_start.elapsed().as_secs_f64();
                    let speed = if elapsed > 0.0 { total_written as f64 / elapsed } else { 0.0 };
                    let progress = if total_bytes > 0 { (total_written as f64 / total_bytes as f64) * 100.0 } else { 0.0 };

                    // Emit progress event
                    let _ = app_handle.emit(&upload_event_name, UploadProgress {
                        task_id: task_id.clone(),
                        session_id: sid.clone(),
                        progress,
                        uploaded_bytes: total_written,
                        total_bytes,
                        status: "uploading".to_string(),
                        message: format!("Uploading... ({:.1} MB/s)", speed / 1024.0 / 1024.0),
                        speed,
                        error: None,
                    });

                    // 4. Brief pause to give other tasks a chance to use the session
                    // if they are waiting for the lock.
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }

                Ok(total_bytes)
            })();

            // Emit final status
            match result {
                Ok(total_bytes) => {
                    let elapsed = upload_start.elapsed().as_secs_f64();
                    let speed = if elapsed > 0.0 { total_bytes as f64 / elapsed } else { 0.0 };
                    let _ = app_handle.emit(&upload_event_name, UploadProgress {
                        task_id: task_id.clone(),
                        session_id: sid,
                        progress: 100.0,
                        uploaded_bytes: total_bytes,
                        total_bytes,
                        status: "success".to_string(),
                        message: "Upload completed successfully".to_string(),
                        speed,
                        error: None,
                    });
                }
                Err(e) => {
                    let _ = app_handle.emit(&upload_event_name, UploadProgress {
                        task_id: task_id.clone(),
                        session_id: sid,
                        progress: 0.0,
                        uploaded_bytes: 0,
                        total_bytes: 0,
                        status: "error".to_string(),
                        message: format!("Upload failed: {}", e),
                        speed: 0.0,
                        error: Some(e.to_string()),
                    });
                }
            }
        });

        // Spawn a lightweight watcher to surface upload task panics
        tokio::spawn(async move {
            match upload_handle.await {
                Ok(()) => {} // success — already emitted via events
                Err(join_err) => {
                    eprintln!("[SFTP upload] task panicked: {}", join_err);
                    let _ = watcher_app.emit(&watcher_event_name, UploadProgress {
                        task_id: watcher_task_id,
                        session_id: watcher_sid,
                        progress: 0.0,
                        uploaded_bytes: 0,
                        total_bytes: 0,
                        status: "error".to_string(),
                        message: "Upload aborted: internal task failure".to_string(),
                        speed: 0.0,
                        error: Some(join_err.to_string()),
                    });
                }
            }
        });

        Ok(())
    }

    /// Probes the remote user's home or current directory without affecting the shell
    pub async fn probe_remote_path(&self, session_id: &SessionId) -> Result<String, SshError> {
        let sess_arc = {
            let channels = self
                .channels
                .read()
                .map_err(|e| SshError::LockPoisoned(e.to_string()))?;
            let info = channels
                .get(session_id)
                .ok_or_else(|| SshError::SessionNotFound(session_id.as_ref().to_string()))?;
            info.sess_arc.clone()
        };

        let sess_mutex = sess_arc.clone();
        tokio::task::spawn_blocking(move || {
            let sess = sess_mutex.blocking_lock();
            sess.set_blocking(true);

            let result = (|| {
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
            })();

            sess.set_blocking(false);
            result
        })
        .await
        .map_err(|e| SshError::TaskError(e.to_string()))?
    }

    /// Updates the monitoring refresh rate for a session
    pub fn set_refresh_rate(&self, session_id: &SessionId, interval_ms: u64) -> Result<(), SshError> {
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

/// Establishes a new SSH connection
///
/// # Tauri Command: `connect_ssh`
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
            SessionId::from(sessionId.clone()),
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

/// Retrieves cached initial output from a session
///
/// # Tauri Command: `get_buffered_ssh_output`
#[tauri::command]
#[allow(non_snake_case)]
pub fn get_buffered_ssh_output(
    state: tauri::State<'_, SshManager>,
    sessionId: String,
) -> Result<Vec<OutputChunk>, SshError> {
    state.get_buffered_ssh_output(&SessionId::from(sessionId))
}

/// Disconnects an SSH session and releases resources
///
/// # Tauri Command: `disconnect_ssh`
#[tauri::command]
#[allow(non_snake_case)]
pub fn disconnect_ssh(
    state: tauri::State<'_, SshManager>,
    sessionId: String,
) -> Result<(), SshError> {
    state.disconnect_ssh(&SessionId::from(sessionId))
}

/// Sends user input to an SSH session
///
/// # Tauri Command: `send_ssh_input`
#[tauri::command]
#[allow(non_snake_case)]
pub fn send_ssh_input(
    state: tauri::State<'_, SshManager>,
    sessionId: String,
    input: String,
) -> Result<(), SshError> {
    state.send_ssh_input(&SessionId::from(sessionId), input)
}

/// Updates the SSH status refresh rate
#[tauri::command]
#[allow(non_snake_case)]
pub async fn set_ssh_status_refresh_rate(
    state: tauri::State<'_, SshManager>,
    sessionId: String,
    intervalMs: u64,
) -> Result<(), SshError> {
    state.set_refresh_rate(&SessionId::from(sessionId), intervalMs)
}

/// Uploads a file to a remote server using SFTP
///
/// # Tauri Command: `upload_file_sftp`
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

/// Probes the current remote working directory
#[tauri::command]
#[allow(non_snake_case)]
pub async fn probe_remote_path(
    state: tauri::State<'_, SshManager>,
    sessionId: String,
) -> Result<String, SshError> {
    state.probe_remote_path(&SessionId::from(sessionId)).await
}
