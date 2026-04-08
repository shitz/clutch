## Context

Clutch is a pure-Rust iced desktop client for the Transmission BitTorrent daemon. The UI follows
the Elm architecture: `update()` is strictly non-blocking, all RPC calls are serialized through a
single `mpsc` worker, and all visual styling is centralized in `src/theme.rs`.

Vanilla iced 0.14 provides no native OS context-menu integration and no built-in floating-menu
widget. The existing torrent list renders rows in a `scrollable` column; rows are selectable via
`mouse_area` on left-press. There is currently no right-click affordance anywhere in the UI.

The `torrent-set-location` Transmission RPC is the standard mechanism for data relocation; it
accepts an `ids` array, a `location` path string, and a boolean `move` flag indicating whether the
daemon should physically move files or only update its internal path record.

## Goals / Non-Goals

**Goals:**

- Track the global mouse cursor position to anchor the context menu at the pointer.
- Wrap torrent rows in a right-click listener (`on_right_press`) that opens the menu.
- Render a floating Material 3 context menu overlay (Start, Pause, Delete, Set Data Location).
  Start and Pause are always shown; whichever is inapplicable to the torrent's current state is
  rendered inactive using the same visual treatment applied to disabled actions elsewhere.
- Clicking anywhere outside the open menu dismisses it.
- Delete re-uses the existing confirmation dialog (identical to toolbar delete).
- "Set Data Location" opens a centered modal with a path input prefilled from the torrent's
  current `downloadDir` and a "Move data to new location" checkbox (default: checked).
- Dispatch `torrent-set-location` through the existing mpsc worker queue.

**Non-Goals:**

- Native OS context-menu integration (not supported by iced).
- Multi-torrent context actions (menu applies to the single right-clicked torrent only).
- Dynamic quadrant detection for boundary-aware menu positioning (basic bottom-edge mitigation
  is sufficient for this iteration).
- Long-press / touch-screen equivalents.

## Decisions

### Decision 1: Global cursor tracking via subscription

To render the menu precisely at the click point, `TorrentListScreen` must know the cursor
coordinates at the moment of right-click.

A subscription using `iced::event::listen_with` filters `Mouse::CursorMoved` events and emits
`Message::CursorMoved(Point)`. The handler stores the value in a new
`last_cursor_position: iced::Point` field and returns `Task::none()` — no layout recalculation is
triggered.

**Alternative considered**: Reading coordinates directly inside the `on_right_press` closure.
Rejected because `mouse_area::on_right_press` does not expose the cursor position in its callback;
the subscription approach is the only idiomatic iced mechanism.

### Decision 2: Right-click detection via `mouse_area`

Each torrent row is wrapped in `iced::widget::mouse_area`:

- `.on_press(Message::SelectTorrent(id))` — unchanged left-click selection.
- `.on_right_press(Message::TorrentRightClicked(id))` — captures the right-click.

When `TorrentRightClicked` fires, `last_cursor_position` is copied into a new state field:
`context_menu: Option<(i64, iced::Point)>`.

### Decision 3: Floating overlay via `stack`

The list view content is wrapped in `iced::widget::stack`. When `context_menu` is `Some`:

1. **Layer 0** — the normal scrollable torrent list.
2. **Layer 1** — a transparent `mouse_area` spanning `Length::Fill × Length::Fill`. Its
   `.on_press` emits `Message::DismissContextMenu`, acting as a click-away catcher.
3. **Layer 2** — the menu card: a tight `column` of action buttons in an M3 `m3_card` container,
   positioned by applying `.padding([point.y, 0.0, 0.0, point.x])` to an outer `container` with
   `Length::Fill` dimensions.

**Bottom-edge mitigation**: if `point.y` exceeds `window_height - 150.0`, the menu is drawn
`150 px` above the cursor instead of below.

**Alternative considered**: iced's `overlay` API. Rejected because it requires implementing the
full `Overlay` trait and does not compose cleanly with `scrollable`. The `stack` approach is
straightforward and fully supported.

### Decision 4: Start/Pause inactive state

Both Start and Pause appear in every context menu. The one inapplicable to the torrent's current
Transmission status code is rendered using the same inactive/disabled visual style used elsewhere
in the application (i.e., the button exists in the widget tree but has no `.on_press` handler
attached), which iced automatically renders as visually disabled.

**Alternative considered**: Hiding the inapplicable action entirely. Rejected in favour of the
user's requirement to always show both with a clear inactive indicator.

### Decision 5: "Set Data Location" modal dialog state

A new `SetLocationDialog` struct (or variant in the existing `AddDialogState` enum) is introduced:

```rust
struct SetLocationDialog {
    torrent_id: i64,
    path: String,         // initialised from torrent's downloadDir
    move_data: bool,      // default: true
}
```

The modal renders identically to the Add Torrent dialog pattern — a centered `m3_card` overlay
on top of the full screen — with:

- An `m3_text_input` for the destination path.
- A checkbox: "Move data to new location".
- "Cancel" and "Apply" `m3_tonal_button` / `m3_primary_button` pair.

### Decision 6: `torrent-set-location` RPC

A new function `torrent_set_location(id: i64, location: &str, move_data: bool)` is added to
`src/rpc/api.rs`. A matching `RpcWork::SetLocation { torrent_id, location, move_data }` variant is
added to the worker's enum. The JSON payload matches the Transmission spec:

```json
{
  "method": "torrent-set-location",
  "arguments": { "ids": [torrent_id], "location": "/new/path", "move": true }
}
```

Asynchronous errors (e.g., the daemon failing to move files on disk) are surfaced on the next
`torrent-get` poll via the torrent's `errorString` field — no additional error-handling machinery
is required. Synchronous errors (malformed response, network failure) are caught by the existing
error banner.

## Risks / Trade-offs

**Cursor tracking overhead** → The `CursorMoved` handler does nothing but store a `Point` and
return `Task::none()`. iced will not diff or re-render the layout for this message. Overhead is
negligible.

**Menu clipping at bottom/right edge** → A fixed 150 px upward offset is applied when the cursor
is within 150 px of the window's bottom; no horizontal mitigation is applied (rare edge case for
this iteration).

**`stack` Z-ordering with `scrollable`** → The transparent click-away `mouse_area` on layer 1
intercepts all pointer events while the menu is open, including scroll wheel events. This means
scrolling is blocked while the menu is visible; acceptable UX given that the menu is ephemeral.

**Pre-fill of path from `downloadDir`** → `downloadDir` is already fetched in the regular
`torrent-get` poll and stored in the `Torrent` model. No additional RPC is needed to populate the
text field.

## Open Questions

None — all design decisions have been resolved.
