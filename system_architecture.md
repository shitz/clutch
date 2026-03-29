# Project Specification & Architecture: Clutch — A Transmission Remote GUI in Rust

## 1. Project Overview

A cross-platform (Windows, macOS, Linux) desktop application built in pure Rust. The application
serves as a remote GUI for a Transmission BitTorrent daemon, communicating exclusively via the
Transmission JSON-RPC API. It uses the `iced` 0.14 GUI framework and follows the Elm architecture
(Model / View / Update) using iced's free-function style.

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

## 3. Architecture

### 3.1 Module Layout

```text
src/
├── main.rs               Entry point. Initialises tracing, registers fonts, sets window constraints, launches iced.
├── app.rs                AppState, Screen router, ThemeMode, Message enum, top-level update/view/subscription.
├── format.rs             Shared torrent data formatting helpers (size, speed, ETA, duration).
├── profile.rs            ProfileStore, ConnectionProfile, GeneralSettings, ThemeConfig, keyring helpers.
├── theme.rs              Material Design 3 theme, Material Icons font constants, shared widget styles.
├── rpc.rs                Async Transmission JSON-RPC client.
└── screens/
    ├── mod.rs
    ├── connection.rs     Connection form screen (new profile + optional Cancel button).
    ├── main_screen.rs    Parent delegating screen: composes list + inspector + profile switcher.
    ├── torrent_list.rs   Torrent list sub-component (toolbar, 9-column header, rows, add dialog, RPC worker, profile dropdown).
    ├── inspector.rs      Detail inspector sub-component (tabbed panel via iced_aw).
    └── settings.rs       Full-screen settings editor (General tab + Connections master-detail).
```

### 3.2 Elm Loop (iced 0.14 free-function style)

iced 0.14 uses free functions instead of the `Application` trait. The entry point wires them together:

```rust
iced::application(AppState::new, update, view)
    .title("Clutch")
    .subscription(subscription)
    .theme(AppState::current_theme)
    .font(theme::MATERIAL_ICONS_BYTES)
    .font(iced_aw::ICED_AW_FONT_BYTES)
    .window(window::Settings { min_size: Some(Size { width: 900.0, height: 500.0 }), ..Default::default() })
    .run()
```

