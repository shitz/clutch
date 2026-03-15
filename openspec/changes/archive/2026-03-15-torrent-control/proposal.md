## Why

The torrent list is read-only today: users can see their torrents but cannot act on them. Adding pause, resume, and delete controls turns Clutch into a functional remote client rather than a read-only dashboard.

## What Changes

- Clicking a torrent row selects it and enables the relevant toolbar action buttons (Pause, Resume, Delete).
- A selected active torrent can be paused via `torrent-stop` RPC.
- A selected stopped torrent can be resumed via `torrent-start` RPC.
- A selected torrent can be removed via `torrent-remove` RPC, with an optional "delete local data" flag.
- Action buttons fire optimistically and trigger an immediate list refresh on completion.
- Toolbar buttons remain disabled when no torrent is selected or when the action is not applicable to the selected torrent's state.

## Capabilities

### New Capabilities

- `torrent-actions`: Pause, resume, and delete a selected torrent via RPC toolbar buttons.

### Modified Capabilities

- `torrent-list`: Row selection state is added — clicking a row highlights it and feeds into action button enable/disable logic.

## Impact

- `src/screens/main_screen.rs`: adds `selected_id: Option<i64>`, new `Message` variants (`PauseClicked`, `ResumeClicked`, `DeleteClicked(bool)`, `ActionCompleted`), updated `update()` and `view()`.
- `src/rpc.rs`: adds `torrent_start`, `torrent_stop`, `torrent_remove` async functions.
- `src/app.rs`: new `Message` variants forwarded to the main screen.
- No new crate dependencies required.
