## 1. RPC Layer

- [x] 1.1 Add `SetLocationArgs` request struct to `src/rpc/models.rs` with fields `ids: Vec<i64>`,
      `location: String`, `move: bool` (serde `rename = "move"`)
- [x] 1.2 Add `torrent_set_location(id: i64, location: &str, move_data: bool)` async function to
      `src/rpc/api.rs` that issues `torrent-set-location` and returns `Result<(), RpcError>`
- [x] 1.3 Add `RpcWork::SetLocation { torrent_id: i64, location: String, move_data: bool }` variant
      to the worker enum in `src/rpc/worker.rs`
- [x] 1.4 Add the match arm for `RpcWork::SetLocation` in the worker loop, calling
      `torrent_set_location` and forwarding any error to the result channel

## 2. State & Messages

- [x] 2.1 Add `last_cursor_position: iced::Point` field to `TorrentListScreen` (default `Point::ORIGIN`)
- [x] 2.2 Add `context_menu: Option<(i64, iced::Point)>` field to `TorrentListScreen`
- [x] 2.3 Add `SetLocationDialog` struct (`torrent_id: i64`, `path: String`, `move_data: bool`) and
      store it as `set_location_dialog: Option<SetLocationDialog>` on `TorrentListScreen`
- [x] 2.4 Add message variants: `CursorMoved(iced::Point)`, `TorrentRightClicked(i64)`,
      `DismissContextMenu`, `ContextMenuStart(i64)`, `ContextMenuPause(i64)`, `ContextMenuDelete(i64)`,
      `OpenSetLocation(i64)`, `SetLocationPathChanged(String)`, `SetLocationMoveToggled`,
      `SetLocationApply`, `SetLocationCancel`

## 3. Cursor Tracking Subscription

- [x] 3.1 Add a `iced::event::listen_with` subscription in `TorrentListScreen::subscription()` (or
      `app::subscription()`) that maps `Mouse::CursorMoved { position }` to `Message::CursorMoved`
- [x] 3.2 Handle `Message::CursorMoved` in `update()`: store the point, return `Task::none()`

## 4. Context Menu Update Handlers

- [x] 4.1 Handle `TorrentRightClicked(id)`: set `context_menu = Some((id, last_cursor_position))`
- [x] 4.2 Handle `DismissContextMenu`: set `context_menu = None`
- [x] 4.3 Handle `ContextMenuStart(id)`: dismiss the menu, enqueue `torrent-start` RPC for `id`
- [x] 4.4 Handle `ContextMenuPause(id)`: dismiss the menu, enqueue `torrent-stop` RPC for `id`
- [x] 4.5 Handle `ContextMenuDelete(id)`: dismiss the menu, open the existing delete-confirmation
      dialog targeting `id`
- [x] 4.6 Handle `OpenSetLocation(id)`: dismiss the menu, look up `downloadDir` for `id` from the
      current torrent list, initialize `SetLocationDialog { torrent_id: id, path: downloadDir, move_data: true }`

## 5. Set Data Location Dialog Update Handlers

- [x] 5.1 Handle `SetLocationPathChanged(s)`: update `set_location_dialog.path`
- [x] 5.2 Handle `SetLocationMoveToggled`: toggle `set_location_dialog.move_data`
- [x] 5.3 Handle `SetLocationCancel`: set `set_location_dialog = None`
- [x] 5.4 Handle `SetLocationApply`: enqueue `RpcWork::SetLocation` with `torrent_id`, `path`,
      and `move_data` from the dialog; set `set_location_dialog = None`

## 6. Torrent Row View Updates

- [x] 6.1 Wrap each torrent row widget in `mouse_area` with both `.on_press(SelectTorrent(id))`
      and `.on_right_press(TorrentRightClicked(id))`

## 7. Context Menu Overlay View

- [x] 7.1 When `context_menu` is `None`, return the plain scrollable list as before
- [x] 7.2 When `context_menu` is `Some((torrent_id, point))`, wrap the list view in a `stack` with:
  - Layer 1: transparent `mouse_area` (`Length::Fill`) with `.on_press(DismissContextMenu)`
  - Layer 2: M3 card menu positioned via container padding at `(point.x, effective_y)`
    where `effective_y = if point.y > window_height − 150 { point.y − 150 } else { point.y }`
- [x] 7.3 Render menu items: Start, Pause (with `.on_press` absent when inactive, per torrent
      status), Delete (always active), Set Data Location (always active)
- [x] 7.4 Apply the application's standard disabled visual style to the inactive Start or Pause
      button (no `.on_press` handler attached)

## 8. Set Data Location Dialog View

- [x] 8.1 When `set_location_dialog` is `Some`, render a centered M3 card overlay (same pattern
      as Add Torrent dialog) on top of the full screen
- [x] 8.2 Dialog contains: path `text_input` styled with `theme::m3_text_input`, "Move data to
      new location" checkbox, "Cancel" `m3_tonal_button`, "Apply" `m3_primary_button`
- [x] 8.3 Dialog is stacked on top of the existing view (and replaces/takes precedence over the
      context menu overlay if somehow both are open)

## 9. Quality Gates

- [x] 9.1 Run `cargo fmt` — no formatting errors
- [x] 9.2 Run `cargo check` — no compilation errors
- [x] 9.3 Run `cargo clippy -- -D warnings` — no warnings
- [x] 9.4 Run `cargo test` — all tests pass

## 10. Inspector General Tab — Data Path & Error

- [x] 10.1 Add `error: i32` and `error_string: String` fields to `TorrentData` in
      `src/rpc/models.rs` (`serde` rename `"error"` / `"errorString"`, default 0 / "")
- [x] 10.2 Add `"error"` and `"errorString"` to the `torrent-get` `fields` list in
      `src/rpc/api.rs`
- [x] 10.3 Add "Data Path" row to `view_general` in `src/screens/inspector.rs`, rendered as
      `info_row("Data Path", torrent.download_dir)` placed after "Upload Speed" in `col2`
- [x] 10.4 Add "Error" row to `view_general`, rendered as `info_row("Error", …)` placed after
      "Ratio" in `col1`; display "none" when `error == 0`, otherwise `error_string` (or
      `"error {code}"` if `error_string` is empty)
