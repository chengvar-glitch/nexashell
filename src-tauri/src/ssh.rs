use serde::Serialize;
use ssh2::{Session, OpenFlags, OpenType};
use std::collections::HashMap;
use std::io::{Read, Write, Seek, SeekFrom};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tauri::{Emitter, Listener};
use thiserror::Error;
use tokio::sync::mpsc;

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

/// Newtype pattern for type-safe session identifiers
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct SessionId(String);

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        SessionId(s)
    }
}

impl AsRef<str> for SessionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

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

/// Represents a chunk of output data from an SSH session
///
/// Each chunk has a monotonically increasing sequence number
/// to enable reliable client-side buffering and deduplication.
#[derive(Debug, Clone, Serialize)]
pub struct OutputChunk {
    pub seq: u64,
    pub output: String,
    pub ts: u128,
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

/// Server performance metrics
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatus {
    pub cpu_usage: f64,
    pub mem_usage: f64,
    pub mem_total: u64,
    pub mem_used: u64,
    pub disk_usage: f64,
    pub net_up: f64,
    pub net_down: f64,
    pub latency: u32,
}

impl OutputChunk {
    /// Creates a new output chunk with current timestamp
    fn new(seq: u64, output: String) -> Self {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        Self { seq, output, ts }
    }
}

/// Contains state and communication handles for an active SSH channel
pub struct SshChannelInfo {
    /// Asynchronous receiver for SSH output chunks
    pub receiver: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<OutputChunk>>>,

    /// Handle to the background tokio task processing the SSH data
    pub handle: Option<tokio::task::JoinHandle<()>>,

    /// Handle to the background monitoring task
    pub status_handle: Option<tokio::task::JoinHandle<()>>,

    /// Sender to transmit user input to the SSH channel
    pub input_sender: mpsc::UnboundedSender<String>,

    /// Atomic flag to signal the background task to terminate
    pub stop_flag: Arc<AtomicBool>,

    /// Monotonically increasing sequence number for output chunks
    #[allow(dead_code)]
    pub next_seq: Arc<AtomicU64>,

    /// Cached initial output (welcome banner) for late-joining clients
    pub initial_outputs: Arc<tokio::sync::Mutex<Vec<OutputChunk>>>,

