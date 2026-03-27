## MODIFIED Requirements

### Requirement: Sticky column header

The torrent list SHALL display a column header row that remains visible while the torrent list scrolls. The header SHALL not be part of the scrollable region. The header SHALL contain the following columns in order: Name, Status, Size, Downloaded, ↓ Speed, ↑ Speed, ETA, Ratio, Progress. Each column header SHALL be a clickable button that cycles through sort states: ascending → descending → unsorted. The currently active sort column and direction SHALL be visually indicated in the header (e.g., an arrow glyph appended to the column label).

#### Scenario: Header stays visible during scroll

- **WHEN** the torrent list contains more rows than fit on screen and the user scrolls down
- **THEN** the column header remains at the top of the list area

#### Scenario: Column widths match between header and rows

- **WHEN** the torrent list is rendered
- **THEN** each column's header cell and each corresponding data cell SHALL have the same width proportions

#### Scenario: Clicking an unsorted column header sorts ascending

- **WHEN** the user clicks a column header that has no active sort
- **THEN** the list is sorted by that column in ascending order and the header shows an ascending indicator

#### Scenario: Clicking an ascending column header sorts descending

- **WHEN** the user clicks a column header that is currently sorted ascending
- **THEN** the list is sorted by that column in descending order and the header shows a descending indicator

#### Scenario: Clicking a descending column header clears the sort

- **WHEN** the user clicks a column header that is currently sorted descending
- **THEN** the sort is cleared and the list reverts to the daemon-returned order

#### Scenario: Only one column may be sorted at a time

- **WHEN** the user clicks a column header while another column is already sorted
- **THEN** the previously sorted column's indicator is cleared and the newly clicked column becomes the active sort column in ascending order

### Requirement: Scrollable torrent rows

The list SHALL display one row per torrent, rendered inside a scrollable region. Each row SHALL include the following columns: Name (text), Status (text), Size (human-readable bytes), Downloaded (human-readable bytes), ↓ Speed (human-readable bytes/s, shown as "—" when zero), ↑ Speed (human-readable bytes/s, shown as "—" when zero), ETA (human-readable duration, shown as "—" when not applicable), Ratio (two decimal places, shown as "—" when no data uploaded), and Progress (color-coded progress bar). All human-readable formatting SHALL use the same helper functions as the detail inspector.

#### Scenario: Each torrent appears as a row

- **WHEN** the app has received torrent data from the daemon
- **THEN** one row per torrent is visible in the list

#### Scenario: All nine columns rendered per row

- **WHEN** a torrent row is rendered
- **THEN** all nine columns (Name, Status, Size, Downloaded, ↓ Speed, ↑ Speed, ETA, Ratio, Progress) are visible with correct values

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

## ADDED Requirements

### Requirement: Column sort

The torrent list SHALL support single-column sorting on any column. The sort is applied in-memory on the currently fetched torrent data. Sort state consists of an active sort column (if any) and a sort direction (ascending or descending). Sorting SHALL be a pure view-time operation that does not modify the underlying fetched data.

#### Scenario: List is sorted when a sort is active

- **WHEN** a sort column and direction are set
- **THEN** all rows appear in the order determined by that column and direction

#### Scenario: List preserves daemon order when no sort is active

- **WHEN** no sort column is set
- **THEN** rows appear in the order returned by the most recent `torrent-get` response

#### Scenario: Sort survives a data refresh

- **WHEN** the daemon returns a new `torrent-get` response while a sort is active
- **THEN** the list is re-sorted by the active column and direction automatically
