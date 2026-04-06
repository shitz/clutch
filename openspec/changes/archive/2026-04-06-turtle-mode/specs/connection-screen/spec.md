## ADDED Requirements

### Requirement: Bandwidth card in Settings

The Settings Connections tab SHALL display a "Bandwidth" card below the "Connection
Details" card. The card header SHALL use bold 16 px text. The card SHALL contain three
labelled sections:

**Standard Global Limits** — two list-tile rows:

- Limit Download (KB/s): toggler bound to `speed_limit_down_enabled`; text input enabled
  only when toggler is ON, bound to `speed_limit_down`.
- Limit Upload (KB/s): toggler bound to `speed_limit_up_enabled`; text input enabled only
  when toggler is ON, bound to `speed_limit_up`.

**Alternative Limits (Turtle Mode)** — two plain value rows (no toggles):

- Download (KB/s): text input bound to `alt_speed_down`.
- Upload (KB/s): text input bound to `alt_speed_up`.

**Seeding** — one list-tile row:

- Stop Seeding at Ratio: toggler bound to `ratio_limit_enabled`; text input enabled only
  when toggler is ON, bound to `ratio_limit` (decimal).

All text inputs SHALL use `theme::m3_text_input`. Numeric-only inputs (KB/s) SHALL accept
only ASCII digit characters. The ratio input SHALL additionally allow a single `'.'`.

All bandwidth values SHALL be persisted to `ConnectionProfile` (and therefore to the TOML
config file) when the user saves the profile.

On successful connect, Clutch SHALL push all configured bandwidth limits to the daemon via
a `session-set` call (using `make_push_bandwidth_task`). If all values are default/zero,
no `session-set` is issued.

If the user saves the profile and only bandwidth fields changed (host, port, and username
are unchanged), the settings screen SHALL return `SettingsResult::ActiveProfileBandwidthSaved`
so that `app::update` applies the new limits to the live daemon without triggering a full
reconnect.

#### Scenario: Bandwidth card is visible in Settings Connections tab

- **WHEN** the Settings screen Connections tab is rendered
- **THEN** a "Bandwidth" card is visible below the Connection Details card

#### Scenario: Standard global limit toggle enables text field

- **WHEN** the "Limit Download (KB/s)" toggler is OFF
- **THEN** the download speed text input is visually disabled and ignores key input
- **WHEN** the toggler is switched ON
- **THEN** the text input becomes active and accepts numeric input

#### Scenario: Bandwidth values saved to profile on Save

- **WHEN** the user modifies bandwidth fields and clicks Save
- **THEN** all bandwidth values are written to `ConnectionProfile` and persisted to TOML

#### Scenario: Bandwidth values pushed to daemon on connect

- **WHEN** a connection probe succeeds and the active profile has non-default bandwidth
  settings
- **THEN** a `session-set` call is dispatched with those values
- **WHEN** all bandwidth values are default (zero / disabled)
- **THEN** no `session-set` call is issued on connect

#### Scenario: Bandwidth-only save applies limits without reconnect

- **WHEN** the user changes only bandwidth fields (not host/port/username) and clicks Save
- **THEN** the new limits are pushed to the daemon via `session-set`
- **THEN** the active connection is NOT torn down or re-established
