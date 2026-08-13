# NexaShell

Lightweight, modern terminal manager and SSH client built with **Rust** and **Vue 3**, packaged as a Tauri desktop application.

[![Version](https://img.shields.io/badge/Version-1.3.1-blue.svg)](https://github.com/chengvar-glitch/nexashell)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

NexaShell combines the safety and performance of Rust with a modern, high-productivity web-based UI to provide a seamless server management experience.

---

## 🚀 Key Features

- **Multi-Session Management**: Organize and switch between multiple SSH sessions using a robust tab-based interface.
- **Session Persistence & Grouping**: Securely store server credentials with support for hierarchical grouping and custom tagging (AES-GCM encrypted).
- **Hardware-Accelerated Terminal**: Integrated terminal with low-latency rendering powered by `xterm.js` and WebGL.
- **Integrated SFTP Support**: Built-in file explorer and transfer capabilities for easy remote file manipulation.
- **Real-time Server Dashboard**: Monitor remote server status (CPU, Memory, Disk) directly from the connection view.
- **Customizable Workspace**: Support for Dark/Light modes and flexible UI layouts.
- **Cross-Platform**: Production-ready for macOS (Apple Silicon/Intel), Windows, and Linux.

---

## 🛠️ Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- [Node.js](https://nodejs.org/) (v18+)
- [bun](https://bun.sh/) (>=1.0)

### Build from Source

1. **Clone the repository**
   ```bash
   git clone https://github.com/chengvar/nexashell.git
   cd nexashell
   ```

2. **Install dependencies**
   ```bash
   bun install
   ```

3. **Run in development mode**
   ```bash
   bun tauri dev
   ```

4. **Build for production**
   ```bash
   bun tauri build
   ```

---

## 🏗️ Architecture Design

NexaShell adopts a **Multi-Process Architecture** powered by [Tauri](https://tauri.app/), separating the UI concerns from the low-level system operations.

### High-Level Overview

```mermaid
graph TD
    subgraph "Frontend Layer (Vue 3)"
        UI[User Interface Components]
        Store[Pinia State Management]
        EB[Event Bus / Shortcuts]
    end

    subgraph "Bridge (Tauri IPC)"
        Invoke[Tauri Invoke/Events]
    end

    subgraph "Backend Layer (Rust Core)"
        SM[SshManager]
        TM[TerminalManager]
        DB[Database Protocol]
        SEC[Encryption Module]
    end

    UI <--> Invoke <--> SM
    UI <--> Invoke <--> TM
    Store <--> Invoke <--> DB
    SM <--> SSH[Remote Server via SSH2]
```

### 🧩 Layer Responsibilities

- **Frontend (View Layer)**: Built with Vue 3 and TypeScript. Pinia handles session lifecycle, user settings, and UI states. Uses `xterm.js` with WebGL acceleration for GPU-accelerated rendering.
- **Backend (Service Layer)**: Built on `ssh2-rs` and `tokio`. Handles high-performance, non-blocking SSH communication and local data persistence with AES encryption.

---

## 🔗 Key Workflows

### SSH Connection Sequence

```mermaid
sequenceDiagram
    participant U as User
    participant F as Vue Frontend
    participant B as Rust Backend (SshManager)
    participant S as Remote Server

    U->>F: Input Credentials & Click "Connect"
    F->>F: Validate Inputs
    F->>B: tauri::invoke("connect_ssh", config)
    B->>B: Initialize SSH2 Session
    B->>S: TCP Handshake (IP:Port)
    S-->>B: TCP Connected
    B->>S: SSH Key Exchange & Auth
    S-->>B: Auth Successful
    B->>B: Spawn Non-blocking Read/Write Tasks
    B-->>F: Success Response + SessionID
    F->>F: Initialize Terminal UI & Tab
```

### Terminal I/O Stream

```mermaid
sequenceDiagram
    participant UI as Terminal UI
    participant PC as Pinia / Event Manager
    participant RB as Rust Backend
    participant RS as Remote Shell

    UI->>PC: User Keypress (e.g., 'ls')
    PC->>RB: tauri::invoke("send_ssh_input", data)
    RB->>RS: Write to SSH Channel
    RS-->>RB: Stdout/Stderr Data
    RB->>RB: Buffer & Optimize Output
    RB->>PC: tauri::emit("ssh_output_event")
    PC->>UI: Update Terminal Buffer
```

---

## 🛠️ Installation & Development

### Prerequisites

- **Node.js**: (recommended via nvm) and **bun**
- **Rust toolchain**: (stable) and **cargo**
- **Tauri CLI dependencies**: Platform-specific (see [Tauri documentation](https://tauri.app/start/prerequisites))

### Quick Setup

```bash
# Install frontend dependencies
bun install

# Run native development (opens the Tauri window)
bun tauri dev
```

### Development Commands

- **Run Vite dev server** (Web only): `bun dev`
- **Run full native app**: `bun tauri dev`
- **Build production bundle**:
  ```bash
  bun build
  bun tauri build
  ```
- **Run production build**:
  ```bash
  bun build          # Frontend (type-check + Vite build)
  bun tauri build    # Desktop bundle (DMG/app, MSI/NSIS, AppImage/deb)
  ```
- **Lint & type-check**:
  ```bash
  bun lint           # ESLint with auto-fix
  bun lint:check     # ESLint without auto-fix (CI-friendly)
  bun type-check     # vue-tsc --noEmit
  ```
- **Tests**:
  ```bash
  bun test           # Run frontend (Vitest) unit tests
  bun test:coverage  # Frontend tests with coverage report
  cargo test         # Rust unit tests (run from src-tauri/)
  ```

---

## ⚙️ Technical Details

### Key IPC / Tauri Commands

The backend exposes these Tauri commands and events (implemented in `src-tauri/src/ssh.rs`, `src-tauri/src/db.rs`):

- **SSH Commands**: `connect_ssh`, `disconnect_ssh`, `send_ssh_input`, `upload_file_sftp`, `probe_remote_path`.
- **Database Management**: `list_sessions`, `add_session`, `save_session`, `add_group`, `list_groups`.
- **System**: `get_platform`, `read_file_preview`, `toggle_maximize`.

### Project Structure

- `src/` — Frontend renderer (Vue 3 + TypeScript)
  - `src/components/` — UI components (SSH form, Terminal, Dashboards)
  - `src/features/` — Feature modules (session, settings, tabs)
  - `src/core/` — Core utilities (i18n, theme, logger, event bus)
- `src-tauri/` — Rust backend
  - `src-tauri/src/ssh.rs` — SSH manager and channel implementation
  - `src-tauri/src/db.rs` — SQLite database manager
  - `src-tauri/src/lib.rs` — Tauri initialization

---

## 🛡️ Security

- **Credential Safety**: All passwords and private keys are encrypted locally before being stored in the SQLite database.
- **Rust Memory Safety**: The core SSH logic is implemented in memory-safe Rust, preventing common security vulnerabilities.
- **Sandboxed WebView**: The frontend runs in a restricted context with communication via secure IPC.

---

**NexaShell** is licensed under the [MIT License](LICENSE).
