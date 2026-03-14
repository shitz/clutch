# Project Specification & Architecture: Clutch — A Transmission Remote GUI in Rust

## 1. Project Overview

A cross-platform (Windows, macOS, Linux) desktop application built in pure Rust. The application
serves as a remote GUI for a Transmission BitTorrent daemon, communicating exclusively via the
Transmission JSON-RPC API. It uses the `iced` 0.14 GUI framework and follows the Elm architecture
(Model / View / Update) using iced's free-function style.

**Current status: v0.1 "Living List" — shipped and working against a real Transmission daemon.**

---

## 2. Core Non-Functional Constraints

These constraints are fixed across all versions and must never be violated:

| Constraint               | Rule                                                                                                |
| ------------------------ | --------------------------------------------------------------------------------------------------- |
| **Non-blocking UI**      | `update()` must return in microseconds. All I/O lives inside `Task::perform()`.                     |
| **Single RPC in-flight** | A new poll is not started until the previous one resolves (`is_loading` guard).                     |
| **Screen-safe state**    | Torrent data is only accessible when `Screen::Main` is active — illegal states are unrepresentable. |
| **Cross-platform**       | No GTK, no web views. Pure Rust dependencies only.                                                  |

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
- Guarding against concurrent RPC calls with an `is_loading` boolean flag.
- Handling session-id rotation transparently (409 → `SessionIdRotated` → retry).
- Providing a toolbar with a **Disconnect** button that routes back to `Screen::Connection`.

Layout uses `FillPortion` weights for columns (Name: 5, Status: 2, Progress: 3). Long torrent names
use `text::Wrapping::WordOrGlyph` to prevent overflow into adjacent columns.

Polling state machine:

```
Idle ──[Tick]──▶ Loading (is_loading = true)
                     │
     ┌───────────────┴──────────────────────────┐
     ▼                                           ▼
TorrentsUpdated(Ok)                    TorrentsUpdated(Err)
     │                                           │
     ▼                                           ▼
Idle, list replaced                   Idle, error shown
     ▲
     │ (409 mid-flight)
SessionIdRotated ──▶ retry torrent-get with new session id
```

### 3.6 RPC Client (`rpc.rs`)

All functions are `async` and designed to be called exclusively from `Task::perform()`.

| Function                                         | Description                                                                                          |
| ------------------------------------------------ | ---------------------------------------------------------------------------------------------------- |
| `post_rpc(url, creds, session_id, method, args)` | Single HTTP POST with a 10 s timeout. Returns `Err(SessionRotated)` on 409, `Err(AuthError)` on 401. |
| `session_get(url, creds, session_id)`            | Connectivity probe. Handles one automatic session rotation. Returns `SessionInfo { session_id }`.    |
| `torrent_get(url, creds, session_id)`            | Fetches the full torrent list (`id`, `name`, `status`, `percentDone`).                               |

**Session-Id lifecycle:** Transmission uses `X-Transmission-Session-Id` as a lightweight CSRF
token. On startup (or after rotation) the daemon returns 409 with the new ID in a response header.
`session_get` retries once automatically; `torrent_get` surfaces `RpcError::SessionRotated(new_id)`
to the caller, which maps it to `Message::SessionIdRotated` for the main screen to handle.

**Timeout:** `reqwest::Client` is built with a 10 s timeout so a non-responding daemon surfaces an
error quickly rather than hanging indefinitely.

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

    // Main screen
    Tick,
    TorrentsUpdated(Result<Vec<TorrentData>, String>),
    SessionIdRotated(String),

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
| `wiremock` _(dev)_     | 0.6     | In-process HTTP mock server for RPC integration tests |

### 3.11 Test Coverage

14 tests, all passing. Two layers:

- **Unit tests** (`screens/connection.rs`, `screens/main_screen.rs`): exercise `update()` logic
  entirely in-memory, no async I/O.
- **Integration tests** (`rpc.rs`): use `wiremock` to stand up a real in-process HTTP server and
  verify the full RPC round-trip including 409 rotation, 401 auth errors, parse errors, and happy
  paths.

---

## 4. Roadmap

Each milestone is a vertical slice: the app remains shippable after every version.

### v0.2 — Torrent Control

Wire up the toolbar buttons that are currently rendered but disabled.

- `torrent-start` / `torrent-stop` RPC calls mapped to Pause / Resume buttons.
- `torrent-remove` (with `delete-local-data` flag) mapped to Delete button.
- Torrent selection: clicking a row highlights it and enables the relevant toolbar buttons.
  Selection state lives in `MainScreen` as `selected_id: Option<i64>`.
- Optimistic UI: button fires immediately, list refreshes once the action RPC completes.
- New messages: `PauseClicked`, `ResumeClicked`, `DeleteClicked(bool)`, `ActionCompleted(Result<(), String>)`.

### v0.3 — Add Torrents

Allow users to add new torrents to the daemon.

- **Magnet links:** A text input in the toolbar accepts a magnet URI; submitting fires `torrent-add`
  with the `filename` field.
- **`.torrent` files:** A file-picker button (via `rfd` — pure-Rust native dialog) reads the file,
  Base64-encodes it, and passes it to `torrent-add` via the `metainfo` field.
- New crates: `rfd` (file dialog), `base64` (encoding).
- New message: `AddMagnetSubmitted(String)`, `TorrentFileSelected(Option<PathBuf>)`,
  `AddCompleted(Result<(), String>)`.

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
