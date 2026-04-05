## Context

Clutch currently downloads every file in a torrent without exception. Transmission already
supports selective downloading via the `files-wanted` / `files-unwanted` fields in
`torrent-add` and `torrent-set`; clutch just does not expose this to the user.

There are two interaction points:

1. **Add-torrent dialog** — before the torrent has been added, the user picks which files to
   schedule. Unchecked files are passed as `files-unwanted` in the `torrent-add` call so the
   daemon never starts downloading them.
2. **Inspector Files tab** — after the torrent has been added, the user can toggle individual
   files on or off. Transmission exposes the current wanted state in the `fileStats[].wanted`
   field returned by `torrent-get`, and accepts updates via `torrent-set`.

The Elm architecture means all state must live in plain data structures; iced's `checkbox`
widget handles rendering with no additional dependencies.

## Goals / Non-Goals

**Goals:**

- Add per-file checkboxes to `AddDialogState::AddFile` (all checked by default).
- Provide **Select All** / **Deselect All** bulk actions in both the add-torrent dialog and
  the inspector Files tab.
- Pass unchecked file indices as `files-unwanted` in `torrent-add`.
- Display per-file `wanted` checkboxes in the inspector Files tab. Checkbox state updates
  immediately on click (optimistic); reverts to daemon state on the next successful poll.
- Toggle wanted state via a new `torrent-set` RPC call dispatched through the existing RPC
  worker queue.

**Non-Goals:**

- File priority levels (high/normal/low) — wanted/unwanted is sufficient for now.
- Magnet-link mode file selection (file metadata is unavailable before the torrent is
  fully discovered by the daemon).
- Persisting selection state across app restarts beyond what the daemon already tracks.

## Decisions

### D1 — Selection state as `Vec<bool>` indexed by file position

`Vec<bool>` is chosen over `HashSet<usize>` because:

- Transmission expects zero-based file index arrays; a `Vec<bool>` maps directly without
  an extra conversion step.
- Access is O(1) by index, matching the iteration pattern in `view_add_dialog`.
- Memory footprint is trivial (one byte per file).

The vector is initialised to `vec![true; files.len()]` when the file-read result arrives,
so every file is wanted by default.

### D2 — Inspector uses optimistic local state, cleared per-index on RPC success

The inspector maintains a `pending_wanted: HashMap<usize, bool>` (file index → desired
wanted state) inside `InspectorScreen`. When the user clicks a checkbox:

1. The index is inserted into `pending_wanted` with the new value.
2. A `torrent-set` RPC call is enqueued, carrying the affected indices as metadata.
3. The view renders `pending_wanted.get(&i).copied().unwrap_or(file_stats[i].wanted)` for
   each row, so the checkbox flips immediately — no poll lag regardless of poll interval.

When the RPC call **succeeds**, a `FileWantedSetSuccess { indices: Vec<usize> }` message
is dispatched and only the specific indices that were just confirmed are removed from
`pending_wanted`. The view then falls through to the daemon-reported value for those
indices on the very next render.

This avoids the race condition where a background `torrent-get` poll completes _after_ the
user clicks but _before_ the `torrent-set` is processed: the pending override stays in
place until the mutation itself is confirmed, not until an arbitrary read-poll arrives.

If `torrent-set` fails, the pending override is also removed (the indices are cleared in
both success and failure paths) so the UI reverts to the last daemon-reported state — the
correct recovery behaviour requiring no special error handling.

The `pending_wanted` map is scoped to `InspectorScreen`; it exists only while the
inspector is mounted.

### D3 — `torrent-set` routed through the existing RPC worker queue

All RPC calls already flow through the `mpsc` worker in `src/rpc/worker.rs`. A new
`RpcWork::SetFileWanted { params, torrent_id, indices, wanted }` variant follows the
same pattern as `TorrentStart` / `TorrentStop`. This gives session-rotation retry for
free and ensures at-most-one-in-flight serialisation.

