## 1. RPC Layer — Models & New Calls

- [x] 1.1 Add `SessionData` struct to `src/rpc/models.rs` with fields `alt_speed_enabled`,
      `alt_speed_down`, `alt_speed_up` (serde-mapped to Transmission JSON keys).
- [x] 1.2 Add seven per-torrent bandwidth fields to `TorrentData` in `src/rpc/models.rs`:
      `download_limited`, `download_limit`, `upload_limited`, `upload_limit`,
      `seed_ratio_limit`, `seed_ratio_mode`, `honors_session_limits`.
- [x] 1.3 Update the `torrent-get` `fields` array in `src/rpc/api.rs` to include the seven
      new field names.
- [x] 1.4 Update the existing `session_get` function in `src/rpc/api.rs` to return
      `SessionData` instead of (or in addition to) the bare session-id; populate `AppState`
      alt-speed fields from the result.
- [x] 1.5 Add `SessionSetArgs` struct and `session_set` async function in
      `src/rpc/api.rs`; only `Some(...)` fields are serialised into the JSON payload.
- [x] 1.6 Add `TorrentBandwidthArgs` struct and `torrent_set_bandwidth` async function in
      `src/rpc/api.rs`.
- [x] 1.7 Add new worker message variants to `src/rpc/worker.rs` for `SessionSet` and
      `TorrentSetBandwidth` and route them through the existing `mpsc` queue.
- [x] 1.8 Write unit tests (wiremock) for `session_set` and `torrent_set_bandwidth` in
      `src/rpc/api.rs`.

## 2. AppState — Global Turtle Mode State

- [x] 2.1 Add `alt_speed_enabled: bool`, `alt_speed_down_kbps: u32`, and
      `alt_speed_up_kbps: u32` to `AppState` in `src/app.rs`.
- [x] 2.2 Add `AppMessage::TurtleModeToggled` and `AppMessage::SessionDataReceived`
      variants; handle them in `app::update` to flip `alt_speed_enabled` (optimistic) and
      dispatch the `session_set` worker call.
- [x] 2.3 On successful connect, call `session_get` and populate `AppState.alt_speed_enabled`,
      `AppState.alt_speed_down_kbps`, and `AppState.alt_speed_up_kbps` from the response.
      Do NOT push any stored values to the daemon on connect.
- [x] 2.4 Add a session-poll counter to `MainScreenState` (or `app::update`). On every
      torrent-poll tick, increment the counter; when it reaches 10 seconds, dispatch a
      `session_get` call and update `AppState` alt-speed fields from the response. Reset
      the counter after each `session-get`.

## 3. Theme — Speed Icon

- [x] 3.1 Add `ICON_SPEED: char = '\u{E9E4}'` constant to `src/theme.rs`.
- [x] 3.2 Add an `active_icon_button` helper (or extend `icon_button`) in `src/theme.rs`
      that accepts an `active: bool` flag; when `true` it renders the icon in the theme's
      primary color.

## 4. Main Screen — Toolbar Turtle Button

- [x] 4.1 Add `TurtleModeToggled` to `screens/main_screen.rs` `Message` enum.
- [x] 4.2 Render the toolbar turtle button using `theme::active_icon_button` in
      `src/screens/main_screen.rs`; pass `AppState.alt_speed_enabled` as the active flag.
- [x] 4.3 On button press, dispatch `Message::TurtleModeToggled` which bubbles up to
      `app::update`.

## 5. Settings Screen — Bandwidth Card

- [x] 5.1 Add bandwidth draft fields to `ProfileDraft` in `src/screens/settings/draft.rs`:
      `speed_limit_down: String`, `speed_limit_down_enabled: bool`,
      `speed_limit_up: String`, `speed_limit_up_enabled: bool`,
      `alt_speed_down: String`, `alt_speed_up: String`,
      `ratio_limit: String`, `ratio_limit_enabled: bool`.
      Initialise from `ConnectionProfile` (not from live daemon state) when the settings
      screen opens.
- [x] 5.1a Add the matching bandwidth fields to `ConnectionProfile` in `src/profile.rs`
      so they are persisted to the TOML config file.
- [x] 5.2 Add a full "Bandwidth" `m3_card` in `src/screens/settings/view.rs`, below the
      Connection Details card, with three sections: - **Standard Global Limits** — toggle+value rows for download and upload speed (KB/s) - **Alternative Limits (Turtle Mode)** — plain value rows for alt download/upload (KB/s) - **Seeding** — toggle+ratio text-input row
      The card header uses bold 16 px text.
