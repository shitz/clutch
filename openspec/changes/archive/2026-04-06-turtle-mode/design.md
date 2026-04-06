## Context

Clutch currently has no bandwidth control surface. Transmission's JSON-RPC API provides
two distinct throttling systems:

1. **Session-level alternative speed limits** (`alt-speed-*`): a global "Turtle Mode"
   that caps all upload/download to a configured ceiling. Toggled via `session-set` with
   `alt-speed-enabled`. The current ceiling values (`alt-speed-down`, `alt-speed-up`) are
   read-back from `session-get` and written with `session-set`.

2. **Per-torrent speed limits** (`downloadLimit`, `downloadLimited`, `uploadLimit`,
   `uploadLimited`, `seedRatioLimit`, `seedRatioMode`, `honorsSessionLimits`): override or
   exempt individual torrents. Read via the existing `torrent-get` call; written via the
   existing `torrent-set` call.

No new third-party crates are required. The existing `reqwest`/`serde_json` transport, the
`mpsc` worker pattern, and the `iced` widget set are all sufficient.

## Goals / Non-Goals

**Goals:**

- Toolbar turtle-mode toggle that reflects live daemon state and round-trips in < 500 ms.
- Settings card so users can configure the alternative speed ceiling without leaving Clutch.
- Inspector "Options" tab for per-torrent bandwidth limits and ratio cap.
- Per-torrent "Honor Global Limits" switch so a torrent can bypass or respect Turtle Mode.
- All mutations dispatched through the existing `mpsc` worker; `update()` stays
  non-blocking.

**Non-Goals:**

- Scheduled Turtle Mode (time-of-day activation) — Transmission supports it but it adds a
  complex UI for the first iteration.
- Bandwidth limit presets or profiles — out of scope.
- Displaying current speed graphs — out of scope.

## Decisions

### Decision 1: Where turtle-mode state lives — `AppState`

