## ADDED Requirements

### Requirement: Detail inspector panel

When a torrent is selected, the main screen SHALL display a detail inspector panel below the torrent list. The inspector SHALL occupy the bottom 1/4 of the available content height, with the torrent list occupying the top 3/4. When no torrent is selected, the inspector panel SHALL NOT be rendered and the torrent list SHALL fill the full content height.

#### Scenario: Inspector appears on selection

- **WHEN** the user clicks a torrent row
- **THEN** the detail inspector panel appears below the torrent list

#### Scenario: Inspector disappears on deselection

- **WHEN** the user clicks the already-selected torrent row
- **THEN** the detail inspector panel is no longer rendered and the torrent list expands to fill the full content height

#### Scenario: Inspector panel height is proportional

- **WHEN** the inspector panel is visible
- **THEN** the torrent list occupies 3/4 and the inspector occupies 1/4 of the content height

### Requirement: Inspector tab bar

The detail inspector SHALL display a row of tab buttons at its top: **General**, **Files**, **Trackers**, **Peers**. Exactly one tab SHALL be active at a time. The active tab button SHALL be visually distinguished from inactive tabs. Clicking an inactive tab button SHALL switch the displayed content to that tab. The selected tab SHALL default to **General** each time a new torrent is selected.

#### Scenario: Tab bar is rendered

- **WHEN** the inspector panel is visible
- **THEN** four tab buttons are rendered: General, Files, Trackers, Peers

#### Scenario: Active tab is highlighted

- **WHEN** a tab is the active tab
- **THEN** its button appears visually distinct from the others (e.g. different background or text weight)

#### Scenario: Switching tabs

- **WHEN** the user clicks an inactive tab button
- **THEN** the inspector content area updates to show that tab's content

#### Scenario: Tab resets on new selection

- **WHEN** the user selects a different torrent
- **THEN** the active tab resets to General

### Requirement: General tab

The General tab SHALL display the following fields for the selected torrent: Name, Total Size, Downloaded, Uploaded, Ratio, ETA, Download Speed, Upload Speed. All sizes SHALL be formatted in human-readable units (B / KiB / MiB / GiB). Speeds SHALL be formatted with per-second units (B/s / KiB/s / MiB/s). ETA SHALL be formatted as a duration string (e.g. "2h 15m"); when ETA is unavailable (sentinel value -1), display "—". Ratio SHALL be formatted to two decimal places; when unavailable (sentinel value -1.0), display "—".

#### Scenario: General tab displays all fields

- **WHEN** the General tab is active and a torrent is selected
- **THEN** all eight fields (Name, Total Size, Downloaded, Uploaded, Ratio, ETA, Download Speed, Upload Speed) are visible

#### Scenario: ETA unavailable

- **WHEN** the torrent's ETA value is -1
- **THEN** the ETA field displays "—"

#### Scenario: Ratio unavailable

- **WHEN** the torrent's upload ratio value is -1.0
- **THEN** the Ratio field displays "—"

#### Scenario: Size formatting

- **WHEN** a size value is rendered
- **THEN** it is expressed in the largest applicable unit (GiB, MiB, KiB, or B) with up to two decimal places

### Requirement: Files tab

The Files tab SHALL display a scrollable list of files belonging to the selected torrent. Each row SHALL show the file path (relative to the torrent root) and a progress bar reflecting that file's download completion. The progress SHALL be computed as `bytes_completed / file_length`; if `file_length` is 0 the progress SHALL be shown as 1.0 (complete). The file list SHALL be scrollable when the number of files exceeds the visible area.

#### Scenario: Files tab lists all files

- **WHEN** the Files tab is active and the torrent has file data
- **THEN** one row per file is rendered, showing the file path

#### Scenario: Per-file progress bar

- **WHEN** a file row is rendered
- **THEN** a progress bar reflects the file's individual download completion fraction

#### Scenario: Fully downloaded file

- **WHEN** a file's `bytes_completed` equals its `length`
- **THEN** the progress bar is at 100%

#### Scenario: Files list is scrollable

- **WHEN** the torrent contains more files than fit in the panel
- **THEN** the file list scrolls independently to reveal all files

### Requirement: Trackers tab

The Trackers tab SHALL display a scrollable list of tracker entries for the selected torrent. Each row SHALL show: the tracker host/URL, seeder count, leecher count, and last announce time formatted as a human-readable relative time or absolute timestamp. When seeder or leecher count is -1 (unknown), the cell SHALL display "—".

#### Scenario: Trackers tab lists all trackers

- **WHEN** the Trackers tab is active and the torrent has tracker data
- **THEN** one row per tracker is rendered

#### Scenario: Unknown counts displayed as dash

- **WHEN** a tracker's seeder count or leecher count is -1
- **THEN** that count is displayed as "—"

#### Scenario: Last announce time displayed

- **WHEN** a tracker row is rendered
- **THEN** the last announce time is shown as a human-readable value

### Requirement: Peers tab

The Peers tab SHALL display a scrollable list of currently connected peers for the selected torrent. Each row SHALL show: the peer's IP address, the peer's client name string, the download rate from that peer (rate_to_client), and the upload rate to that peer (rate_to_peer), both formatted in human-readable speed units. When there are no connected peers, the tab SHALL display a "No peers connected" message.

#### Scenario: Peers tab lists all peers

- **WHEN** the Peers tab is active and there are connected peers
- **THEN** one row per peer is rendered with address, client name, and speeds

#### Scenario: No peers message

- **WHEN** the Peers tab is active and the peer list is empty
- **THEN** a message "No peers connected" is displayed

#### Scenario: Peer speeds formatted

- **WHEN** a peer row is rendered
- **THEN** download and upload rates are shown in human-readable speed units (B/s / KiB/s / MiB/s)
