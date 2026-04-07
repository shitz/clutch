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

## ADDED Requirements

### Requirement: Toolbar uses M3 icon button and tonal/primary styles

All icon-only toolbar action buttons (Pause, Resume, Delete, Settings, Disconnect, theme toggle) SHALL use the `icon_button` helper style (transparent background, circular hover highlight). The primary Add Torrent action SHALL use the `m3_primary_button` style. All secondary/cancel actions SHALL use the `m3_tonal_button` style.

#### Scenario: Icon buttons show circular hover highlight

- **WHEN** the user hovers over a toolbar icon button
- **THEN** a circular tinted background appears behind the icon

#### Scenario: Add Torrent button is a filled primary pill

- **WHEN** the torrent list toolbar is rendered
- **THEN** the Add Torrent button has fully rounded ends and the solid brand primary color as background

### Requirement: Empty state view

When the torrent list contains no torrents, the list area SHALL display the Clutch logo image centered at reduced opacity (~25%), accompanied by muted helper text ("No torrents. Add one with +"). The empty state SHALL not be shown while the app is still loading.

#### Scenario: Empty state shown when list is empty

- **WHEN** torrent data has been received and the list contains zero items
- **THEN** the centered logo and helper text are shown instead of an empty scroll area

### Requirement: Delete torrent confirmation modal

When the user clicks the Delete toolbar button, a modal overlay dialog SHALL appear centered over
the torrent list. The dialog SHALL contain:

- A title: `Delete "<torrent name>"?`
- A body message: "This cannot be undone."
- A **checkbox** labelled "Also delete local data" that controls whether the torrent's downloaded
  files are removed from disk.
- A right-aligned button row with **Cancel** (`m3_tonal_button`) and **Confirm Delete** (danger
  pill button).

The dialog SHALL block interaction with the underlying list via an `opaque` wrapping on the overlay
layer. The toolbar SHALL remain unchanged (showing normal toolbar icons) while the modal is open.

#### Scenario: Modal appears on delete click

- **WHEN** a torrent is selected and the user clicks the Delete button
- **THEN** the confirmation modal is shown centered over the torrent list
- **THEN** the toolbar continues to show normal icons (the modal does not replace the toolbar)

#### Scenario: Cancel dismisses the modal without action

- **WHEN** the confirmation modal is open and the user clicks Cancel
- **THEN** the modal is dismissed and no RPC call is issued

#### Scenario: Confirm Delete removes the torrent

- **WHEN** the confirmation modal is open and the user clicks Confirm Delete
- **THEN** a `torrent-remove` RPC call is issued with the selected torrent's id and the current
  state of the "Also delete local data" checkbox

#### Scenario: Also delete local data checkbox defaults to unchecked

- **WHEN** the confirmation modal first opens
- **THEN** the "Also delete local data" checkbox is unchecked

#### Scenario: Cancel uses tonal button style

- **WHEN** the delete confirmation modal is shown
- **THEN** the Cancel button renders with the tonal wash style

#### Scenario: Confirm Delete uses danger pill style

- **WHEN** the delete confirmation modal is shown
- **THEN** the Confirm Delete button renders with a destructive pill background

### Requirement: Column header tooltips

Each column header button SHALL show a tooltip (styled with `m3_tooltip`) on hover displaying the full column name.

#### Scenario: Hovering a column header shows its full name

- **WHEN** the user hovers over a column header
- **THEN** a tooltip appears with the full column name in the `m3_tooltip` elevated dark style
