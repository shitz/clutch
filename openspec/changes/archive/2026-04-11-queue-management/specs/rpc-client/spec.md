## ADDED Requirements

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

- **WHEN** `SessionSetArgs { download_queue_enabled: Some(true), download_queue_size: Some(3), seed_queue_enabled: None, seed_queue_size: None, .. }` is used
- **THEN** the POST body contains `"download-queue-enabled": true` and
  `"download-queue-size": 3`
- **THEN** the POST body does NOT contain `"seed-queue-enabled"` or `"seed-queue-size"`
