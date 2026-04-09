## 1. RPC Layer — Bulk IDs

- [x] 1.1 Update `RpcWork::TorrentStart` variant: rename `id: i64` → `ids: Vec<i64>` in `src/rpc/worker.rs`
- [x] 1.2 Update `RpcWork::TorrentStop` variant: rename `id: i64` → `ids: Vec<i64>` in `src/rpc/worker.rs`
- [x] 1.3 Update `RpcWork::TorrentRemove` variant: rename `id: i64` → `ids: Vec<i64>` in `src/rpc/worker.rs`
- [x] 1.4 Update `RpcWork::SetLocation` variant: rename `torrent_id: i64` → `ids: Vec<i64>` in `src/rpc/worker.rs`
- [x] 1.5 Update `RpcWork::TorrentSetBandwidth` variant: rename `torrent_id: i64` → `ids: Vec<i64>` in `src/rpc/worker.rs`
- [x] 1.6 Update `api::torrent_start` async fn: change `id: i64` parameter to `ids: &[i64]`; update JSON body to serialize `ids` array
- [x] 1.7 Update `api::torrent_stop` async fn: change `id: i64` parameter to `ids: &[i64]`; update JSON body
- [x] 1.8 Update `api::torrent_remove` async fn: change `id: i64` parameter to `ids: &[i64]`; update JSON body
- [x] 1.9 Update `api::torrent_set_location` async fn: change `torrent_id: i64` parameter to `ids: &[i64]`; update JSON body
- [x] 1.10 Update `api::torrent_set_bandwidth` async fn: change `torrent_id: i64` parameter to `ids: &[i64]`; update JSON body
- [x] 1.11 Update the `worker.rs` dispatch arms to pass `ids.as_slice()` (or `&ids`) to each updated API function
- [x] 1.12 Run `cargo check` and fix all compilation errors introduced by the RPC signature changes

## 2. Selection State Model

- [x] 2.1 Add `selected_ids: HashSet<i64>`, `selection_anchor: Option<i64>`, and `modifiers: iced::keyboard::Modifiers` fields to `TorrentListScreen` in `src/screens/torrent_list/mod.rs`
- [x] 2.2 Remove `selected_id: Option<i64>` from `TorrentListScreen`
- [x] 2.3 Update `TorrentListScreen::new()` to initialise the three new fields (empty set, None, default modifiers)
- [x] 2.4 Update `selected_torrent()` helper to return `Some` only when `selected_ids.len() == 1`
- [x] 2.5 Add `Message::ModifiersChanged(iced::keyboard::Modifiers)` variant to `torrent_list::Message`
- [x] 2.6 Add `modifiers_subscription()` method to `TorrentListScreen` that returns an always-active subscription mapping `iced::keyboard::Event::ModifiersChanged` to `Message::ModifiersChanged`
- [x] 2.7 Merge `modifiers_subscription()` into `MainScreen::subscription()` (alongside tick, worker, dialog_kb, cursor)

## 3. Click Selection Semantics in `update()`

- [x] 3.1 Extract a `visible_torrents<'a>(&'a self) -> Vec<&'a TorrentData>` pure helper on `TorrentListScreen` that applies the current sort and filter to return the rendered row order (reuse logic from `view::view`); this helper is used by both the view layer and all selection handlers
- [x] 3.2 Rewrite the `Message::TorrentSelected(id)` handler in `src/screens/torrent_list/update.rs`:
      plain click → clear set, insert `id`, set anchor;
      Ctrl/Cmd click → toggle `id`, update anchor;
      Shift click → call `visible_torrents()` at click time, find `selection_anchor` ID's current index (fall back to plain-click if anchor not found), find `id`'s index, union the range into `selected_ids`
- [x] 3.3 Add handler for `Message::ModifiersChanged(m)` in `update.rs` → `state.modifiers = m; Task::none()`
- [x] 3.4 Add `Message::KeyboardSelectAll` variant to `torrent_list::Message`
- [x] 3.5 Extend the cursor subscription (`cursor_subscription()` in `mod.rs`) to also intercept `iced::Event::Keyboard(KeyPressed { key: Named::A, modifiers })` and emit `Message::KeyboardSelectAll` when `modifiers.command() || modifiers.control()` — guard to return `None` when the add-torrent dialog is open
- [x] 3.6 Add handler for `Message::KeyboardSelectAll` in `update.rs`: populate `selected_ids` with IDs from `visible_torrents()`, set `selection_anchor` to the first visible ID (or `None` if empty)

