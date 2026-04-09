## Why

Clutch forces users to interact with one torrent at a time, making common power-user workflows
(pausing a whole category, deleting multiple finished downloads, moving a batch of files) tedious.
The Transmission JSON-RPC protocol natively supports bulk `ids` arrays on every mutation endpoint,
so all the infrastructure is already there — only the client-side selection model needs to change.

## What Changes

- Replace `TorrentListScreen::selected_id: Option<i64>` with `selected_ids: HashSet<i64>` and a
  new `selection_anchor: Option<i64>` for Shift-click range tracking.
- Add a `modifiers: iced::keyboard::Modifiers` field to `TorrentListScreen`, populated by a new
  `ModifiersChanged` subscription, so the row-click handler can apply the correct selection
  semantics (plain click clears+selects, Ctrl/Cmd+Click toggles, Shift+Click extends a range).
- Update the toolbar and context-menu action guards to evaluate against the aggregate selection
  (Start enabled if _any_ selected torrent is startable; Pause enabled if _any_ is pausable).
- Change all RPC mutation call sites to pass `Vec<i64>` instead of `i64`, and update
  `RpcWork` variants and `api.rs` async functions to accept `&[i64]` slices.
- Add `InspectorBulkOptionsState` to `InspectorScreen`; when `selected_ids.len() > 1` the
  inspector enters Bulk Edit mode, hides General/Files/Trackers/Peers tabs, and shows a blank
  Options form where only explicitly changed fields are sent in the RPC payload.
- Adapt the delete-confirmation dialog to display a generic count message for bulk deletes
  (`"Remove 5 selected torrents?"`) instead of a named single-torrent message.
- **BREAKING**: The `RpcWork::TorrentStart`, `TorrentStop`, `TorrentRemove`, `SetLocation`, and
  `TorrentSetBandwidth` variants change from single-`i64` to `Vec<i64>` for the ID field.

## Capabilities

### New Capabilities

- `bulk-selection`: Multi-select state model, keyboard-modifier subscription, and all three
  click-selection semantics (plain, Ctrl/Cmd, Shift) at the torrent-list row level.

### Modified Capabilities

- `torrent-list`: Row selection model changes from single `Option<i64>` to `HashSet<i64>` with
  anchor tracking; aggregate-based toolbar action guards.
- `rpc-client`: Mutation API signatures (`torrent-start`, `torrent-stop`, `torrent-remove`,
  `torrent-set-location`, `torrent-set`) updated to accept slices of IDs.
- `context-menu`: Right-click selection behavior adapts to existing selection set; actions
  operate on the full current selection rather than a single torrent.
- `torrent-options`: Inspector Options tab gains a Bulk Edit mode with blank defaults and
  sparse-field dispatch; General/Files/Trackers/Peers tabs suppressed in bulk mode.

## Impact

- **`src/screens/torrent_list/mod.rs`** — `TorrentListScreen` struct; `Message` enum (new
  `ModifiersChanged` variant; context-menu messages drop their `i64` arg).
- **`src/screens/torrent_list/update.rs`** — Selection handler rewritten; `confirming_delete`
  changes to `Option<(Vec<i64>, bool)>`; RPC enqueue sites updated to pass `Vec<i64>`.
- **`src/screens/torrent_list/toolbar.rs`** — Action-button enable logic updated.
- **`src/screens/torrent_list/dialogs.rs`** — Delete dialog adapts to single vs. bulk context.
- **`src/screens/torrent_list/view.rs`** — Row rendering adjusts `selected_ids` highlight logic.
- **`src/screens/inspector/state.rs`** — New `InspectorBulkOptionsState` struct added.
- **`src/screens/inspector/mod.rs`** — Inspector `view` handles optional/absent `TorrentData`.
- **`src/screens/main_screen.rs`** — Selection intercept and inspector-sync logic updated.
- **`src/rpc/api.rs`** — Five async fn signatures updated to `ids: &[i64]`.
- **`src/rpc/worker.rs`** — `RpcWork` variants updated; worker dispatch updated.
- No new external dependencies required.
