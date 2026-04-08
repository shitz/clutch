## Purpose

Provide a safe, async Transmission JSON-RPC client that serializes all daemon
calls through a single MPSC worker, guaranteeing at most one in-flight HTTP
connection at any time while remaining fully non-blocking on the iced UI thread.
## Requirements
### Requirement: Session-Id lifecycle handling

The RPC client SHALL implement the Transmission `X-Transmission-Session-Id` handshake. On receiving a 409 response, the client SHALL extract the new session ID from the response header and retry the request exactly once with the updated ID.

#### Scenario: Initial 409 triggers session-id capture and retry

- **WHEN** an RPC request receives a 409 Conflict response
- **THEN** the client extracts `X-Transmission-Session-Id` from the response headers
- **THEN** the client retries the same request with the new session ID
- **THEN** the updated session ID is returned to the caller for storage

#### Scenario: Successful request with valid session-id

- **WHEN** an RPC request is sent with a valid session ID
- **THEN** the response is parsed and returned without retrying

### Requirement: session-get probe

The RPC client SHALL provide a `session-get` call used to verify connectivity and authentication before entering the main screen.

#### Scenario: session-get succeeds

- **WHEN** `session-get` is called with valid credentials
- **THEN** the call returns success and the current session ID

#### Scenario: session-get fails with auth error

- **WHEN** `session-get` receives a 401 Unauthorized response
- **THEN** an `AuthError` variant is returned to the caller

#### Scenario: session-get fails with connectivity error

- **WHEN** `session-get` cannot reach the daemon (connection refused, timeout)
- **THEN** a `ConnectionError` variant is returned to the caller

### Requirement: torrent-get call

The RPC client SHALL provide a `torrent-get` call that fetches name, status, and percent-done for all torrents.

#### Scenario: torrent-get returns torrent list

- **WHEN** `torrent-get` is called with a valid session ID
- **THEN** a list of `TorrentData` structs is returned, each containing id, name, status, and percentDone

#### Scenario: JSON deserialization on caller thread

- **WHEN** `torrent-get` response is received
- **THEN** JSON parsing SHALL complete inside the async function (not in `update()`)

### Requirement: No blocking calls

All RPC functions SHALL be `async` and MUST NOT use blocking I/O, `std::thread::sleep`, or `block_on`. They SHALL be called exclusively inside `Command::perform()`.

#### Scenario: update() remains non-blocking

- **WHEN** any RPC call is in-flight
- **THEN** the `update()` function returns immediately without waiting for the result

### Requirement: torrent-add call

The RPC client SHALL provide a `torrent_add` function that accepts an `AddPayload` enum and issues
a `torrent-add` JSON-RPC call. `AddPayload::Magnet(uri)` maps to the `filename` field;
`AddPayload::Metainfo(base64)` maps to the `metainfo` field.

#### Scenario: torrent-add with magnet URI succeeds

- **WHEN** `torrent_add` is called with `AddPayload::Magnet(uri)` and valid credentials
- **THEN** a POST is issued with `{ "method": "torrent-add", "arguments": { "filename": "<uri>" } }`
- **THEN** `Ok(())` is returned on a `"success"` result field

#### Scenario: torrent-add with metainfo succeeds

- **WHEN** `torrent_add` is called with `AddPayload::Metainfo(base64)` and valid credentials
- **THEN** a POST is issued with `{ "method": "torrent-add", "arguments": { "metainfo": "<base64>" } }`
- **THEN** `Ok(())` is returned on a `"success"` or `"torrent-duplicate"` result field

#### Scenario: torrent-add includes download-dir when provided

- **WHEN** `torrent_add` is called with a non-empty `download_dir`
- **THEN** the RPC arguments include `"download-dir": "<path>"`

#### Scenario: torrent-add omits download-dir when empty

- **WHEN** `torrent_add` is called with `download_dir = None` or an empty string
- **THEN** the RPC arguments do not include a `"download-dir"` field

#### Scenario: torrent-add session rotation handled

- **WHEN** `torrent_add` receives a 409 response
- **THEN** `RpcError::SessionRotated(new_id)` is returned to the caller

#### Scenario: torrent-add auth failure

- **WHEN** `torrent_add` receives a 401 response
- **THEN** `RpcError::AuthError` is returned

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
- `honors_session_limits: Option<bool>` (serialized as `"honorSessionLimits"` — singular,
  matching Transmission's `torrent-set` field name)

Only fields with `Some(...)` values SHALL be included in the JSON payload.

Note: Transmission uses different spellings for this field: `"honorsSessionLimits"`
(plural) in `torrent-get` responses and `"honorSessionLimits"` (singular) in `torrent-set`
requests.

#### Scenario: torrent-set-bandwidth dispatches correct RPC

- **WHEN** `torrent_set_bandwidth` is called with a torrent ID and non-None fields
- **THEN** a POST is issued with `"method": "torrent-set"` and an `"ids"` array containing
  the given torrent ID
- **THEN** each non-None field is represented in `arguments` with its Transmission JSON key

#### Scenario: torrent-set session rotation handled

- **WHEN** `torrent_set_bandwidth` receives a 409 response
- **THEN** `RpcError::SessionRotated(new_id)` is returned to the caller

### Requirement: torrent-set-location call

The RPC client SHALL provide a `torrent_set_location` async function that accepts a torrent id,
an absolute destination path string, and a boolean `move_data` flag, and issues a
`torrent-set-location` JSON-RPC call.

#### Scenario: torrent-set-location with move=true

- **WHEN** `torrent_set_location(id, "/new/path", true)` is called with valid credentials
- **THEN** a POST is issued with:
  `{"method":"torrent-set-location","arguments":{"ids":[id],"location":"/new/path","move":true}}`
- **THEN** `Ok(())` is returned on a `"success"` result field

#### Scenario: torrent-set-location with move=false

- **WHEN** `torrent_set_location(id, "/new/path", false)` is called
- **THEN** the JSON-RPC body contains `"move": false`
- **THEN** `Ok(())` is returned on a `"success"` result field

#### Scenario: torrent-set-location session rotation handled

- **WHEN** `torrent_set_location` receives a 409 response
- **THEN** `RpcError::SessionRotated(new_id)` is returned to the caller

#### Scenario: torrent-set-location auth failure

- **WHEN** `torrent_set_location` receives a 401 response
- **THEN** `RpcError::AuthError` is returned

### Requirement: torrent-get includes error and data-path fields

The `torrent-get` fields list SHALL include `"error"`, `"errorString"`, and `"downloadDir"`.
`TorrentData` SHALL expose these as typed fields:

- `error: i32` — Transmission error code; 0 means no error
- `error_string: String` — human-readable error message (empty when `error == 0`)
- `download_dir: String` — absolute path to the torrent's download directory on the daemon

#### Scenario: error fields are populated from torrent-get response

- **WHEN** the daemon returns `"error": 3, "errorString": "disk full"` for a torrent
- **THEN** `TorrentData::error == 3` and `TorrentData::error_string == "disk full"`

#### Scenario: error fields default to zero/empty when absent

- **WHEN** the daemon omits `"error"` and `"errorString"` fields
- **THEN** `TorrentData::error == 0` and `TorrentData::error_string` is empty

