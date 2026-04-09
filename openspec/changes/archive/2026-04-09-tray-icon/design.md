## Context

Clutch wraps the Transmission JSON-RPC daemon. Torrents should keep seeding when the user closes
the window; today, closure terminates the process. Vanilla iced 0.14 provides no built-in tray
API. The Tauri team's `tray-icon` crate offers safe, cross-platform (macOS NSMenu, Windows HMENU,
Linux libappindicator) OS bindings that can be integrated alongside iced's event loop.

`AppState` currently holds `screen: Screen`, `profiles: ProfileStore`, `alt_speed_enabled: bool`,
and several auth-dialog fields. The `update()` surface is a free function following iced 0.14's
functional style. All I/O goes through the single `mpsc` RPC worker in `src/rpc/worker.rs`.

## Goals / Non-Goals

**Goals:**

- Keep the process alive when the window is closed; hide the window instead.
- Render a static tray icon with a native context menu.
- Context menu items: ↓ Speed (disabled text), ↑ Speed (disabled text), separator, Resume All,
  Pause All, separator, Turtle Mode (check item), separator, Show Clutch, Exit.
- Bridge native menu click events into `update()` without blocking the iced event loop.
- Update speed labels and Turtle Mode checked state reactively from existing poll results.
- Graceful degradation on Linux environments without libappindicator.

**Non-Goals:**

- Dynamic tray icon graphics (changing color per download state). Static icon only.
- Desktop notifications for completed torrents.
- Tray on platforms where `tray-icon` has no support.

## Decisions

### Decision 1 — `TrayState` Newtype in `AppState`

`tray_icon::TrayIcon` does not implement `Debug`. `AppState` derives `Debug`. The chosen
solution is a `TrayState` newtype with a manual `Debug` impl:

```rust
pub struct TrayState {
    /// Held for its lifetime side-effect (drops destroy the OS tray icon).
    _icon: tray_icon::TrayIcon,
    pub items: TrayMenuItems,
}

impl std::fmt::Debug for TrayState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrayState").finish_non_exhaustive()
    }
}
```

`AppState` gains `pub tray: Option<TrayState>` (`None` on Linux fallback or before init).

**Alternative considered:** Module-level static — rejected because it would be global mutable
state, conflicting with the Elm model's single source of truth.

### Decision 2 — Tray and Menu Initialization Order

`tray_icon` components must be built on the main OS thread before the iced event loop takes
control. This maps exactly onto `AppState::new()`, which iced calls synchronously on startup.

The tray is built in `src/tray.rs::build()`, called from `AppState::new()`. On failure (Linux
without libappindicator), the error is logged with `tracing::warn!` and
`state.tray = None`. The app continues as a normal windowed application.

**Alternative considered:** Lazy init on first `ProfilesLoaded` — rejected because that fires
asynchronously after `new()` and may run on a non-main thread.

### Decision 3 — Event Bridging via Polling Subscription

`tray_icon` uses `crossbeam-channel` receivers:

- `tray_icon::TrayIconEvent::receiver()` — icon left/right click
- `tray_icon::menu::MenuEvent::receiver()` — menu item activated

iced subscriptions are `async`. The bridge polls both channels every 100 ms using
`iced::subscription::channel` with a recursive async stream:

```rust
async fn tray_event_stream(output: &mut mpsc::Sender<Message>) {
    // short sleep, then try_recv on both channels, yield messages
}
```

Each menu activation yields `Message::TrayAction(TrayAction)` where `TrayAction` is an enum
keyed by a stable `u32` ID assigned at menu build time.

**Alternative considered:** Spawning a background `tokio::task` that blocks on `recv()` and
sends via `mpsc`. Rejected — adds complexity and a second thread; the 100 ms timer is
imperceptible overhead.

### Decision 4 — Menu IDs and `TrayAction` Enum

Menu item IDs are assigned via `tray_icon::menu::MenuItem::with_id(u32, ...)`. A companion enum
provides ergonomic dispatch:

```rust
#[derive(Debug, Clone, Copy)]
pub enum TrayAction {
    ResumeAll,
    PauseAll,
    ToggleTurtle,
    ShowWindow,
    Exit,
}
```

`TrayMenuItems` stores the `MenuItem` / `CheckMenuItem` handles for `speed_down`, `speed_up`,
and `turtle_mode` so their labels and checked state can be mutated at any time.

### Decision 5 — Pause All / Resume All Dispatch

From `update()`, Pause All and Resume All require the full live torrent ID list. This list lives
in `MainScreen::list.torrents`. The handler pattern:

