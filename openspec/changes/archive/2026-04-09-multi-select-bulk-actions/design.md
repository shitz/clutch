## Context

Clutch is a pure-Rust iced (Elm-architecture) desktop BitTorrent client that wraps the
Transmission JSON-RPC daemon. All mutation RPCs (`torrent-start`, `torrent-stop`,
`torrent-remove`, `torrent-set-location`, `torrent-set`) natively accept an `ids` array, so
the daemon side requires no changes. The work is entirely client-side.

Today `TorrentListScreen` holds `selected_id: Option<i64>` and every downstream consumer
(inspector, toolbar, context menu, delete dialog) is built around a single optional ID.
The goal is to upgrade to a `HashSet<i64>` selection model while keeping all existing
single-torrent behaviour intact and adding bulk paths for the new cases.

All file paths below are relative to `src/`.

## Goals / Non-Goals

**Goals:**

- Disjoint and range multi-select with standard desktop modifier semantics.
- Aggregate enable/disable logic for toolbar and context-menu actions.
- Bulk dispatch of start, stop, remove, and set-location RPC calls.
- Inspector "Bulk Options" mode with sparse-field dispatch (only touched fields sent).
- Adapting the delete confirmation dialog for the multi-torrent case.
- Cmd+A / Ctrl+A to select all visible (filtered) torrents.
- Keeping single-select behaviour identical to today.

**Non-Goals:**

- Indeterminate visual states (tri-state checkboxes) in the bulk options form.
- Bulk file-priority management (the Files tab stays single-torrent only).

## Decisions

### Decision 1 — Selection State Model

`TorrentListScreen` gains:

```rust
pub selected_ids: HashSet<i64>,
pub selection_anchor: Option<i64>,
pub modifiers: iced::keyboard::Modifiers,
```

`selected_id: Option<i64>` is removed entirely. All existing reads of `selected_id` migrate to
`selected_ids.iter().next().copied()` for single-selection views or to direct set operations for
new multi-select logic.

`selection_anchor` records the ID of the last row that a plain or Ctrl/Cmd-click landed on. It
is the fixed end of a Shift-click range.

**Alternative considered:** keeping `selected_id` as a "primary" alongside the set. Rejected
because two sources of truth invite drift bugs and neither toolbar nor inspector has a concept
of a "primary" torrent.

### Decision 2 — Modifier Tracking via Subscription

A new `modifiers_subscription()` method on `TorrentListScreen` returns an
`iced::keyboard::listen()` subscription that maps `Event::ModifiersChanged(m)` to a new
`Message::ModifiersChanged(iced::keyboard::Modifiers)` variant.

This subscription is always active on the main screen, similar to `cursor_subscription()`.
The modifiers state is stored so row-click handlers can read it synchronously inside `update()`.

**Alternative considered:** passing modifiers through the row-click message itself (e.g.
`TorrentSelected(i64, Modifiers)`). Rejected because iced's event model does not reliably
deliver mouse and keyboard events in a single batched payload.

### Decision 3 — Click Semantics in `update()`

Three cases inside the `TorrentSelected(id)` handler (renamed or kept; the view always emits
the same message):

| Modifier held | Behaviour                                                                     |
| ------------- | ----------------------------------------------------------------------------- |
| None          | Clear `selected_ids`, insert `id`, set `selection_anchor = Some(id)`.         |
| Ctrl or Cmd   | Toggle `id` in `selected_ids`; update `selection_anchor = Some(id)`.          |
| Shift         | Resolve anchor ID to its current index in `visible_torrents()`; compute range |
|               | to `id`'s current index; union that range into `selected_ids`.                |

The anchor is stored as an **ID** (`selection_anchor: Option<i64>`), not an index. The index is
resolved freshly from `visible_torrents()` at the moment of each Shift-click. This means live
poll updates that reorder the list between the anchor-click and the Shift-click are handled
correctly: the range is always computed over the current on-screen order, and the anchor row
simply shifts along with the data.

If `selection_anchor` holds an ID that is **no longer visible** (filtered out or removed by a
poll), the Shift-click falls back to plain-click behaviour: select only `id`, update anchor.

**Alternative considered:** emitting three separate message variants (`TorrentClicked`,
`TorrentCtrlClicked`, `TorrentShiftClicked`). Rejected to keep the view layer thin — modifier
state belongs in the update function.

### Decision 4 — `RpcWork` Variants Switch to `Vec<i64>`

The following `RpcWork` variants change their `id: i64` field to `ids: Vec<i64>`:

```rust
TorrentStart { params, ids: Vec<i64> }
TorrentStop  { params, ids: Vec<i64> }
TorrentRemove { params, ids: Vec<i64>, delete_local_data: bool }
SetLocation   { params, ids: Vec<i64>, location: String, move_data: bool }
TorrentSetBandwidth { params, ids: Vec<i64>, args: TorrentBandwidthArgs }
```

The corresponding `api.rs` async functions are updated to:

