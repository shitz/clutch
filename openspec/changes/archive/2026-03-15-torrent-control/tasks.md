## 1. RPC Layer

- [x] 1.1 Add `torrent_start(url, creds, session_id, id)` async function to `rpc.rs`
- [x] 1.2 Add `torrent_stop(url, creds, session_id, id)` async function to `rpc.rs`
- [x] 1.3 Add `torrent_remove(url, creds, session_id, id, delete_local_data)` async function to `rpc.rs`
- [x] 1.4 Add integration tests for `torrent_start`, `torrent_stop`, and `torrent_remove` (happy path + 409 rotation + 401 auth error)

## 2. Message Enum

- [x] 2.1 Add `TorrentSelected(i64)` to the `Message` enum in `app.rs`
- [x] 2.2 Add `PauseClicked`, `ResumeClicked`, `DeleteClicked` to `Message`
- [x] 2.3 Add `DeleteLocalDataToggled(bool)`, `DeleteConfirmed`, `DeleteCancelled` to `Message`
- [x] 2.4 Add `ActionCompleted(Result<(), String>)` to `Message`

## 3. Selection and Confirmation State

- [x] 3.1 Add `selected_id: Option<i64>` field to `MainScreen`
- [x] 3.2 Handle `TorrentSelected(id)` in `MainScreen::update()` — toggle selection (set if different, clear if same)
- [x] 3.3 Add `confirming_delete: Option<(i64, bool)>` field to `MainScreen` (id + delete_local_data checkbox state)

## 4. Toolbar Enable/Disable Logic

- [x] 4.1 Update `MainScreen::view()` to derive Pause button enabled state from selected torrent status (active statuses: 3, 4, 5, 6)
- [x] 4.2 Update `MainScreen::view()` to derive Resume button enabled state (status == 0)
- [x] 4.3 Update `MainScreen::view()` to derive Delete button enabled state (any selection)

## 5. Row Selection UI

- [x] 5.1 Make each torrent row a `button` (or `mouse_area`) that emits `TorrentSelected(id)` on press
- [x] 5.2 Apply a distinct background style to the selected row

## 6. Action Handlers

- [x] 6.1 Handle `PauseClicked` in `MainScreen::update()` — call `Task::perform(rpc::torrent_stop(...))` mapping result to `ActionCompleted`
- [x] 6.2 Handle `ResumeClicked` in `MainScreen::update()` — call `Task::perform(rpc::torrent_start(...))` mapping result to `ActionCompleted`
- [x] 6.3 Handle `DeleteClicked` in `MainScreen::update()` — set `confirming_delete = Some((selected_id, false))`, no RPC
- [x] 6.4 Handle `DeleteLocalDataToggled(val)` — update the bool in `confirming_delete`
- [x] 6.5 Handle `DeleteCancelled` — clear `confirming_delete`
- [x] 6.6 Handle `DeleteConfirmed` — call `Task::perform(rpc::torrent_remove(...))` with id and flag from `confirming_delete`, clear `confirming_delete`
- [x] 6.7 Handle `ActionCompleted(Ok)` — set `is_loading = false`, fire immediate `torrent-get` refresh
- [x] 6.8 Handle `ActionCompleted(Err)` — set `is_loading = false`, store error message for inline display

## 7. Confirmation UI

- [x] 7.1 Render confirmation row in `MainScreen::view()` when `confirming_delete` is `Some` — show torrent name, "Delete local data" checkbox, Confirm and Cancel buttons
- [x] 7.2 Hide normal toolbar action buttons while confirmation row is visible

## 8. Tests

- [x] 8.1 Unit test: `TorrentSelected` toggles `selected_id` correctly (select, re-select to deselect, select different)
- [x] 8.2 Unit test: Pause/Resume/Delete button enabled states match spec scenarios
- [x] 8.3 Unit test: `DeleteClicked` sets `confirming_delete` and issues no task
- [x] 8.4 Unit test: `DeleteCancelled` clears `confirming_delete`
- [x] 8.5 Unit test: `DeleteConfirmed` issues `torrent_remove` task with correct `delete_local_data` flag
- [x] 8.6 Unit test: `DeleteLocalDataToggled` updates checkbox state in `confirming_delete`
- [x] 8.7 Unit test: `ActionCompleted(Ok)` triggers an immediate poll task
- [x] 8.8 Unit test: `ActionCompleted(Err)` stores error and does not start a poll
- [x] 8.9 Unit test: poll tick is ignored while action is in-flight (`is_loading = true`)
