use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use tauri::{Emitter, Listener};
use thiserror::Error;
use tokio::sync::mpsc;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Debug, Error, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TerminalError {
    #[error("Failed to spawn shell: {0}")]
    SpawnFailed(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("State lock poisoned: {0}")]
    LockPoisoned(String),
}

// ============================================================================
// Constants
// ============================================================================

const TERMINAL_BUFFER_SIZE: usize = 4096;
const BATCH_TIME_MS: u64 = 20;

// ============================================================================
// Data Structures
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct SessionId(String);

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        SessionId(s)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct OutputChunk {
    pub seq: u64,
    pub output: String,
    pub ts: u128,
}

impl OutputChunk {
    fn new(seq: u64, output: String) -> Self {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        Self { seq, output, ts }
    }
}

pub struct TerminalInfo {
    pub handle: Option<tokio::task::JoinHandle<()>>,
    pub input_sender: mpsc::UnboundedSender<String>,
    pub stop_flag: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct TerminalManager {
    channels: Arc<RwLock<HashMap<SessionId, TerminalInfo>>>,
}

impl TerminalManager {
    pub async fn connect_local(
        &self,
        app_handle: Option<tauri::AppHandle>,
        session_id: SessionId,
        cols: u16,
        rows: u16,
    ) -> Result<(), TerminalError> {
        let channels_arc = Arc::clone(&self.channels);

        // 1. Setup PTY
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| TerminalError::SpawnFailed(format!("Failed to open PTY: {}", e)))?;

        // 2. Spawn shell
        #[cfg(target_os = "windows")]
        let shell = "powershell.exe";
        #[cfg(not(target_os = "windows"))]
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "zsh".to_string());

        let mut cmd = CommandBuilder::new(shell);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");

        let _child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| TerminalError::SpawnFailed(format!("Failed to spawn shell: {}", e)))?;

        // 3. Setup communication channels
        let (input_sender, mut input_receiver) = mpsc::unbounded_channel::<String>();
        let stop_flag = Arc::new(AtomicBool::new(false));
        let next_seq = Arc::new(AtomicU64::new(1));

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| TerminalError::SpawnFailed(format!("Failed to clone reader: {}", e)))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| TerminalError::SpawnFailed(format!("Failed to take writer: {}", e)))?;

        // 4. Register event listeners for user input
        let master = Arc::new(Mutex::new(pair.master));
        if let Some(h) = &app_handle {
            Self::register_input_listener(h, &session_id, &input_sender);
            Self::register_resize_listener(h, &session_id, Arc::clone(&master));
        }

        // 5. Spawn I/O tasks
        let session_id_clone = session_id.clone();
        let app_handle_clone = app_handle.clone();
        let mut reader_clone = reader;
        let stop_flag_reader = stop_flag.clone();
        let next_seq_reader = next_seq.clone();

        // Output Task
        let output_handle = tokio::task::spawn_blocking(move || {
            let mut buffer = [0u8; TERMINAL_BUFFER_SIZE];

            loop {
                if stop_flag_reader.load(Ordering::SeqCst) {
                    break;
                }

                match reader_clone.read(&mut buffer) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let seq = next_seq_reader.fetch_add(1, Ordering::SeqCst);
                        let output = String::from_utf8_lossy(&buffer[..n]).to_string();
                        let chunk = OutputChunk::new(seq, output);

                        if let Some(h) = &app_handle_clone {
                            let _ = h.emit(&format!("ssh-output-{}", session_id_clone.0), &chunk);
                        }
                    }
                    Err(_) => break,
                }
            }
            stop_flag_reader.store(true, Ordering::SeqCst);
        });

        // Input Task
        let stop_flag_writer = stop_flag.clone();
        let mut writer_clone = writer;
        tokio::spawn(async move {
            while let Some(input) = input_receiver.recv().await {
                if stop_flag_writer.load(Ordering::SeqCst) {
                    break;
                }
                let _ = writer_clone.write_all(input.as_bytes());
                let _ = writer_clone.flush();
            }
        });

        // 6. Save channel info
        {
            let mut channels = channels_arc
                .write()
                .map_err(|e| TerminalError::LockPoisoned(e.to_string()))?;
            channels.insert(
                session_id,
                TerminalInfo {
                    handle: Some(output_handle),
                    input_sender,
                    stop_flag,
                },
            );
        }

        Ok(())
    }

    fn register_input_listener(
        app_handle: &tauri::AppHandle,
        session_id: &SessionId,
        input_sender: &mpsc::UnboundedSender<String>,
    ) {
        let event_name = format!("ssh-input-{}", session_id.0);
        let input_tx = input_sender.clone();

        app_handle.listen(&event_name, move |event: tauri::Event| {
            #[derive(Deserialize)]
            struct InputPayload {
                input: String,
            }
            if let Ok(payload) = serde_json::from_str::<InputPayload>(event.payload()) {
                let _ = input_tx.send(payload.input);
            }
        });
    }

    fn register_resize_listener(
        app_handle: &tauri::AppHandle,
        session_id: &SessionId,
        master: Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
    ) {
        let resize_event_name = format!("ssh-resize-{}", session_id.0);

        app_handle.listen(&resize_event_name, move |event: tauri::Event| {
            #[derive(Deserialize)]
            struct ResizePayload {
                cols: u16,
                rows: u16,
            }
            if let Ok(payload) = serde_json::from_str::<ResizePayload>(event.payload()) {
                if let Ok(m) = master.lock() {
                    let _ = m.resize(PtySize {
                        rows: payload.rows,
                        cols: payload.cols,
                        pixel_width: 0,
                        pixel_height: 0,
                    });
                }
            }
        });
    }

    pub fn disconnect_local(&self, session_id: &SessionId) -> Result<(), TerminalError> {
        if let Ok(mut channels) = self.channels.write() {
            if let Some(mut info) = channels.remove(session_id) {
                info.stop_flag.store(true, Ordering::SeqCst);
                if let Some(handle) = info.handle.take() {
                    handle.abort();
                }
            }
        }
        Ok(())
    }
}

#[tauri::command]
pub async fn connect_local(
    state: tauri::State<'_, TerminalManager>,
    app_handle: tauri::AppHandle,
    sessionId: String,
    cols: u16,
    rows: u16,
) -> Result<(), TerminalError> {
    state
        .connect_local(Some(app_handle), SessionId::from(sessionId), cols, rows)
        .await
}

#[tauri::command]
pub fn disconnect_local(
    state: tauri::State<'_, TerminalManager>,
    sessionId: String,
) -> Result<(), TerminalError> {
    state.disconnect_local(&SessionId::from(sessionId))
}