```rust
pub async fn torrent_start(url, credentials, session_id, ids: &[i64])
pub async fn torrent_stop(url, credentials, session_id, ids: &[i64])
// etc.
```

`SetFileWanted` continues using a single `torrent_id: i64` because bulk file-priority management
is out of scope.

All existing single-torrent call sites wrap the scalar in `vec![id]` during the changeover.
Net effect: callers and the worker are updated together in the same PR with no intermediate state.

### Decision 5 — Context-Menu Right-Click Selection Rules

`TorrentRightClicked(i64)` updates selection:

- If the right-clicked ID **is** in `selected_ids` → leave selection unchanged.
- If the right-clicked ID **is not** in `selected_ids` → clear set, select only that ID,
  reset `selection_anchor`.

Context-menu action messages (`ContextMenuStart`, `ContextMenuPause`, `ContextMenuDelete`,
`OpenSetLocation`) no longer carry an `i64`. They operate on the current `selected_ids` set,
which is already correct by the time the menu opens.

### Decision 6 — Inspector Bulk Edit Mode

`InspectorScreen` gains a `bulk_options: InspectorBulkOptionsState` field alongside the
existing `options: InspectorOptionsState`. `InspectorBulkOptionsState` mirrors
`InspectorOptionsState` but all primitive values are `Option<_>` (all start as `None`):

```rust
pub struct InspectorBulkOptionsState {
    pub download_limited: Option<bool>,
    pub download_limit_val: String,   // empty string = unset
    pub upload_limited: Option<bool>,
    pub upload_limit_val: String,
    pub ratio_mode: Option<u8>,
    pub ratio_limit_val: String,
    pub honors_session_limits: Option<bool>,
}
```

**Checkbox two-state interaction pattern (no tri-state):** Standard iced checkboxes render
`true` or `false`, so there is no visual representation for `None` ("untouched"). The view
layer MUST bind boolean controls to `state.bulk_options.download_limited.unwrap_or(false)` (and
likewise for the other `Option<bool>` fields). The `Message::BulkDownloadLimitToggled(v)`
handler MUST wrap the incoming value in `Some(v)` unconditionally — this is the moment a field
becomes "touched". As a result, explicitly setting a field to `false` requires the user to
check the box (→ `Some(true)`) and then uncheck it (→ `Some(false)`). This double-click
pattern is the standard acceptable trade-off when avoiding tri-state UI components.

`main_screen.rs` determines mode at the point of inspector rendering:

- `selected_ids.len() == 0` → inspector hidden (current behaviour).
- `selected_ids.len() == 1` → pass the single `TorrentData` ref; single-torrent mode.
- `selected_ids.len() > 1` → bulk mode; inspector renders only the Options tab using
  `bulk_options`.

The `inspector::view` signature changes from `(state, torrent: &TorrentData)` to
`(state, torrent: Option<&TorrentData>, selected_count: usize)`. When `selected_count > 1`
and `torrent` is `None` the view renders the bulk form.

When bulk Options messages are dispatched (`main_screen.rs` intercept), RPC is sent with
`ids: selected_ids.iter().copied().collect()`.

### Decision 7 — Cmd+A / Ctrl+A Select All

A new `Message::KeyboardSelectAll` variant is added to `torrent_list::Message`. The existing
`modifiers_subscription()` is extended (or the cursor subscription reused) to also map a
`KeyPressed { key: Key::Named(Named::A), modifiers }` event to `Message::KeyboardSelectAll`
when `modifiers.command()` (macOS Cmd) or `modifiers.control()` (Linux/Windows Ctrl) is held.

The handler in `update.rs` populates `selected_ids` with the IDs of all torrents currently in
`visible_torrents()`. `selection_anchor` is set to the ID of the first visible torrent (or
cleared if the list is empty). The selection respects the active filter — only visible torrents
are selected.

**Why not a separate key subscription?** The cursor subscription already uses
`iced::event::listen_with` which intercepts all window events. Piggy-backing on the existing
listener avoids a second global event tap.

### Decision 8 — Delete Confirmation Dialog State

`confirming_delete: Option<(i64, bool)>` changes to `Option<(Vec<i64>, bool)>`.

The dialog view function receives the list of IDs:

- `len() == 1` → current wording: `"Delete \"<name>\"?"`.
- `len() > 1` → `"Remove N selected torrents?"`.

The name lookup for the single-torrent case reads `self.torrents.iter().find(|t| t.id == ids[0])`.

### Decision 9 — `selected_torrent()` Helper

The existing `selected_torrent() -> Option<&TorrentData>` on `TorrentListScreen` is retained
but returns `None` when `selected_ids.len() != 1`. This makes the single-torrent path
backward-compatible everywhere in `main_screen.rs` without hunting down every usage.

## Risks / Trade-offs

- **Shift-click on a live-updating list can shift anchor position**: Because Clutch polls the
  daemon every few seconds and can reorder rows (e.g. sort by Download Speed), the index of the
  anchor torrent may change between the anchor click and the Shift-click. Mitigation: store the
  anchor as an ID (not an index) and resolve it to its current index in `visible_torrents()` at
  Shift-click time. If the anchor ID has been filtered out or removed, fall back to plain-click.
  The `visible_torrents()` helper (pure function of sort and filter state) is called from both
  the view and the selection handler, ensuring both always see the same row order.

