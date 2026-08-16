//! SSH port forwarding / tunneling subsystem.
//!
//! Supports two kinds of forward, mirroring `ssh -L` and `ssh -D`:
//!   - **local**  : bind a listener on the local machine; each inbound
//!     connection is tunnelled over the SSH connection to a remote `host:port`.
//!   - **dynamic**: bind a local listener speaking SOCKS5 (`ssh -D`); clients
//!     connect to arbitrary destinations through the SSH tunnel.
//!
//! Every tunnel opens its own authenticated SSH connection (reusing the same
//! proven connection pattern as long-running SFTP transfers), so tunnels do not
//! contend with the interactive/helper sessions of a session. Tunnels are tied
//! to a session: starting them requires a live session (for credentials), and
//! they are torn down when the session disconnects.

use crate::ssh::{SshAuth, SshManager};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::JoinHandle;
use std::time::Duration;

/// How this tunnel forwards connections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TunnelDirection {
    /// `ssh -L` : forward local listener -> remote target over SSH.
    Local,
    /// `ssh -D` : a local SOCKS5 proxy that tunnels arbitrary destinations.
    Dynamic,
}

/// A persisted forwarding rule, stored on its session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelRule {
    pub id: String,
    pub direction: TunnelDirection,
    /// Listener interface to bind locally. Defaults to "127.0.0.1".
    #[serde(default = "default_listen_host")]
    pub listen_host: String,
    /// Listener port to bind locally.
    pub listen_port: u16,
    /// Remote forwarding target host (local/direct only).
    #[serde(default)]
    pub target_host: String,
    /// Remote forwarding target port (local/direct only).
    pub target_port: u16,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_listen_host() -> String {
    "127.0.0.1".to_string()
}
fn default_enabled() -> bool {
    true
}

/// Runtime status reported back to the frontend.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TunnelStatus {
    pub rule_id: String,
    pub direction: TunnelDirection,
    pub listen_host: String,
    pub listen_port: u16,
    pub target_host: String,
    pub target_port: u16,
    pub state: &'static str,
    pub accepted: u64,
    pub error: Option<String>,
}

/// A single running tunnel (one rule) and its accept loop.
struct SshTunnel {
    rule_id: String,
    status: Arc<RwLock<TunnelStatus>>,
    stop: Arc<AtomicBool>,
    join: Arc<Mutex<Option<JoinHandle<()>>>>,
}

/// Manages all tunnels across every live session, keyed by session id.
#[derive(Default)]
pub struct TunnelManager {
    tunnels: Arc<RwLock<HashMap<String, Vec<Arc<SshTunnel>>>>>,
}

