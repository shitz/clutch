## Why

BitTorrent clients are background applications — users expect them to keep seeding after closing
the main window. Today Clutch exits when the window is closed, making it unusable as a persistent
daemon manager. A system tray icon solves this with minimal UI surface: hide on close, expose
at-a-glance speeds, and give one-click access to Pause All / Resume All and Turtle Mode.

## What Changes

- Add the `tray-icon` crate (Tauri team) to provide native OS tray icon bindings for macOS,
  Windows, and Linux.
- Intercept the window `CloseRequested` event and hide the window instead of exiting.
- Construct a native context menu with four groups: Speeds (read-only), Bulk Actions, Global
  Toggles, and Lifecycle (Show / Exit).
- Bridge native menu click events into the iced event loop via a polling subscription (100 ms
  interval, `try_recv` on the tray-icon crossbeam channels).
- Reactively update speed text and Turtle Mode checked state on every torrent/session poll
  result that reaches `update()`.
- Add a `TrayState` wrapper type (with manual `Debug` impl) inside `AppState` to hold the
  `TrayIcon` handle and mutable `TrayMenuItems` references for live mutation.
- `main.rs`: Set `exit_on_close_request: false` on `iced::window::Settings`.

## Capabilities

### New Capabilities

- `tray-icon`: System tray icon lifecycle, native context menu construction, event bridging
  subscription, window hide/show on close/restore, and reactive menu label/state updates.

### Modified Capabilities

- `torrent-list`: The window close gesture no longer exits the app — it hides it instead.
  This is a user-visible behaviour change captured in the torrent-list spec.

## Impact

- **`Cargo.toml`**: Add `tray-icon = { version = "0.21", features = ["default"] }`. Audit
  required before merging (security-sensitive OS binding).
- **`src/main.rs`**: Add `exit_on_close_request: false` to `iced::window::Settings`.
- **`src/app.rs`**: Add `TrayState` to `AppState`; new `Message::TrayMenuItemClicked(u32)`,
  `Message::WindowCloseRequested`, `Message::TrayShowWindow`, `Message::TrayExit` variants;
  extend `subscription()` to include the tray event poll; update speed/turtle state on each
  `TorrentsUpdated` / `SessionDataLoaded` message.
- **`src/tray.rs`** (new): `TrayState` struct, `TrayMenuItems`, tray construction helpers, and
  subscription stream.
- No changes to `rpc/`, `screens/`, or `theme/` modules.
- No new RPC calls — all tray actions reuse existing `Message::Main(TurtleModeToggled)`,
  `Message::Main(TorrentStart/Stop)`, and `iced::exit()`.
