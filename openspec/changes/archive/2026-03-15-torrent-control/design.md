## Context

Clutch v0.1 ships a working read-only torrent list. The `MainScreen` already renders toolbar buttons
(Pause, Resume, Delete) but they are permanently disabled. The RPC layer (`rpc.rs`) has no action
functions yet. This design wires the three action verbs together with selection state.

Current constraints (from system_architecture.md) that must hold:

- `update()` must return in microseconds — all RPC calls go through `Task::perform()`.
- A new poll is not started while one is in-flight (`is_loading` guard already present).

## Goals / Non-Goals

**Goals:**

- Single-torrent selection via row click; clicking again deselects.
- Pause (torrent-stop), Resume (torrent-start), Delete (torrent-remove) against the selected torrent.
- Toolbar buttons contextually enabled/disabled based on selection and torrent status.
- Immediate list refresh after a successful action.
- Inline error display on action failure.
- Confirmation step before delete, with a "delete local data" checkbox.

**Non-Goals:**

- Multi-torrent selection (deferred to a later version).
- Keyboard shortcuts for actions.
- Undo / undo-after-delete.

## Decisions

### 1. Selection state as `Option<i64>` on `MainScreen`

Store `selected_id: Option<i64>` on `MainScreen`. A new `TorrentSelected(i64)` message toggles
selection (set if different, clear if same). This is the simplest model and aligns with the
architecture doc. An alternative — storing the full `TorrentData` — would require keeping a stale
copy in sync; using just the ID avoids that.

### 2. Enable/disable logic derived from live torrent data

Button states are computed in `view()` from the current `torrents` list and `selected_id`, rather
than stored as flags. This is always consistent with the latest poll result without extra bookkeeping.

- **Pause**: selected torrent status ∈ {3 (QueueDL), 4 (DL), 5 (QueueSeed), 6 (Seeding)}.
- **Resume**: selected torrent status = 0 (Stopped) or 2 (Checking — treat as re-queueable).
- **Delete**: any selected torrent (always enabled when selection is non-empty).

### 3. Action → immediate refresh pattern

`ActionCompleted(Ok(_))` clears `is_loading` and fires another `torrent-get` immediately (via
`Task::perform`) rather than waiting for the next 5 s tick. This gives fast feedback without
changing the polling subscription. Alternative — polling at 1 s while an action is recent — adds
state complexity without meaningful benefit.

### 4. Inline confirmation row for delete

When the Delete toolbar button is clicked, the app does **not** fire an RPC immediately. Instead, a
confirmation row appears below the toolbar with:

- The name of the torrent to be deleted.
- A "Delete local data" checkbox (default off).
- **Confirm Delete** and **Cancel** buttons.

This is modelled as `confirming_delete: Option<(i64, bool)>` on `MainScreen` — `None` when idle,
`Some((id, delete_local_data))` when awaiting confirmation. The `bool` in the tuple tracks the
checkbox state.

New messages: `DeleteClicked` (no payload — reads `selected_id`), `DeleteLocalDataToggled(bool)`,
`DeleteConfirmed`, `DeleteCancelled`. The earlier `DeleteClicked(bool)` variant is superseded.

Alternative considered: a floating modal overlay. Rejected because iced 0.14 has no built-in modal
widget and a custom overlay adds complexity disproportionate to the benefit at this stage.

### 5. Three new RPC functions following the existing pattern

`torrent_start(url, creds, session_id, id)`, `torrent_stop(...)`, `torrent_remove(..., delete_local_data)`
all call `post_rpc` with the appropriate method and a `{"ids": [id]}` argument body. No new crates
are required. Session rotation surfaces as `RpcError::SessionRotated` identically to `torrent_get`.

## Risks / Trade-offs

- **Race between action and next poll**: If the poll tick fires immediately after an action
  `Task::perform`, `is_loading` blocks the tick, so only one RPC is in-flight. The extra poll
  started by `ActionCompleted` can still race with a background tick, but the `is_loading` guard
  prevents any overlap. → No mitigation needed beyond the existing guard.

- **Status codes are undocumented integers**: Pause/Resume enable logic depends on `status` integer
  values from the Transmission RPC spec. They are stable across Transmission versions 2.x–4.x but
  not formally versioned. → Document the values in code comments; revisit if a version mismatch
  is ever reported.

- **Confirmation row occupies toolbar space**: The inline confirmation row replaces normal toolbar
  content while visible, which means Pause/Resume/Add are temporarily hidden. → Acceptable trade-off
  for simplicity; the row is ephemeral and dismissed on Confirm or Cancel.