- [x] 5.3 Add the corresponding message variants in `src/screens/settings/update.rs`
      (`DraftSpeedLimitDownChanged`, `DraftSpeedLimitDownEnabledToggled`,
      `DraftSpeedLimitUpChanged`, `DraftSpeedLimitUpEnabledToggled`,
      `DraftAltSpeedDownChanged`, `DraftAltSpeedUpChanged`,
      `DraftRatioLimitChanged`, `DraftRatioLimitEnabledToggled`).
      Apply the digit-only guard to all numeric fields; also allow `'.'` in the ratio field.
- [x] 5.4 On Save, write all bandwidth fields to `ConnectionProfile` and persist to TOML.
      If the active profile is being saved and only bandwidth fields changed (host / port /
      username unchanged), return `SettingsResult::ActiveProfileBandwidthSaved` instead of
      `ActiveProfileSaved` — this pushes limits to the daemon via `session-set` without
      triggering a full reconnect.
- [x] 5.5 In `app::update`, add a `make_push_bandwidth_task` helper that builds a
      `session-set` call from a `ConnectionProfile`'s bandwidth fields. Call it both on
      initial connect (after `session-get` probe) and in the `ActiveProfileBandwidthSaved`
      handler.

## 6. Inspector — Options Tab

- [x] 6.1 Add `Options` variant to `ActiveTab` enum in `src/screens/inspector.rs`.
- [x] 6.2 Add `InspectorOptionsState` struct to `src/screens/inspector.rs` with fields:
      `download_limited: bool`, `download_limit_val: String`,
      `upload_limited: bool`, `upload_limit_val: String`,
      `ratio_mode: u8` (0=Global, 1=Custom, 2=Unlimited),
      `ratio_limit_val: String`, `honors_session_limits: bool`.
      Attach to `InspectorScreen`.
- [x] 6.3 Add messages (all prefixed `Options`):
      `OptionsDownloadLimitToggled(bool)`, `OptionsDownloadLimitChanged(String)`, `OptionsDownloadLimitSubmitted`,
      `OptionsUploadLimitToggled(bool)`, `OptionsUploadLimitChanged(String)`, `OptionsUploadLimitSubmitted`,
      `OptionsRatioModeChanged(u8)`, `OptionsRatioLimitChanged(String)`, `OptionsRatioLimitSubmitted`,
      `OptionsHonorGlobalToggled(bool)`.
      Apply digit-only guard to KB/s messages; allow digits + single `'.'` for the ratio
      message. There is **no** `OptionsSaveClicked` message.
- [x] 6.4 Reset `InspectorOptionsState` on torrent selection change in `main_screen::update`
      (via `InspectorOptionsState::from_torrent`). Draft is NOT reset on every poll tick —
      polling would clobber in-progress text edits.
- [x] 6.5 Implement `inspector::view_options` with a **two-column card layout**: - **Left card** (`FillPortion(1)`): sub-label "Speed Limits", scrollable column with
      download toggle+value row, upload toggle+value row, and "Honor Global Limits"
      toggle row. **No Save button.** - **Right card** (`FillPortion(1)`): sub-label "Seeding Ratio", 3-way segmented
      control [Global | Custom | Unlimited] bound to `ratio_mode`. When mode is Custom,
      an additional "Custom ratio" text-input row appears below.
- [x] 6.6 Changes are applied **immediately** via `main_screen::update` intercepting each
      `Options*` message: - Toggle messages enqueue a `TorrentSetBandwidth` RPC immediately. - Text-change messages only update local draft state. - Submit messages (`*Submitted`) enqueue the RPC (only when the corresponding
      toggle is enabled; otherwise ignored). - `OptionsHonorGlobalToggled` sends the **full** bandwidth state in one RPC call
      (`honors_session_limits`, `download_limited`, `download_limit`, `upload_limited`,
      `upload_limit`) so the per-torrent limits are enforced immediately when the torrent
      opts out of global limits.
- [x] 6.7 `seedRatioMode` encoding: segmented mode 0=Global, 1=Custom, 2=Unlimited.
      `seedRatioLimit` is only included in the RPC payload when mode is 1 or when the
      ratio submit fires.

## 7. Quality Gates

- [x] 7.1 Run `cargo fmt && cargo check && cargo clippy -- -D warnings` with zero warnings.
- [x] 7.2 Run `cargo test` — all existing and new tests pass.
- [x] 7.3 Manual smoke test: connect to a live Transmission daemon, toggle Turtle Mode from
      toolbar, verify icon highlights and daemon reflects the change; open a torrent's Options
      tab, set a download limit, save, and verify it sticks after the next poll.
- [x] 7.4 Update `CHANGELOG.md` "Unreleased" section with the new features.
