# NexaShell

Lightweight terminal manager and SSH client packaged as a Tauri desktop application.

**Status:** active development

**License:** MIT — see LICENSE

**Contents**

- **Overview** — what the project is
- **Installation** — dependencies and local setup
- **Development** — how to develop and run locally
- **Running** — building and running the app
- **Terminal architecture** — how frontend and backend interact
- **Key IPC commands** — Tauri RPC surface for SSH
- **Project structure** — important files and locations

**Overview**

NexaShell is a desktop terminal manager built with Vue 3 + TypeScript (Vite) and packaged with Tauri (Rust) for native integrations. The frontend renders terminals and UI; the Rust backend manages SSH connections and system-level operations.

**Installation (macOS / Linux / Windows)**

Prerequisites:

- Node.js (recommended via nvm) and pnpm
- Rust toolchain (stable) and cargo
- Tauri CLI dependencies (platform-specific; see Tauri docs)

Quick setup:

```bash
# install frontend deps
pnpm install

# (optional) ensure Rust toolchain is installed
rustup toolchain install stable

# run native development (opens the Tauri window)
pnpm tauri dev
```

If you only want to run the web/renderer side during UI development:

```bash
pnpm dev
```

**Development**

- Run the Vite dev server with hot reload:

```bash
pnpm dev
```

- Run the full native app (Tauri) for end-to-end testing:

```bash
pnpm tauri dev
```

- Build production renderer assets and package the Tauri app:

```bash
pnpm build
pnpm tauri build
```

- Tests:

```bash
pnpm test         # unit tests (vitest)
pnpm test:ui      # Vitest UI
pnpm test:coverage
```

**Running (release)**

After `pnpm build`, package using Tauri to produce platform-native bundles (macOS .app/.dmg, Windows .msi/.exe, etc.):

```bash
pnpm tauri build
```

Artifacts will be created in the Tauri target folders (see `src-tauri/target/`).

**Terminal architecture**

The terminal subsystem is split between the frontend (renderer) and the Rust backend:

- Frontend
  - Built with Vue 3 + TypeScript and uses `xterm.js` for terminal rendering (see `src/components/terminal/TerminalView.vue`).
  - The UI manages tabs, session state, and dispatches resize / input events to the backend via Tauri events.

- Backend (Rust, Tauri)
  - The SSH connection manager is implemented in `src-tauri/src/ssh.rs` and exposes a lightweight session/channel model.
  - `SshManager` tracks active `SshSession` entries and `SshChannelInfo` per session. Channels encapsulate:
    - an asynchronous receiver for output chunks,
    - a sender for frontend input,
    - a background Tokio task that reads/writes the SSH channel,
    - a shutdown-capable `TcpStream` used to terminate connections cleanly.
  - Important runtime behaviors implemented in the backend:
    - PTY allocation and dynamic resize handling (requests `request_pty` on the SSH channel),
    - Non-blocking SSH session I/O with batching: backend accumulates output into `OutputChunk`s and emits them periodically to reduce event overhead,
    - Backpressure protection for frontend input: input processing limits to avoid starving the read loop or blocking the Tokio task.

**Key IPC / Tauri commands (RPC surface)**

The backend exposes these Tauri commands and events for the renderer to consume (implemented in `src-tauri/src/ssh.rs`, `src-tauri/src/db.rs` and wired in `src-tauri/src/lib.rs`):

- SSH Commands:
  - `connect_ssh` — open and initialize an SSH session and PTY
  - `disconnect_ssh` — terminate a session and clean up resources
  - `send_ssh_input` — forward user input to the SSH channel

- Database Commands (Management):
  - `list_sessions`, `add_session`, `edit_session`, `delete_session` — manage SSH connections
  - `list_groups`, `add_group`, `edit_group`, `delete_group` — manage connection groups
  - `list_tags`, `add_tag`, `edit_tag`, `delete_tag` — manage session tags
  - `link_session_group`, `link_session_tag` — create associations

- Events (emitted by backend):
  - `ssh-output-<session_id>` — carries `OutputChunk` payloads with seq, output, and timestamp
  - `ssh-input-<session_id>` — listened for by the backend to receive input from UI (renderer emits this)
  - `ssh-resize-<session_id>` — resize payload for PTY adjustments

**Frontend rendering architecture**

The terminal frontend uses `xterm.js` with WebGL acceleration for GPU-accelerated rendering. To optimize performance:

- SSH output is received via Tauri events (push-based, not polling)
- Writes to xterm are batched and flushed using `requestAnimationFrame` for per-frame rendering
- Output deduplication is enforced via sequence numbers to prevent duplicate or out-of-order text

**Project structure (high level)**

- `src/` — frontend renderer (Vue 3 + TypeScript)
  - `src/components/` — reusable UI components
  - `src/features/` — feature-based modules (session, settings, tabs, etc.)
    - `src/features/session/api.ts` — Tauri RPC wrappers
  - `src/core/` — core utilities, constants, and i18n
- `src-tauri/` — Rust backend and Tauri config
  - `src-tauri/src/ssh.rs` — SSH manager and channel implementation
  - `src-tauri/src/db.rs` — SQLite database manager
  - `src-tauri/src/lib.rs` — Tauri initialization and command registration

**Contributing**

Contributions are welcome. Please follow these steps:

1. Fork the repository.
2. Create a feature branch.
3. Run tests and ensure linting/formatting conventions pass (`pnpm lint`, `pnpm format`).
4. Open a pull request with a clear description of changes.

**License**

This project is released under the MIT License. See the `LICENSE` file for full terms.

If you'd like, I can also:

- add a short developer quickstart script,
- generate a CONTRIBUTING.md,
- or add CI steps for building and testing. Let me know which you prefer.
