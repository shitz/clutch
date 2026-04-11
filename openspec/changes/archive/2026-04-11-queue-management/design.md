## Context

Transmission exposes a native queue system: torrents have a `queuePosition` integer, and four
bulk RPCs (`queue-move-top`, `queue-move-up`, `queue-move-down`, `queue-move-bottom`) reorder
them. Separately, `session-get`/`session-set` expose four fields that cap how many transfers
run concurrently: `download-queue-{enabled,size}` and `seed-queue-{enabled,size}`.

Clutch already has a well-defined RPC pipeline: all mutations flow through `RpcWork` variants
dispatched via the `mpsc` worker queue, keeping `update()` non-blocking. Session configuration
is already fetched and saved through `SessionData` / `SessionSetArgs`. The settings Connections
tab already has a "Bandwidth" card using `m3_card` + toggle + text-input controls. The torrent
list already supports sortable columns via `SortColumn`, `sort_torrents`, and
`view_column_header`. The right-click context menu exists in `dialogs.rs::view_context_menu_overlay`.

## Goals / Non-Goals

**Goals:**

- Expose daemon queue-limit settings (download and seed queue enable/size) in a new
  "Queueing" card on the Connections settings tab.
- Allow users to move selected torrents up/down/top/bottom in the queue via context menu.
- All mutations go through the existing `RpcWork`/worker pipeline.

**Non-Goals:**

- Drag-and-drop queue reordering in the list (mouse-based reordering is a separate concern).
- Per-torrent queue priority (separate from global queue position).
- Displaying a queue progress indicator or ETA based on position.
- Any changes to the authentication or crypto layers.

## Decisions

### 1. Four dedicated `RpcWork` variants for queue movement

Each queue-move direction gets its own `RpcWork` variant (e.g. `QueueMoveTop { params, ids }`)
mirroring the existing `TorrentStart` / `TorrentStop` pattern. The four RPCs share the same
response shape (success/error), so their `RpcResult` variant can be a single
`QueueMoved(Vec<i64>)` that carries back the affected IDs for potential UI refresh.

**Alternative considered:** A single `QueueMove { direction: QueueDir, params, ids }` variant.
Rejected because the match arms in `execute_work` would still need four branches, and the
single-variant approach adds an extra enum type without meaningful savings.

### 2. Extend `SessionData` and `SessionSetArgs` in-place

The existing `parse_session_data` already extracts fields from the `session-get` response.
Adding four new fields (`download_queue_enabled`, `download_queue_size`, `seed_queue_enabled`,
`seed_queue_size`) follows the same pattern used for `alt_speed_*`. No new structs or API
functions are needed.

### 3. Queueing card mirrors Bandwidth card layout

The new "Queueing" card sits directly below the "Bandwidth" card in the Connections tab column.
It uses the same `m3_card` container, 16 px heading, toggle + text-input rows, and
`SettingsDraft` field pattern so the existing dirty-flag and save/discard guard logic continues
to work without modification.

### 5. Context menu queue group is unconditional

The four queue-movement items are shown for any right-click selection, not gated on the
torrent's current status. This avoids complicated enabled/disabled logic; the daemon silently
ignores moves for already-stopped or errored torrents.

## Risks / Trade-offs

- **Stale queue positions after a move** → After a `queue-move-*` RPC, the list reflects the
  old positions until the next poll cycle. Mitigation: trigger a `TorrentGet` refresh immediately
  after a successful queue-move (same pattern used by `SetLocation`).

- **Bulk moves for large selections may reorder unexpectedly** → Transmission applies bulk
  moves atomically per the spec, but the relative ordering among the selected items after
  `queue-move-up/down` depends on their current positions. This is consistent with how other
  clients (e.g. Transmission-Qt) behave; no mitigation needed beyond documenting it.

- **Settings round-trip for queue size** → Queue size is a free-entry integer text field with no
  client-side minimum enforcement. Invalid or zero values are rejected by Transmission with an
  error response. Mitigation: display a toast or inline error if `session-set` returns an error,
  reusing the existing error-display path.

## Migration Plan

No data migration is required. All changes are additive:

- New `RpcWork` variants are added; existing variants are untouched.
- New `SessionData` fields default to safe values (`false` / `0`) if absent from the response.

Rollback: reverting the code restores the previous behaviour without any persistent side
effects since queue positions are owned entirely by the Transmission daemon.

## Open Questions

_None at this time. The scope is well-bounded by the existing Transmission RPC specification
and Clutch's established patterns._
