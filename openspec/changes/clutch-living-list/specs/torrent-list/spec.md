## ADDED Requirements

### Requirement: Sticky column header
The torrent list SHALL display a column header row (Name, Status, Progress) that remains visible while the torrent list scrolls. The header SHALL not be part of the scrollable region.

#### Scenario: Header stays visible during scroll
- **WHEN** the torrent list contains more rows than fit on screen and the user scrolls down
- **THEN** the column header remains at the top of the list area

#### Scenario: Column widths match between header and rows
- **WHEN** the torrent list is rendered
- **THEN** each column's header cell and each corresponding data cell SHALL have the same width proportions

### Requirement: Scrollable torrent rows
The list SHALL display one row per torrent, rendered inside a scrollable region. Each row SHALL include: Name (text), Status (text), and Progress (progress bar).

#### Scenario: Each torrent appears as a row
- **WHEN** the app has received torrent data from the daemon
- **THEN** one row per torrent is visible in the list

#### Scenario: Progress shown as a bar
- **WHEN** a torrent row is rendered
- **THEN** the Progress column shows a visual progress bar reflecting the torrent's percent completion

### Requirement: Background polling
The app SHALL poll the Transmission daemon every 5 seconds and update the torrent list in-place. Polling SHALL NOT block the UI thread.

#### Scenario: List updates without user action
- **WHEN** 5 seconds have elapsed since the last successful poll
- **THEN** a new `torrent-get` request is issued and the torrent list is refreshed upon response

#### Scenario: Poll skipped when call in-flight
- **WHEN** a tick fires while an RPC call is already in-flight
- **THEN** the tick is ignored and no duplicate request is issued

#### Scenario: UI remains responsive during poll
- **WHEN** a poll request is in-flight
- **THEN** the UI continues to render at full frame rate

### Requirement: Toolbar buttons visible but disabled
The main screen SHALL render toolbar buttons for Add, Pause, Resume, and Delete. In v0.1, all buttons SHALL be permanently disabled.

#### Scenario: Toolbar rendered on main screen
- **WHEN** the torrent list screen is shown
- **THEN** Add, Pause, Resume, and Delete buttons are visible but non-interactive
