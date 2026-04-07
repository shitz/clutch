## MODIFIED Requirements

### Requirement: Scrollable torrent rows

The list SHALL display one row per torrent that matches the current active filter set, rendered
inside a scrollable region. When no filter is active for a given torrent, that torrent SHALL be
omitted from the rendered rows. Each row SHALL include the following columns: Name (text), Status
(text), Size (human-readable bytes), Downloaded (human-readable bytes), ↓ Speed (human-readable
bytes/s, shown as "—" when zero), ↑ Speed (human-readable bytes/s, shown as "—" when zero), ETA
(human-readable duration, shown as "—" when not applicable), Ratio (two decimal places, shown as
"—" when no data uploaded), and Progress (color-coded progress bar). All human-readable formatting
SHALL use the same helper functions as the detail inspector.

#### Scenario: Each matching torrent appears as a row

- **WHEN** the app has received torrent data from the daemon
- **THEN** one row per torrent that matches the active filter set is visible in the list

#### Scenario: All nine columns rendered per row

- **WHEN** a torrent row is rendered
- **THEN** all nine columns (Name, Status, Size, Downloaded, ↓ Speed, ↑ Speed, ETA, Ratio,
  Progress) are visible with correct values

#### Scenario: Progress bar is green while downloading

- **WHEN** a torrent's status is Downloading (status = 4)
- **THEN** the progress bar fill color is green

#### Scenario: Progress bar is blue while seeding

- **WHEN** a torrent's status is Seeding (status = 6)
- **THEN** the progress bar fill color is blue

#### Scenario: Progress bar is gray when paused or stopped

- **WHEN** a torrent's status is Stopped (status = 0) or any non-active state
- **THEN** the progress bar fill color is gray

#### Scenario: Zero transfer speeds shown as dash

- **WHEN** a torrent's download or upload rate is 0 bytes/s
- **THEN** the corresponding speed column displays "—" instead of "0 B/s"

#### Scenario: ETA shown as dash when not downloading

- **WHEN** a torrent is not actively downloading or ETA is -1
- **THEN** the ETA column displays "—"

#### Scenario: Ratio shown as dash when nothing has been uploaded

- **WHEN** a torrent's upload ratio is -1 or 0 with no data ever uploaded
- **THEN** the Ratio column displays "—"

#### Scenario: Filtered-out torrent not rendered

- **WHEN** a torrent's status bucket does not match any chip in the active filter set
- **THEN** no row is rendered for that torrent
