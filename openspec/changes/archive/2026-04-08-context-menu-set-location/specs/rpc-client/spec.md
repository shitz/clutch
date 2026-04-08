## ADDED Requirements

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