## 4. Context-Menu Right-Click Selection Drift

- [x] 4.1 Update the `Message::TorrentRightClicked(id)` handler: if `id` is in `selected_ids`, do nothing; otherwise clear `selected_ids`, insert `id`, reset anchor
- [x] 4.2 Remove the `i64` parameter from `ContextMenuStart`, `ContextMenuPause`, `ContextMenuDelete`, and `OpenSetLocation` message variants (they now operate on `selected_ids`)
- [x] 4.3 Update all match arms and view call sites that produce or consume these messages

## 5. Toolbar Action Guards and Bulk Dispatch

- [x] 5.1 Update `src/screens/torrent_list/toolbar.rs`: compute `can_start` with `.any()` over `selected_ids`; compute `can_pause` with `.any()`; compute `can_delete` as `!selected_ids.is_empty()`
- [x] 5.2 Update the `PauseClicked` handler in `update.rs` to enqueue `RpcWork::TorrentStop { ids: selected_ids.iter().copied().collect() }`
- [x] 5.3 Update the `ResumeClicked` handler similarly for `RpcWork::TorrentStart`
- [x] 5.4 Update the `ContextMenuStart` handler to enqueue `TorrentStart` with all `selected_ids`
- [x] 5.5 Update the `ContextMenuPause` handler to enqueue `TorrentStop` with all `selected_ids`
- [x] 5.6 Update context-menu Start/Pause enable logic in `src/screens/torrent_list/dialogs.rs` to use aggregate `.any()` over selected torrents

## 6. Delete Confirmation Dialog — Bulk Support

- [x] 6.1 Change `confirming_delete: Option<(i64, bool)>` → `Option<(Vec<i64>, bool)>` in `TorrentListScreen`
- [x] 6.2 Update `DeleteClicked` handler to set `confirming_delete = Some((selected_ids.iter().copied().collect(), false))`
- [x] 6.3 Update `ContextMenuDelete` handler analogously
- [x] 6.4 Update `DeleteConfirmed` handler to read `ids` from `confirming_delete` and enqueue `RpcWork::TorrentRemove { ids, delete_local_data }`
- [x] 6.5 Update `view_delete_dialog` in `dialogs.rs` to accept `ids: &[i64]` and a torrent-name lookup; render single vs. bulk title text
- [x] 6.6 Update the call site in `view.rs` (or wherever the dialog is rendered) to pass the ids from `confirming_delete`

## 7. Multi-Row Highlight in View

- [x] 7.1 Update `src/screens/torrent_list/view.rs` row rendering: apply `theme::selected_row` style when `state.selected_ids.contains(&torrent.id)` (replace the old `state.selected_id == Some(id)` check)
- [x] 7.2 Update the `SetLocationDialog` `torrent_id: i64` field to `ids: Vec<i64>` and adjust `OpenSetLocation` handler and `SetLocationApply` handler to dispatch `RpcWork::SetLocation { ids, ... }`

## 8. Inspector Bulk Edit Mode

