## 1. Dependency and Module Setup

- [ ] 1.1 Add `tray-icon = { version = "0.21", features = ["default"] }` to `[dependencies]`
      in `Cargo.toml`; run `cargo audit` and resolve any findings
- [ ] 1.2 Create `src/tray.rs` with the Apache 2.0 license header
- [ ] 1.3 Declare `mod tray;` in `src/lib.rs`

## 2. `TrayState` and `TrayMenuItems` Structs

- [ ] 2.1 Define `TrayMenuItems` struct in `src/tray.rs` holding:
      `speed_down: tray_icon::menu::MenuItem`, `speed_up: tray_icon::menu::MenuItem`,
      `turtle_mode: tray_icon::menu::CheckMenuItem`
- [ ] 2.2 Define `TrayState` struct in `src/tray.rs` holding `_icon: tray_icon::TrayIcon`
      and `pub items: TrayMenuItems`
- [ ] 2.3 Implement `std::fmt::Debug` for `TrayState` manually using `finish_non_exhaustive()`
- [ ] 2.4 Define `TrayAction` enum in `src/tray.rs`:
      `ResumeAll`, `PauseAll`, `ToggleTurtle`, `ShowWindow`, `Exit`
- [ ] 2.5 Define stable `u32` ID constants in `src/tray.rs`:
      `MENU_ID_RESUME`, `MENU_ID_PAUSE`, `MENU_ID_TURTLE`, `MENU_ID_SHOW`, `MENU_ID_EXIT`

## 3. Tray Construction

- [ ] 3.1 Implement `pub fn build() -> Option<TrayState>` in `src/tray.rs` that:
      creates `MenuItem` and `CheckMenuItem` instances using the ID constants,
      assembles the `Menu` in the documented order (speed items disabled, separator,
      resume/pause, separator, turtle, separator, show/exit), constructs
      `tray_icon::TrayIconBuilder` with the application icon bytes and the menu, calls
      `.build()`, and returns `Some(TrayState)` on success or `None` if `.build()` returns
      an error (logging the error with `tracing::warn!`)
- [ ] 3.2 Decode the tray icon from `crate::theme::ICON_256_BYTES` using the `image` crate
      (already available as an indirect dependency via `iced`'s `"image"` feature):
      call `image::load_from_memory(bytes)?.into_rgba8()`, read `dimensions()`, then pass
      the raw buffer to `tray_icon::Icon::from_rgba(buf, w, h)`. **Do NOT pass the raw
      `include_bytes!` slice directly to `from_rgba` — those are PNG-encoded, not flat RGBA.**
      If decoding fails, log with `tracing::warn!` and return `None` from `build()`.
- [ ] 3.3 Call `tray::build()` inside `AppState::new()` in `src/app.rs` and store the result
      in `AppState.tray: Option<TrayState>`

## 4. `AppState` Changes

- [ ] 4.1 Add `pub tray: Option<TrayState>` to `AppState` in `src/app.rs`
- [ ] 4.2 Add `pub main_window_id: Option<iced::window::Id>` to `AppState` in `src/app.rs`
      (used for hide/show commands)
- [ ] 4.3 Initialise both new fields to `None` in `AppState::new()`

## 5. New Message Variants

- [ ] 5.1 Add `Message::TrayAction(crate::tray::TrayAction)` variant to `app::Message`
- [ ] 5.2 Add `Message::WindowEvent(iced::window::Id, iced::window::Event)` variant (or reuse
      iced's built-in window event listening — confirm `iced::event::listen_with` approach)

## 6. Tray Event Bridging Subscription

- [ ] 6.1 Implement `pub fn subscription() -> iced::Subscription<super::app::Message>` in
      `src/tray.rs` using `iced::subscription::channel` that: - polls `tray_icon::TrayIconEvent::receiver().try_recv()` and
      `tray_icon::menu::MenuEvent::receiver().try_recv()` on every tick - maps `MenuEvent.id` to `TrayAction` using the ID constants - maps tray icon left-click to `TrayAction::ShowWindow` - yields `Message::TrayAction(action)` for each event - sleeps 100 ms between polls using `tokio::time::sleep`
- [ ] 6.2 Merge `tray::subscription()` into `app::subscription()` in `src/app.rs` alongside
      the keyboard subscription using `Subscription::batch`

## 7. Window Hide / Show Handling

- [ ] 7.1 Set `exit_on_close_request: false` in `iced::window::Settings` in `src/main.rs`
- [ ] 7.2 Add `iced::event::listen_with` subscription in `app::subscription()` that forwards
      `iced::Event::Window(id, iced::window::Event::CloseRequested)` as
      `Message::WindowEvent(id, iced::window::Event::CloseRequested)` and captures the
      window ID from any window event to set `AppState.main_window_id`
- [ ] 7.3 Handle `Message::WindowEvent(id, iced::window::Event::CloseRequested)` in
      `routing::handle_global_message`: return `iced::window::change_mode(id, Mode::Hidden)`
      instead of exiting
- [ ] 7.4 Handle `Message::TrayAction(TrayAction::ShowWindow)` in
      `routing::handle_global_message`: return
      `Task::batch([iced::window::change_mode(id, Mode::Windowed), iced::window::gain_focus(id)])`
      using `state.main_window_id`; no-op if `main_window_id` is `None`

## 8. Tray Action Handlers

- [ ] 8.1 Handle `Message::TrayAction(TrayAction::PauseAll)` in `routing::handle_global_message`:
      if `Screen::Main(main)`, collect `main.list.torrents.iter().map(|t| t.id)`, enqueue
      `RpcWork::TorrentStop { params: main.list.params.clone(), ids }`; otherwise no-op
- [ ] 8.2 Handle `Message::TrayAction(TrayAction::ResumeAll)` analogously for
      `RpcWork::TorrentStart`
- [ ] 8.3 Handle `Message::TrayAction(TrayAction::ToggleTurtle)` by injecting
      `Message::Main(main_screen::Message::TurtleModeToggled)` — return
      `Task::done(Message::Main(main_screen::Message::TurtleModeToggled))` so the existing
      `handle_global_message` path handles the RPC; no-op when not in Main screen
- [ ] 8.4 Handle `Message::TrayAction(TrayAction::Exit)` by returning `iced::exit()`

## 9. Reactive Menu Updates

- [ ] 9.1 After processing `Message::Main(main_screen::Message::SessionDataLoaded(data))` in
      `routing::handle_global_message`, call the tray update helper if `state.tray` is `Some`:
      `tray.items.turtle_mode.set_checked(data.alt_speed_enabled)`
- [ ] 9.2 In the `TorrentsUpdated(Ok)` handler, when `state.screen` is `Screen::Main` and
      `state.tray` is `Some`, sum `.rate_download` and `.rate_upload` across `main.list.torrents`
      and update `tray.items.speed_down` / `tray.items.speed_up` via `.set_text(...)` using
      `crate::format::format_speed`. **Use per-torrent rates from `TorrentData`, not session-get
      fields — `SessionData` in this codebase has no live speed data.** The torrent-sum approach
      matches what the daemon reports per-torrent; protocol overhead (<2%) is acceptable for a
      tray display.

## 10. Quality Gates

- [ ] 10.1 Run `cargo fmt` and fix any formatting issues
- [ ] 10.2 Run `cargo check` — zero errors
- [ ] 10.3 Run `cargo clippy -- -D warnings` — zero warnings
- [ ] 10.4 Run `cargo test` — all tests pass
- [ ] 10.5 Update `CHANGELOG.md` with the tray icon feature under `[Unreleased]`