impl TunnelManager {
    /// Start every enabled rule for a session. Tunnels that fail to bind are
    /// recorded as `failed` in status rather than aborting the whole batch.
    pub fn start_session_tunnels(
        &self,
        session_id: &str,
        rules: Vec<TunnelRule>,
        conn: (String, String, u16, SshAuth),
    ) -> Vec<TunnelStatus> {
        let mut started: Vec<Arc<SshTunnel>> = Vec::new();
        let mut statuses: Vec<TunnelStatus> = Vec::new();

        for rule in rules {
            if !rule.enabled {
                continue;
            }
            let initial = Self::initial_status(&rule);
            let status = Arc::new(RwLock::new(initial.clone()));

            let tunnel = Arc::new(SshTunnel {
                rule_id: rule.id.clone(),
                status: status.clone(),
                stop: Arc::new(AtomicBool::new(false)),
                join: Arc::new(Mutex::new(None)),
            });

            let (a, h, p) = (conn.0.clone(), conn.1.clone(), conn.2);
            let au = conn.3.clone();
            let tstop = tunnel.stop.clone();
            let tstatus = status.clone();

            let rule_id = rule.id.clone();
            let thread_name = format!("nexashell-tunnel-{}", rule_id);
            let handle = std::thread::Builder::new()
                .name(thread_name)
                .spawn(move || Self::run_listener(a, h, p, au, rule, tstop, tstatus));

            match handle {
                Ok(h) => {
                    // Wait briefly (<=300ms) for bind so failures surface in
                    // the returned statuses instead of appearing only on query.
                    let deadline = std::time::Instant::now() + Duration::from_millis(300);
                    loop {
                        let st = status.read().unwrap();
                        let done = st.state == "listening" || st.state == "failed";
                        if done {
                            statuses.push(st.clone());
                            break;
                        }
                        drop(st);
                        if std::time::Instant::now() > deadline {
                            statuses.push(status.read().unwrap().clone());
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    *tunnel.join.lock().unwrap() = Some(h);
                    started.push(tunnel);
                }
                Err(e) => {
                    let mut st = initial;
                    st.state = "failed";
                    st.error = Some(format!("Failed to spawn tunnel thread: {}", e));
                    statuses.push(st);
                }
            }
        }

        if !started.is_empty() {
            let mut map = self.tunnels.write().unwrap();
            map.entry(session_id.to_string())
                .or_default()
                .extend(started);
        }
        statuses
    }

    /// Start a single rule (used for manually toggling a rule after connect).
    pub fn start_tunnel(
        &self,
        session_id: &str,
        rule: TunnelRule,
        conn: (String, String, u16, SshAuth),
    ) -> Vec<TunnelStatus> {
        let rule_id = rule.id.clone();
        // Stop any stale listener already bound for this rule, then start fresh.
        self.stop_tunnel(session_id, &rule_id);
        self.start_session_tunnels(session_id, vec![rule], conn)
    }

    /// Stop all tunnels for a session; returns true if any were running.
    pub fn stop_session_tunnels(&self, session_id: &str) -> bool {
        let removed = {
            let mut map = self.tunnels.write().unwrap();
            map.remove(session_id)
        };
        if let Some(list) = removed {
            for t in &list {
                t.stop.store(true, Ordering::SeqCst);
            }
            for t in list {
                if let Some(h) = t.join.lock().unwrap().take() {
                    h.thread().unpark();
                }
            }
            true
        } else {
            false
        }
    }

    /// Stop a single rule within a session.
    pub fn stop_tunnel(&self, session_id: &str, rule_id: &str) {
        let target: Option<Arc<SshTunnel>> = {
            let mut map = self.tunnels.write().unwrap();
            if let Some(list) = map.get_mut(session_id) {
                let found = list.iter().find(|t| t.rule_id == rule_id).cloned();
                if found.is_some() {
                    list.retain(|t| t.rule_id != rule_id);
                }
                found
            } else {
                None
            }
        };
        if let Some(t) = target {
            t.stop.store(true, Ordering::SeqCst);
        }
        let empty = self
            .tunnels
            .read()
            .unwrap()
            .get(session_id)
            .map(|l| l.is_empty())
            .unwrap_or(false);
        if empty {
            self.tunnels.write().unwrap().remove(session_id);
        }
    }

    /// Current status of all tunnels for a session.
    pub fn status_all(&self, session_id: &str) -> Vec<TunnelStatus> {
        let map = self.tunnels.read().unwrap();
        let mut out = Vec::new();
        if let Some(list) = map.get(session_id) {
            for t in list {
                out.push(t.status.read().unwrap().clone());
            }
        }
        out
    }

    fn initial_status(rule: &TunnelRule) -> TunnelStatus {
        TunnelStatus {
            rule_id: rule.id.clone(),
            direction: rule.direction,
            listen_host: rule.listen_host.clone(),
            listen_port: rule.listen_port,
            target_host: rule.target_host.clone(),
            target_port: rule.target_port,
            state: "starting",
            accepted: 0,
            error: None,
        }
    }

    /// Own the accept loop for a single tunnel rule until `stop` is set.
    fn run_listener(
        conn_addr: String,
        host_for_err: String,
        conn_port: u16,
        auth: SshAuth,
        rule: TunnelRule,
        stop: Arc<AtomicBool>,
        status: Arc<RwLock<TunnelStatus>>,
    ) {
        let listener = match TcpListener::bind((rule.listen_host.as_str(), rule.listen_port)) {
            Ok(l) => {
                status.write().unwrap().state = "listening";
                l
            }
            Err(e) => {
                let mut st = status.write().unwrap();
                st.state = "failed";
                st.error = Some(format!(
                    "Failed to bind {}:{}: {}",
                    rule.listen_host, rule.listen_port, e
                ));
                return;
            }
        };
        log::info!(
            "Tunnel {} listening on {}:{} -> {}:{}",
            rule.id,
            rule.listen_host,
            rule.listen_port,
            rule.target_host,
            rule.target_port
        );

        while !stop.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, peer)) => {
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                    let (a, h, p) = (conn_addr.clone(), host_for_err.clone(), conn_port);
                    let au = auth.clone();
                    let tstop = stop.clone();
                    let rule_id = rule.id.clone();
                    let direction = rule.direction;
                    let target_host = rule.target_host.clone();
                    let target_port = rule.target_port;
                    status.write().unwrap().accepted += 1;

                    let _ = std::thread::Builder::new()
                        .name(format!("nexashell-tunnel-conn-{}", rule_id))
                        .spawn(move || {
                            let _ = Self::handle_connection(
                                direction,
                                &a,
                                &h,
                                p,
                                &au,
                                stream,
                                peer,
                                &target_host,
                                target_port,
                                &tstop,
                            );
                        });
                }
                Err(e) => {
                    if !stop.load(Ordering::SeqCst) {
                        log::debug!("Tunnel {} accept error: {}", rule.id, e);
                        std::thread::sleep(Duration::from_millis(50));
                    }
                }
            }
        }
        status.write().unwrap().state = "stopped";
    }

    /// Handle a single accepted client connection. Opens a fresh authenticated
    /// SSH connection (so tunnelled traffic never contends with the live
    /// interactive/helper sessions) and pumps bytes in both directions on the
    /// dedicated channel.
    #[allow(clippy::too_many_arguments)]
    fn handle_connection(
        direction: TunnelDirection,
        addr: &str,
        host_for_err: &str,
        port: u16,
        auth: &SshAuth,
        mut client: TcpStream,
        client_peer: SocketAddr,
        default_target: &str,
        default_target_port: u16,
        stop: &AtomicBool,
    ) -> Result<(), String> {
        let _ = client.set_nodelay(true);

        // Dynamic SOCKS tunnels negotiate the destination first.
        let (remote_host, remote_port) = if direction == TunnelDirection::Dynamic {
            socks5_negotiate(&mut client)?
        } else {
            (default_target.to_string(), default_target_port)
        };

        let sess = SshManager::connect_authenticated(addr, host_for_err, port, auth)
            .map_err(|e| format!("SSH connect failed: {}", e))?;
        // Non-blocking so the single-threaded pump below can service both
        // directions without deadlocking on a blocking read.
        sess.set_blocking(false);

        let client_ip = client_peer.ip().to_string();
        let mut chan = sess
            .channel_direct_tcpip(
                &remote_host,
                remote_port,
                Some((client_ip.as_str(), client_peer.port())),
            )
            .map_err(|e| format!("Failed to open direct-tcpip channel to {}:{}: {}", remote_host, remote_port, e))?;

        pump_bidirectional(&mut client, &mut chan, stop);
        Ok(())
    }
}

