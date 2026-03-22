## 1. Data Model — rpc.rs

- [x] 1.1 Add `TorrentFile` struct with fields `name: String`, `length: i64` (serde rename: `name`, `length`)
- [x] 1.2 Add `TorrentFileStats` struct with field `bytes_completed: i64` (serde rename: `bytesCompleted`)
- [x] 1.3 Add `TrackerStat` struct with fields `host: String`, `seeder_count: i32`, `leecher_count: i32`, `last_announce_time: i64` (serde renames: `host`, `seederCount`, `leecherCount`, `lastAnnounceTime`)
- [x] 1.4 Add `PeerInfo` struct with fields `address: String`, `client_name: String`, `rate_to_client: i64`, `rate_to_peer: i64` (serde renames: `address`, `clientName`, `rateToClient`, `rateToPeer`)
- [x] 1.5 Extend `TorrentData` with new scalar fields (`total_size`, `downloaded_ever`, `uploaded_ever`, `upload_ratio`, `eta`, `rate_download`, `rate_upload`) all with `#[serde(default)]` and appropriate serde renames
- [x] 1.6 Extend `TorrentData` with new vec fields (`files: Vec<TorrentFile>`, `file_stats: Vec<TorrentFileStats>`, `tracker_stats: Vec<TrackerStat>`, `peers: Vec<PeerInfo>`) all with `#[serde(default)]`

## 2. RPC — torrent-get field list

- [x] 2.1 Update the `torrent_get` field list constant/array to include all new fields: `totalSize`, `downloadedEver`, `uploadedEver`, `uploadRatio`, `eta`, `rateDownload`, `rateUpload`, `files`, `fileStats`, `trackerStats`, `peers`
- [x] 2.2 Add integration tests in `rpc.rs` for `torrent_get` that verify the new fields deserialize correctly from a mock JSON response

## 3. Polling Interval

- [x] 3.1 Change `iced::time::every(Duration::from_secs(5))` to `iced::time::every(Duration::from_secs(1))` in `main_screen.rs`
- [x] 3.2 Update the relevant unit test in `main_screen.rs` that asserts the polling interval (if one exists)

## 4. `InspectorScreen` sub-module — `src/screens/inspector.rs`

- [x] 4.1 Create `src/screens/inspector.rs` with `pub struct InspectorScreen { active_tab: ActiveTab }` and `pub fn new() -> InspectorScreen`
- [x] 4.2 Add `#[derive(Debug, Clone, Copy, PartialEq, Default)] pub enum ActiveTab { #[default] General, Files, Trackers, Peers }` inside `inspector.rs`
- [x] 4.3 Add `pub enum InspectorMessage { TabSelected(ActiveTab) }` inside `inspector.rs`
- [x] 4.4 Implement `pub fn update(state: &mut InspectorScreen, msg: InspectorMessage) -> Task<InspectorMessage>`: on `TabSelected(tab)` set `state.active_tab = tab` and return `Task::none()`
- [x] 4.5 Implement `pub fn view<'a>(state: &InspectorScreen, torrent: &'a TorrentData) -> Element<'a, InspectorMessage>`: renders tab bar + active tab content (stubs for now — filled in sections 7–10)
- [x] 4.6 Register `inspector` in `src/screens/mod.rs`

## 5. `TorrentListScreen` sub-module — `src/screens/torrent_list.rs`

- [x] 5.1 Create `src/screens/torrent_list.rs`; move all list-specific state out of `MainScreen`: `torrents: Vec<TorrentData>`, `selected_id: Option<i64>`, `is_loading: bool`, `sender: Option<Sender<RpcWork>>`, `confirming_delete: Option<(i64, bool)>`, `add_dialog: AddDialogState`
- [x] 5.2 Define `pub enum TorrentListMessage` containing all former `MainScreen`-level messages: `TorrentsUpdated`, `RpcWorkerReady`, `SessionIdRotated`, `ActionCompleted`, `AddCompleted`, `TorrentSelected`, `Pause/Resume/Delete/DeleteLocalDataToggled/DeleteConfirmed/DeleteCancelled`, `AddTorrentClicked`, `TorrentFileRead`, `AddLinkClicked`, `AddDialogMagnetChanged`, `AddDialogDestinationChanged`, `AddConfirmed`, `AddCancelled`
- [x] 5.3 Implement `pub fn update(state: &mut TorrentListScreen, msg: TorrentListMessage) -> Task<TorrentListMessage>`: move all existing message-handling logic from `MainScreen::update()` here
- [x] 5.4 Implement `pub fn view(state: &TorrentListScreen) -> Element<'_, TorrentListMessage>`: move all existing list+toolbar view logic from `MainScreen::view()` here
- [x] 5.5 Add `pub fn selected_torrent(&self) -> Option<&TorrentData>` method: returns the `TorrentData` for `selected_id` if present
- [x] 5.6 Reset `inspector.active_tab` to `ActiveTab::General` in the `TorrentSelected` handler when changing to a different torrent — NOTE: since the inspector is a sibling not a child, the actual reset is performed in `MainScreen::update()` after delegating the `List(TorrentSelected)` message
- [x] 5.7 Register `torrent_list` in `src/screens/mod.rs`

