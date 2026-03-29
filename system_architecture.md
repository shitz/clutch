# Clutch — System Architecture

A cross-platform desktop application built in pure Rust that serves as a remote GUI for a
Transmission BitTorrent daemon, communicating via the Transmission JSON-RPC API. Built with
the `iced` 0.14 GUI framework using the Elm architecture.

---

## 1. Core Invariants

| Invariant             | Rule                                                                                                                 |
| --------------------- | -------------------------------------------------------------------------------------------------------------------- |
| **Non-blocking UI**   | `update()` returns in microseconds. All I/O lives in `Task::perform()` or the RPC worker subscription.               |
| **Serialized RPC**    | All daemon calls flow through a single `tokio::sync::mpsc` channel — at most one HTTP request in-flight at any time. |
| **Screen-safe state** | Torrent data is only accessible when `Screen::Main` is active — illegal UI states are unrepresentable.               |
| **Cross-platform**    | No GTK, no web views. Pure Rust dependencies only.                                                                   |

---

## 2. Module Layout

```text
src/
├── main.rs                 Entry point: tracing, fonts, window constraints, iced launch.
├── app.rs                  AppState, Screen router, Message enum, top-level update/view/subscription.
├── format.rs               Shared formatting helpers (size, speed, ETA, duration).
├── profile.rs              ProfileStore, ConnectionProfile, GeneralSettings, ThemeConfig, keyring.
├── theme.rs                Material Design 3 palettes, icon font, shared widget styles.
├── rpc/
│   ├── mod.rs              Re-exports.
│   ├── models.rs           Data types: TorrentData, TransmissionCredentials, ConnectionParams, etc.
│   ├── error.rs            RpcError enum (implements std::error::Error).
│   ├── transport.rs        Low-level HTTP: shared reqwest::Client, post_rpc().
│   ├── api.rs              High-level functions: session_get, torrent_get, torrent_start/stop/remove/add.
│   └── worker.rs           RpcWork, RpcResult, execute_work() with session-rotation retry.
└── screens/
    ├── mod.rs
    ├── connection.rs        Connection launchpad (saved profiles tab + quick-connect tab).
    ├── main_screen.rs       Parent component: composes torrent list + inspector.
    ├── inspector.rs         Tabbed detail panel (General, Files, Trackers, Peers).
    ├── torrent_list/
    │   ├── mod.rs           TorrentListScreen state, Message enum, re-exports.
    │   ├── update.rs        Elm update function.
    │   ├── view.rs          Toolbar, column headers, torrent rows.
    │   ├── sort.rs          Pure sort logic (SortColumn, SortDir, sort_torrents).
    │   ├── add_dialog.rs    Add-torrent modal (AddDialogState, TorrentFileInfo, view).
    │   └── worker.rs        Serialized RPC worker subscription stream.
    └── settings/
        ├── mod.rs           SettingsTab, SettingsResult, Message enum, re-exports.
        ├── state.rs         SettingsScreen struct, PendingNavigation, helpers.
        ├── update.rs        Elm update function (with unsaved-change guard).
        ├── view.rs          All view methods (header, tabs, general, connections, overlay dialogs).
        └── draft.rs         ProfileDraft — in-memory editable copy of a connection profile.
```

---

## 3. Screen Router

The application runs exactly one screen at a time, modelled as a discriminated enum:

```rust
pub enum Screen {
    Connection(ConnectionScreen),  // Startup launchpad, after Disconnect
    Main(MainScreen),              // Active torrent list + inspector
    Settings(SettingsScreen),      // Full-screen settings editor
}
```

`AppState` holds the current screen, resolved theme, profile store, active profile UUID, and an
optional stashed `MainScreen` (preserved while Settings is open so closing Settings can restore
it without reconnecting).

### Startup Flow

1. `ProfilesLoaded` → resolve theme, check `last_connected`.
2. If a last-connected profile exists → fire `session-get` probe → `AutoConnectResult`.
3. Success → `Screen::Main`. Failure → `Screen::Connection` with saved profiles.

### Message Architecture

Each screen has its own `Message` enum. The top-level `app::Message` wraps them:

```rust
pub enum Message {
    ProfilesLoaded(ProfileStore),
    AutoConnectResult(Result<SessionInfo, String>),
    Connection(connection::Message),
    Main(main_screen::Message),
    Settings(settings::Message),
    Noop,
}
```

`app::update()` handles startup messages and global intercepts (Disconnect, OpenSettings,
ManageProfiles), then dispatches to the active screen.

---

## 4. Connection Screen

Two-tab launchpad:

- **Saved Profiles** (default when profiles exist) — clickable profile cards plus "Manage / Add
  Profile" link to Settings.
- **Quick Connect** (default when empty) — ephemeral host/port/user/pass form. Nothing persisted.

`update()` returns `(Task<Message>, Option<ConnectSuccess>)`. When `ConnectSuccess` is `Some`,
the caller transitions to `Screen::Main`.