    /// Session handle for opening new channels
    pub sess_arc: Arc<tokio::sync::Mutex<Session>>,
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
        cols: u32,
        rows: u32,
    ) -> Result<(), SshError> {
        let sessions_arc = Arc::clone(&self.sessions);
        let channels_arc = Arc::clone(&self.channels);

        let addr = format!("{}:{}", ip, port);
        let username_for_spawn = username.clone();
        let password_for_spawn = password.clone();

        // 1. Establish connection and authenticate (blocking part in separate thread)
        let connection_res = tokio::task::spawn_blocking(move || {
            use std::net::ToSocketAddrs;
            let socket_addr = addr
                .to_socket_addrs()
                .map_err(|e| SshError::ConnectionFailed {
                    host: addr.clone(),
                    port,
                    reason: format!("Failed to resolve address: {}", e),
                })?
                .next()
                .ok_or_else(|| SshError::ConnectionFailed {
                    host: addr.clone(),
                    port,
                    reason: "No addresses found".to_string(),
                })?;

            let tcp =
                TcpStream::connect_timeout(&socket_addr, Duration::from_secs(30)).map_err(|e| {
                    SshError::ConnectionFailed {
                        host: addr.clone(),
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

            sess.userauth_password(&username_for_spawn, &password_for_spawn)
                .map_err(|_| SshError::AuthenticationFailed("Invalid credentials".to_string()))?;

            if !sess.authenticated() {
                return Err(SshError::AuthenticationFailed(
                    "Authentication failed".to_string(),
                ));
            }

            let mut channel = sess
                .channel_session()
                .map_err(|e| SshError::ChannelError(format!("Create channel failed: {}", e)))?;

            channel
                .request_pty("xterm-256color", None, Some((cols, rows, 0, 0)))
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
        let (output_sender, output_receiver) = mpsc::unbounded_channel::<OutputChunk>();
        let (input_sender, input_receiver) = mpsc::unbounded_channel::<String>();
        let stop_flag = Arc::new(AtomicBool::new(false));
        let next_seq = Arc::new(AtomicU64::new(1));
        let initial_outputs = Arc::new(tokio::sync::Mutex::new(Vec::new()));

        let channel_arc = Arc::new(tokio::sync::Mutex::new(channel));
        let sess_arc = Arc::new(tokio::sync::Mutex::new(sess));

        // 3. Register event listeners for user input and resize
        if let Some(h) = &app_handle {
            Self::register_input_listener(h, &session_id, &input_sender, &stop_flag);
            Self::register_resize_listener(h, &session_id, &channel_arc, &stop_flag);
        }

        // 4. Spawn I/O task
        let handle = Self::spawn_io_task(
            channel_arc,
            sess_arc.clone(),
            stop_flag.clone(),
            next_seq.clone(),
            initial_outputs.clone(),
            input_receiver,
            output_sender,
            app_handle.clone(),
            session_id.clone(),
        );

        // 5. Spawn monitoring task
        let status_handle = Self::spawn_monitoring_task(
            app_handle,
            session_id.clone(),
            sess_arc.clone(),
            stop_flag.clone(),
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
                    receiver: Arc::new(tokio::sync::Mutex::new(output_receiver)),
                    handle: Some(handle),
                    status_handle: Some(status_handle),
                    input_sender,
                    stop_flag,
                    next_seq,
                    initial_outputs,
                    sess_arc,
                },
            );
        }

        Ok(())
    }

    /// Registers event listener for user input (keyboard)
    fn register_input_listener(
        app_handle: &tauri::AppHandle,
        session_id: &SessionId,
        input_sender: &mpsc::UnboundedSender<String>,
        stop_flag: &Arc<AtomicBool>,
    ) {
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
        });
    }

    /// Registers event listener for terminal resize events
    fn register_resize_listener(
        app_handle: &tauri::AppHandle,
        session_id: &SessionId,
        channel_arc: &Arc<tokio::sync::Mutex<ssh2::Channel>>,
        stop_flag: &Arc<AtomicBool>,
    ) {
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
        });
    }

    /// Spawns the background I/O task that processes SSH input/output
    fn spawn_io_task(
        channel_arc: Arc<tokio::sync::Mutex<ssh2::Channel>>,
        sess_arc: Arc<tokio::sync::Mutex<Session>>,
        stop_flag: Arc<AtomicBool>,
        next_seq: Arc<AtomicU64>,
        initial_outputs: Arc<tokio::sync::Mutex<Vec<OutputChunk>>>,
        mut input_receiver: mpsc::UnboundedReceiver<String>,
        output_sender: mpsc::UnboundedSender<OutputChunk>,
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

            loop {
                if stop_flag.load(Ordering::SeqCst) {
                    break;
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
                        pending_output.push_str(&String::from_utf8_lossy(&buffer[..n]));
                    }
                    Some(Err(_)) => {
                        stop_flag.store(true, Ordering::SeqCst);
                        break;
                    }
                    None => {
                        // No data available, yield to other tasks
                        tokio::task::yield_now().await;
                    }
                }

                // Check if initial buffering phase has ended
                if in_initial_buffering
                    && initial_buffering_start.elapsed()
                        > Duration::from_millis(INITIAL_BUFFERING_TIMEOUT_MS)
                {
                    in_initial_buffering = false;
                    // Flush any remaining pending output
                    if !pending_output.is_empty() {
                        let seq = next_seq.fetch_add(1, Ordering::SeqCst);
                        let chunk = OutputChunk::new(seq, pending_output.clone());
                        if let Some(h) = &app_handle {
                            let _ = h.emit(&format!("ssh-output-{}", session_id.0), &chunk);
                        }
                        let _ = output_sender.send(chunk);
                        pending_output.clear();
                        last_emit = std::time::Instant::now();
                        seen_first_output = true;
                    }
                }

                // Batch and emit output
                let (size_threshold, time_threshold_ms) =
                    if in_initial_buffering && !seen_first_output {
                        (INITIAL_BATCH_SIZE_THRESHOLD, INITIAL_BATCH_TIME_MS)
                    } else {
                        (NORMAL_BATCH_SIZE_THRESHOLD, NORMAL_BATCH_TIME_MS)
                    };

                if !pending_output.is_empty()
                    && (pending_output.len() > size_threshold
                        || last_emit.elapsed() > Duration::from_millis(time_threshold_ms))
                {
                    let seq = next_seq.fetch_add(1, Ordering::SeqCst);
                    let chunk = OutputChunk::new(seq, pending_output.clone());

                    // Cache initial outputs for late-joining clients
                    if in_initial_buffering {
                        let mut cache = initial_outputs.lock().await;
                        cache.push(chunk.clone());
                    }

                    // Emit event to frontend
                    if let Some(h) = &app_handle {
                        let _ = h.emit(&format!("ssh-output-{}", session_id.0), &chunk);
                    }

                    let _ = output_sender.send(chunk);
                    pending_output.clear();
                    last_emit = std::time::Instant::now();
                    seen_first_output = true;
                }

                // Process queued user input
                while let Ok(input) = input_receiver.try_recv() {
                    let _sess_lock = sess_arc.lock().await;
                    let mut ch = channel_arc.lock().await;
                    let _ = ch.write_all(input.as_bytes()).and_then(|_| ch.flush());
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
                let status_res = {
                    let sess = sess_arc.lock().await;
                    Self::fetch_server_status(&sess, last_cpu_read).await
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

                tokio::time::sleep(Duration::from_millis(1500)).await;
            }
        })
    }

    /// Fetches server performance metrics via a short-lived SSH channel
    async fn fetch_server_status(
        sess: &Session,
        last_cpu: Option<(u64, u64)>,
    ) -> Result<(ServerStatus, (u64, u64)), SshError> {
        let mut channel = sess
            .channel_session()
            .map_err(|e| SshError::ChannelError(e.to_string()))?;

        // Use more robust commands that work on various Linux environments
        // 1. CPU: /proc/stat
        // 2. Mem: free (with fallback)
        // 3. Disk: df (without -b flag which is not standard)
        // 4. Net: /proc/net/dev (with fallback)
        let cmd = "LC_ALL=C awk '/^cpu / {print $2+$3+$4+$5+$6+$7+$8, $5}' /proc/stat 2>/dev/null || echo '0 0'; \
                   LC_ALL=C free -b 2>/dev/null | awk 'NR==2{print $2,$3}' || echo '0 0'; \
                   LC_ALL=C df / 2>/dev/null | awk 'NR==2{print $2,$3,$5}' || echo '0 0 0%'; \
                   LC_ALL=C cat /proc/net/dev 2>/dev/null | awk 'NR>2{rx+=$2; tx+=$10} END{print rx+0,tx+0}' || echo '0 0'";

        loop {
            match channel.exec(cmd) {
                Ok(_) => break,
                Err(ref e) if e.code() == ssh2::ErrorCode::Session(-37) => {
                    tokio::task::yield_now().await;
                }
                Err(e) => return Err(SshError::ChannelError(e.to_string())),
            }
        }

        let mut output = String::new();
        loop {
            let mut buf = [0u8; 1024];
            match channel.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => output.push_str(&String::from_utf8_lossy(&buf[..n])),
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    tokio::task::yield_now().await;
                }
                Err(e) => return Err(SshError::OperationFailed(e.to_string())),
            }
        }

        let lines: Vec<&str> = output
            .lines()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if lines.len() < 4 {
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

        // Parse Memory
        let mem_parts: Vec<u64> = lines[1]
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        let (mem_total, mem_used) = if mem_parts.len() == 2 && mem_parts[0] > 0 {
            (mem_parts[0], mem_parts[1])
        } else {
            (1, 0)
        };
        let mem_usage = ((mem_used as f64 / mem_total as f64) * 100.0).clamp(0.0, 100.0);

        // Parse Disk
        let disk_parts: Vec<&str> = lines[2].split_whitespace().collect();
        let disk_usage = if disk_parts.len() >= 3 {
            disk_parts[2]
                .replace('%', "")
                .parse::<f64>()
                .unwrap_or(0.0)
                .clamp(0.0, 100.0)
        } else {
            0.0
        };

        // Parse Network Raw
        let net_parts: Vec<f64> = lines[3]
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        let (net_down_raw, net_up_raw) = if net_parts.len() == 2 {
            (net_parts[0], net_parts[1])
        } else {
            (0.0, 0.0)
        };

        Ok((
            ServerStatus {
                cpu_usage,
                mem_usage,
                mem_total,
                mem_used,
                disk_usage,
                net_down: net_down_raw,
                net_up: net_up_raw,
                latency: 0,
            },
            (current_cpu_total, current_cpu_idle),
        ))
    }

    /// Retrieves all pending output chunks from a session
    ///
    /// This drains the output receiver, so each chunk is returned only once.
    pub fn get_session_output(&self, session_id: &SessionId) -> Result<Vec<OutputChunk>, SshError> {
        let channels = self
            .channels
            .read()
            .map_err(|e| SshError::LockPoisoned(e.to_string()))?;

        if let Some(channel_info) = channels.get(session_id) {
            let mut outputs = Vec::new();
            let mut receiver = channel_info.receiver.blocking_lock();
            while let Ok(chunk) = receiver.try_recv() {
                outputs.push(chunk);
            }
            Ok(outputs)
        } else {
            Err(SshError::SessionNotFound(session_id.0.clone()))
        }
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
            Err(SshError::SessionNotFound(session_id.0.clone()))
        }
    }

    /// Retrieves cached initial output (welcome banner, login prompts) for a session
    ///
    /// Useful for clients that connect after the session has started.
    pub fn get_buffered_ssh_output(
        &self,
        session_id: &SessionId,
    ) -> Result<Vec<OutputChunk>, SshError> {
        let channels = self
            .channels
            .read()
            .map_err(|e| SshError::LockPoisoned(e.to_string()))?;

        if let Some(channel_info) = channels.get(session_id) {
            let outputs = channel_info.initial_outputs.blocking_lock().clone();
            Ok(outputs)
        } else {
            Err(SshError::SessionNotFound(session_id.0.clone()))
        }
    }

    /// Disconnects a specific SSH session and cleans up resources
    pub fn disconnect_ssh(&self, session_id: &SessionId) -> Result<(), SshError> {
        // Remove from channels and clean up task
        if let Ok(mut channels) = self.channels.write() {
            if let Some(mut info) = channels.remove(session_id) {
                info.stop_flag.store(true, Ordering::SeqCst);
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
        let sess_arc = {
            let channels = self
                .channels
                .read()
                .map_err(|e| SshError::LockPoisoned(e.to_string()))?;
            let info = channels
                .get(&session_id)
                .ok_or_else(|| SshError::SessionNotFound(session_id.as_ref().to_string()))?;
            info.sess_arc.clone()
        };

        // Perform the upload in background thread to avoid blocking the main thread
        // We do NOT await this spawn to ensure true async behavior
        std::thread::spawn(move || {
            let sid = session_id.as_ref().to_string();
            let upload_start = std::time::Instant::now();
            
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
                    let _ = app_handle.emit("upload-progress", UploadProgress {
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
                    let _ = app_handle.emit("upload-progress", UploadProgress {
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
                    let _ = app_handle.emit("upload-progress", UploadProgress {
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

/// Retrieves all pending output chunks from a session
///
/// # Tauri Command: `get_ssh_output`
#[tauri::command]
#[allow(non_snake_case)]
pub fn get_ssh_output(
    state: tauri::State<'_, SshManager>,
    sessionId: String,
) -> Result<Vec<OutputChunk>, SshError> {
    state.get_session_output(&SessionId::from(sessionId))
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
