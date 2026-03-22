## Why

The torrent list shows only name, status, and progress — there is no way to see transfer speeds, file breakdown, tracker health, or peer activity without switching to the Transmission web UI. The detail inspector closes this gap by surfacing per-torrent detail directly inside Clutch, making the app fully self-contained for day-to-day monitoring.

## What Changes

- A detail panel appears below the torrent list when a torrent is selected, replacing the current empty space below the list.
- The panel is split into four tabs: **General**, **Files**, **Trackers**, **Peers**.
- **General** tab shows: name, total size, downloaded, uploaded, ratio, ETA, download speed, upload speed.
- **Files** tab shows: per-file path and a progress bar derived from `fileStats`.
- **Trackers** tab shows: URL, seeder count, leecher count, last announce time.
- **Peers** tab shows: IP address, client string, per-peer download and upload rate.
- The polling interval drops from 5 s to 1 s once the main screen is active (needed for live speed display).
- `torrent-get` field list expands to include `totalSize`, `downloadedEver`, `uploadedEver`, `uploadRatio`, `eta`, `rateDownload`, `rateUpload`, `files`, `fileStats`, `trackerStats`, `peers`.
- `TorrentData` struct gains all new fields; the `torrent-list` spec is updated to reflect the richer data model.

## Capabilities

### New Capabilities

- `detail-inspector`: A tabbed detail panel shown below the torrent list when a torrent is selected, displaying general stats, files, trackers, and peers.

### Modified Capabilities

- `torrent-list`: The `TorrentData` model gains new fields (`totalSize`, `downloadedEver`, `uploadedEver`, `uploadRatio`, `eta`, `rateDownload`, `rateUpload`, `files`, `fileStats`, `trackerStats`, `peers`) and the polling interval changes from 5 s to 1 s.

## Impact

- `src/rpc.rs`: `torrent_get` field list expands; `TorrentData` struct gains new fields; new sub-types for file, tracker, and peer data.
- `src/screens/main_screen.rs`: Layout split into list + inspector pane; tab state added to `MainScreen`; polling ticker interval changes from 5 s to 1 s.
- No new crate dependencies required.
- No breaking changes to the RPC worker or connection screen.
