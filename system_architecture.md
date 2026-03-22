# Project Specification & Architecture: Clutch — A Transmission Remote GUI in Rust

## 1. Project Overview

A cross-platform (Windows, macOS, Linux) desktop application built in pure Rust. The application
serves as a remote GUI for a Transmission BitTorrent daemon, communicating exclusively via the
Transmission JSON-RPC API. It uses the `iced` 0.14 GUI framework and follows the Elm architecture
(Model / View / Update) using iced's free-function style.

**Current status: v0.3 "Add Torrents" — shipped and working against a real Transmission daemon.**

---

## 2. Core Non-Functional Constraints

These constraints are fixed across all versions and must never be violated:

| Constraint             | Rule                                                                                                                                              |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Non-blocking UI**    | `update()` must return in microseconds. All I/O lives in the RPC worker subscription or `Task::perform()`.                                        |
| **Serialized RPC**     | All RPC calls flow through a single `tokio::sync::mpsc` channel processed one at a time — at most one HTTP connection to the daemon is in-flight. |
| **Ordered submission** | Work items are enqueued from `update()` (synchronous), so submission order is deterministic regardless of the tokio scheduler.                    |
| **Screen-safe state**  | Torrent data is only accessible when `Screen::Main` is active — illegal states are unrepresentable.                                               |
| **Cross-platform**     | No GTK, no web views. Pure Rust dependencies only.                                                                                                |

---

## 3. Implemented Architecture (v0.1)

### 3.1 Module Layout

```
src/
├── main.rs               Entry point. Initialises tracing, launches iced.
├── app.rs                AppState, Screen router, Message enum, top-level update/view/subscription.
├── rpc.rs                Async Transmission JSON-RPC client.
└── screens/
    ├── mod.rs
    ├── connection.rs     Connection form screen.
    └── main_screen.rs    Torrent list screen.
```

### 3.2 Elm Loop (iced 0.14 free-function style)

iced 0.14 uses free functions instead of the `Application` trait. The entry point wires them together:

```rust
iced::application(AppState::new, update, view)
    .title("Clutch")
    .subscription(subscription)
    .run()
```