| Elm role          | Implementation                                                                                                       |
| ----------------- | -------------------------------------------------------------------------------------------------------------------- |
| **Model**         | `AppState { screen, theme, profiles: ProfileStore, active_profile: Option<Uuid>, stashed_main: Option<MainScreen> }` |
| **View**          | `fn view(state: &AppState) -> Element<'_, Message>`                                                                  |
| **Update**        | `fn update(state: &mut AppState, msg: Message) -> Task<Message>`                                                     |
| **Effects**       | `Task<Message>` (replaces iced 0.13's `Command`)                                                                     |
| **Subscriptions** | `fn subscription(state: &AppState) -> Subscription<Message>`                                                         |

### 3.3 Screen Router

Routing is done via a discriminated enum. Only one screen exists at a time:

```rust
pub enum Screen {
    Connection(ConnectionScreen),   // Shown at startup, after Disconnect, and for Add New Connection
    Main(MainScreen),               // Shown after a successful session-get probe
    Settings(SettingsScreen),       // Full-screen settings / profiles editor
}

/// Resolved display theme — always Light or Dark (never System).
pub enum ThemeMode { Dark, Light }

pub struct AppState {
    pub screen: Screen,
    pub theme: ThemeMode,                  // drives current_theme() → Theme::custom()
    pub profiles: ProfileStore,            // loaded from disk on startup
    pub active_profile: Option<Uuid>,      // UUID of the connected profile
    pub stashed_main: Option<MainScreen>,  // saved while Settings is open; restored on Close
}
```

`AppState::new()` loads the profile store synchronously via `ProfileStore::load_sync()`, then
emits `Task::done(Message::ProfilesLoaded(initial))` to trigger initialization on the first
event-loop tick without a second disk read.

**Startup flow:**

1. `ProfilesLoaded(store)` arrives → resolve theme, store `profiles`, check `last_connected`.
2. If `last_connected` is set and the profile exists → fire `session-get` probe → `AutoConnectResult`.
3. `AutoConnectResult(Ok)` → transition to `Screen::Main`.
4. `AutoConnectResult(Err)` → show `Screen::Connection` pre-filled with last-used profile data.

`update()` intercepts several global messages before the screen match:

- `ProfilesLoaded` / `AutoConnectResult` — startup flow (see above).
- `SwitchProfile(id)` / `ProfileSwitchResult(id, result)` — profile switching.
- `Main(Disconnect)` / `Main(DisconnectClicked)` — transition to `Screen::Connection`.
- `Main(List(ThemeToggled))` — toggle `AppState::theme` Dark↔Light.
- `Main(OpenSettingsClicked)` → stash `Screen::Main` into `stashed_main`, push `Screen::Settings(SettingsScreen::new(&profiles, active_id, General))`.
- `Main(ManageConnectionsClicked)` → stash + push `Screen::Settings` with `Connections` tab.
- `Main(AddNewConnectionClicked)` → `Screen::Connection` with `return_to_main: true`.
- `Main(SwitchProfileClicked(id))` → delegates to `SwitchProfile(id)` handler.
- `CancelConnect` (on connection screen with `return_to_main: true`) → restore `Screen::Main`.
- `Settings(msg)` → delegate to `SettingsScreen::update`; handle `SettingsResult` variants.

**Settings integration:** When `SettingsScreen` signals a `SettingsResult`:

- `GeneralSettingsSaved` → update `profiles` and resolve new theme; also propagate new `refresh_interval` to `stashed_main` if present.
- `ActiveProfileSaved` → update `profiles`, then trigger `SwitchProfile(id)` to reconnect.
- `StoreUpdated` → update `profiles` (non-active profile changed).
- `Closed` → update `profiles`, restore `stashed_main` as `Screen::Main` (no reconnect/refetch); fallback to building new `MainScreen` or `Screen::Connection`.

### 3.4 Connection Screen (`screens/connection.rs`)

A **two-tab launchpad** shown at startup before any connection is established.

**Saved Profiles tab** (default when profiles exist):

- Shows a clickable card row for each saved profile (`name — host:port`).
- Clicking a card fires a `session-get` probe immediately using `profile.credentials()` (reads
  keyring at click time).
- A **⚙ Manage / Add Profile…** button navigates to `Screen::Settings(Connections tab)`.

**Quick Connect tab** (default when no profiles exist):

- Ephemeral credentials form (Host, Port, Username, Password).
- On success, `ConnectSuccess.profile_id` is `None` — nothing is persisted.
- Defaults: Host = "localhost", Port = 9091.

Constructors: `ConnectionScreen::new_launchpad(profiles)` / `ConnectionScreen::default()` (empty, Quick Connect tab).

`update()` signature: `fn update(&mut self, message: Message) -> (Task<Message>, Option<ConnectSuccess>)`.
Returns `Some(ConnectSuccess)` on a successful probe; caller transitions to `Screen::Main`.

State transitions:

```
Idle ──[ConnectProfile(id) | ConnectClicked]──▶ Connecting
                                                      │
                   ┌──────────────────────────────────┴──────────────────────────┐
                   ▼                                                              ▼
      SessionProbeResult(Ok)                               SessionProbeResult(Err)
                   │                                                              │
                   ▼                                                              ▼
    (return ConnectSuccess)                                         Idle (inline error)
```

### 3.5 Main Screen (`screens/main_screen.rs`, `screens/torrent_list.rs`, `screens/inspector.rs`)

`MainScreen` is a **parent Elm component** that owns two child components and delegates to them:

- **`TorrentListScreen`** (`torrent_list.rs`) — toolbar, sticky header, scrollable torrent rows,
  add-torrent modal, and the serialized RPC worker. All state and messaging previously on the
  monolithic main screen now lives here.
- **`InspectorScreen`** (`inspector.rs`) — tabbed detail panel shown below the list
  when a torrent is selected.

`MainScreen` also holds:

- `connect_label: String` — `"host:port"` shown in the loading splash.
- `profile_name: Option<String>` — profile display name; `None` for ephemeral quick-connect.
- `refresh_interval: u8` — daemon poll interval in seconds (from `GeneralSettings`); drives the tick subscription.

**Loading splash:** Until the first `TorrentsUpdated` response arrives (`initial_load_done = false`),
`MainScreen::view()` renders a full-screen centred label:
`"Connecting to host:port (Profile Name)…"` (profile name omitted for quick-connect).

#### Layout

When no torrent is selected the list fills the full height. When a torrent is selected the content
area splits vertically using `Length::FillPortion`:

```
┌──────────────────────────────────────────────────────┐
│   TorrentListScreen            (top 3/4)             │
├──────────────────────────────────────────────────────┤
│   InspectorScreen              (bottom 1/4)          │
└──────────────────────────────────────────────────────┘
```

`main_screen::view()` checks `list.selected_torrent()` — if `None`, returns only the list element;
if `Some`, wraps both in a `column!` with `FillPortion(3)` / `FillPortion(1)` containers, each
`width(Length::Fill)`.

#### Message Routing

`MainScreen` wraps child messages and intercepts two cross-cutting concerns before delegating:

1. `List(TorrentListMessage::Disconnect)` → intercepted, escalated via
   `Task::done(Message::Disconnect)` to `app::update`.
2. `List(TorrentListMessage::TorrentSelected(id))` → delegate to `torrent_list::update` first,
   then reset `inspector.active_tab` to `General` if a _new_ torrent was selected.

All other `List(_)` messages are delegated to `torrent_list::update`; all `Inspector(_)` messages
are delegated to `inspector::update`.

#### TorrentListScreen responsibilities

- **9-column sticky header**: Name (fill), Status, Size, Downloaded, ↓ Speed, ↑ Speed, ETA, Ratio,
  Progress. Column headers are clickable buttons that cycle sort Asc → Desc → None.
- **Column sort**: `sort_column: Option<SortColumn>` + `sort_dir: SortDir` in state; `sort_torrents()`
  is a pure function that returns a sorted `Vec<&TorrentData>` used by `view()`.
- **Progress bar**: color-coded by status — green (downloading), blue (seeding), gray (all others).
- **Icon toolbar** (Group 1: add, Group 2: torrent actions, Group 3: right-aligned):
  - Group 3 contains: settings gear icon (→ `OpenSettingsClicked`), and a
    **profile switcher button** showing the active profile name.
  - Clicking the profile switcher toggles `profile_dropdown_open`. The dropdown lists all saved
    profiles (active profile prefixed with ✓), then a divider, "Add New Connection…",
    "Manage Connections…", another divider, and "Disconnect".
  - `profile_dropdown_open: bool` field added to `TorrentListScreen`.
- **Selected row** is rendered inside an `elevated_surface` container (rounded corners, drop shadow).
- **`initial_load_done: bool`** — set to `true` on the first `TorrentsUpdated` response (success or
  error); used by `MainScreen` to decide when to hide the loading splash.
- Polling the daemon at the configured `refresh_interval` (default **1 second**) via `iced::time::every`.
- Serializing all RPC calls through the MPSC worker subscription (see §3.6a).
- `is_loading` efficiency guard, session-id rotation, delete confirmation row, and add-torrent / add-link flows.
- `selected_torrent() -> Option<&TorrentData>` — exposes the currently selected torrent to parent.
- `Message::Disconnect`, `ProfileSelected(Uuid)`, `AddNewConnectionClicked`, `ManageConnectionsClicked`,
  `OpenSettingsClicked`, `DisconnectClicked`, `ThemeToggled` — intercepted by parent / app; never
  processed by `torrent_list::update`.

`view()` signature: `pub fn view<'a>(state: &'a TorrentListScreen, theme_mode, active_profile_name: &'a str, profiles: &'a [ConnectionProfile], switching_profile: bool, switch_error: Option<&'a str>) -> Element<'a, Message>`.

#### InspectorScreen responsibilities

- Renders the detail panel for the torrent passed in from the parent via `inspector::view(state, torrent)`.
- Maintains `active_tab: ActiveTab` (default `General`).
- `update()` handles `Message::TabSelected(tab)` only.
- **Tab bar** implemented with `iced_aw::Tabs` widget (requires `iced_aw` 0.13 `tabs` feature and
  `iced_aw::ICED_AW_FONT_BYTES` registered in `main.rs`).
- Four tabs — **General**, **Files**, **Trackers**, **Peers** — each rendered by a dedicated private function.
- The panel is wrapped in an `inspector_surface` container (rounded top corners, subtle shadow).
- Formatting is delegated to `crate::format`: `format_size`, `format_speed`, `format_eta`, `format_ago`.

#### `format.rs` — Formatting helpers

Shared formatting utilities extracted into a top-level module so both the inspector and torrent list can use them.

| Function       | Input       | Example output | Sentinel behaviour         |
| -------------- | ----------- | -------------- | -------------------------- |
| `format_size`  | `i64` bytes | `"2.4 GB"`     | `≤0 → "0 B"`               |
| `format_speed` | `i64` Bps   | `"1.2 MB/s"`   | `≤0 → "—"` (idle / paused) |
| `format_eta`   | `i64` secs  | `"3h 21m"`     | `-1 → "—"`                 |
| `format_ago`   | `i64` epoch | `"5m ago"`     | `≤0 → "—"`                 |

#### `theme.rs` — Material Design 3 theme

All styling in one file, used by every screen.

- **`MATERIAL_ICONS_BYTES`**: raw bytes of the bundled `fonts/MaterialIcons-Regular.ttf`.
- **`MATERIAL_ICONS`**: `Font::with_name("Material Icons")`.
- **Icon constants**: `ICON_PAUSE`, `ICON_PLAY`, `ICON_DELETE`, `ICON_ADD`, `ICON_LINK`,
  `ICON_SETTINGS`, `ICON_TRASH`, `ICON_CLOSE`, `ICON_SAVE`, `ICON_UNDO`.
- **`icon(codepoint)`**: returns a 24 px `Text` widget rendered in the Material Icons font.
- **`material_dark_theme()`** / **`material_light_theme()`**: return `Theme::custom()` with
  Material Design 3 palettes (#1C1B1F dark surface, #FFFBFE light background).
- **`elevated_surface(&Theme)`**: card-like container style (12px radius, drop shadow).
- **`inspector_surface(&Theme)`**: inspector panel style (rounded top corners, subtle upward shadow).
- **`progress_bar_style(status: i32)`**: returns a closure for `progress_bar::Style` colourised by
  Transmission status code (4=green, 6=blue, else gray).
- **`tab_active`** / **`tab_inactive`** / **`tab_underline`**: shared button/container styles for
  the settings tab bar.

### 3.5a Profile Store (`profile.rs`)

Persistent connection profiles and application-wide preferences.

```rust
pub enum ThemeConfig { Light, Dark, System }  // user-visible preference; System = detect at runtime
pub struct GeneralSettings {
    theme: ThemeConfig,
    refresh_interval: u8,   // daemon poll interval, seconds 1–30. Default: 1
}

pub struct ConnectionProfile {
    pub id: Uuid,             // stable UUID used as keyring account key
    pub name: String,         // display name e.g. "Home NAS"
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    // password NOT stored here — lives in OS keyring
}

pub struct ProfileStore {
    pub last_connected: Option<Uuid>,
    pub general: GeneralSettings,
    pub profiles: Vec<ConnectionProfile>,
}
```

- **Config location**: `dirs::config_dir()/clutch/config.toml` (e.g. `~/Library/Application Support/clutch/config.toml` on macOS).
- **Keyring scheme**: service `"clutch"`, account = `profile.id.to_string()`.
- **`ProfileStore::load() -> Self`** (async) — reads TOML; falls back to `Default` on missing or corrupt file.
- **`ProfileStore::save(&self) -> io::Result<()>`** (async) — atomic write: `.toml.tmp` then rename.
- **`ProfileStore::get_password(id: Uuid) -> Option<String>`** — reads from keyring; logs warning on error.
- **`ProfileStore::set_password(id: Uuid, pw: &str)`** — stores in keyring; logs warning on error.
- **`ProfileStore::delete_password(id: Uuid)`** — deletes from keyring; silently ignores `NoEntry`.
- **`resolve_theme_config(ThemeConfig) -> ThemeMode`** — calls `dark_light::detect()` for `System`.
- **`ConnectionProfile::credentials(&self) -> TransmissionCredentials`** — auto-fetches password from keyring.
- **`ConnectionProfile::new_blank() -> Self`** — fresh UUID v4, name "New Profile", host "localhost", port 9091.

### 3.5b Settings Screen (`screens/settings.rs`)

Full-screen editor for general preferences and connection profiles.

- **Two tabs**: General (theme selector, refresh interval) and Connections (master-detail profile list).
- **Tab bar**: custom implementation using `iced` buttons with a 2px underline indicator for the
  active tab. Tabs use size-15 text inheriting the application default font.
- **General tab**: Theme selector (Light/Dark/System), Refresh interval (1–30 s). Icon Revert/Save
  buttons pinned to the bottom-left. A `"✓ Settings saved"` toast appears after saving. Theme
  changes preview immediately without requiring Save.
- **Connections tab**: master-detail layout. Profile list on the left (220 px) with [+]/[🗑] at
  the bottom. Detail form on the right: Profile Name, Host, Port, Username, Password fields plus
  Test Connection. Icon Revert/Save buttons pinned to the bottom-left of the detail panel,
  horizontally aligned with the form labels.
- **Password loading**: the keychain is queried only when Test Connection is clicked (not on
  profile selection), to avoid triggering OS keychain unlock prompts during passive browsing.
- **Unsaved-change guard**: when the user tries to navigate away with an uncommitted draft, a modal
  confirmation dialog is shown (Save / Discard / Cancel). Implemented with `iced::widget::stack!` +
  `opaque()` — no third-party Modal widget.
- **Delete confirmation**: a second overlay asks before deleting a profile. The delete button is
  disabled for the currently connected profile.
- **Test Connection button**: fires a bare `session-get` probe (5 s timeout) with draft credentials;
  shows `"Testing connection…"` while in-flight; shows `"✓ Connection test successful!"` or
  `"✗ Connection test failed: …"` ond result.
- **`SettingsScreen::new(store: &ProfileStore, active_id: Option<Uuid>, initial_tab: SettingsTab)`**.
- **`SettingsScreen::update(msg: Message) -> (Task<Message>, Option<SettingsResult>)`**.
- **`SettingsResult` variants**:
  - `GeneralSettingsSaved { theme_config, store }` → `app::update` updates theme and stashed_main.
  - `ActiveProfileSaved { profile_id, store }` → `app::update` triggers reconnect via re-probe.
  - `StoreUpdated(store)` → `app::update` replaces `profiles` (non-active change).
  - `Closed(store)` → `app::update` replaces `profiles` and restores `stashed_main`.

### 3.6 RPC Client (`rpc.rs`)

All public functions are `async` and invoked either from `Task::perform()` (connection probe) or
inside the serialized worker (all main-screen calls).

| Function                                                        | Description                                                                                                                                                                                                                                                                                                                                                       |
| --------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `post_rpc(url, creds, session_id, method, args, timeout)`       | Single HTTP POST. Returns `Err(SessionRotated)` on 409, `Err(AuthError)` on 401. Timeout is caller-supplied (10 s standard, 60 s for `torrent-add`).                                                                                                                                                                                                              |
| `session_get(url, creds, session_id)`                           | Connectivity probe. Handles one automatic session rotation. Returns `SessionInfo { session_id }`. Uses a **5 s timeout**. Called both from `Task::perform()` on the connection screen and from the settings Test Connection feature.                                                                                                                              |
| `torrent_get(url, creds, session_id)`                           | Fetches the full torrent list (`id`, `name`, `status`, `percentDone`, `totalSize`, `downloadedEver`, `uploadedEver`, `uploadRatio`, `eta`, `rateDownload`, `rateUpload`, `files`, `fileStats`, `trackerStats`, `peers`).                                                                                                                                          |
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

**`TorrentListScreen` integration:**

- `TorrentListScreen.sender: Option<tokio::sync::mpsc::Sender<RpcWork>>` — populated when `Message::RpcWorkerReady` arrives from the subscription.
- `TorrentListScreen::enqueue(work)` — calls `tx.try_send(work)`; logs an error if the 32-item buffer is full (unreachable under normal usage).
- `TorrentListScreen::enqueue_torrent_get()` — convenience wrapper used by `Tick` and post-action refresh.
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

// One row in the torrent list (all fields fetched in torrent-get)
pub struct TorrentData {
    pub id: i64,
    pub name: String,
    pub status: i32,          // 0=Stopped 1=QueueCheck 2=Checking 3=QueueDL 4=DL 5=QueueSeed 6=Seeding
    pub percent_done: f64,    // [0.0, 1.0]
    pub total_size: i64,      // bytes
    pub downloaded_ever: i64, // bytes
    pub uploaded_ever: i64,   // bytes
    pub upload_ratio: f64,
    pub eta: i64,             // seconds; -1 = unknown
    pub rate_download: i64,   // bytes/s
    pub rate_upload: i64,     // bytes/s
    pub files: Vec<TorrentFile>,
    pub file_stats: Vec<TorrentFileStat>,
    pub tracker_stats: Vec<TrackerStat>,
    pub peers: Vec<Peer>,
}

// Sort state for TorrentListScreen
pub enum SortColumn { Name, Status, Size, Downloaded, SpeedDown, SpeedUp, Eta, Ratio, Progress }
pub enum SortDir { Asc, Desc }

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

// Add-torrent modal dialog state (lives in TorrentListScreen)
pub enum AddDialogState {
    Hidden,
    AddLink { magnet: String, destination: String, error: Option<String> },
    AddFile { metainfo_b64: String, files: Vec<TorrentFileInfo>, destination: String, error: Option<String> },
}
```

### 3.8 Message Enum

```rust
pub enum Message {
    // Startup
    ProfilesLoaded(ProfileStore),
    AutoConnectResult(Result<SessionInfo, String>),

    // Connection screen
    HostChanged(String), PortChanged(String), UsernameChanged(String), PasswordChanged(String),
    ProfileNameChanged(String),
    ConnectClicked,
    CancelConnect,                         // Cancel on connection screen with return_to_main
    SessionProbeResult(Result<SessionInfo, String>),

    // Main screen (all events delegated through MainScreen)
    Main(main_screen::Message),

    // Settings screen
    Settings(settings::Message),

    // Profile switching
    SwitchProfile(Uuid),
    ProfileSwitchResult(Uuid, Result<SessionInfo, String>),
}

// main_screen::Message
pub enum Message {
    List(torrent_list::Message),
    Inspector(inspector::Message),
    Disconnect,              // escalated from List(Disconnect)
    DisconnectClicked,       // Disconnect menu item
    OpenSettingsClicked,     // Settings gear icon
    ManageConnectionsClicked,
    AddNewConnectionClicked,
    SwitchProfileClicked(Uuid),
}

// torrent_list::Message
pub enum Message {
    Tick, TorrentsUpdated(Result<Vec<TorrentData>, String>), SessionIdRotated(String),
    RpcWorkerReady(mpsc::Sender<RpcWork>),
    TorrentSelected(i64),
    PauseClicked, ResumeClicked, DeleteClicked,
    DeleteLocalDataToggled(bool), DeleteConfirmed, DeleteCancelled,
    ActionCompleted(Result<(), String>),
    AddTorrentClicked, TorrentFileRead(Result<FileReadResult, String>),
    AddLinkClicked, AddDialogMagnetChanged(String), AddDialogDestinationChanged(String),
    AddConfirmed, AddCancelled, AddCompleted(Result<(), String>),
    ColumnHeaderClicked(SortColumn),
    ThemeToggled,             // intercepted by app::update
    Disconnect,               // intercepted by main_screen::update
    ProfileDropdownToggled,   // toggle profile switcher open/closed
    ProfileSelected(Uuid),    // escalated to app::update via main_screen::update
    AddNewConnectionClicked,  // escalated
    ManageConnectionsClicked, // escalated
    OpenSettingsClicked,      // escalated
    DisconnectClicked,        // escalated
}

// inspector::Message
pub enum inspector::Message { TabSelected(ActiveTab) }
```

### 3.9 Observability

Structured logging via `tracing` 0.1 + `tracing-subscriber` 0.3. Initialised in `main()` via
`tracing_subscriber::fmt::init()`. Run with `RUST_LOG=clutch=debug` for full RPC traces.

| Level    | Examples                                                                                                            |
| -------- | ------------------------------------------------------------------------------------------------------------------- |
| `error!` | Transport failures, auth errors (401), JSON parse errors                                                            |
| `info!`  | Connect attempted/succeeded, disconnect, torrent list refreshed                                                     |
| `debug!` | Every outgoing request (url, method, session_id), every response status, session rotation details, tick guard skips |

### 3.10 Crate Stack

| Crate                  | Version | Purpose                                                       |
| ---------------------- | ------- | ------------------------------------------------------------- |
| `iced`                 | 0.14    | GUI framework (features: `tokio`, `canvas`, `image`)          |
| `iced_aw`              | 0.13    | iced add-on widgets: `Tabs` (feature: `tabs`)                 |
| `tokio`                | 1       | Async runtime (via iced's built-in integration)               |
| `reqwest`              | 0.12    | HTTP client for RPC calls (feature: `json`)                   |
| `serde` + `serde_json` | 1       | RPC payload serialization / deserialization                   |
| `toml`                 | 0.8     | Config file serialization                                     |
| `dirs`                 | 5       | OS-appropriate config directory path                          |
| `uuid`                 | 1       | Profile UUID v4 generation and serde (feature: `v4`, `serde`) |
| `keyring`              | 2       | OS keyring integration for password storage                   |
| `dark-light`           | 1       | Detect current OS light/dark theme mode                       |
| `tracing`              | 0.1     | Structured logging instrumentation                            |
| `tracing-subscriber`   | 0.3     | Log formatting and `RUST_LOG` env-filter                      |
| `rfd`                  | 0.15    | Native async file picker dialog                               |
| `base64`               | 0.22    | Base64 encoding of `.torrent` file bytes                      |
| `lava_torrent`         | 0.11    | Local `.torrent` file parsing (file list + sizes)             |
| `wiremock` _(dev)_     | 0.6     | In-process HTTP mock server for RPC integration tests         |

### 3.11 Test Coverage

100 tests, all passing. Three layers:

- **Unit tests** (`screens/connection.rs`, `screens/torrent_list.rs`, `screens/main_screen.rs`,
  `screens/inspector.rs`, `format.rs`, `profile.rs`): exercise `update()` logic and formatting helpers
  entirely in-memory, no async I/O.
  - `connection.rs`: connect-clicked state, probe failure, host→profile-name mirror, manual-edit guard.
  - `profile.rs`: TOML parse, empty TOML defaults, corrupt TOML rejection.
  - `app.rs`: theme toggle cycles Dark→Light→Dark.
    The inspector module includes tests covering tab switching.
    The main screen tests verify tab-reset behaviour and the inspector visibility invariant.
    `format.rs` has dedicated tests for all formatting functions.
- **Integration tests** (`rpc.rs`): use `wiremock` to stand up a real in-process HTTP server and
  verify the full RPC round-trip including 409 rotation, 401 auth errors, parse errors, and happy
  paths for `session_get`, `torrent_get`, `torrent_start`, `torrent_stop`, `torrent_remove`, and
  `torrent_add` (magnet, metainfo, duplicate, empty `download_dir`, rotation, auth error).

---

## 4. Roadmap

Each milestone is a vertical slice: the app remains shippable after every version.

### v0.7 — Settings & Profiles ✅ _complete_

- **Settings persistence:** Config file at `dirs::config_dir()/clutch/config.toml` (TOML via `serde`).
- **Connection profiles:** UUID-keyed profiles, passwords in OS keyring via `keyring` crate.
- **Profile switcher dropdown** in the main toolbar (replaces the Disconnect button).
- **Settings screen**: full-screen General tab (theme, refresh interval 1–30 s, default 1 s) + Connections master-detail.
  - General and Connections tabs both have icon Revert/Save buttons pinned to bottom-left.
  - Password is only read from the keychain when Test Connection is triggered.
  - Closing Settings restores the live main screen without reconnect/refetch.
  - Loading splash shown from connect until first torrent response arrives.
- **Auto-connect**: last-used profile is probed on startup; falls back to connection screen on failure.
- **Add New Connection**: reuses the connection screen with a Cancel button (`return_to_main = true`).

### Future

- **Error recovery:** Automatic reconnect with exponential back-off when the daemon becomes
  unreachable while the main screen is active.
- **CI:** GitHub Actions workflow running `cargo test` + `cargo clippy -- -D warnings` on
  push to main for Linux, macOS, and Windows.
- **Packaging:** Distribute via Github Releases as platform-specific binaries.
