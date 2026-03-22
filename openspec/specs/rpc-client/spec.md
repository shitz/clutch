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

