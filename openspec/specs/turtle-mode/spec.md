## Requirements

### Requirement: Turtle Mode toolbar toggle

The main screen toolbar SHALL display a speed-limit toggle button (using `ICON_SPEED`)
immediately to the left of the Settings gear button. The button SHALL visually indicate the
active state: when alternative speed limits are enabled on the daemon the icon SHALL be
rendered in the theme's primary (blue) color; when disabled it SHALL use the default icon
color.

Pressing the button SHALL dispatch a `session-set` call that flips the `alt-speed-enabled`
flag on the daemon (optimistic toggle in UI, corrected on next `session-get` response).

#### Scenario: Button shown inactive when turtle mode is off

- **WHEN** the main screen is rendered and `AppState.alt_speed_enabled` is `false`
- **THEN** the toolbar displays the speed icon in the default (non-highlighted) color

#### Scenario: Button shown active when turtle mode is on

- **WHEN** the main screen is rendered and `AppState.alt_speed_enabled` is `true`
- **THEN** the toolbar displays the speed icon in the theme's primary color

#### Scenario: Toggle dispatches session-set and flips optimistic state

- **WHEN** the user clicks the turtle mode toolbar button
- **THEN** `AppState.alt_speed_enabled` is toggled immediately in the UI
- **THEN** a `session-set` RPC call is dispatched with the new `alt-speed-enabled` value

#### Scenario: session-get polling corrects optimistic state

- **WHEN** a `session-get` response is received
- **THEN** `AppState.alt_speed_enabled` is updated to the daemon-reported value

### Requirement: Alt-speed state read on connect

Immediately after a successful connection probe, the app SHALL issue a `session-get` call
and populate `AppState.alt_speed_enabled`, `AppState.alt_speed_down_kbps`, and
`AppState.alt_speed_up_kbps` from the response. The daemon is the single source of truth
— Clutch SHALL NOT push locally-stored values to the daemon on connect.

#### Scenario: session-get populates alt-speed state on connect

- **WHEN** the connection probe succeeds and `session-get` is called
- **THEN** `AppState.alt_speed_enabled` reflects the daemon's `alt-speed-enabled` value
- **THEN** `AppState.alt_speed_down_kbps` reflects the daemon's `alt-speed-down` value
- **THEN** `AppState.alt_speed_up_kbps` reflects the daemon's `alt-speed-up` value

### Requirement: Periodic session-get poll

While the main screen is active, the app SHALL issue a `session-get` call every 10 seconds
to keep `AppState` alt-speed fields in sync with the daemon. This ensures that changes
made by other clients (web UI, mobile app) are reflected in the toolbar button and Settings
card without requiring a reconnect.

#### Scenario: Periodic poll updates toolbar state after external change

- **WHEN** another client toggles `alt-speed-enabled` on the daemon
- **AND** the next periodic `session-get` response is received by Clutch
- **THEN** `AppState.alt_speed_enabled` is updated to reflect the daemon's current value
- **THEN** the toolbar button re-renders with the correct active/inactive color

#### Scenario: Periodic poll updates alt-speed limit values

- **WHEN** another client changes `alt-speed-down` or `alt-speed-up` on the daemon
- **AND** the next periodic `session-get` response is received
- **THEN** `AppState.alt_speed_down_kbps` and `AppState.alt_speed_up_kbps` are updated
- **THEN** the Settings card fields reflect the new values on next open