- **Accidental bulk edits**: A user with 50 torrents selected could accidentally apply a global
  limit. Mitigation: the bulk options form starts blank (`None` everywhere); only fields the user
  explicitly touches generate RPC payloads.

- **Modifier tracking race conditions on macOS**: `ModifiersChanged` events may occasionally
  arrive after a click event due to OS batching. In practice the modifier state is "sticky"
  (held keys persist across events), making this benign — the worst case is a single click
  behaving as a plain click.

- **Inspector view signature is a breaking change for callers**: Only one call site
  (`main_screen.rs`) calls `inspector::view`, so the impact is contained.

## Migration Plan

This change is entirely client-side and stateless (no persisted data involved). No migration is
required. Rollback is a simple revert.

## Open Questions

None — all decisions are resolved by `bulk.md`, codebase analysis, and the edge-case review.

## Post-Implementation Iterations

After the initial implementation the following additional issues were discovered and resolved.

### Iteration 1 — ETA for completed / seeding torrents

**Problem:** The torrent list showed a large positive ETA (e.g. "41776h 6m") for torrents that
were 100% downloaded and seeding. The Transmission daemon reports a seeding-goal ETA even after
download completes, which Clutch displayed verbatim.

**Fix:** In `src/screens/torrent_list/view.rs` the ETA cell now checks `t.percent_done >= 1.0`
and renders `"—"` unconditionally — `format_eta` is never called for a complete torrent.

### Iteration 2 — Inspector tab bar in bulk mode

**Problem:** The "Bulk Edit" inspector hid the segmented tab control completely and replaced it
with a plain header label `"Bulk Edit — Options only"`. Users found this disorienting because the
tab bar is a navigation landmark.

**Fix:** The full 5-tab bar is always rendered. When `selected_count > 1`, `active_tab` is forced
to `Options` and clicks on any other tab are absorbed (mapped back to `TabSelected(Options)`).
A subtitle line `"Editing options for multiple selected torrents"` appears below the tabs to clarify
the constraint. The old `view_bulk()` function was removed.

### Iteration 3 — Clicking empty space below the last row

**Problem:** Clicking in the vertical gap below the last torrent row did nothing. A `Space` with
`Length::Fill` inside a `scrollable` resolves to 0 px because scrollables give content unbounded
virtual height, so the spacer had zero click area.

**Fix:** Replaced the column+space-inside-scrollable approach with a `stack`: a full-height
`mouse_area` (bottom layer) fires `TorrentSelected(last_visible_id)` for empty-space clicks; the
`scrollable` with the rows (top layer) captures row clicks first. Events fall through the
scrollable to the background only where no interactive child is present.

### Iteration 4 — Selection not pruned when filter changes

**Problem:** Toggling a filter chip did not remove filtered-out torrents from `selected_ids`.
With two torrents selected, filtering one out left `selected_ids.len() == 2`, causing the
inspector to show bulk-edit mode even though only one torrent was visible.

**Fix:** Added `prune_selection_to_visible()` to `TorrentListScreen` in `mod.rs`. It intersects
`selected_ids` with the IDs returned by `visible_torrents()` and clears `selection_anchor` if
the anchor is no longer visible. The method is called after every `FilterToggled`,
`FilterAllClicked`, and `TorrentsUpdated(Ok)` message.

### Iteration 5 — Tests for multi-select and filter interaction

Eight new unit tests were added to `src/screens/torrent_list/mod.rs`:

| Test                                                               | What it covers                                                     |
| ------------------------------------------------------------------ | ------------------------------------------------------------------ |
| `cmd_click_creates_multi_select_and_selected_torrent_returns_none` | Cmd-click adds second torrent; `selected_torrent()` returns `None` |
| `cmd_click_deselects_already_selected_torrent`                     | Toggle-off for Cmd-click                                           |
| `filter_toggle_prunes_hidden_torrent_from_selection`               | Filter change removes hidden ID from set                           |
| `filter_all_clicked_prunes_selection_to_visible`                   | `FilterAllClicked` reconciles selection                            |
| `torrents_updated_removes_deleted_torrent_from_selection`          | Daemon removal clears selection                                    |
| `prune_selection_to_visible_removes_filtered_ids`                  | Direct unit test of the helper                                     |
| `shift_click_range_respects_filter`                                | Shift range contains only visible (non-filtered) rows              |
| `keyboard_select_all_only_selects_visible`                         | Cmd+A honours the active filter                                    |

### Iteration 6 — Enter key in Set Data Location dialog

**Problem:** Pressing Enter while the path field in the Set Data Location dialog was focused did
nothing — the user had to click the "Apply" button.

**Fix:** Added `.on_submit(Message::SetLocationApply)` to the `text_input` widget in
`view_set_location_dialog` in `src/screens/torrent_list/dialogs.rs`.
