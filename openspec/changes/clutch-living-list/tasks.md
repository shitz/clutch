## 1. Project Bootstrap

- [x] 1.1 Run `cargo new clutch` and initialise git repository
- [x] 1.2 Add dependencies to `Cargo.toml`: `iced` (features: tokio, canvas, image), `tokio` (features: full), `reqwest` (features: json), `serde` (features: derive), `serde_json`; add `wiremock` as a `[dev-dependency]`
- [x] 1.3 Verify `cargo build` compiles with an empty `main.rs`

## 2. RPC Client Module (`src/rpc.rs`)

- [x] 2.1 Define `TransmissionCredentials` struct (host, port, username, password) with `//!` module doc and `///` doc comments on the struct and all fields
- [x] 2.2 Define `RpcError` enum with variants: `SessionRotated(String)`, `AuthError`, `ConnectionError(String)`, `ParseError(String)` — add `///` doc comment per variant explaining when it is returned
- [x] 2.3 Define `TorrentData` struct with fields: `id: i64`, `name: String`, `status: i32`, `percent_done: f64` — derived `Deserialize`; document the `status` integer values (0=stopped, 4=downloading, 6=seeding) in the field doc comment
- [x] 2.4 Implement internal `post_rpc` async fn accepting a full `url: &str` (not constructed from credentials) with a given session-id; returns raw response or `Err(RpcError::SessionRotated(new_id))` on 409; add `///` doc comment describing the 409 retry contract
- [x] 2.5 Implement `session_get` async fn using `post_rpc`, mapping 401 → `AuthError` and network errors → `ConnectionError`; add `///` doc comment
- [x] 2.6 Implement `torrent_get` async fn that fetches id, name, status, percentDone for all torrents and deserialises into `Vec<TorrentData>` inside this function; add `///` doc comment noting that deserialization is intentionally done here to avoid blocking `update()`

## 3. App Skeleton and Screen Model (`src/main.rs`, `src/app.rs`)

- [x] 3.1 Define `Screen` enum with variants `Connection(ConnectionScreen)` and `Main(MainScreen)`; add `//!` module doc to `app.rs` stating the non-blocking invariant explicitly; add `///` doc comments on `Screen` and each variant
- [x] 3.2 Define top-level `App` struct holding `screen: Screen`, implement `iced::Application` with `new()` returning `Screen::Connection`
- [x] 3.3 Define `Message` enum covering all events for both screens (see design); add `///` doc comment on each variant describing what triggers it
- [x] 3.4 Implement stub `update()` and `view()` that match on `Screen` and delegate to each screen — confirm app window opens; add `///` doc comment on `update()` explicitly stating it must return in microseconds

## 4. Connection Screen (`src/screens/connection.rs`)

- [x] 4.1 Define `ConnectionScreen` struct with fields for each input value (host, port, username, password as `String`s) plus `is_connecting: bool` and `error: Option<String>`; add `//!` module doc and `///` doc comments on the struct and all fields
- [x] 4.2 Implement `view()` for the connection screen: two required text inputs (Host, Port pre-filled with defaults), two optional inputs (Username, Password masked), and a Connect button disabled when `is_connecting`; add `///` doc comment on `view()`
- [x] 4.3 Wire text input changes to update `ConnectionScreen` fields via messages
- [x] 4.4 Implement `ConnectClicked` message handler: set `is_connecting = true`, clear error, construct the RPC URL from credentials and pass it to `rpc::session_get(...)`; return `Command::perform(...)` mapping result to `SessionProbeResult`
- [x] 4.5 Implement `SessionProbeResult` handler: on success transition to `Screen::Main`; on failure set `is_connecting = false` and populate `error` with a human-readable message and log to console

## 5. Main Screen (`src/screens/main.rs`)

- [x] 5.1 Define `MainScreen` struct with `session_id: String`, `credentials: TransmissionCredentials`, `torrents: Vec<TorrentData>`, `is_loading: bool`; add `//!` module doc and `///` doc comments on the struct and fields; document the `is_loading` guard invariant
- [x] 5.2 Implement `view()` header row (Name / Status / Progress) outside the scrollable, using `Length::FillPortion` for column widths; define column weight constants with `///` doc comments
- [x] 5.3 Implement `view()` scrollable torrent rows using the same `FillPortion` constants as the header — Status maps the integer status code to a human-readable string, Progress renders as a `progress_bar`
- [x] 5.4 Implement `view()` toolbar row with Add, Pause, Resume, Delete buttons — all disabled
- [x] 5.5 Implement `iced::Subscription` returning `time::every(Duration::from_secs(5))` emitting `Message::Tick`; add `///` doc comment on `subscription()` explaining the polling interval choice
- [x] 5.6 Implement `Tick` handler: if `is_loading` is true, do nothing; otherwise set `is_loading = true`, construct the RPC URL from credentials, and return `Command::perform(rpc::torrent_get(...))`
- [x] 5.7 Implement `TorrentsUpdated` handler: replace `torrents` with new data, set `is_loading = false`
- [x] 5.8 Implement `SessionIdRotated` handler: update stored session-id in `MainScreen`, re-fire the `torrent_get` command

## 6. Unit Tests for `update()` Logic

- [x] 6.1 Test `Tick` when `is_loading = true`: assert no command is returned and state is unchanged
- [x] 6.2 Test `Tick` when `is_loading = false`: assert `is_loading` becomes `true` and a command is returned
- [x] 6.3 Test `TorrentsUpdated(Ok(data))`: assert `torrents` is replaced with new data and `is_loading` is cleared
- [x] 6.4 Test `TorrentsUpdated(Err(...))`: assert `is_loading` is cleared and an error state is set
- [x] 6.5 Test `SessionIdRotated(new_id)`: assert `session_id` field is updated to `new_id`
- [x] 6.6 Test `ConnectClicked` on `ConnectionScreen`: assert `is_connecting` becomes `true` and `error` is cleared
- [x] 6.7 Test `SessionProbeResult(Err(...))` on `ConnectionScreen`: assert `is_connecting` is cleared and `error` is populated

## 7. RPC Integration Tests (wiremock)

- [x] 7.1 Test `post_rpc` 409 handling: stub a 409 response with `X-Transmission-Session-Id: new-id`; assert `Err(RpcError::SessionRotated("new-id"))` is returned
- [x] 7.2 Test `session_get` success: stub a 200 response with valid `session-get` JSON; assert `Ok` is returned with the session-id
- [x] 7.3 Test `session_get` auth failure: stub a 401 response; assert `Err(RpcError::AuthError)` is returned
- [x] 7.4 Test `session_get` connectivity failure: use a port with no server; assert `Err(RpcError::ConnectionError(_))` is returned
- [x] 7.5 Test `torrent_get` success: stub a 200 response with a valid `torrent-get` JSON payload (2 torrents); assert `Ok(vec)` with correct `id`, `name`, `status`, `percent_done` values
- [x] 7.6 Test `torrent_get` with malformed JSON: stub a 200 response with invalid body; assert `Err(RpcError::ParseError(_))` is returned

## 8. Manual Integration Testing

- [ ] 8.1 Run the app against a real Transmission daemon; verify the connection screen pre-fills defaults and Connect transitions to the torrent list
- [ ] 8.2 Verify the list auto-refreshes every 5 seconds with updated torrent data
- [ ] 8.3 Test connection failure (wrong port): verify error appears on the connection screen with fields intact and Connect button re-enabled
- [ ] 8.4 Test auth failure (wrong credentials): verify a distinct "authentication failed" error message appears
- [ ] 8.5 Run with a large torrent list (20+ torrents) and confirm the header stays sticky while scrolling
