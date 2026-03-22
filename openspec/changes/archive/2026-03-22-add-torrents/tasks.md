## 1. Dependencies

- [x] 1.1 Add `rfd` (async file dialog) to `Cargo.toml` dependencies
- [x] 1.2 Add `base64` crate to `Cargo.toml` dependencies
- [x] 1.3 Add `lava_torrent` crate to `Cargo.toml` dependencies (local `.torrent` parsing)

## 2. RPC Client

- [x] 2.1 Define `AddPayload` enum in `rpc.rs` with `Magnet(String)` and `Metainfo(String)` variants
- [x] 2.2 Implement `torrent_add(url, creds, session_id, payload, download_dir: Option<String>)` in `rpc.rs`; include `download-dir` in RPC arguments only when `Some` and non-empty
- [x] 2.3 Write integration tests for `torrent_add` in `rpc.rs` (magnet success, metainfo success, download-dir included, download-dir omitted, 409 rotation, 401 auth error)

## 3. App Messages & State Types

- [x] 3.1 Define `TorrentFileInfo { path: String, size_bytes: u64 }` and `AddDialogState` enum (`Hidden`, `AddLink { magnet, destination, error }`, `AddFile { metainfo_b64, files, destination, error }`) in `main_screen.rs`
- [x] 3.2 Define `FileReadResult { metainfo_b64: String, files: Vec<TorrentFileInfo> }` for use as the success payload of `TorrentFileRead`
- [x] 3.3 Add message variants to `Message` in `app.rs`: `AddTorrentClicked`, `TorrentFileRead(Result<FileReadResult, String>)`, `AddLinkClicked`, `AddDialogMagnetChanged(String)`, `AddDialogDestinationChanged(String)`, `AddConfirmed`, `AddCancelled`, `AddCompleted(Result<(), String>)`

## 4. Main Screen State & Update

- [x] 4.1 Add `add_dialog: AddDialogState` field to `MainScreen`; initialize to `Hidden`
- [x] 4.2 Handle `AddTorrentClicked` — issue `Task::perform` that opens `rfd::AsyncFileDialog`, reads bytes, Base64-encodes, parses with `lava_torrent`, returns `TorrentFileRead`
- [x] 4.3 Handle `TorrentFileRead(Ok)` — set `add_dialog = AddFile { metainfo_b64, files, destination: String::new(), error: None }`
- [x] 4.4 Handle `TorrentFileRead(Err)` — surface error (e.g. set a transient toolbar error or log); dialog does not open
- [x] 4.5 Handle `AddLinkClicked` — set `add_dialog = AddLink { magnet: String::new(), destination: String::new(), error: None }`
- [x] 4.6 Handle `AddDialogMagnetChanged` — update `magnet` field in `AddLink` state
- [x] 4.7 Handle `AddDialogDestinationChanged` — update `destination` field in `AddLink` or `AddFile` state
- [x] 4.8 Handle `AddCancelled` — set `add_dialog = Hidden`
- [x] 4.9 Handle `AddConfirmed` — read payload and destination from `add_dialog`; guard empty magnet; issue `Task::perform(torrent_add(payload, download_dir))`
- [x] 4.10 Handle `AddCompleted(Ok)` — set `add_dialog = Hidden`, trigger immediate `torrent-get` poll
- [x] 4.11 Handle `AddCompleted(Err)` — store error in the active `add_dialog` variant's `error` field

## 5. Main Screen View

- [x] 5.1 Add "Add Torrent" and "Add Link" buttons to the main toolbar row, wired to `AddTorrentClicked` and `AddLinkClicked`
- [x] 5.2 Implement `view_add_dialog` helper that renders the modal overlay using `iced::widget::stack`; the overlay is a centered bordered container with a semi-transparent backdrop fill
- [x] 5.3 Dialog layout for `AddFile`: destination text input, scrollable file list (name + human-readable size per row), Add/Cancel buttons, optional error label
- [x] 5.4 Dialog layout for `AddLink`: magnet URI text input, destination text input, static note "File list unavailable for magnet links", Add/Cancel buttons, optional error label
- [x] 5.5 Wire `view_add_dialog` into the main `view()` — wrap root element in `stack!` when `add_dialog != Hidden`

## 6. Tests

- [x] 6.1 Unit test `AddLinkClicked` — `add_dialog` transitions to `AddLink`
- [x] 6.2 Unit test `AddConfirmed` with empty magnet — no task emitted, dialog stays open
- [x] 6.3 Unit test `AddConfirmed` with a valid magnet — task is emitted
- [x] 6.4 Unit test `AddCancelled` — `add_dialog` resets to `Hidden`
- [x] 6.5 Unit test `TorrentFileRead(Ok)` — `add_dialog` transitions to `AddFile` with correct file list
- [x] 6.6 Unit test `AddCompleted(Ok)` — `add_dialog` set to `Hidden`, immediate poll triggered
- [x] 6.7 Unit test `AddCompleted(Err)` — error stored in `add_dialog`, dialog not dismissed
