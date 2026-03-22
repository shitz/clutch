## Context

Clutch currently shows a flat torrent list (Name / Status / Progress). Users wanting speed data, file breakdown, tracker health, or peer activity must leave the app. The v0.4 inspector adds a tabbed detail pane to the right of the list that shows live per-torrent detail pulled from the same `torrent-get` call the app already makes.

**Current state:**

- `TorrentData` has four fields: `id`, `name`, `status`, `percent_done`.
- `torrent-get` requests six fields: `id`, `name`, `status`, `percentDone`.
- The main screen occupies the full window below the toolbar; there is no second panel.
- The polling interval is 5 s via `iced::time::every(Duration::from_secs(5))`.
- All state, messages, and view logic for the main screen live in `main_screen.rs` as a single flat struct.

**Constraints (from system_architecture.md):**

- `update()` must return in microseconds; all I/O in the RPC worker or `Task::perform()`.
- At most one in-flight HTTP connection (serialized worker).
- No new GTK/web-view dependencies.

## Goals / Non-Goals

**Goals:**

- Display a tabbed inspector panel to the right of the torrent list when a torrent is selected.
- Inspector occupies the right 1/3 of the content area; the list shrinks to 2/3.
- When nothing is selected, the list fills the full width and the inspector is absent.
- Four tabs: General, Files, Trackers, Peers.
- Reduce polling interval to 1 s to support live speed display.
- Expand `TorrentData` with all fields needed by the inspector tabs.
- Introduce two child sub-modules (`torrent_list.rs`, `inspector.rs`) each with their own Elm loop; `MainScreen` delegates to them.

**Non-Goals:**

- Column sort or column visibility toggles (v0.5 scope).
- Per-file pause/skip controls (future scope).
- Tracker management (add/remove trackers).
- Configurable inspector width or resizable split pane.
- Persistence of the selected tab across sessions.

## Decisions

### D1 — Layout: vertical `column` split with `FillPortion`

**Decision:** Split the main content area with a `column` widget. When a torrent is selected the list region gets `FillPortion(3)` (top 3/4) and the inspector gets `FillPortion(1)` (bottom 1/4). When nothing is selected only the list is rendered (full height — no `FillPortion` needed, just `Length::Fill`).

**Alternatives considered:**

- _Fixed pixel heights_ — fragile across window sizes and DPI settings.
- _Side-by-side (right of list)_ — considered first; the split was initially implemented as a `row!` with `FillPortion(2)` / `FillPortion(1)` placing the inspector on the right 1/3, but discarded in favour of the horizontal split because more horizontal real-estate is available for the list.
- _Resizable split pane_ — iced 0.14 has no built-in splitter; implementing a draggable handle adds significant complexity out of scope for this milestone.

### D2 — Separate Elm sub-modules: `TorrentListScreen` and `InspectorScreen`

**Decision:** Extract the torrent list and the inspector into independent sub-modules, each owning their state, message enum, `update()`, and `view()`:

```
src/screens/
├── main_screen.rs   — MainScreen: owns TorrentListScreen + InspectorScreen, routes messages
├── torrent_list.rs  — TorrentListScreen + TorrentListMessage + update + view
└── inspector.rs     — InspectorScreen + InspectorMessage + update + view
```

`MainScreen::Message` wraps child messages:

```rust
pub enum Message {
    List(TorrentListMessage),
    Inspector(InspectorMessage),
    // subscription-level: Tick, SessionIdRotated, RpcWorkerReady,
    // TorrentsUpdated, ActionCompleted, AddCompleted  → routed as List(...)
    Disconnect,
}
```

`MainScreen::view()` composes the two children:

```rust
let list_elem = torrent_list::view(&self.list).map(Message::List);
match self.list.selected_torrent() {
    None    => list_elem,
    Some(t) => column![
        container(list_elem).height(FillPortion(3)).width(Fill),
        container(inspector::view(&self.inspector, t).map(Message::Inspector))
            .height(FillPortion(1)).width(Fill),
    ].into(),
}
```

`MainScreen::update()` delegates:

```rust
Message::List(msg)      => torrent_list::update(&mut self.list, msg).map(Message::List),
Message::Inspector(msg) => inspector::update(&mut self.inspector, msg).map(Message::Inspector),
```

Subscription-sourced results (`TorrentsUpdated`, `RpcWorkerReady`, `SessionIdRotated`, `ActionCompleted`, `AddCompleted`) are mapped to `Message::List(TorrentListMessage::...)` at the subscription boundary so they flow into `TorrentListScreen::update()`.

**Alternatives considered:**