The Transmission RPC spec expects `files-wanted` / `files-unwanted` as **arrays of
zero-based integer indices**, e.g. `[0, 2, 5]`. The worker carries the same `Vec<i64>`
back through a new `RpcResult::FileWantedSet(Result<(), RpcError>, Vec<usize>)` so the
app can dispatch `FileWantedSetSuccess` (or failure) with the exact indices involved.

For the bulk "Select All" / "Deselect All" action, the message handler MUST compile a
single `RpcWork::SetFileWanted` with the full index range `[0..n]` rather than enqueuing
one item per file. This keeps the queue depth at one regardless of torrent size.

### D4 — Inspector dispatches messages via `app::Message` wrapping

`InspectorScreen::Message` gains:

- `FileWantedToggled { torrent_id: i64, file_index: usize, wanted: bool }`
- `AllFilesWantedToggled { torrent_id: i64, wanted: bool }`
- `FileWantedSetSuccess { indices: Vec<usize> }` (emitted on RPC success **or** failure —
  clears the pending override in both cases)

`app.rs` maps the two toggle variants to `RpcWork::SetFileWanted` enqueues. When
`RpcResult::FileWantedSet(_, indices)` is received from the worker, `app.rs` dispatches
`inspector::Message::FileWantedSetSuccess { indices }` to the inspector. No architectural
changes beyond adding this result arm are needed.

### D5 — Tri-state "Select All" checkbox via `CheckState` + `m3_tristate_checkbox`

Iced 0.14's built-in `checkbox` widget only models `bool`. A tri-state header checkbox
(showing _indeterminate_ when some but not all files are wanted) requires a custom
component. The implementation follows the Material Icons codepoint convention:

| State     | Icon codepoint                         | Constant            |
| --------- | -------------------------------------- | ------------------- |
| Checked   | `\u{e834}` (`check_box`)               | `ICON_CB_CHECKED`   |
| Unchecked | `\u{e835}` (`check_box_outline_blank`) | `ICON_CB_UNCHECKED` |
| Mixed     | `\u{e909}` (`indeterminate_check_box`) | `ICON_CB_MIXED`     |

A `CheckState` enum (`Checked / Unchecked / Mixed`) lives in `src/theme.rs` alongside a
`m3_tristate_checkbox(state, label, on_toggle)` helper that renders a borderless button
composed of the icon + a `text` label. Clicking a `Mixed` checkbox produces
`CheckState::Checked` (select all); clicking `Checked` produces `Unchecked`; clicking
`Unchecked` produces `Checked`.

The aggregate state is derived at render time:

- All wanted (after pending overrides) → `Checked`
- None wanted → `Unchecked`
- Otherwise → `Mixed`

The same `m3_tristate_checkbox` helper is used for the "Select All" row in both the
add-torrent dialog and the inspector Files tab.

## Risks / Trade-offs

- **Per-index clearing on both success and failure** — `FileWantedSetSuccess` is emitted
  regardless of whether the RPC succeeded or errored. On failure the UI reverts to the
  last daemon-reported state (correct behaviour). There is no ambiguity because
  `TorrentFileStats::wanted` always reflects the daemon's ground truth.
- **Concurrent clicks before first RPC returns** — If the user toggles the same file
  twice quickly, two `SetFileWanted` items are queued. The second enqueue overwrites the
  `pending_wanted` entry; both RPCs fire in order; the final daemon state will be
  consistent. The intermediate flicker is cosmetic and bounded by the queue drain time.
- **`files-unwanted` on magnet links** — We do not support file selection for magnet links
  because the file list is unknown at add-time. Mitigation: this is explicit in Non-Goals
  and the existing "File list unavailable for magnet links" notice already sets expectations.
- **fileStats length mismatch** — If `torrent-get` returns a `fileStats` vector shorter
  than `files`, index-based access could panic or silently skip files. Mitigation: all
  accesses use `.get(i)` with a safe fallback (already the pattern in `view_files`);
  `wanted` defaults to `true` when the entry is absent.

## Migration Plan

No persistent state changes. The `wanted` field added to `TorrentFileStats` uses
`#[serde(default)]` so existing cached responses (none — the app does not persist RPC
responses) and test fixtures deserialise cleanly. No daemon configuration changes required.
