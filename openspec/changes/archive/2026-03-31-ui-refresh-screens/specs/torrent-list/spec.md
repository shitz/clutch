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

The list SHALL display one row per torrent, rendered inside a scrollable region. Each row SHALL include all nine data columns. Row vertical padding SHALL be at least 10 px (top + bottom combined). Row spacing in the list SHALL be at least 4 px. All human-readable formatting SHALL use the same helper functions as the detail inspector.

#### Scenario: Each torrent appears as a row

- **WHEN** the app has received torrent data from the daemon
- **THEN** one row per torrent is visible in the list

#### Scenario: All nine columns rendered per row

- **WHEN** a torrent row is rendered
- **THEN** all nine columns (Name, Status, Size, Downloaded, ↓ Speed, ↑ Speed, ETA, Ratio, Progress) are visible with correct values

#### Scenario: Rows have comfortable vertical spacing

- **WHEN** the torrent list is rendered
- **THEN** row padding provides at least 10 px of vertical space per row and rows are visually separated

#### Scenario: Progress bar is green while downloading

- **WHEN** a torrent's status is Downloading (status = 4)
- **THEN** the progress bar fill color is green

#### Scenario: Progress bar is blue while seeding

- **WHEN** a torrent's status is Seeding (status = 6)
- **THEN** the progress bar fill color is blue

#### Scenario: Progress bar is gray when paused or stopped

- **WHEN** a torrent's status is Stopped (status = 0) or any non-active state
- **THEN** the progress bar fill color is gray

#### Scenario: Progress bar has rounded ends

- **WHEN** any progress bar is rendered in a torrent row
- **THEN** both the track background and the filled bar have fully rounded corners (radius 100.0)

#### Scenario: Zero transfer speeds shown as dash

- **WHEN** a torrent's download or upload rate is 0 bytes/s
- **THEN** the corresponding speed column displays "—" instead of "0 B/s"

#### Scenario: ETA shown as dash when not downloading

- **WHEN** a torrent is not actively downloading or ETA is -1
- **THEN** the ETA column displays "—"

#### Scenario: Ratio shown as dash when nothing has been uploaded

- **WHEN** a torrent's upload ratio is -1 or 0 with no data ever uploaded
- **THEN** the Ratio column displays "—"

### Requirement: Toolbar uses M3 icon button and tonal/primary styles

All icon-only toolbar action buttons (Pause, Resume, Delete, Settings, Disconnect, theme toggle) SHALL use the `icon_button` helper style (transparent background, circular hover highlight). The primary Add Torrent action SHALL use the `m3_primary_button` style. All secondary/cancel actions SHALL use the `m3_tonal_button` style.

#### Scenario: Icon buttons show circular hover highlight

- **WHEN** the user hovers over a toolbar icon button
- **THEN** a circular tinted background appears behind the icon

#### Scenario: Add Torrent button is a filled primary pill

- **WHEN** the torrent list toolbar is rendered
- **THEN** the Add Torrent button has fully rounded ends and the solid brand primary color as background

## ADDED Requirements

### Requirement: Empty state view

When the torrent list contains no torrents, the list area SHALL display the Clutch logo image centered and desaturated (rendered at reduced opacity, approximately 25%), accompanied by muted helper text below it (e.g., "No torrents. Add one with +"). The empty state SHALL not be shown while the app is still loading torrent data.

#### Scenario: Empty state shown when list is empty

- **WHEN** torrent data has been received and the list contains zero items
- **THEN** the centered logo and helper text are shown instead of an empty scroll area

#### Scenario: Empty state not shown during loading

- **WHEN** the app is waiting for the first torrent list response
- **THEN** no empty state is shown (normal loading behavior applies)

### Requirement: M3-styled delete confirmation dialog

The delete torrent confirmation dialog SHALL use `m3_tonal_button` for the "Cancel" action and a danger-colored pill button for the "Delete" action. The button row SHALL be right-aligned. No iced built-in button styles SHALL be used in this dialog.

#### Scenario: Cancel uses tonal button style

- **WHEN** the delete confirmation dialog is shown
- **THEN** the Cancel button renders with the tonal wash style

#### Scenario: Delete uses danger pill style

- **WHEN** the delete confirmation dialog is shown
- **THEN** the Delete button renders with a destructive (danger-red) pill background

### Requirement: Column header tooltips

Each column header button SHALL show a tooltip (styled with `m3_tooltip`) on hover that displays the full, unabbreviated column name (e.g., "Downloaded", "↓ Download Speed", "Upload Speed", "Time Remaining").

#### Scenario: Hovering a column header shows its full name

- **WHEN** the user hovers over a column header
- **THEN** a tooltip appears with the full column name in the `m3_tooltip` dark elevated style


#### Scenario: Empty state disappears when torrents are added

- **WHEN** the torrent list transitions from empty to containing at least one torrent
- **THEN** the empty state view is replaced by the normal torrent list rows