## 6. Refactor `MainScreen` to delegate to children

- [x] 6.1 Redefine `MainScreen` struct to own `list: TorrentListScreen` and `inspector: InspectorScreen` (remove all fields that moved to children)
- [x] 6.2 Redefine `MainScreen::Message` (in `app.rs` or `main_screen.rs`) as `List(TorrentListMessage)`, `Inspector(InspectorMessage)`, `Disconnect`
- [x] 6.3 Update `app.rs` `Message` enum: replace all former main-screen message variants with `Main(main_screen::Message)` or keep the wrapper pattern matching the existing app delegation style
- [x] 6.4 Implement `MainScreen::update()`: delegate `List(msg)` via `torrent_list::update(&mut self.list, msg).map(Message::List)`; delegate `Inspector(msg)` via `inspector::update(&mut self.inspector, msg).map(Message::Inspector)`; on `List(TorrentSelected(id))` also reset `self.inspector.active_tab` to `General`
- [x] 6.5 Update subscription wiring in `app.rs`/`main_screen.rs` so that `RpcWorkerReady`, `TorrentsUpdated`, `SessionIdRotated`, `ActionCompleted`, `AddCompleted` are emitted as `Message::List(TorrentListMessage::...)`
- [x] 6.6 Implement `MainScreen::view()`: build `list_elem = torrent_list::view(&self.list).map(Message::List)`; when `self.list.selected_torrent()` is `None` return `list_elem`; when `Some(t)` return a `row!` with `container(list_elem).width(FillPortion(2))` and `container(inspector::view(&self.inspector, t).map(Message::Inspector)).width(FillPortion(1))`

## 7. Formatting Helpers — inspector.rs

- [x] 7.1 Implement private `fn format_size(bytes: i64) -> String` (thresholds: ≥ GiB, ≥ MiB, ≥ KiB, else B; two decimal places)
- [x] 7.2 Implement private `fn format_speed(bps: i64) -> String` (same thresholds, append "/s")
- [x] 7.3 Implement private `fn format_eta(secs: i64) -> String` (returns "—" for -1, otherwise "Xs" / "Xm Xs" / "Xh Xm")
- [x] 7.4 Add unit tests for all three formatters (edge cases: -1 sentinel, 0, exact boundary values, GiB-scale)

## 8. General Tab — inspector.rs view()

- [x] 8.1 Render a two-column label/value grid for: Name, Total Size, Downloaded, Uploaded, Ratio, ETA, Download Speed, Upload Speed
- [x] 8.2 Apply `format_size` for Total Size, Downloaded, Uploaded; `format_speed` for Download Speed and Upload Speed; `format_eta` for ETA; ratio to two decimal places (or "—")

## 9. Files Tab — inspector.rs view()

- [x] 9.1 Render a scrollable list; each row: file path (left) + progress bar (right) with fraction = `bytes_completed / length` (clamp to [0.0, 1.0]; treat length=0 as 1.0)
- [x] 9.2 Show an empty-state message if `files` is empty

## 10. Trackers Tab — inspector.rs view()

- [x] 10.1 Render a scrollable list; each row: host, seeder count (or "—" if -1), leecher count (or "—" if -1), last announce time formatted as a timestamp string (Unix epoch → human-readable)
- [x] 10.2 Show an empty-state message if `tracker_stats` is empty

## 11. Peers Tab — inspector.rs view()

- [x] 11.1 Render a scrollable list; each row: address, client name, `format_speed(rate_to_client)` (↓), `format_speed(rate_to_peer)` (↑)
- [x] 11.2 Show "No peers connected" message when `peers` is empty

## 12. Tests

- [x] 12.1 Add unit test in `torrent_list.rs`: selecting a torrent sets `selected_id`; `selected_torrent()` returns the matching `TorrentData`
- [x] 12.2 Add unit test in `inspector.rs`: `TabSelected` updates `active_tab`
- [x] 12.3 Add unit test in `main_screen.rs`: delegating `List(TorrentSelected(id))` resets `inspector.active_tab` to `General`
- [x] 12.4 Add unit test in `main_screen.rs`: when `selected_torrent()` returns `None`, view composes only the list (no inspector element); when `Some`, both panes are present
- [x] 12.5 Run `cargo test` and confirm all tests pass
- [x] 12.6 Run `cargo clippy -- -D warnings` and resolve any warnings
