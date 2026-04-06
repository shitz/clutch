## ADDED Requirements

### Requirement: session-get returns alt-speed fields

The `session-get` call SHALL return a `SessionData` struct containing:

- `alt_speed_enabled: bool` — maps to JSON field `"alt-speed-enabled"`
- `alt_speed_down: u32` — maps to JSON field `"alt-speed-down"` (KB/s)
- `alt_speed_up: u32` — maps to JSON field `"alt-speed-up"` (KB/s)

#### Scenario: session-get populates alt-speed fields

- **WHEN** `session-get` is called and succeeds
- **THEN** the returned `SessionData` contains `alt_speed_enabled`, `alt_speed_down`, and
  `alt_speed_up` deserialized from the JSON response

### Requirement: session-set call

The RPC client SHALL provide a `session_set` async function that accepts a
`SessionSetArgs` struct and issues a `session-set` JSON-RPC call.

`SessionSetArgs` SHALL support the following optional fields:

- `alt_speed_enabled: Option<bool>`
- `alt_speed_down: Option<u32>` (KB/s)
- `alt_speed_up: Option<u32>` (KB/s)
- `speed_limit_down_enabled: Option<bool>`
- `speed_limit_down: Option<u32>` (KB/s)
- `speed_limit_up_enabled: Option<bool>`
- `speed_limit_up: Option<u32>` (KB/s)
- `seed_ratio_limit: Option<f64>`
- `seed_ratio_limited: Option<bool>`

Only fields with `Some(...)` values SHALL be included in the JSON payload.

#### Scenario: session-set toggles alt-speed-enabled

- **WHEN** `session_set` is called with `alt_speed_enabled: Some(true)`
- **THEN** a POST is issued with `{ "method": "session-set", "arguments": { "alt-speed-enabled": true } }`
- **THEN** `Ok(())` is returned on a `"success"` result field

#### Scenario: session-set sets alt-speed limits

- **WHEN** `session_set` is called with `alt_speed_down: Some(500)` and `alt_speed_up: Some(50)`
- **THEN** the POST body `arguments` object contains `"alt-speed-down": 500` and `"alt-speed-up": 50`

#### Scenario: session-set sets standard speed limits

- **WHEN** `session_set` is called with `speed_limit_down_enabled: Some(true)` and
  `speed_limit_down: Some(1000)`
- **THEN** the POST body `arguments` contains `"speed-limit-down-enabled": true` and
  `"speed-limit-down": 1000`

#### Scenario: session-set sets seeding ratio

- **WHEN** `session_set` is called with `seed_ratio_limited: Some(true)` and
  `seed_ratio_limit: Some(2.0)`
- **THEN** the POST body `arguments` contains `"seedRatioLimited": true` and
  `"seedRatioLimit": 2.0`

#### Scenario: session-set omits None fields

- **WHEN** `session_set` is called with `alt_speed_enabled: None`
- **THEN** the POST body does not contain an `"alt-speed-enabled"` key

#### Scenario: session-set session rotation handled

- **WHEN** `session_set` receives a 409 response
- **THEN** `RpcError::SessionRotated(new_id)` is returned to the caller

### Requirement: torrent-get extended fields

The `torrent-get` call SHALL request and deserialize the following additional fields for
each torrent: `downloadLimit`, `downloadLimited`, `uploadLimit`, `uploadLimited`,
`seedRatioLimit`, `seedRatioMode`, `honorsSessionLimits`.

#### Scenario: torrent-get includes per-torrent limit fields in request

- **WHEN** `torrent-get` is called
- **THEN** the `fields` array in the JSON body includes `"downloadLimit"`,
  `"downloadLimited"`, `"uploadLimit"`, `"uploadLimited"`, `"seedRatioLimit"`,
  `"seedRatioMode"`, and `"honorsSessionLimits"`

#### Scenario: torrent-get deserializes per-torrent limit fields

- **WHEN** the `torrent-get` response contains per-torrent limit fields
- **THEN** each `TorrentData` entry has the correct values for all seven fields

### Requirement: torrent-set bandwidth call

The RPC client SHALL provide a `torrent_set_bandwidth` async function that accepts a
torrent ID and a `TorrentBandwidthArgs` struct, then issues a `torrent-set` JSON-RPC call.

`TorrentBandwidthArgs` SHALL support the following optional fields:

- `download_limited: Option<bool>`
- `download_limit: Option<u64>` (KB/s)
- `upload_limited: Option<bool>`
- `upload_limit: Option<u64>` (KB/s)
- `seed_ratio_limit: Option<f64>`
- `seed_ratio_mode: Option<u8>`
- `honors_session_limits: Option<bool>`

Only fields with `Some(...)` values SHALL be included in the JSON payload.

#### Scenario: torrent-set-bandwidth dispatches correct RPC

- **WHEN** `torrent_set_bandwidth` is called with a torrent ID and non-None fields
- **THEN** a POST is issued with `"method": "torrent-set"` and an `"ids"` array containing
  the given torrent ID
- **THEN** each non-None field is represented in `arguments` with its Transmission JSON key

#### Scenario: torrent-set session rotation handled

- **WHEN** `torrent_set_bandwidth` receives a 409 response
- **THEN** `RpcError::SessionRotated(new_id)` is returned to the caller
