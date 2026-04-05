## 1. Data Model & Theme

- [x] 1.1 Add `wanted: bool` field (with `#[serde(default = "default_true")]`) to `TorrentFileStats` in `src/rpc/models.rs`
- [x] 1.2 Add `selected: Vec<bool>` field to `AddDialogState::AddFile` in `src/screens/torrent_list/add_dialog.rs`
- [x] 1.3 Initialise `selected` to `vec![true; files.len()]` wherever `AddDialogState::AddFile` is constructed in `src/screens/torrent_list/update.rs`
- [x] 1.4 Add `pending_wanted: HashMap<usize, bool>` field to `InspectorScreen` in `src/screens/inspector.rs`
- [x] 1.5 Add `CheckState` enum (`Checked`, `Unchecked`, `Mixed`) to `src/theme.rs`
- [x] 1.6 Add icon constants `ICON_CB_CHECKED` (`\u{e834}`), `ICON_CB_UNCHECKED` (`\u{e835}`), `ICON_CB_MIXED` (`\u{e909}`) to `src/theme.rs`
- [x] 1.7 Add `m3_tristate_checkbox(state: CheckState, label: &str, on_toggle: impl Fn(CheckState) -> Message)` helper in `src/theme.rs` — renders a borderless button with the appropriate icon (primary colour for Checked/Mixed, text colour for Unchecked)

## 2. RPC Layer

- [x] 2.1 Add `files_unwanted: Vec<i64>` parameter to `torrent_add` in `src/rpc/api.rs`; include `"files-unwanted"` in the JSON body only when the vec is non-empty
- [x] 2.2 Add `torrent_set_file_wanted(url, credentials, session_id, torrent_id: i64, file_indices: &[i64], wanted: bool)` function in `src/rpc/api.rs`
- [x] 2.3 Add `RpcWork::SetFileWanted { params, torrent_id: i64, file_indices: Vec<i64>, wanted: bool }` variant to `src/rpc/worker.rs`
- [x] 2.4 Add `RpcResult::FileWantedSet(Result<(), RpcError>, Vec<usize>)` variant to `src/rpc/worker.rs` to carry the original indices back to the caller
- [x] 2.5 Handle `RpcWork::SetFileWanted` in `execute_work` in `src/rpc/worker.rs` with session-rotation retry, returning `RpcResult::FileWantedSet(result, indices)`
- [x] 2.6 Update the `TorrentAdd` arm in `execute_work` to forward `files_unwanted` to `api::torrent_add`
- [x] 2.7 Update `RpcWork::TorrentAdd` in `src/rpc/worker.rs` to carry `files_unwanted: Vec<i64>`

## 3. Add-Torrent Dialog UI

- [x] 3.1 Add `Message::AddDialogFileToggled(usize)`, `Message::AddDialogSelectAll`, and `Message::AddDialogDeselectAll` variants to the message enum in `src/screens/torrent_list/mod.rs`
- [x] 3.2 Handle `Message::AddDialogFileToggled(index)` in `src/screens/torrent_list/update.rs` to flip `selected[index]`
- [x] 3.3 Handle `Message::AddDialogSelectAll` and `Message::AddDialogDeselectAll` in `src/screens/torrent_list/update.rs` to set all `selected` entries to `true` / `false`
- [x] 3.4 Update `view_add_dialog` in `src/screens/torrent_list/add_dialog.rs` to render a `checkbox` widget at the start of each file row, emitting `Message::AddDialogFileToggled(i)`
- [x] 3.5 Compute `aggregate_state: CheckState` from `selected` (all true → Checked, none true → Unchecked, otherwise Mixed) and render `m3_tristate_checkbox` as the header row above the file list in `view_add_dialog`
- [x] 3.6 Update the `AddConfirmed` handler in `src/screens/torrent_list/update.rs` to compute `files_unwanted` (indices where `selected[i] == false`) and pass it to `RpcWork::TorrentAdd`

## 4. Inspector Files Tab

- [x] 4.1 Add `Message::FileWantedToggled { torrent_id: i64, file_index: usize, wanted: bool }`, `Message::AllFilesWantedToggled { torrent_id: i64, wanted: bool }`, and `Message::FileWantedSetSuccess { indices: Vec<usize> }` to `src/screens/inspector.rs`
- [x] 4.2 Handle `Message::FileWantedToggled` in `inspector::update`: insert into `pending_wanted` and enqueue `RpcWork::SetFileWanted`
- [x] 4.3 Handle `Message::AllFilesWantedToggled` in `inspector::update`: populate `pending_wanted` for all file indices and enqueue a **single** `RpcWork::SetFileWanted` with the full index range `[0..n-1]`
- [x] 4.4 Handle `Message::FileWantedSetSuccess { indices }` in `inspector::update`: remove each index in `indices` from `pending_wanted` (applies on both RPC success and failure)
- [x] 4.5 Update `view_files` in `src/screens/inspector.rs` to render a `checkbox` per row using `pending_wanted.get(&i).copied().unwrap_or(file_stats[i].wanted)` as displayed state
- [x] 4.6 Compute `aggregate_state: CheckState` from effective per-file states and render `m3_tristate_checkbox` as the header row above the file list in `view_files`
- [x] 4.7 Map `inspector::Message::FileWantedToggled` and `inspector::Message::AllFilesWantedToggled` to `RpcWork::SetFileWanted` enqueues in `src/app.rs` / `src/screens/main_screen.rs`
- [x] 4.8 When `RpcResult::FileWantedSet(_, indices)` is received from the worker in `app.rs`, dispatch `inspector::Message::FileWantedSetSuccess { indices }` to the inspector

## 5. Tests

- [x] 5.1 Add RPC unit test for `torrent_set_file_wanted` with `wanted = true` — verify `"files-wanted"` in the request body in `src/rpc/api.rs`
- [x] 5.2 Add RPC unit test for `torrent_set_file_wanted` with `wanted = false` — verify `"files-unwanted"` in the request body in `src/rpc/api.rs`
- [x] 5.3 Add RPC unit test for `torrent_add` with `files_unwanted` populated — verify the array appears in the request body in `src/rpc/api.rs`
- [x] 5.4 Add unit tests for `AddDialogSelectAll` / `AddDialogDeselectAll` message handling: verify all entries in `selected` are set correctly in `src/screens/torrent_list/update.rs`
- [x] 5.5 Add unit test for `FileWantedToggled`: verify `pending_wanted` is updated with the toggled index in `src/screens/inspector.rs`
- [x] 5.6 Add unit test for `FileWantedSetSuccess`: verify only the specified indices are removed from `pending_wanted`, leaving any other pending entries intact
- [x] 5.7 Add unit test for `AllFilesWantedToggled`: verify all indices are populated in `pending_wanted`
- [x] 5.8 Add unit test verifying that a `torrent-get` poll result arriving while a `torrent-set` is in-flight does NOT clear `pending_wanted` (i.e. no `TorrentDataRefreshed` path exists)

## 6. Changelog & Quality Gates

- [x] 6.1 Add an entry under `## [Unreleased] > ### Added` in `CHANGELOG.md` describing selective file downloading, per-file checkboxes, tri-state Select All header, and optimistic UI with race-condition-free reconciliation
- [x] 6.2 Run `cargo fmt` — no formatting changes
- [x] 6.3 Run `cargo check` — no errors
- [x] 6.4 Run `cargo clippy -- -D warnings` — no warnings
- [x] 6.5 Run `cargo test` — all tests pass
