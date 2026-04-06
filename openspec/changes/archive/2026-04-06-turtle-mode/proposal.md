## Why

Clutch exposes Transmission's core torrent management capabilities but provides no way to
control bandwidth — either globally (Turtle Mode / Alternative Speed Limits) or per-torrent.
Users who share network connections or seed in the background have no mechanism to throttle
traffic without leaving the app and opening the Transmission web UI.

## What Changes

- Add a **Turtle Mode toolbar button** to the main screen toolbar (top-right, next to
  Settings). The button highlights when alternative speed limits are active; toggling it
  calls `session-set` on the daemon.
- Add an **Alternative Speed Limits card** to the Connection Settings screen so users can
  configure the alternative download/upload speed cap (in KB/s) alongside their connection
  profile.
- Add an **Options tab** to the Detail Inspector (`General | Files | Trackers | Peers |
Options`) exposing per-torrent bandwidth controls:
  - Limit Download Speed (switch + KB/s text field)
  - Limit Upload Speed (switch + KB/s text field)
  - Stop Seeding at Ratio (switch + ratio text field)
  - Honor Global Limits / Alternative Speeds (switch)
- Extend the RPC client with the new calls required: `session-get` fields for alternative
  speed state, `session-set` for toggling turtle mode and saving alternative limits,
  `torrent-set` for per-torrent bandwidth fields, and extended `torrent-get` fields for
  per-torrent limit state.

## Capabilities

### New Capabilities

- `turtle-mode`: Global alternative speed limits — toolbar toggle, settings card, and the
  underlying `session-get`/`session-set` RPC calls.
- `torrent-options`: Per-torrent bandwidth limits and seeding ratio — the new "Options" tab
  in the Detail Inspector and the `torrent-set` / extended `torrent-get` RPC calls.

### Modified Capabilities

- `rpc-client`: New `torrent-get` fields (`downloadLimit`, `downloadLimited`,
  `uploadLimit`, `uploadLimited`, `seedRatioLimit`, `seedRatioMode`,
  `honorsSessionLimits`), new `session-get` fields (`alt-speed-enabled`,
  `alt-speed-down`, `alt-speed-up`), and new mutation calls (`session-set`,
  `torrent-set`).
- `connection-screen`: Settings screen gains the "Alternative Speed Limits" card; values
  are read from the daemon via `session-get` on connect and written back via `session-set`
  on save (no profile storage).

## Impact

- **`src/rpc/`**: New model structs/fields; new async functions in `api.rs`; new worker
  message variants.
- **`src/screens/main_screen.rs`**: Toolbar turtle button; new `AppMessage` variants for
  toggling turtle mode; polling `session-get` to reflect current state.
- **`src/screens/inspector.rs`**: New `Options` tab with four switch+field rows; new
  `InspectorMessage` variants; `torrent-set` dispatch on change.
- **`src/screens/settings/`**: Alternative Speed Limits card in the settings view; draft
  fields for `alt_speed_down` / `alt_speed_up` initialised from `AppState`; `session-set`
  dispatched on save (values are never persisted to `profile.rs`).
- **`src/theme.rs`**: `ICON_TURTLE` constant (Material Icons codepoint `\u{e35c}` —
  "speed" icon, or nearest available glyph).
- No new third-party dependencies required.
