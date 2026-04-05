## Why

Users have no way to choose which files inside a torrent to download — adding a torrent always
schedules every file. Selective downloading is a core BitTorrent client feature that reduces
wasted disk space and bandwidth when only a subset of files in a torrent is needed.

## What Changes

- The add-torrent dialog (file mode) adds a checkbox column to the file list. All files are
  checked by default; the user can uncheck individual files before confirming. **Select All**
  and **Deselect All** buttons allow bulk toggling.
- The inspector **Files** tab adds a checkbox column so the user can toggle individual files
  on/off on an already-added torrent at any time, including **Select All** / **Deselect All**
  bulk actions.
- `torrent-add` RPC calls gain `files-unwanted` support: unchecked file indices are passed
  to the daemon so only the selected files are scheduled from the start.
- A new `torrent-set` RPC call is introduced to update per-file wanted state after a torrent
  has been added (used by the inspector Files tab).
- `TorrentFileStats` gains a `wanted` field (already returned by Transmission) so the UI can
  render the current wanted state for each file.

## Capabilities

### New Capabilities

- `file-selection`: Per-file selection UI in the inspector Files tab and the
  `torrent-set` RPC call used to communicate wanted/unwanted file changes to the daemon.

### Modified Capabilities

- `add-torrent`: The add-torrent dialog's file list gains checkbox selection; the
  `torrent-add` RPC call gains `files-unwanted` support.

## Impact

- `src/screens/torrent_list/add_dialog.rs` — `AddDialogState::AddFile` gains
  a `selected` bitset; view gains a checkbox column.
- `src/screens/torrent_list/update.rs` — `AddConfirmed` handler passes
  `files-unwanted` to the RPC call.
- `src/screens/inspector.rs` — `view_files` gains a checkbox column and Select All /
  Deselect All buttons; new `Message::FileWantedToggled(torrent_id, file_index, wanted)`
  and `Message::AllFilesWantedToggled(torrent_id, wanted)` dispatch `torrent-set` via the
  RPC worker. `InspectorScreen` gains a `pending_wanted` map for immediate optimistic UI
  updates, cleared on each `TorrentDataRefreshed` message.
- `src/rpc/api.rs` — new `torrent_set_file_wanted` function.
- `src/rpc/models.rs` — `TorrentFileStats` gains `wanted: bool`.
- `src/rpc/worker.rs` — new `RpcCommand::SetFileWanted` variant.
