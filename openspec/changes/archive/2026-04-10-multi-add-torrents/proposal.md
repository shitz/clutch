## Why

Users frequently want to add multiple torrents at once, but the current file picker only accepts
a single file, forcing them to repeat the add flow N times. There is also no history of previously
used download directories, so users must retype long paths on every add.

## What Changes

- The native file picker is switched from single-file to multi-file selection.
- A sequential dialog queue ("carousel") processes each picked torrent one at a time, with
  "Cancel This" and "Cancel All" options when more than one torrent is pending.
- Each `ConnectionProfile` persists the last 5 download directories used with that server.
- The add-torrent dialog displays recent paths as clickable suggestion chips immediately below
  the destination text input.
- When the dialog opens from the queue, the destination field is pre-filled with the most
  recently used path for the current profile (if any).

## Capabilities

### New Capabilities

- `recent-download-paths`: Per-profile storage and UI display of the last 5 download directories
  used when adding torrents. Paths are persisted in `ConnectionProfile`, deduplicated, and shown
  as suggestion chips in the add-torrent dialog.

### Modified Capabilities

- `add-torrent`: Multi-file pick replaces single-file pick; dialog now processes a queue of
  pending torrents with "Cancel This" / "Cancel All" actions; destination field is pre-filled
  from `recent-download-paths` and writing to it updates the history on Add.

## Impact

- `src/profile.rs` — `ConnectionProfile` gains a `recent_download_paths: Vec<String>` field.
- `src/screens/torrent_list/` — dialog state gains a `pending_torrents` queue; update logic is
  extended for the carousel flow.
- `src/app.rs` / `src/app/routing.rs` — new `Message::ProfilePathUsed(String)` routed to
  `AppState` to update and persist the profile.
- No new dependencies required.