- [x] 8.1 Add `InspectorBulkOptionsState` struct to `src/screens/inspector/state.rs` with all-optional fields (as specified in design.md Decision 6), deriving `Default`
- [x] 8.2 Add `bulk_options: InspectorBulkOptionsState` field to `InspectorScreen`, initialised by `Default`
- [x] 8.3 Change `inspector::view` signature in `src/screens/inspector/mod.rs` from `(state, torrent: &TorrentData)` to `(state, torrent: Option<&TorrentData>, selected_count: usize)`
- [x] 8.4 In `inspector::view`: when `selected_count > 1`, render only the Options tab (hide/disable others) using `bulk_options`; when `selected_count == 1`, use the existing single-torrent rendering path
- [x] 8.5 Update `main_screen.rs` to pass `(torrent: self.list.selected_torrent(), selected_count: self.list.selected_ids.len())` to `inspector::view`
- [x] 8.6 Add new `inspector::Message` variants for bulk toggle/submit interactions (e.g. `BulkDownloadLimitToggled(bool)`, `BulkUploadLimitToggled(bool)`, `BulkRatioModeChanged(u8)`, `BulkDownloadLimitSubmitted`, `BulkUploadLimitSubmitted`, `BulkRatioLimitSubmitted`, `BulkHonorGlobalToggled(bool)`)
- [x] 8.7 Add `update` handlers in `src/screens/inspector/` for each bulk message variant that update `bulk_options` wrapping the value in `Some(v)` — ensuring any first interaction marks the field as "touched" — and enqueue `RpcWork::TorrentSetBandwidth { ids: all_selected, args }` with only the touched field set (others remain `None`)
- [x] 8.8 Update bulk message intercepts in `main_screen.rs` to collect `ids` from `self.list.selected_ids` and enqueue the appropriate `RpcWork`
- [x] 8.9 In the bulk options view, bind all `Option<bool>` checkbox/toggler values to `state.bulk_options.download_limited.unwrap_or(false)` (and likewise for upload and honor-global togglers) so the UI shows a coherent false/unchecked initial state while internally distinguishing "untouched" (`None`) from "explicitly false" (`Some(false)`)

## 9. Inspector Sync on Selection Change

- [x] 9.1 Update the `TorrentSelected` intercept in `main_screen.rs`: when the selection transitions to a single torrent, reset `inspector.active_tab`, clear `pending_wanted`, and repopulate `inspector.options`; when it transitions to multi-select, reset `inspector.bulk_options` to default
- [x] 9.2 Update the `TorrentSelected` intercept to not reset the inspector when the selection size stays > 1 (a Ctrl/Cmd click within an existing multi-select should not wipe bulk_options)

## 10. Quality Gates

- [x] 10.1 Run `cargo fmt` and fix any formatting issues
- [x] 10.2 Run `cargo check` — zero errors
- [x] 10.3 Run `cargo clippy -- -D warnings` — zero warnings
- [x] 10.4 Run `cargo test` — all tests pass
- [x] 10.5 Update `CHANGELOG.md` with the new multi-select feature under `[Unreleased]`

## 11. Bug Fixes and Follow-up Iterations

- [x] 11.1 **ETA bug**: In `src/screens/torrent_list/view.rs`, suppress the ETA cell for
      torrents where `percent_done >= 1.0` (render `"—"` instead of calling `format_eta`).
      The daemon reports a positive seeding-goal ETA even for fully downloaded torrents.
- [x] 11.2 **Inspector bulk mode — tab bar**: Replace the separate "Bulk Edit" header panel
      with the full 5-tab segmented control. In bulk mode `active_tab` is forced to `Options`
      and clicks on any other tab are absorbed (mapped to `TabSelected(Options)`). A small
      subtitle under the tab bar explains the bulk-edit context.
- [x] 11.3 **Click-to-select in empty space**: Wrap the scrollable rows in a `stack` with a
      full-height `mouse_area` as the bottom layer; clicking empty space below the last row
      sends `TorrentSelected(last_visible_id)` through the background layer.
- [x] 11.4 **Filter + selection consistency**: Add `prune_selection_to_visible()` to
      `TorrentListScreen` in `mod.rs`; call it after every `FilterToggled`,
      `FilterAllClicked`, and `TorrentsUpdated(Ok)` so `selected_ids` always reflects only
      the currently visible (non-filtered, daemon-present) set. `selection_anchor` is also
      cleared when it is no longer visible.
- [x] 11.5 **Multi-select and filter tests**: Add 8 new unit tests covering Cmd-click
      multi-select, `selected_torrent()` returning `None` for multi-select,
      filter-change pruning, `FilterAllClicked` pruning, daemon-removal pruning,
      `prune_selection_to_visible` directly, Shift-click range respecting filters,
      and `KeyboardSelectAll` selecting only visible torrents.
- [x] 11.6 **Set Data Location — Enter key**: Add `.on_submit(Message::SetLocationApply)` to
      the path `text_input` in `view_set_location_dialog` so pressing Enter triggers Apply
      without requiring a mouse click.
