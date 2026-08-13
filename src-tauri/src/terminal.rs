use crate::common::{OutputChunk, SessionId};
use portable_pty::{Child, CommandBuilder, PtySize, native_pty_system};
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

    #[error("State lock poisoned: {0}")]
    LockPoisoned(String),
}

// ============================================================================
// Constants
// ============================================================================

const TERMINAL_BUFFER_SIZE: usize = 4096;

// ============================================================================
// Data Structures
// ============================================================================

pub struct TerminalInfo {
    pub output_handle: Option<tokio::task::JoinHandle<()>>,
    pub input_handle: Option<tokio::task::JoinHandle<()>>,
    pub stop_flag: Arc<AtomicBool>,
    pub child: Option<Arc<std::sync::Mutex<Box<dyn Child + Send>>>>,
    /// Event listener IDs so we can unlisten them on disconnect.
    pub input_listener_id: Option<tauri::EventId>,
    pub resize_listener_id: Option<tauri::EventId>,
    pub app_handle: Option<tauri::AppHandle>,
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

        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| TerminalError::SpawnFailed(format!("Failed to open PTY: {}", e)))?;

        #[cfg(target_os = "windows")]
        let shell = "powershell.exe";
        #[cfg(not(target_os = "windows"))]
        let shell = std::env::var("SHELL")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "/bin/bash".to_string());

        let mut cmd = CommandBuilder::new(&shell);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");

        let child = pair.slave.spawn_command(cmd).map_err(|e| {
            TerminalError::SpawnFailed(format!("Failed to spawn shell '{}': {}", shell, e))
        })?;

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

        // Register event listeners and retain their IDs for cleanup.
        let master = Arc::new(Mutex::new(pair.master));
        let (input_listener_id, resize_listener_id) = if let Some(h) = &app_handle {
            let in_id = Self::register_input_listener(h, &session_id, &input_sender);
            let rs_id = Self::register_resize_listener(h, &session_id, Arc::clone(&master));
            (Some(in_id), Some(rs_id))
        } else {
            (None, None)
        };

        let session_id_clone = session_id.clone();
        let app_handle_clone = app_handle.clone();
        let stop_flag_reader = stop_flag.clone();
        let next_seq_reader = next_seq.clone();
        let mut reader_clone = reader;

        let output_handle = tokio::task::spawn_blocking(move || {
            let mut buffer = [0u8; TERMINAL_BUFFER_SIZE];

            loop {
                if stop_flag_reader.load(Ordering::SeqCst) {
                    break;
                }

                match reader_clone.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        let seq = next_seq_reader.fetch_add(1, Ordering::SeqCst);
                        let output = String::from_utf8_lossy(&buffer[..n]).into_owned();
                        let chunk = OutputChunk::new(seq, output);

                        if let Some(h) = &app_handle_clone {
                            let _ = h.emit(&format!("ssh-output-{}", session_id_clone.0), &chunk);
                        }
                    }
                    Err(e) => {
                        log::debug!("[local PTY] read ended: {}", e);
                        break;
                    }
                }
            }
            stop_flag_reader.store(true, Ordering::SeqCst);
        });

        // Input writer runs on an async task. PTY master writes are normally
        // fast; if the buffer is full the OS will apply backpressure.
        // Drains all currently-pending inputs into a single buffer and issues
        // one write per batch, so large pastes don't spawn one syscall per
        // input event.
        let stop_flag_writer = stop_flag.clone();
        let mut writer_clone = writer;
        let input_handle = tokio::spawn(async move {
            let mut batch = Vec::<u8>::with_capacity(4096);
            loop {
                if stop_flag_writer.load(Ordering::SeqCst) {
                    break;
                }

                // Wait for at least one input, then drain everything else that
                // is already queued (non-blocking) into the same batch.
                match input_receiver.recv().await {
                    Some(input) => batch.extend_from_slice(input.as_bytes()),
                    None => break,
                }
                while let Ok(input) = input_receiver.try_recv() {
                    batch.extend_from_slice(input.as_bytes());
                }

                if batch.is_empty() {
                    continue;
                }

                if writer_clone
                    .write_all(&batch)
                    .and_then(|_| writer_clone.flush())
                    .is_err()
                {
                    break;
                }
                batch.clear();
            }
        });

        {
            let mut channels = channels_arc
                .write()
                .map_err(|e| TerminalError::LockPoisoned(e.to_string()))?;
            channels.insert(
                session_id,
                TerminalInfo {
                    output_handle: Some(output_handle),
                    input_handle: Some(input_handle),
                    stop_flag,
                    child: Some(Arc::new(std::sync::Mutex::new(child))),
                    input_listener_id,
                    resize_listener_id,
                    app_handle: app_handle.clone(),
                },
            );
        }

        Ok(())
    }

    fn register_input_listener(
        app_handle: &tauri::AppHandle,
        session_id: &SessionId,
        input_sender: &mpsc::UnboundedSender<String>,
    ) -> tauri::EventId {
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
        })
    }

    fn register_resize_listener(
        app_handle: &tauri::AppHandle,
        session_id: &SessionId,
        master: Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
    ) -> tauri::EventId {
        let resize_event_name = format!("ssh-resize-{}", session_id.0);

        app_handle.listen(&resize_event_name, move |event: tauri::Event| {
            #[derive(Deserialize)]
            struct ResizePayload {
                cols: u16,
                rows: u16,
            }
            if let Ok(payload) = serde_json::from_str::<ResizePayload>(event.payload())
                && let Ok(m) = master.lock()
                && let Err(e) = m.resize(PtySize {
                    rows: payload.rows,
                    cols: payload.cols,
                    pixel_width: 0,
                    pixel_height: 0,
                })
            {
                log::warn!("PTY resize failed: {}", e);
            }
        })
    }

    pub fn disconnect_local(&self, session_id: &SessionId) -> Result<(), TerminalError> {
        if let Ok(mut channels) = self.channels.write()
            && let Some(mut info) = channels.remove(session_id)
        {
            info.stop_flag.store(true, Ordering::SeqCst);

            if let Some(handle) = info.output_handle.take() {
                handle.abort();
            }
            if let Some(handle) = info.input_handle.take() {
                handle.abort();
            }

            // Unregister event listeners — this was missing previously and
            // caused the closures (which hold the PTY master Arc) to live
            // for the lifetime of the app.
            if let Some(ref app_handle) = info.app_handle {
                if let Some(id) = info.input_listener_id.take() {
                    app_handle.unlisten(id);
                }
                if let Some(id) = info.resize_listener_id.take() {
                    app_handle.unlisten(id);
                }
            }

            if let Some(child_arc) = info.child.take()
                && let Ok(mut child) = child_arc.lock()
            {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        Ok(())
    }

    /// Disconnect all active local terminals. Called on app exit so child
    /// processes are not orphaned.
    pub fn disconnect_all(&self) {
        let ids: Vec<SessionId> = if let Ok(channels) = self.channels.read() {
            channels.keys().cloned().collect()
        } else {
            Vec::new()
        };
        for id in ids {
            let _ = self.disconnect_local(&id);
        }
    }
}

#[tauri::command]
#[allow(non_snake_case)]
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
#[allow(non_snake_case)]
pub fn disconnect_local(
    state: tauri::State<'_, TerminalManager>,
    sessionId: String,
) -> Result<(), TerminalError> {
    state.disconnect_local(&SessionId::from(sessionId))
}
