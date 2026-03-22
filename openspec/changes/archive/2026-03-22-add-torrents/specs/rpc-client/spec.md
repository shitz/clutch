## ADDED Requirements

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