/// Bidirectionally copy bytes between a local client stream and an SSH channel.
/// Ownership: both live on this one thread, and the channel is non-blocking so a
/// single-threaded poll loop can service both directions without deadlocking.
fn pump_bidirectional(
    client: &mut TcpStream,
    chan: &mut ssh2::Channel,
    stop: &AtomicBool,
) {
    let mut chan_buf = [0u8; 16384];
    let mut client_buf = [0u8; 16384];
    let _ = client.set_read_timeout(Some(Duration::from_millis(40)));

    loop {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        // SSH channel -> client
        match chan.read(&mut chan_buf) {
            Ok(0) => break, // EOF on channel
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
                break; // client closed
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
/// (host + port). Only supports the no-auth + CONNECT subset of RFC 1928, which
/// is all a personal proxy tunnel needs.
fn socks5_negotiate(stream: &mut TcpStream) -> Result<(String, u16), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(30)))
        .map_err(|e| e.to_string())?;

    // Greeting: VER(5) NMETHODS methods...
    let mut header = [0u8; 2];
    stream.read_exact(&mut header).map_err(|e| format!("SOCKS read greeting: {}", e))?;
    if header[0] != 0x05 {
        return Err(format!("Not a SOCKS5 client (ver={})", header[0]));
    }
    let nmethods = header[1] as usize;
    let mut methods = vec![0u8; nmethods];
    stream.read_exact(&mut methods).map_err(|e| format!("SOCKS read methods: {}", e))?;
    // Reply: no-auth
    stream.write_all(&[0x05, 0x00]).map_err(|e| format!("SOCKS write method reply: {}", e))?;

    // Request: VER(5) CMD(1) RSV(1) ATYP(1) ADDR PORT(2)
    let mut req0 = [0u8; 4];
    stream.read_exact(&mut req0).map_err(|e| format!("SOCKS read request: {}", e))?;
    if req0[0] != 0x05 {
        return Err(format!("SOCKS5 request bad ver: {}", req0[0]));
    }
    if req0[1] != 0x01 {
        return Err(format!("Only CONNECT supported (cmd={})", req0[1]));
    }
    let atyp = req0[3];
    let host = match atyp {
        0x01 => {
            // IPv4
            let mut a = [0u8; 4];
            stream.read_exact(&mut a).map_err(|e| format!("SOCKS read ipv4: {}", e))?;
            a.iter()
                .map(|b| b.to_string())
                .collect::<Vec<_>>()
                .join(".")
        }
        0x03 => {
            // Domain name
            let mut lenb = [0u8; 1];
            stream.read_exact(&mut lenb).map_err(|e| format!("SOCKS read domain len: {}", e))?;
            let mut dom = vec![0u8; lenb[0] as usize];
            stream.read_exact(&mut dom).map_err(|e| format!("SOCKS read domain: {}", e))?;
            String::from_utf8_lossy(&dom).to_string()
        }
        0x04 => {
            // IPv6
            let mut a = [0u8; 16];
            stream
                .read_exact(&mut a)
                .map_err(|e| format!("SOCKS read ipv6: {}", e))?;
            let octets = a
                .chunks(2)
                .map(|c| format!("{:02x}{:02x}", c[0], c[1]))
                .collect::<Vec<_>>();
            [
                octets[0].clone(),
                octets[1].clone(),
                octets[2].clone(),
                octets[3].clone(),
                octets[4].clone(),
                octets[5].clone(),
                octets[6].clone(),
                octets[7].clone(),
            ]
            .join(":")
        }
        _ => return Err(format!("Unsupported SOCKS5 address type: {}", atyp)),
    };
    let mut portb = [0u8; 2];
    stream
        .read_exact(&mut portb)
        .map_err(|e| format!("SOCKS read port: {}", e))?;
    let port = u16::from_be_bytes(portb);

    // Success reply.
    stream
        .write_all(&[0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .map_err(|e| format!("SOCKS write reply: {}", e))?;

    let _ = stream.set_read_timeout(None);
    let _ = stream.set_write_timeout(None);
    stream.set_nodelay(true).ok();

    Ok((host, port))
}

// ----------------------------------------------------------------------------
// Tauri command wrappers
// ----------------------------------------------------------------------------

/// Convert a DB tunnel-rule row into the tunnel subsystem's rule shape.
fn rule_from_row(row: crate::db::TunnelRuleRow) -> TunnelRule {
    TunnelRule {
        id: row.id,
        direction: if row.direction == "dynamic" {
            TunnelDirection::Dynamic
        } else {
            TunnelDirection::Local
        },
        listen_host: row.listen_host,
        listen_port: row.listen_port as u16,
        target_host: row.target_host,
        target_port: row.target_port as u16,
        enabled: row.enabled,
    }
}

/// Start every enabled, persisted tunnel rule for a live session.
#[tauri::command]
#[allow(non_snake_case)]
pub fn start_session_tunnels(
    tunnel_state: tauri::State<'_, TunnelManager>,
    ssh_state: tauri::State<'_, crate::ssh::SshManager>,
    sessionId: String,
) -> Result<Vec<TunnelStatus>, String> {
    let rules = crate::db::list_tunnel_rules(sessionId.clone())?;
    let rules: Vec<TunnelRule> = rules.into_iter().map(rule_from_row).collect();
    let conn = ssh_state
        .session_connection(&crate::common::SessionId::from(sessionId.clone()))
        .map_err(|e| format!("Session is not connected: {}", e))?;
    Ok(tunnel_state.start_session_tunnels(&sessionId, rules, conn))
}

/// Start a single rule (used for manual toggles after connect). The rule must
/// already be persisted; it is re-read here so the command is self-contained.
#[tauri::command]
#[allow(non_snake_case)]
pub fn start_tunnel_rule(
    tunnel_state: tauri::State<'_, TunnelManager>,
    ssh_state: tauri::State<'_, crate::ssh::SshManager>,
    sessionId: String,
    ruleId: String,
) -> Result<Vec<TunnelStatus>, String> {
    let rules = crate::db::list_tunnel_rules(sessionId.clone())?;
    let rule = rules
        .into_iter()
        .find(|r| r.id == ruleId)
        .ok_or_else(|| format!("Tunnel rule not found: {}", ruleId))?;
    let conn = ssh_state
        .session_connection(&crate::common::SessionId::from(sessionId.clone()))
        .map_err(|e| format!("Session is not connected: {}", e))?;
    Ok(tunnel_state.start_tunnel(&sessionId, rule_from_row(rule), conn))
}

/// Stop all tunnels for a session.
#[tauri::command]
#[allow(non_snake_case)]
pub fn stop_session_tunnels(
    tunnel_state: tauri::State<'_, TunnelManager>,
    sessionId: String,
) -> Result<(), String> {
    tunnel_state.stop_session_tunnels(&sessionId);
    Ok(())
}

/// Stop a single tunnel rule for a session.
#[tauri::command]
#[allow(non_snake_case)]
pub fn stop_tunnel_rule(
    tunnel_state: tauri::State<'_, TunnelManager>,
    sessionId: String,
    ruleId: String,
) -> Result<(), String> {
    tunnel_state.stop_tunnel(&sessionId, &ruleId);
    Ok(())
}

/// Report the current status of all tunnels for a session.
#[tauri::command]
#[allow(non_snake_case)]
pub fn list_tunnel_status(
    tunnel_state: tauri::State<'_, TunnelManager>,
    sessionId: String,
) -> Result<Vec<TunnelStatus>, String> {
    Ok(tunnel_state.status_all(&sessionId))
}