`alt_speed_enabled: bool` (and the session's `alt_speed_down` / `alt_speed_up`) belongs in
`AppState`, not inside a individual screen, because both the toolbar (main_screen) and the
settings screen need to read and mutate it.

**Alternative considered**: Store in `MainScreenState` and pass down. Rejected because the
settings screen (which is a separate `Screen` variant) would need a separate copy, leading
to synchronisation bugs.

### Decision 2: Toolbar button visual — highlighted icon_button

Use `theme::icon_button(icon(ICON_SPEED))` just left of the settings gear. When
`alt_speed_enabled` is `true`, wrap the button in a container or extend `icon_button` to
accept an `active: bool` flag that switches the icon color to `palette.primary`.
`ICON_SPEED` maps to Material Icons codepoint `\u{E9E4}` ("speed"). This avoids adding a
custom turtle SVG and stays within the existing icon font.

**Alternative considered**: Distinct turtle codepoint. Material Icons does not ship a
turtle glyph; the next-closest semantic match is "speed" (`\u{E9E4}`).

### Decision 3: Profile stores all bandwidth settings; pushed to daemon on connect

All bandwidth and seeding settings (standard global download/upload, alternative limits,
seed-ratio) are stored in `ConnectionProfile` and persisted to the TOML config file. On
connect, `make_push_bandwidth_task` builds a `session-set` call from the profile and
issues it after the initial `session-get` probe.

**Rationale**: Users expect their configured limits to be restored whenever Clutch connects
to a daemon. Storing them in the profile makes this deterministic across restarts and
connections from multiple devices.

**Alternative speed ceiling values** (`alt-speed-down`, `alt-speed-up`) and the Turtle
Mode enabled flag (`alt-speed-enabled`) are also stored per-profile. However they are only
pushed on connect when non-zero / non-default. The daemon's live enabled state is always
read back from `session-get`, so the toolbar button reflects reality even when Turtle Mode
was toggled by another client between sessions.

**Original decision (reversed)**: The initial design stored nothing in the profile and
read all values from the daemon on every connect. Reversed because standard global limits
are Clutch-specific user preferences that should survive across daemon restarts and client
changes by other apps.

### Decision 4: per-torrent Options tab — inline immediate save

Each control change in the Options tab is applied **immediately** without a Save button:

- Toggle messages (`OptionsDownload/UploadLimitToggled`, `OptionsHonorGlobalToggled`,
  `OptionsRatioModeChanged`) enqueue a `TorrentSetBandwidth` RPC call as soon as the user
  flips the control.
- Text-change messages only update local draft state; no RPC is issued on keystrokes.
- Submit messages (fired on Enter in a text field) enqueue the RPC only when the
  corresponding toggle is on; otherwise they are silently ignored.

When `OptionsHonorGlobalToggled` fires, the RPC call includes **all** current bandwidth
state (`honors_session_limits`, `download_limited`, `download_limit`, `upload_limited`,
`upload_limit`) so Transmission can enforce per-torrent limits immediately when the torrent
opts out of the global cap. Sending only `honorsSessionLimits=false` without the rest
caused the global limit to still apply (the bug this design avoids).

**Alternative considered**: Draft state + Save button (original design). Rejected because
the tab layout evolved from a flat four-row form to a two-column card layout where each
card is semantically independent; a single Save button is less intuitive in that context.
Immediate apply is also the established pattern for the file-wanted checkboxes.

### Decision 5: Text inputs for speed values — no sliders

Speed inputs use `text_input` (KB/s) per the user's explicit requirement. Parsing: accept
an empty string as "0 / unlimited"; reject non-numeric input silently (do not update state
on non-digit characters). Display placeholder `"KB/s"`.

### Decision 6: `seedRatioMode` encoding — three-way segmented control

Transmission's `seedRatioMode` is an integer enum:

- `0` = use global ratio limit
- `1` = use per-torrent limit (`seedRatioLimit`)
- `2` = unlimited (no ratio limit)

The Options tab exposes all three modes via a segmented control labelled
"Global | Custom | Unlimited". When Custom is selected, a "Custom ratio" text-input row
appears below the control. When Global or Unlimited is selected, no text input is shown.
`seedRatioLimit` is only included in the `torrent-set` payload when the mode is being set
to Custom (1).

**Original design**: A toggle ON/OFF mapped only to modes 1 and 0, omitting "Unlimited".
Revised to include the third mode because Transmission supports it and omitting it would
force users to the web UI to remove a ratio cap.

### Decision 7: `honorsSessionLimits` meaning

`honorsSessionLimits = true` → the torrent respects whichever global speed limit is
currently active on the daemon:

- When Turtle Mode is **off**, the standard global limits (`speed-limit-down/up`, when
  enabled) apply.
- When Turtle Mode is **on**, the alternative limits (`alt-speed-down/up`) apply instead.

`honorsSessionLimits = false` → the torrent **ignores ALL session-level global speed
limits** (both standard and alternative). Only the torrent's own per-torrent limits
(`downloadLimit` when `downloadLimited=true`, `uploadLimit` when `uploadLimited=true`)
cap its speed. If neither per-torrent limit is enabled, the torrent runs uncapped.

The switch label: **"Honor Global Speed Limits"**. Switch ON = honors all session limits
(`true`). Switch OFF = bypasses all session limits (`false`), using only per-torrent caps.

### Decision 8: Periodic `session-get` poll for multi-client correctness

The turtle-mode toolbar button reflects `AppState.alt_speed_enabled`, which is populated
on connect. However, another client (phone, web UI) may toggle Turtle Mode or change the
speed ceiling at any time. To stay in sync, `app::update` hooks into the existing
`torrent-get` subscription tick and piggybacks a `session-get` call every 10 seconds on a
separate counter. `AppState` alt-speed fields are updated from each response; the toolbar
icon re-renders automatically.

**Alternative considered**: Separate subscription timer. Rejected — adding a second
time-based subscription is unnecessary complexity when the torrent-poll tick already fires
regularly and the worker serialises all calls.

### Decision 9: Inspector draft reset on torrent selection change

`InspectorOptionsState` is reset to daemon values (from `TorrentData`) whenever
`Message::List(torrent_list::Message::TorrentSelected(_))` fires in `app::update` (or the
equivalent torrent-changed path). This prevents unsaved draft values leaking from one
torrent's Options tab to another's when the user switches selection without saving.

**Alternative considered**: Prompt the user to save/discard on selection change. Rejected
— silent discard is the established pattern in Clutch (file-wanted toggles auto-save;
settings cancel discards). An unsaved Options draft is cheap to discard.

### Decision 10: Numeric-only filtering for speed/ratio text inputs

All KB/s and ratio `text_input` fields accept only ASCII digit characters (plus empty
string). Non-digit input is silently discarded in `update()` before the draft is updated:

```rust
Message::AltSpeedDownChanged(val) => {
    if val.is_empty() || val.chars().all(|c| c.is_ascii_digit()) {
        state.alt_speed_down_draft = val;
    }
    Task::none()
}
```

The ratio field additionally allows a single decimal point (`'.'`) so the user can enter
values like `1.5`. This is enforced in the same guard. No external validation crate is
needed.

### Decision 11: Bandwidth-only save avoids reconnect (`ActiveProfileBandwidthSaved`)

When the user saves a connection profile and only bandwidth/seeding fields changed (host,
port, and username are unchanged), `settings::update` returns
`SettingsResult::ActiveProfileBandwidthSaved` instead of `ActiveProfileSaved`.
`app::update` handles this by calling `make_push_bandwidth_task` — issuing a `session-set`
with the new limits to the live daemon — without tearing down and re-establishing the
connection. A full reconnect would interrupt pending polls and torrent operations.

**Alternative considered**: Always reconnect on save. Rejected — reconnect flushes the
torrent list and resets all UI state, which is disruptive for a pure bandwidth change.

## Risks / Trade-offs

- **Race condition on turtle toggle**: If the user rapidly toggles, the worker serialises
  calls but the UI shows optimistic state. Worst case: two calls, daemon ends up in the
  correct final state. Acceptable.
- **Settings UI shows stale data during connect**: There is a brief window between app
  launch and the first `session-get` response where the alt-speed fields in the Settings
  card show empty/zero. This is acceptable; the fields populate as soon as the connect
  probe completes.
- **No validation of KB/s field**: Transmission silently clamps extreme values. We accept
  any non-negative integer and let the daemon enforce bounds, which keeps the code simple.
- **Icon choice**: `ICON_SPEED` is a recognisable speedometer, not a turtle. Power users
  familiar with other clients may need a moment to identify it. A tooltip mitigates this.