---

## 5. Main Screen

Parent Elm component composing two children:

- **TorrentListScreen** — toolbar, 9-column sortable header, scrollable rows, add-torrent modal,
  serialized RPC worker subscription.
- **InspectorScreen** — tabbed detail panel (General, Files, Trackers, Peers) shown below the
  list when a torrent is selected.

Layout: list fills full height when no selection. With a selection, split vertically
`FillPortion(3)` (list) / `FillPortion(1)` (inspector).

A loading splash is shown until the first `TorrentsUpdated` response arrives.

---

## 6. Settings Screen

Full-screen editor with two tabs:

- **General** — Theme (Light/Dark/System), Refresh interval (1–30 s).
- **Connections** — Master-detail profile list. Edit name, host, port, username, password.
  Test Connection button. Password loaded from OS keyring on demand only.

`update()` returns `(Task<Message>, Option<SettingsResult>)`. Results signal the parent:

| Result                 | Effect                                              |
| ---------------------- | --------------------------------------------------- |
| `GeneralSettingsSaved` | Update theme + refresh interval.                    |
| `ActiveProfileSaved`   | Reconnect with updated credentials.                 |
| `StoreUpdated`         | Persist profile changes (non-active profile).       |
| `Closed`               | Restore stashed main screen, persist store updates. |

An unsaved-change guard modal prevents accidental data loss on tab switch, profile switch, or close.

---

## 7. RPC Layer

### Transport

A shared `reqwest::Client` (`LazyLock` static) issues all HTTP requests. Per-request timeouts
are set via `RequestBuilder::timeout()` (5 s for probes, 10 s for standard calls, 60 s for
`torrent-add`).

Session rotation (HTTP 409) is surfaced as `RpcError::SessionRotated`. `session_get` retries
once automatically; all other calls are retried by the worker.

### Serialized Worker

All main-screen RPC calls are serialized through an `iced::Subscription` backed by a
`tokio::sync::mpsc::channel(32)`:

```text
update() ──[try_send(RpcWork)]──▶ mpsc channel ──▶ worker loop ──▶ execute_work()
                                                          │
                                                  emit Message back to update()
```

This guarantees at most one in-flight HTTP connection, preventing Transmission's RPC handler
from wedging on concurrent requests.

### ConnectionParams

All RPC calls use `ConnectionParams` — a bundle of URL, credentials, and session ID — avoiding
the need to pass three separate values through every call site.

---

## 8. Profile Store

Persistent config at `dirs::config_dir()/clutch/config.toml` (TOML). Passwords stored in the
OS keyring (service: `"clutch"`, account: profile UUID).

Key operations:

- `load_sync()` / `load()` — read config from disk (sync for initial theme, async for runtime).
- `save()` — atomic write (`.tmp` then rename).
- `get_password()` / `set_password()` / `delete_password()` — OS keyring via `keyring` crate.
- `adopt_last_connected(from)` — merge `last_connected` from another store, clearing if deleted.

---

## 9. Theme

Material Design 3 dark and light palettes. Theme preference (`Light`/`Dark`/`System`) is resolved
at startup and on change via `dark_light::detect()`.

All widget styles are centralised in `theme.rs`: tab bar styles, progress bar colouring by torrent
status, inspector surface, selected-row highlight, and Material icon rendering.

---

## 10. Observability

Structured logging via `tracing`. Run with `RUST_LOG=clutch=debug` for full RPC traces.

---

## 11. Crate Stack

| Crate                  | Version | Purpose                                |
| ---------------------- | ------- | -------------------------------------- |
| `iced`                 | 0.14    | GUI framework                          |
| `iced_aw`              | 0.13    | Additional widgets                     |
| `reqwest`              | 0.12    | HTTP client                            |
| `tokio`                | 1       | Async runtime                          |
| `serde` + `serde_json` | 1       | JSON serialization                     |
| `toml`                 | 0.8     | Config serialization                   |
| `uuid`                 | 1       | Profile UUIDs                          |
| `keyring`              | 2       | OS keyring for passwords               |
| `dirs`                 | 5       | Config directory path                  |
| `dark-light`           | 1       | OS theme detection                     |
| `tracing`              | 0.1     | Structured logging                     |
| `rfd`                  | 0.15    | Native file picker                     |
| `base64`               | 0.22    | `.torrent` file encoding               |
| `lava_torrent`         | 0.11    | `.torrent` file parsing                |
| `wiremock` _(dev)_     | 0.6     | HTTP mock server for integration tests |

---

## 12. Test Coverage

95 tests across three layers:

- **Unit tests** — exercise `update()` logic and formatting helpers in-memory. Cover all screens
  (connection, torrent list, settings, inspector, main screen) and the format module.
- **Integration tests** — use `wiremock` for full RPC round-trips including session rotation,
  auth errors, parse errors, and happy paths for all six RPC methods.
- **Sort tests** — verify pure sort logic for all column types and edge cases.
