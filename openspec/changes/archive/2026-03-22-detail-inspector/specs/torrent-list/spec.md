## MODIFIED Requirements

### Requirement: Background polling

The app SHALL poll the Transmission daemon every **1 second** and update the torrent list in-place. Polling SHALL NOT block the UI thread.

#### Scenario: List updates without user action

- **WHEN** 1 second has elapsed since the last successful poll
- **THEN** a new `torrent-get` request is issued and the torrent list is refreshed upon response

#### Scenario: Poll skipped when call in-flight

- **WHEN** a tick fires while an RPC call is already in-flight
- **THEN** the tick is ignored and no duplicate request is issued

#### Scenario: UI remains responsive during poll

- **WHEN** a poll request is in-flight
- **THEN** the UI continues to render at full frame rate

### Requirement: Scrollable torrent rows

The list SHALL display one row per torrent, rendered inside a scrollable region. Each row SHALL include: Name (text), Status (text), and Progress (progress bar). The underlying `TorrentData` model SHALL carry the following additional fields used by the detail inspector: `total_size`, `downloaded_ever`, `uploaded_ever`, `upload_ratio`, `eta`, `rate_download`, `rate_upload`, `files`, `file_stats`, `tracker_stats`, `peers`. These fields SHALL be populated by the `torrent-get` RPC response and default to zero/empty when absent.

#### Scenario: Each torrent appears as a row

- **WHEN** the app has received torrent data from the daemon
- **THEN** one row per torrent is visible in the list

#### Scenario: Progress shown as a bar

- **WHEN** a torrent row is rendered
- **THEN** the Progress column shows a visual progress bar reflecting the torrent's percent completion

#### Scenario: Extended fields present in model

- **WHEN** `torrent-get` returns a response
- **THEN** each `TorrentData` entry includes `totalSize`, `downloadedEver`, `uploadedEver`, `uploadRatio`, `eta`, `rateDownload`, `rateUpload`, `files`, `fileStats`, `trackerStats`, and `peers` fields from the response
