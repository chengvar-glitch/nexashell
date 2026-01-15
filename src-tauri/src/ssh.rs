use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use ssh2::Session;
use std::net::TcpStream;
use std::io::{Read, Write};
use std::time::Duration;
use tauri::{Listener, Emitter};
use std::sync::atomic::{AtomicU64, Ordering, AtomicBool};
use serde::Serialize;
use tokio::sync::mpsc;
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

/// Custom error type for SSH operations with detailed context
#[derive(Debug, Error, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SshError {
    #[error("Failed to connect to {host}:{port} - {reason}")]
    ConnectionFailed { host: String, port: u16, reason: String },
    
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
const INITIAL_BATCH_SIZE_THRESHOLD: usize = 200;
const INITIAL_BATCH_TIME_MS: u64 = 5;

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
    
    /// Sender to transmit user input to the SSH channel
    pub input_sender: mpsc::UnboundedSender<String>,
    
    /// Atomic flag to signal the background task to terminate
    pub stop_flag: Arc<AtomicBool>,
    
    /// Monotonically increasing sequence number for output chunks
    #[allow(dead_code)]
    pub next_seq: Arc<AtomicU64>,
    
    /// Cached initial output (welcome banner) for late-joining clients
    pub initial_outputs: Arc<tokio::sync::Mutex<Vec<OutputChunk>>>,
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
            let tcp = TcpStream::connect(&addr).map_err(|e| {
                SshError::ConnectionFailed {
                    host: addr.clone(),
                    port: 22,
                    reason: e.to_string(),
                }
            })?;
            
            let mut sess = Session::new()
                .map_err(|e| SshError::OperationFailed(format!("Failed to create session: {}", e)))?;
            sess.set_tcp_stream(tcp);
            sess.handshake()
                .map_err(|e| SshError::OperationFailed(format!("Handshake failed: {}", e)))?;
            
            sess.userauth_password(&username_for_spawn, &password_for_spawn)
                .map_err(|_| SshError::AuthenticationFailed("Invalid credentials".to_string()))?;
            
            if !sess.authenticated() {
                return Err(SshError::AuthenticationFailed("Authentication failed".to_string()));
            }

            let mut channel = sess.channel_session()
                .map_err(|e| SshError::ChannelError(format!("Create channel failed: {}", e)))?;
            
            let _ = channel.request_pty("xterm", None, Some((cols, rows, 640, 480)));
            channel.shell()
                .map_err(|e| SshError::ChannelError(format!("Failed to start shell: {}", e)))?;
            
            // Set non-blocking mode for async I/O
            sess.set_blocking(false);
            
            Ok((sess, channel))
        }).await;

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
            Self::register_input_listener(&h, &session_id, &input_sender, &stop_flag);
            Self::register_resize_listener(&h, &session_id, &channel_arc, &stop_flag);
        }

        // 4. Spawn I/O task
        let handle = Self::spawn_io_task(
            channel_arc,
            sess_arc,
            stop_flag.clone(),
            next_seq.clone(),
            initial_outputs.clone(),
            input_receiver,
            output_sender,
            app_handle,
            session_id.clone(),
        );

        // 5. Save session state
        {
            let mut sessions = sessions_arc.write()
                .map_err(|e| SshError::LockPoisoned(e.to_string()))?;
            sessions.insert(session_id.clone(), SshSession { ip, port, username });
            
            let mut channels = channels_arc.write()
                .map_err(|e| SshError::LockPoisoned(e.to_string()))?;
            channels.insert(session_id, SshChannelInfo {
                receiver: Arc::new(tokio::sync::Mutex::new(output_receiver)),
                handle: Some(handle),
                input_sender,
                stop_flag,
                next_seq,
                initial_outputs,
            });
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
                    let _ = ch.request_pty("xterm", None, Some((payload.cols, payload.rows, 0, 0)));
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
            let _keep_alive = sess_arc;
            let mut buffer = [0u8; SSH_BUFFER_SIZE];
            let mut pending_output = String::new();
            let mut last_emit = std::time::Instant::now();
            let mut seen_first_output = false;

            loop {
                if stop_flag.load(Ordering::SeqCst) {
                    break;
                }

                // Attempt non-blocking read from SSH channel
                let read_result = {
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

                // Batch and emit output
                let (size_threshold, time_threshold_ms) = if !seen_first_output && !pending_output.is_empty() {
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
                    if !seen_first_output {
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
                    let mut ch = channel_arc.lock().await;
                    let _ = ch.write_all(input.as_bytes()).and_then(|_| ch.flush());
                }
            }
        })
    }

    /// Retrieves all pending output chunks from a session
    /// 
    /// This drains the output receiver, so each chunk is returned only once.
    pub fn get_session_output(&self, session_id: &SessionId) -> Result<Vec<OutputChunk>, SshError> {
        let channels = self.channels.read()
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
        let channels = self.channels.read()
            .map_err(|e| SshError::LockPoisoned(e.to_string()))?;
        
        if let Some(channel_info) = channels.get(session_id) {
            channel_info.input_sender.send(input)
                .map_err(|_| SshError::ChannelError("Failed to send input".to_string()))
        } else {
            Err(SshError::SessionNotFound(session_id.0.clone()))
        }
    }

    /// Retrieves cached initial output (welcome banner, login prompts) for a session
    /// 
    /// Useful for clients that connect after the session has started.
    pub fn get_buffered_ssh_output(&self, session_id: &SessionId) -> Result<Vec<OutputChunk>, SshError> {
        let channels = self.channels.read()
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
            }
        }
        
        // Remove from sessions
        if let Ok(mut sessions) = self.sessions.write() {
            sessions.remove(session_id);
        }
        
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