- _Flat state in `MainScreen`_ — the original approach; rejected because it puts all state and logic in one growing struct and does not satisfy the separate-Elm-architectures requirement.
- _Inspector owns a visibility flag_ — couples the inspector to selection logic that belongs in the list or parent.

### D3 — Data model: flat `TorrentData` with `Option<Vec<_>>` sub-collections

**Decision:** Extend `TorrentData` directly with:

- Scalar fields: `total_size: i64`, `downloaded_ever: i64`, `uploaded_ever: i64`, `upload_ratio: f64`, `eta: i64`, `rate_download: i64`, `rate_upload: i64` — all `#[serde(default)]` so old responses or removed torrents do not break parsing.
- Vec fields: `files: Vec<TorrentFile>`, `file_stats: Vec<TorrentFileStats>`, `tracker_stats: Vec<TrackerStat>`, `peers: Vec<PeerInfo>` — all `#[serde(default)]` yielding empty vecs when absent.

New sub-types added to `rpc.rs`:

```
TorrentFile      { name: String, length: i64 }
TorrentFileStats { bytes_completed: i64 }
TrackerStat      { host: String, seeder_count: i32, leecher_count: i32, last_announce_time: i64 }
PeerInfo         { address: String, client_name: String, rate_to_client: i64, rate_to_peer: i64 }
```

**Alternatives considered:**

- _Separate RPC call for detail data_ — would require a new `torrent-get-detail` path through the worker, adding two round-trips and complicating the worker's result types. Folding the fields into the existing poll is simpler and the payload increase is negligible.
- _`Option<TorrentDetail>` as a separate struct_ — adds indirection without benefit since the inspector is always shown for the selected torrent that is already in the list.

### D4 — Polling interval: change `every(5s)` to `every(1s)`

**Decision:** Change the `iced::time::every` duration from 5 s to 1 s globally (not conditionally). The `is_loading` guard already prevents queue saturation.

**Alternatives considered:**

- _Conditional interval based on inspector visibility_ — plausible but requires the subscription to observe `selected_id`, and iced subscriptions are recreated from scratch on state change anyway; the 1 s flat interval is simpler.
- _Keep 5 s and add a separate speed-only subscription_ — adds a second worker path for no benefit.

### D5 — Inspector only renders when a torrent is selected

**Decision:** `TorrentListScreen` exposes a `selected_torrent() -> Option<&TorrentData>` method. `MainScreen::view()` branches: when `None` the list fills the full width; when `Some(t)` the horizontal split is composed with `FillPortion(2)` / `FillPortion(1)`. `InspectorScreen` itself has no concept of "hidden" — it is simply not rendered by the parent.

**Alternatives considered:**

- _Always show inspector with placeholder_ — wastes horizontal space and makes the list cramped for users who have not selected anything.

### D6 — Human-readable formatting helpers (module-private functions in `inspector.rs`)

**Decision:** Add small private formatting helpers inside `inspector.rs`:

- `format_size(bytes: i64) -> String` — formats bytes as B / KiB / MiB / GiB.
- `format_speed(bytes_per_sec: i64) -> String` — formats as B/s / KiB/s / MiB/s.
- `format_eta(secs: i64) -> String` — formats as "∞", "Xs", "Xm Xs", "Xh Xm".

No new crate dependency needed.

## Risks / Trade-offs

- **Payload size increase** — requesting `files`, `fileStats`, `peers`, `trackerStats` on every poll for every torrent returns significantly more data than before. For daemons managing hundreds of torrents with many peers, this could become a large JSON payload. _Mitigation:_ Transmission's RPC allows scoping `torrent-get` by IDs; a future optimization could fetch detail fields only for the selected torrent. For v0.4, the full-list approach is simpler and acceptable for typical home-use daemon sizes.
- **1 s polling and battery / CPU** — polling every second instead of every 5 s increases CPU usage slightly on low-power devices. _Mitigation:_ iced's subscription reuse and the `is_loading` guard prevent runaway requests; impact is minimal for a desktop app.
- **`eta = -1` sentinel** — Transmission returns `eta: -1` when ETA is unknown. `format_eta` must handle this explicitly and render "—" rather than a nonsensical duration. _Mitigation:_ covered by `format_eta` implementation.
- **`upload_ratio = -1.0` sentinel** — similar to ETA; must render "—". _Mitigation:_ handled in the General tab renderer.

- **Message re-mapping at subscription boundary** — subscription callbacks must wrap their output in `Message::List(TorrentListMessage::...)`. Forgetting this would leave `TorrentListScreen` state stale. _Mitigation:_ covered by test 11.1 which validates RPC results reach list state.

## Open Questions

None — the v0.4 scope is fully specified in `system_architecture.md`. Implementation can proceed directly from the tasks.
