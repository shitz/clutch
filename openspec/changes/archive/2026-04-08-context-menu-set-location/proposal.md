## Why

Power users expect right-click context menus for rapid torrent actions without reaching for the
toolbar, and Clutch currently lacks any way to move a torrent's downloaded data on the daemon's
filesystem — a critical workflow for NAS and seedbox users who reorganize storage regularly.

## What Changes

- Each torrent row gains a right-click listener that opens a floating Material 3 context menu
  anchored to the cursor position.
- The context menu exposes four actions: **Start**, **Pause**, **Delete**, and
  **Set Data Location**. Start and Pause are always visible; whichever action is not applicable to
  the current torrent state is rendered in the disabled/inactive visual style used throughout the
  application.
- **Delete** re-uses the existing confirmation dialog (same behaviour as the toolbar button).
- **Set Data Location** opens a new centered modal dialog with a path input (prefilled with the
  torrent's current download directory) and a "Move data to new location" checkbox (default: on),
  then dispatches a `torrent-set-location` RPC call.
- The global mouse cursor position is tracked via an `iced::event::listen_with` subscription so
  the menu can be anchored precisely under the pointer.
- Clicking anywhere outside the open menu dismisses it.
- A new `torrent-set-location` RPC operation is added to the worker queue and API layer.

## Capabilities

### New Capabilities

- `context-menu`: Floating right-click context menu overlay for torrent rows — cursor tracking
  subscription, right-click detection via `mouse_area`, Z-ordered `stack` rendering, click-away
  dismissal, and conditional enable/disable of Start/Pause actions.
- `set-data-location`: "Set Data Location" modal dialog (path input prefilled from current
  download dir, move-data checkbox) and the backing `torrent-set-location` RPC integration.

### Modified Capabilities

- `torrent-list`: Rows now respond to right-click (`on_right_press`) in addition to left-click
  selection; the list view wraps its content in a `stack` to host the context-menu overlay.
- `rpc-client`: New `torrent-set-location` operation added to the `RpcWork` enum and the API
  function surface.

## Impact

- `src/screens/torrent_list/view.rs` — wrap rows in `mouse_area`, add `stack` overlay for menu.
- `src/screens/torrent_list/update.rs` — handle `CursorMoved`, `TorrentRightClicked`,
  `DismissContextMenu`, `ContextMenuAction`, and `SetLocationDialog*` messages.
- `src/screens/torrent_list/worker.rs` — new `RpcWork::SetLocation` variant dispatched to the
  worker.
- `src/rpc/api.rs` — `torrent_set_location(id, path, move_data)` function.
- `src/rpc/worker.rs` — match arm for the new `SetLocation` variant.
- `src/rpc/models.rs` — request struct for `torrent-set-location` arguments.
- `src/app.rs` — route new subscription for cursor tracking.
- No new external dependencies required (iced primitives only).
