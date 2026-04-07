## Why

As a user's torrent library grows, a flat unfiltered list becomes difficult to navigate. Users
need a way to quickly isolate active downloads, seeding torrents, or torrents in an error state
without sacrificing screen real estate or discoverability.

## What Changes

- Add a horizontal row of M3 Filter Chips between the main toolbar and the torrent list.
- Support multi-select filtering: users can view multiple status categories simultaneously.
- Display real-time torrent counts embedded inside each chip (e.g., `Downloading 3`).
- Include an "All" master chip to reset/toggle the entire filter state.
- Consolidate Transmission's 7 granular integer statuses into 5 semantic UI buckets:
  `Downloading`, `Seeding`, `Paused`, `Active` (derived from rate > 0), and `Checking/Error`.
- Filtering is strictly client-side with zero additional RPC overhead.
- Show a "No torrents match the selected filters." placeholder when the filtered list is empty.

## Capabilities

### New Capabilities

- `torrent-status-filtering`: Filter chip row UI widget with multi-select, dynamic counts, and
  "All" master chip logic for filtering the torrent list by consolidated status buckets.

### Modified Capabilities

- `torrent-list`: The torrent list view gains a filter chip row above it and the rendered rows
  are now filtered against the active `HashSet<StatusFilter>` before display.

## Impact

- `src/screens/torrent_list/` — new filter state, view helper for the chip row, update handler
  for toggle messages.
- `src/screens/torrent_list/view.rs` — integrate chip row above list headers; apply filter pass
  before row rendering.
- `src/screens/torrent_list/update.rs` — handle `ToggleFilter` message.
- `src/theme.rs` — may need filter chip style helpers if not covered by existing segmented
  control helpers.
- No new RPC calls or external dependencies required.
