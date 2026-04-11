## ADDED Requirements

### Requirement: torrent-start call

The RPC client SHALL provide a `torrent_start` async function that accepts a slice of torrent
IDs and issues a `torrent-start` JSON-RPC call.

#### Scenario: torrent-start dispatches for multiple IDs

- **WHEN** `torrent_start(ids: &[i64])` is called with IDs `[1, 2, 3]`
- **THEN** a POST is issued with `{"method":"torrent-start","arguments":{"ids":[1,2,3]}}`
- **THEN** `Ok(())` is returned on a `"success"` result field

#### Scenario: torrent-start dispatches for a single ID

- **WHEN** `torrent_start(ids: &[42])` is called
- **THEN** a POST is issued with `{"method":"torrent-start","arguments":{"ids":[42]}}`

### Requirement: torrent-stop call

The RPC client SHALL provide a `torrent_stop` async function that accepts a slice of torrent
IDs and issues a `torrent-stop` JSON-RPC call.

#### Scenario: torrent-stop dispatches for multiple IDs

- **WHEN** `torrent_stop(ids: &[i64])` is called with IDs `[1, 2]`
- **THEN** a POST is issued with `{"method":"torrent-stop","arguments":{"ids":[1,2]}}`
- **THEN** `Ok(())` is returned on a `"success"` result field

#### Scenario: torrent-stop dispatches for a single ID

- **WHEN** `torrent_stop(ids: &[42])` is called
- **THEN** a POST is issued with `{"method":"torrent-stop","arguments":{"ids":[42]}}`

### Requirement: torrent-remove call

The RPC client SHALL provide a `torrent_remove` async function that accepts a slice of torrent
IDs, a `delete_local_data: bool` flag, and issues a `torrent-remove` JSON-RPC call.

#### Scenario: torrent-remove dispatches for multiple IDs with delete flag

- **WHEN** `torrent_remove(ids: &[1, 2], delete_local_data: true)` is called
- **THEN** a POST is issued with:
  `{"method":"torrent-remove","arguments":{"ids":[1,2],"delete-local-data":true}}`
- **THEN** `Ok(())` is returned on a `"success"` result field

#### Scenario: torrent-remove dispatches for a single ID without delete

- **WHEN** `torrent_remove(ids: &[42], delete_local_data: false)` is called
- **THEN** the JSON-RPC body contains `"delete-local-data": false`

## MODIFIED Requirements

### Requirement: torrent-set bandwidth call

The RPC client SHALL provide a `torrent_set_bandwidth` async function that accepts a **slice**
of torrent IDs (`ids: &[i64]`) and a `TorrentBandwidthArgs` struct, then issues a `torrent-set`
JSON-RPC call. The `ids` array in the payload SHALL contain all provided IDs.

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

#### Scenario: torrent-set-bandwidth dispatches correct RPC for multiple IDs

- **WHEN** `torrent_set_bandwidth(ids: &[1, 2, 3], args)` is called with non-None fields
- **THEN** a POST is issued with `"method": "torrent-set"` and an `"ids"` array containing
  `[1, 2, 3]`
- **THEN** each non-None field is represented in `arguments` with its Transmission JSON key

#### Scenario: torrent-set-bandwidth dispatches for a single ID

- **WHEN** `torrent_set_bandwidth(ids: &[42], args)` is called
- **THEN** the `"ids"` array contains `[42]`

#### Scenario: torrent-set session rotation handled

- **WHEN** `torrent_set_bandwidth` receives a 409 response
- **THEN** `RpcError::SessionRotated(new_id)` is returned to the caller

### Requirement: torrent-set-location call

The RPC client SHALL provide a `torrent_set_location` async function that accepts a **slice**
of torrent IDs (`ids: &[i64]`), an absolute destination path string, and a boolean `move_data`
flag, and issues a `torrent-set-location` JSON-RPC call.

#### Scenario: torrent-set-location with move=true for multiple IDs

- **WHEN** `torrent_set_location(ids: &[1, 2], "/new/path", true)` is called
- **THEN** a POST is issued with:
  `{"method":"torrent-set-location","arguments":{"ids":[1,2],"location":"/new/path","move":true}}`
- **THEN** `Ok(())` is returned on a `"success"` result field

#### Scenario: torrent-set-location with move=false for a single ID

- **WHEN** `torrent_set_location(ids: &[42], "/new/path", false)` is called
- **THEN** the JSON-RPC body contains `"ids":[42]` and `"move": false`

#### Scenario: torrent-set-location session rotation handled

- **WHEN** `torrent_set_location` receives a 409 response
- **THEN** `RpcError::SessionRotated(new_id)` is returned to the caller

#### Scenario: torrent-set-location auth failure

- **WHEN** `torrent_set_location` receives a 401 response
- **THEN** `RpcError::AuthError` is returned

### Requirement: queue-move-top call

