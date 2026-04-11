## 1. RPC Models

- ~~1.1 Add `queue_position: Option<u32>` field to `TorrentData`~~ _(removed — queue position not displayed in UI)_
- [x] 1.2 Add `download_queue_enabled`, `download_queue_size`, `seed_queue_enabled`, and
      `seed_queue_size` fields to `SessionData` in `src/rpc/models.rs`
- [x] 1.3 Add the same four fields as `Option<…>` to `SessionSetArgs` in `src/rpc/models.rs`
      with correct `serde(rename)` attributes (`"download-queue-enabled"` etc.)

## 2. RPC API Layer

- [x] 2.1 Update `parse_session_data` in `src/rpc/api.rs` to populate the four new
      `SessionData` queue fields from the `session-get` response JSON
- [x] 2.2 Add `queue_move_top(url, credentials, session_id, ids)` async function to
      `src/rpc/api.rs`, issuing a `queue-move-top` JSON-RPC call
- [x] 2.3 Add `queue_move_up` async function following the same pattern
- [x] 2.4 Add `queue_move_down` async function following the same pattern
- [x] 2.5 Add `queue_move_bottom` async function following the same pattern
- [x] 2.6 Write unit tests (using `wiremock`) for each of the four new queue-move functions,
      including a session-rotation test for at least one

## 3. RPC Worker

- [x] 3.1 Add four new `RpcWork` variants to the enum in `src/rpc/worker.rs`:
      `QueueMoveTop`, `QueueMoveUp`, `QueueMoveDown`, `QueueMoveBottom`, each carrying
      `params: RpcParams` and `ids: Vec<i64>`
- [x] 3.2 Add a `QueueMoved(Vec<i64>)` variant (or reuse an existing unit variant) to
      `RpcResult` for the outcome of a queue-move call
- [x] 3.3 Add match arms in `execute_work` for the four new variants, calling the
      corresponding `api::queue_move_*` functions

## 4. Torrent List — Queue Position Column

- ~~4.1 Add `QueuePosition` variant to `SortColumn`~~ _(removed — queue position column not displayed)_
- ~~4.2 Extend `sort_torrents` for `SortColumn::QueuePosition`~~ _(removed)_
- ~~4.3 Add `#` column header~~ _(removed)_
- ~~4.4 Render `queue_position` cell in each torrent row~~ _(removed)_
- ~~4.5 Add unit tests for `QueuePosition` sort~~ _(removed)_

## 5. Torrent List — Context Menu Queue Actions

- [x] 5.1 Add four new message variants to the torrent list `Message` enum:
      `QueueMoveTop`, `QueueMoveUp`, `QueueMoveDown`, `QueueMoveBottom`
- [x] 5.2 Add the four queue-movement action buttons to `view_context_menu_overlay` in
      `src/screens/torrent_list/dialogs.rs`, grouped visually below the existing actions
- [x] 5.3 Handle the four new messages in `update` in
      `src/screens/torrent_list/update.rs`, enqueuing the corresponding `RpcWork` variants
      with the current `selected_ids`
- [x] 5.4 After a successful queue-move `RpcResult`, trigger a `TorrentGet` refresh to
      update positions in-list (following the `SetLocation` pattern)

## 6. Settings — Queueing Card

- [x] 6.1 Add `download_queue_enabled: bool`, `download_queue_size: String`,
      `seed_queue_enabled: bool`, and `seed_queue_size: String` to `SettingsDraft` (or the
      equivalent per-profile draft struct in `src/screens/settings/`)
- [x] 6.2 Populate the new draft fields from `SessionData` when the settings screen is
      opened / a session is fetched
- [x] 6.3 Add message variants for toggling and editing the four new fields in the
      settings `Message` enum
- [x] 6.4 Handle those messages in the settings `update` function, marking the draft dirty;
      in the `on_input` handler for queue size fields, silently discard any non-digit
      characters so the `String` only ever contains digits or is empty (same defensive
      pattern used for bandwidth limit inputs)
- [x] 6.5 Include the four new fields when constructing `SessionSetArgs` during save;
      parse size strings with `.parse::<u32>().unwrap_or(0)` to safely handle empty or
      partially-deleted input
- [x] 6.6 Build the "Queueing" card view in `src/screens/settings/view.rs`, placed directly
      below the existing "Bandwidth" card; use `m3_card`, toggles, and `text_input` controls
      matching the bandwidth card's visual layout; gate each size input's `.on_input` on its
      corresponding `enabled` toggle

## 7. Quality Gates

- [x] 7.1 Run `cargo fmt` and resolve any formatting issues
- [x] 7.2 Run `cargo clippy -- -D warnings` and fix all warnings
- [x] 7.3 Run `cargo test` and confirm all tests pass, including newly added tests
- [x] 7.4 Update `CHANGELOG.md` under `[Unreleased]` with Added entries for
      queue-management and queue-configuration, and Changed entries for torrent-list,
      context-menu, and rpc-client