```rust
Message::TrayAction(TrayAction::PauseAll) => {
    if let Screen::Main(main) = &mut state.screen {
        let ids: Vec<i64> = main.list.torrents.iter().map(|t| t.id).collect();
        // enqueue RpcWork::TorrentStop { params, ids }
    }
    Task::none()
}
```

If the screen is not `Main` (not yet connected), the action is silently ignored.

### Decision 6 — Turtle Mode via Existing Message Path

Tray Turtle Mode click forwards as `Message::Main(main_screen::Message::TurtleModeToggled)`,
routed through the existing `handle_global_message` path in `routing.rs`. This reuses identical
logic for session-set RPC, no duplication.

### Decision 7 — Window Hide/Show

`iced::window::Settings { exit_on_close_request: false, .. }` is set in `main.rs`.

`handle_global_message` in `routing.rs` intercepts
`Message::WindowEvent(iced::window::Event::CloseRequested)` (surfaced via
`iced::event::listen_with`) and returns `iced::window::close(id)` replaced by
`iced::window::change_mode(id, iced::window::Mode::Hidden)`.

"Show Clutch" dispatches `iced::window::change_mode(id, iced::window::Mode::Windowed)` plus
`iced::window::gain_focus(id)`.

The main window ID is obtained by listening for `iced::Event::Window(id, _)` on first render
and storing it in `AppState`.

### Decision 8 — Icon Image Decoding

`crate::theme::ICON_256_BYTES` is a PNG-encoded byte slice (embedded via `include_bytes!`).
`tray_icon::Icon::from_rgba` expects a flat decoded `[R, G, B, A, ...]` buffer — passing
encoded bytes directly produces garbage or a panic.

The fix uses the `image` crate, which is already an indirect dependency via
`iced = { features = ["image"] }` and is re-exported as `iced::widget::image`:

```rust
let img = image::load_from_memory(crate::theme::ICON_256_BYTES)
    .map(|i| i.into_rgba8());
if let Ok(rgba) = img {
    let (w, h) = rgba.dimensions();
    tray_icon::Icon::from_rgba(rgba.into_raw(), w, h).ok()
} else {
    None
}
```

If decoding fails the tray build returns `None` (graceful fallback).

### Decision 9 — Reactive Menu Label Updates

The Turtle Mode checked state is updated from `SessionDataLoaded` (already carries
`alt_speed_enabled`). No changes needed to `SessionData`.

For transfer speeds: the current `SessionData` model (populated by `session-get`) contains only
configuration fields — it has no live speed data. Transmission's live global speeds are available
via `session-stats`, but adding a new periodic RPC call is out of scope for this change.

Instead, Clutch sums `rate_download` and `rate_upload` across all torrents in `TorrentData` at
every `TorrentsUpdated(Ok)` event. Per-torrent rates come directly from the daemon and are what
Transmission itself shows in its web UI torrent list. Protocol overhead (DHT, PEX, tracker
announces) is real but typically ≤1–2% — immaterial for a glanceable tray display. The values
are clearly labelled as ↓ / ↑ in the menu, not as network interface totals.

```rust
if let (Screen::Main(main), Some(tray)) = (&state.screen, &state.tray) {
    let dl: i64 = main.list.torrents.iter().map(|t| t.rate_download).sum();
    let ul: i64 = main.list.torrents.iter().map(|t| t.rate_upload).sum();
    tray.items.speed_down.set_text(format!("↓ {}", format_speed(dl)));
    tray.items.speed_up.set_text(format!("↑ {}", format_speed(ul)));
}
```

These calls are synchronous and cheap; `tray-icon` internals are `Arc`-backed and thread-safe.

## Risks / Trade-offs

- **Linux libappindicator missing** → Graceful fallback: `tracing::warn!` and `tray = None`;
  app functions as a standard windowed application.
- **macOS main-thread requirement** → Mitigated by building the tray in `AppState::new()`,
  which iced calls on the main thread before the event loop starts.
- **100 ms polling overhead** → Negligible: two non-blocking `try_recv` calls per tick.
- **Window ID availability** → The first `iced::Event::Window` event yields the main window ID;
  stored in `AppState.main_window_id: Option<iced::window::Id>`. Hide/show commands are no-ops
  until this ID is known (never an issue in practice since events only arrive after the window
  opens).
- **tray-icon version compatibility** → Pin to a specific minor version in `Cargo.toml` and run
  `cargo audit` after adding.

## Migration Plan

Pure additive change — no existing features are removed or modified except the window-close
behaviour. Linux users on environments without libappindicator see no change. Rollback is a
dependency removal and revert of `main.rs`.

## Open Questions

None — all decisions are resolved above.