The RPC client SHALL provide a `queue_move_top` async function that accepts a slice of torrent
IDs and issues a `queue-move-top` JSON-RPC call. The function SHALL return `Ok(())` on
success.

#### Scenario: queue-move-top dispatches correct RPC for multiple IDs

- **WHEN** `queue_move_top(ids: &[2, 5])` is called
- **THEN** a POST is issued with `{"method":"queue-move-top","arguments":{"ids":[2,5]}}`
- **THEN** `Ok(())` is returned when the result field is `"success"`

#### Scenario: queue-move-top session rotation handled

- **WHEN** `queue_move_top` receives a 409 response
- **THEN** `RpcError::SessionRotated(new_id)` is returned to the caller

### Requirement: queue-move-up call

The RPC client SHALL provide a `queue_move_up` async function that accepts a slice of torrent
IDs and issues a `queue-move-up` JSON-RPC call. The function SHALL return `Ok(())` on success.

#### Scenario: queue-move-up dispatches correct RPC

- **WHEN** `queue_move_up(ids: &[3])` is called
- **THEN** a POST is issued with `{"method":"queue-move-up","arguments":{"ids":[3]}}`
- **THEN** `Ok(())` is returned on success

#### Scenario: queue-move-up session rotation handled

- **WHEN** `queue_move_up` receives a 409 response
- **THEN** `RpcError::SessionRotated(new_id)` is returned to the caller

### Requirement: queue-move-down call

The RPC client SHALL provide a `queue_move_down` async function that accepts a slice of torrent
IDs and issues a `queue-move-down` JSON-RPC call. The function SHALL return `Ok(())` on
success.

#### Scenario: queue-move-down dispatches correct RPC

- **WHEN** `queue_move_down(ids: &[3])` is called
- **THEN** a POST is issued with `{"method":"queue-move-down","arguments":{"ids":[3]}}`
- **THEN** `Ok(())` is returned on success

#### Scenario: queue-move-down session rotation handled

- **WHEN** `queue_move_down` receives a 409 response
- **THEN** `RpcError::SessionRotated(new_id)` is returned to the caller

### Requirement: queue-move-bottom call

The RPC client SHALL provide a `queue_move_bottom` async function that accepts a slice of
torrent IDs and issues a `queue-move-bottom` JSON-RPC call. The function SHALL return `Ok(())`
on success.

#### Scenario: queue-move-bottom dispatches correct RPC for multiple IDs

- **WHEN** `queue_move_bottom(ids: &[1, 4])` is called
- **THEN** a POST is issued with `{"method":"queue-move-bottom","arguments":{"ids":[1,4]}}`
- **THEN** `Ok(())` is returned on success

#### Scenario: queue-move-bottom session rotation handled

- **WHEN** `queue_move_bottom` receives a 409 response
- **THEN** `RpcError::SessionRotated(new_id)` is returned to the caller

### Requirement: session-get queue fields

The `session_get` function SHALL parse the following additional fields from the
`session-get` response into `SessionData`:

- `download_queue_enabled: bool` (from `"download-queue-enabled"`, default `false`)
- `download_queue_size: u32` (from `"download-queue-size"`, default `0`)
- `seed_queue_enabled: bool` (from `"seed-queue-enabled"`, default `false`)
- `seed_queue_size: u32` (from `"seed-queue-size"`, default `0`)

#### Scenario: session-get populates queue fields when present

- **WHEN** the `session-get` response includes `"download-queue-enabled": true` and
  `"download-queue-size": 4`
- **THEN** `SessionData.download_queue_enabled` is `true` and
  `SessionData.download_queue_size` is `4`

#### Scenario: session-get defaults queue fields when absent

- **WHEN** the `session-get` response omits the queue fields
- **THEN** `SessionData.download_queue_enabled` is `false` and
  `SessionData.download_queue_size` is `0`

### Requirement: session-set queue fields

`SessionSetArgs` SHALL include four optional queue fields sent to the daemon via `session-set`:

- `download_queue_enabled: Option<bool>` (serialized as `"download-queue-enabled"`)
- `download_queue_size: Option<u32>` (serialized as `"download-queue-size"`)
- `seed_queue_enabled: Option<bool>` (serialized as `"seed-queue-enabled"`)
- `seed_queue_size: Option<u32>` (serialized as `"seed-queue-size"`)

Only fields with `Some(...)` values SHALL be included in the JSON payload.

#### Scenario: session-set sends only Some queue fields

- **WHEN** `SessionSetArgs { download_queue_enabled: Some(true), download_queue_size: Some(3),
seed_queue_enabled: None, seed_queue_size: None, .. }` is used
- **THEN** the POST body contains `"download-queue-enabled": true` and
  `"download-queue-size": 3`
- **THEN** the POST body does NOT contain `"seed-queue-enabled"` or `"seed-queue-size"`
