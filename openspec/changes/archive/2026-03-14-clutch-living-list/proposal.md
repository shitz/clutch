## Why

Existing Transmission remote GUIs block the UI thread during RPC calls, producing sluggish, unresponsive interfaces. Clutch is a from-scratch native GUI built on `iced` (Elm architecture) that guarantees the UI thread is never blocked by network I/O or state updates.

## What Changes

- New Rust application: `src/main.rs` and supporting modules
- Screen-based state model (`Screen::Connection` → `Screen::Main`)
- Async RPC client for Transmission JSON-RPC API (session-id lifecycle, 409 retry)
- Connection screen with host/port/credentials input and inline error feedback
- Torrent list screen with sticky header, scrollable rows, progress bars, and a 5-second polling subscription
- All I/O performed inside `Command::perform()` — `update()` never blocks

## Capabilities

### New Capabilities

- `connection-screen`: Connection input form with host, port, optional username/password, connect action, and inline error display on failure
- `torrent-list`: Scrollable torrent list with sticky column header (Name, Status, Progress), per-row progress bar, and background polling that keeps data fresh without touching the UI thread
- `rpc-client`: Transmission JSON-RPC client handling the `X-Transmission-Session-Id` 409 dance, Basic Auth, and `torrent-get` / `session-get` calls

### Modified Capabilities

## Impact

- New project: `Cargo.toml` with dependencies `iced` (tokio, canvas, image features), `tokio`, `reqwest` (json feature), `serde`, `serde_json`
- No existing code affected (greenfield)