| Elm role          | Implementation                                                   |
| ----------------- | ---------------------------------------------------------------- |
| **Model**         | `AppState { screen: Screen }`                                    |
| **View**          | `fn view(state: &AppState) -> Element<'_, Message>`              |
| **Update**        | `fn update(state: &mut AppState, msg: Message) -> Task<Message>` |
| **Effects**       | `Task<Message>` (replaces iced 0.13's `Command`)                 |
| **Subscriptions** | `fn subscription(state: &AppState) -> Subscription<Message>`     |

### 3.3 Screen Router

Routing is done via a discriminated enum. Only one screen exists at a time and its type guarantees
which state fields are accessible:

```rust
pub enum Screen {
    Connection(ConnectionScreen),   // Shown at startup and after Disconnect
    Main(MainScreen),               // Shown after a successful session-get probe
}

pub struct AppState {
    pub screen: Screen,
}
```

`update()` dispatches `Message::Disconnect` before the screen match (it is screen-agnostic), then
delegates all other messages to the active screen's own `update()` method.

### 3.4 Connection Screen (`screens/connection.rs`)

Responsible for:

- Presenting a host / port / username / password form (password field masked with `.secure(true)`).
- Firing a `session-get` probe via `Task::perform()` when "Connect" is clicked.
- Transitioning to `Screen::Main` on success, or showing an inline error on failure.
- Returning `(Task<Message>, Option<Screen>)` from `update()` — the `Option<Screen>` signals a
  screen transition to `app::update()` without taking a reference to the outer state.

State transitions:

```
Idle ──[ConnectClicked]──▶ Connecting
                                │
            ┌───────────────────┴──────────────────┐
            ▼                                       ▼
   SessionProbeResult(Ok)             SessionProbeResult(Err)
            │                                       │
            ▼                                       ▼
    → Screen::Main                       Idle (inline error)
```

### 3.5 Main Screen (`screens/main_screen.rs`)

Responsible for:

- Displaying a sticky column header + scrollable torrent rows (Name / Status / Progress).
- Polling the daemon every 5 seconds via an `iced::time::every` subscription.
- Serializing all RPC calls through a single MPSC worker subscription (see §3.6 RPC Worker).
- Using an `is_loading` boolean as an **efficiency guard** to skip redundant tick enqueues (not for correctness — the worker provides the serialization guarantee).
- Handling session-id rotation transparently (409 → `SessionIdRotated` → persists new ID; the retry happens inside the worker).
- Providing a toolbar with a **Disconnect** button that routes back to `Screen::Connection`.
- **Single-torrent selection:** clicking a row highlights it and enables action buttons; clicking again deselects.
- **Toolbar actions:** Pause (`torrent-stop`), Resume (`torrent-start`), Delete (`torrent-remove`) operate on the selected torrent.
- **Delete confirmation row:** clicking Delete replaces the toolbar with an inline confirmation row showing the torrent name, a "Delete local data" checkbox, and Confirm/Cancel buttons. The RPC is only issued after confirmation.
- **Immediate refresh:** after any successful action a `torrent-get` poll fires immediately without waiting for the next 5-second tick.
- **Add Torrent button:** opens a native file picker (via `rfd`); the selected `.torrent` file is read, Base64-encoded, and parsed locally with `lava_torrent` to extract the file list — all in a single `Task::perform`. On success, the add-torrent dialog opens.
- **Add Link button:** opens the add-torrent dialog in magnet-link mode.
- **Add-torrent dialog:** a modal overlay rendered via `iced::widget::stack`. Contains a destination folder text input, a scrollable file list preview (name + size per file), and Add/Cancel buttons. Both flows (file and magnet) share this dialog. After the user confirms, `torrent-add` is called and the list is refreshed immediately on success.

Layout uses `FillPortion` weights for columns (Name: 5, Status: 2, Progress: 3). Long torrent names
use `text::Wrapping::WordOrGlyph` to prevent overflow into adjacent columns.

Polling state machine:

```
Idle ──[Tick]──▶ is_loading=true, enqueues TorrentGet to worker
                     │
              worker processes it
                     │
     ┌───────────────┴──────────────────────────┐
     ▼                                           ▼
TorrentsUpdated(Ok)                    TorrentsUpdated(Err)
     │                                           │
     ▼                                           ▼
Idle, list replaced                   Idle, error shown

(409 inside worker)
execute_work retries once with new session id, then emits SessionIdRotated
to persist the new id in MainScreen.session_id
```

### 3.6 RPC Client (`rpc.rs`)

All public functions are `async` and invoked either from `Task::perform()` (connection probe) or
inside the serialized worker (all main-screen calls).

| Function                                                        | Description                                                                                                                                                                                                                                                                                                                                                       |
| --------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `post_rpc(url, creds, session_id, method, args, timeout)`       | Single HTTP POST. Returns `Err(SessionRotated)` on 409, `Err(AuthError)` on 401. Timeout is caller-supplied (10 s standard, 60 s for `torrent-add`).                                                                                                                                                                                                              |
| `session_get(url, creds, session_id)`                           | Connectivity probe. Handles one automatic session rotation. Returns `SessionInfo { session_id }`. Called directly from `Task::perform()` on the connection screen.                                                                                                                                                                                                |
| `torrent_get(url, creds, session_id)`                           | Fetches the full torrent list (`id`, `name`, `status`, `percentDone`).                                                                                                                                                                                                                                                                                            |
| `torrent_start(url, creds, session_id, id)`                     | Resumes the torrent with the given ID (`torrent-start`).                                                                                                                                                                                                                                                                                                          |
| `torrent_stop(url, creds, session_id, id)`                      | Pauses the torrent with the given ID (`torrent-stop`).                                                                                                                                                                                                                                                                                                            |
| `torrent_remove(url, creds, session_id, id, delete_local_data)` | Removes the torrent; when `delete_local_data=true` also deletes downloaded files.                                                                                                                                                                                                                                                                                 |
| `torrent_add(url, creds, session_id, payload, download_dir)`    | Adds a torrent via `AddPayload::Magnet(uri)` or `AddPayload::Metainfo(base64)`. Optional `download_dir` sets the destination; empty/`None` uses the daemon default. Both `"success"` and `"torrent-duplicate"` results are treated as `Ok(())`. Uses a 60 s timeout because Transmission does synchronous disk I/O (preallocation, hash-check) before responding. |
| `execute_work(work: RpcWork)`                                   | Executes one `RpcWork` item and returns `(Option<String>, RpcResult)`. The `Option<String>` carries a new session ID when 409 rotation occurred (it retried automatically). Called exclusively by the RPC worker loop.                                                                                                                                            |

**Session-Id lifecycle:** Transmission uses `X-Transmission-Session-Id` as a lightweight CSRF
token. On startup (or after rotation) the daemon returns 409 with the new ID in a response header.
`session_get` retries once automatically. The main-screen worker calls `execute_work`, which also
retries once on rotation and emits `Message::SessionIdRotated` to persist the new ID.

**Timeouts:** `post_rpc` accepts a caller-supplied `Duration`. Standard calls use 10 s;
`torrent_add` uses 60 s — Transmission 3.x and 4.x perform synchronous disk work before replying,
and dropping the connection mid-response permanently wedges the single-threaded RPC socket handler.

### 3.6a Serialized RPC Worker

All calls to the daemon from the main screen are serialized through an MPSC channel worker
implemented as an `iced::Subscription`.

**Why:** Transmission's RPC server (both 3.x and 4.x) does not handle concurrent connections
robustly. A second HTTP connection arriving while the first is in-flight (e.g. a poll tick firing
during a slow `torrent-add`) can permanently wedge the socket handler. The worker guarantees at
most one in-flight HTTP connection at any time, with deterministic submission order.

**Architecture:**

```text
  update() [synchronous]
      │
      │  tx.try_send(RpcWork::TorrentGet { … })
      │  tx.try_send(RpcWork::TorrentAdd { … })
      ▼
  tokio::sync::mpsc::channel(32) — FIFO queue
      │
      ▼
  rpc_worker_stream() [iced::Subscription]
      │  loop:
      │    work = rx.recv().await          // blocks until work arrives
      │    (new_sid, result) = execute_work(work).await
      │    if new_sid: emit SessionIdRotated
      │    emit TorrentsUpdated | ActionCompleted | AddCompleted
      ▼
  Message → update()
```

**Key types:**

```rust
// Work item: one variant per RPC call, carries all required params.
pub enum RpcWork {
    TorrentGet   { url, credentials, session_id },
    TorrentStart { url, credentials, session_id, id },
    TorrentStop  { url, credentials, session_id, id },
    TorrentRemove{ url, credentials, session_id, id, delete_local_data },
    TorrentAdd   { url, credentials, session_id, payload, download_dir },
}

// Typed outcome returned by execute_work.
pub enum RpcResult {
    TorrentsLoaded(Result<Vec<TorrentData>, RpcError>),
    ActionDone(Result<(), RpcError>),
    TorrentAdded(Result<(), RpcError>),
}
```

**`MainScreen` integration:**

- `MainScreen.sender: Option<tokio::sync::mpsc::Sender<RpcWork>>` — populated when `Message::RpcWorkerReady` arrives from the subscription.
- `MainScreen::enqueue(work)` — calls `tx.try_send(work)`; logs an error if the 32-item buffer is full (unreachable under normal usage).
- `MainScreen::enqueue_torrent_get()` — convenience wrapper used by `Tick` and post-action refresh.
- `is_loading` remains as an **efficiency guard only**: when `true`, incoming `Tick` messages are dropped so the queue doesn't accumulate redundant poll requests. It no longer provides the serialization invariant.

### 3.7 Data Models

```rust
// Credentials passed to every RPC call
pub struct TransmissionCredentials {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

// One row in the torrent list
pub struct TorrentData {
    pub id: i64,
    pub name: String,
    pub status: i32,       // 0=Stopped 1=QueueCheck 2=Checking 3=QueueDL 4=DL 5=QueueSeed 6=Seeding
    pub percent_done: f64, // [0.0, 1.0]
}

// Payload discriminator for torrent-add
pub enum AddPayload {
    Magnet(String),   // sent as "filename"
    Metainfo(String), // Base64-encoded .torrent bytes, sent as "metainfo"
}

// File entry extracted from a .torrent file for the add dialog preview
pub struct TorrentFileInfo {
    pub path: String,
    pub size_bytes: u64,
}

// Result of reading + parsing a .torrent file; payload of TorrentFileRead
pub struct FileReadResult {
    pub metainfo_b64: String,
    pub files: Vec<TorrentFileInfo>,
}

// Add-torrent modal dialog state (lives in MainScreen)
pub enum AddDialogState {
    Hidden,
    AddLink { magnet: String, destination: String, error: Option<String> },
    AddFile { metainfo_b64: String, files: Vec<TorrentFileInfo>, destination: String, error: Option<String> },
}
```

### 3.8 Message Enum (implemented)

```rust
pub enum Message {
    // Connection screen
    HostChanged(String),
    PortChanged(String),
    UsernameChanged(String),
    PasswordChanged(String),
    ConnectClicked,
    SessionProbeResult(Result<SessionInfo, String>),

    // Main screen — polling
    Tick,
    TorrentsUpdated(Result<Vec<TorrentData>, String>),
    SessionIdRotated(String),

    // Main screen — torrent actions (v0.2)
    TorrentSelected(i64),
    PauseClicked,
    ResumeClicked,
    DeleteClicked,
    DeleteLocalDataToggled(bool),
    DeleteConfirmed,
    DeleteCancelled,
    ActionCompleted(Result<(), String>),

    // Main screen — add torrent (v0.3)
    AddTorrentClicked,
    TorrentFileRead(Result<FileReadResult, String>),
    AddLinkClicked,
    AddDialogMagnetChanged(String),
    AddDialogDestinationChanged(String),
    AddConfirmed,
    AddCancelled,
    AddCompleted(Result<(), String>),

    // RPC worker subscription (v0.3 hardening)
    /// Emitted once by the worker subscription on startup.
    /// The sender is stored in MainScreen and used for all subsequent RPC calls.
    RpcWorkerReady(tokio::sync::mpsc::Sender<rpc::RpcWork>),

    // Screen-agnostic
    Disconnect,
}
```

### 3.9 Observability

Structured logging via `tracing` 0.1 + `tracing-subscriber` 0.3. Initialised in `main()` via
`tracing_subscriber::fmt::init()`. Run with `RUST_LOG=clutch=debug` for full RPC traces.

| Level    | Examples                                                                                                            |
| -------- | ------------------------------------------------------------------------------------------------------------------- |
| `error!` | Transport failures, auth errors (401), JSON parse errors                                                            |
| `info!`  | Connect attempted/succeeded, disconnect, torrent list refreshed                                                     |
| `debug!` | Every outgoing request (url, method, session_id), every response status, session rotation details, tick guard skips |

### 3.10 Crate Stack (current)

| Crate                  | Version | Purpose                                               |
| ---------------------- | ------- | ----------------------------------------------------- |
| `iced`                 | 0.14    | GUI framework (features: `tokio`, `canvas`, `image`)  |
| `tokio`                | 1       | Async runtime (via iced's built-in integration)       |
| `reqwest`              | 0.12    | HTTP client for RPC calls (feature: `json`)           |
| `serde` + `serde_json` | 1       | RPC payload serialization / deserialization           |
| `tracing`              | 0.1     | Structured logging instrumentation                    |
| `tracing-subscriber`   | 0.3     | Log formatting and `RUST_LOG` env-filter              |
| `rfd`                  | 0.15    | Native async file picker dialog                       |
| `base64`               | 0.22    | Base64 encoding of `.torrent` file bytes              |
| `lava_torrent`         | 0.11    | Local `.torrent` file parsing (file list + sizes)     |
| `wiremock` _(dev)_     | 0.6     | In-process HTTP mock server for RPC integration tests |

### 3.11 Test Coverage

48 tests, all passing. Two layers:

- **Unit tests** (`screens/connection.rs`, `screens/main_screen.rs`): exercise `update()` logic
  entirely in-memory, no async I/O.
- **Integration tests** (`rpc.rs`): use `wiremock` to stand up a real in-process HTTP server and
  verify the full RPC round-trip including 409 rotation, 401 auth errors, parse errors, and happy
  paths for `session_get`, `torrent_get`, `torrent_start`, `torrent_stop`, `torrent_remove`, and
  `torrent_add` (magnet, metainfo, duplicate, empty `download_dir`, rotation, auth error).

---

## 4. Roadmap

Each milestone is a vertical slice: the app remains shippable after every version.

### ~~v0.2 — Torrent Control~~ ✓ Shipped

Wire up the toolbar buttons that are currently rendered but disabled.

- `torrent-start` / `torrent-stop` RPC calls mapped to Pause / Resume buttons.
- `torrent-remove` (with `delete-local-data` flag) mapped to Delete button.
- Torrent selection: clicking a row highlights it and enables the relevant toolbar buttons.
  Selection state lives in `MainScreen` as `selected_id: Option<i64>`.
- Delete confirmation row: clicking Delete shows an inline row with a "Delete local data"
  checkbox and Confirm/Cancel buttons (`confirming_delete: Option<(i64, bool)>`).
- Optimistic UI: button fires immediately, list refreshes once the action RPC completes.
- New messages: `TorrentSelected(i64)`, `PauseClicked`, `ResumeClicked`, `DeleteClicked`,
  `DeleteLocalDataToggled(bool)`, `DeleteConfirmed`, `DeleteCancelled`, `ActionCompleted(Result<(), String>)`.

### ~~v0.3 — Add Torrents~~ ✓ Shipped

Allow users to add new torrents to the daemon.

- **Add Torrent button** in the toolbar opens a native file picker (via `rfd`). The selected
  `.torrent` file is read, Base64-encoded, and parsed locally with `lava_torrent` (file list
  extraction) — all in a single `Task::perform`.
- **Add Link button** in the toolbar opens the add dialog in magnet-link mode.
- **Unified add-torrent modal dialog** rendered via `iced::widget::stack`: destination folder
  text input, scrollable file list preview (name + human-readable size), Add/Cancel buttons,
  inline error label.
- **Magnet links:** no file preview (metadata unavailable before peer connection); a static
  note is shown in the file list area.
- **`torrent_add` RPC function** with `AddPayload` enum (`Magnet` / `Metainfo`) and optional
  `download_dir`; treats `"torrent-duplicate"` as success.
- Immediate list refresh after a successful add.
- New messages: `AddTorrentClicked`, `TorrentFileRead(Result<FileReadResult, String>)`,
  `AddLinkClicked`, `AddDialogMagnetChanged(String)`, `AddDialogDestinationChanged(String)`,
  `AddConfirmed`, `AddCancelled`, `AddCompleted(Result<(), String>)`.

### v0.4 — Detail Inspector

A detail panel that slides in below the list when a torrent is selected.

- Split the main area: list occupies ~60%, inspector ~40% (proportions configurable later).
- Inspector tabs (row of toggle buttons): **General**, **Files**, **Trackers**, **Peers**.
- **General:** name, total size, downloaded, uploaded, ratio, ETA, download/upload speeds.
- **Files:** a scrollable list of files with per-file progress bars.
- **Trackers:** URL, seeder count, leecher count, last announce time.
- **Peers:** IP, client string, upload/download rate contribution.
- New RPC field requests: `totalSize`, `downloadedEver`, `uploadedEver`, `uploadRatio`, `eta`,
  `rateDownload`, `rateUpload`, `files`, `fileStats`, `trackerStats`, `peers`.
- Polling interval drops to **2 s** now that the panel shows live transfer speeds.

### v0.5 — Extended Torrent List Columns

Expand the list view with additional columns visible in v0.4's data.

- New columns: Size, Downloaded, ↓ Speed, ↑ Speed, ETA, Ratio.
- Column visibility toggle (a settings popover or context menu).
- Column sort: clicking a header sorts ascending/descending.

### v1.0 — Polish & Settings

- **Settings persistence:** Save connection profiles and UI preferences to a config file (via
  `directories` + `toml` or `serde_json`).
- **Multiple connection profiles:** A dropdown on the connection screen to select saved daemons.
- **Polling interval:** Configurable (1–30 s) with a sensible default of 2 s.
- **Theme:** Light / dark mode toggle via iced's built-in theme system.
- **Error recovery:** Automatic reconnect with exponential back-off when the daemon becomes
  unreachable while the main screen is active.
- **CI:** GitHub Actions workflow running `cargo test` + `cargo clippy -- -D warnings` on
  push to main for Linux, macOS, and Windows.
