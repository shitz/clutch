## Why

Power users with large libraries rely on queues to prevent disk I/O thrashing and network
saturation. While Transmission manages queue limits natively, Clutch does not yet expose the
ability to configure those limits or manually reorder the execution queue — leaving users with no
visibility into or control over which transfers run next.

## What Changes

- **RPC Models:** `SessionData` and `SessionSetArgs` gain `download-queue-enabled`,
  `download-queue-size`, `seed-queue-enabled`, and `seed-queue-size`.
- **RPC Endpoints:** Four new array-based queue mutation calls: `queue-move-top`,
  `queue-move-up`, `queue-move-down`, and `queue-move-bottom`.
- **Settings Screen (Connections Tab):** A new "Queueing" card is appended below the existing
  "Bandwidth" card, containing toggles and numeric inputs for download and seed queue limits,
  visually mirroring the bandwidth controls.
- **Context Menu:** A new action group — "Move to Top", "Move Up", "Move Down",
  "Move to Bottom" — is added to the right-click menu and operates on the full
  `selected_ids` set via bulk RPC calls.

## Capabilities

### New Capabilities

- `queue-management`: Shifting the priority of pending/downloading torrents up or down the
  global execution stack via bulk context-menu actions.
- `queue-configuration`: Reading and writing the daemon's active parallel-transfer limits
  (download queue size, seed queue size, and their enabled flags) from the Connections
  settings pane.

### Modified Capabilities

- `context-menu`: Gains a new action group with four queue-movement items that dispatch bulk
  `queue-move-*` RPCs for the selected torrents.
- `rpc-client`: Extended with four new queue-movement method calls and updated session
  get/set coverage for queue configuration fields.

## Impact

- **`src/rpc/models.rs`** — Add queue fields to `SessionData` and `SessionSetArgs`.
- **`src/rpc/api.rs`** and **`src/rpc/worker.rs`** — Add four new `RpcWork` variants and their
  corresponding dispatch logic.
- **`src/screens/settings/`** — Add the "Queueing" card to the Connections tab view, wiring up
  new text inputs and toggle controls.
- **`src/screens/torrent_list/dialogs.rs`** — Add the four queue-movement items to
  `view_context_menu_overlay`.
- **`src/screens/torrent_list/update.rs`** — Handle the new context-menu messages, enqueuing
  the correct `RpcWork` variants.
