## Context

Clutch is a Transmission remote GUI. v0.2 shipped torrent control (pause, resume, delete).
v0.3 adds the final core workflow: **adding new torrents**. The main screen toolbar is the natural
home for the entry points — it already owns the connection-lifecycle controls (Disconnect) and
per-torrent actions (Pause, Resume, Delete).

Two ingestion paths exist in the Transmission RPC:

- `filename` field — accepts a magnet URI directly.
- `metainfo` field — accepts a Base64-encoded `.torrent` file.

Both paths map to the same `torrent-add` RPC method and now share a unified confirmation dialog
that lets the user set the download destination before committing.

## Goals / Non-Goals

**Goals:**

- Two toolbar buttons: **Add Torrent** (file picker → parse → dialog) and **Add Link** (magnet
  text input → dialog).
- A unified add-torrent dialog showing: destination folder (text input) + file list preview
  (name + size per file) + Add / Cancel.
- Local parsing of `.torrent` files (via `lava_torrent`) to populate the file list without
  contacting the daemon first.
- Extend `rpc.rs` with a `torrent_add` function that accepts an optional `download_dir`.
- Immediately refresh the torrent list after a successful add.
- Keep the non-blocking invariant: all I/O inside `Task::perform()`.

**Non-Goals:**

- Per-file selection / deselection within the dialog.
- Per-torrent bandwidth or priority settings.
- Drag-and-drop file support.
- File preview for magnet links (metadata is not available before the torrent is added).

## Decisions

### UI pattern — modal overlay via `iced::widget::stack`

**Decision:** Render the add dialog as an overlay layer on top of the main screen using
`iced::widget::stack`. The stack places the dialog widget (a centered, bordered container) over
the existing torrent list + toolbar. The underlying list is still rendered but interaction is
blocked by the dialog layer sitting on top.

**Alternatives considered:**

- Inline toolbar row (original plan): does not support destination input or file preview in a
  usable layout.
- A separate `Screen::AddDialog` state: works but duplicates credentials and session-id into a
  third screen; using a dialog within `MainScreen` is simpler and consistent with the
  `confirming_delete` pattern already in `MainScreen`.

### `AddDialogState` enum in `MainScreen`

`MainScreen` gains an `add_dialog: AddDialogState` field:

```rust
pub enum AddDialogState {
    Hidden,
    AddLink {
        magnet: String,
        destination: String,
        error: Option<String>,
    },
    AddFile {
        metainfo_b64: String,  // Base64-encoded .torrent bytes
        files: Vec<TorrentFileInfo>,
        destination: String,
        error: Option<String>,
    },
}

pub struct TorrentFileInfo {
    pub path: String,
    pub size_bytes: u64,
}
```

When `AddDialogState` is not `Hidden`, the view renders the modal overlay; all three variants use
the same dialog widget, differing only in whether the file list is populated.

### Local `.torrent` parsing — `lava_torrent`

**Decision:** Use the `lava_torrent` crate to parse `.torrent` files locally inside
`Task::perform()`. This extracts the file list (name + size) without a round-trip to the daemon.

**Alternatives considered:**

- Add torrent as paused first, then query `torrent-get` for files: requires two RPCs and error
  recovery if the user cancels (must remove the torrent). Local parsing is simpler and faster.
- Manual bencode parsing: adds implementation cost for something a crate solves cleanly.

### File read + parse in one `Task::perform`

The `TorrentFileRead` task opens the file picker, reads the file bytes, Base64-encodes them, and
parses the torrent metadata — all in a single async closure. This avoids a chain of messages
and keeps the happy path to one round-trip before the dialog opens.

```
AddTorrentClicked
  └─ Task::perform → TorrentFileRead(Result<FileReadResult, String>)
        FileReadResult { metainfo_b64, files, destination_hint }
```

### Single `torrent_add` RPC function with `AddPayload` + `download_dir`

**Decision:** Expose one `torrent_add(url, creds, session_id, payload, download_dir)` function
in `rpc.rs`, where `AddPayload` is:

```rust
pub enum AddPayload {
    Magnet(String),   // sent as "filename"
    Metainfo(String), // Base64-encoded bytes, sent as "metainfo"
}
```

`download_dir` is `Option<String>`; an empty string from the dialog is treated as `None`
(daemon uses its configured default).

### New message variants

| Variant                                           | Trigger                           |
| ------------------------------------------------- | --------------------------------- |
| `AddTorrentClicked`                               | "Add Torrent" button pressed      |
| `TorrentFileRead(Result<FileReadResult, String>)` | File opened + parsed              |
| `AddLinkClicked`                                  | "Add Link" button pressed         |
| `AddDialogMagnetChanged(String)`                  | User types in magnet field        |
| `AddDialogDestinationChanged(String)`             | User edits destination field      |
| `AddConfirmed`                                    | "Add" button in dialog pressed    |
| `AddCancelled`                                    | "Cancel" button in dialog pressed |
| `AddCompleted(Result<(), String>)`                | `torrent_add` RPC resolves        |

## Risks / Trade-offs

- **`rfd` on macOS requires the app to run on the main thread for the dialog to appear.** iced's
  tokio integration spawns async tasks on a thread pool. Using `rfd::AsyncFileDialog` addresses this
  because rfd internally dispatches to the main thread on Apple platforms.
- **`lava_torrent` adds a compile-time dependency.** It pulls in a bencode parser and is
  well-maintained; the trade-off is acceptable for the feature value.
- **Base64 encoding + torrent parsing are synchronous inside `Task::perform()`.** Torrent files
  are typically small (<1 MB) so this is fine. `tokio::task::spawn_blocking` can wrap both if
  needed in future.
- **Transmission returns `"torrent-duplicate"` as the `result` field (not an error HTTP status)
  when the torrent is already present.** The current design surfaces this as a success; a future
  version could distinguish it with a softer "already added" message.
- **No file preview for magnet links.** The `AddLink` dialog renders an empty file list with a
  note explaining that metadata is unavailable until the torrent connects to peers.
