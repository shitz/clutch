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

Each torrent row SHALL be wrapped in an `iced::widget::mouse_area` that handles both left-click
selection (`on_press`) and right-click to open the context menu (`on_right_press`).

The `selected_row` container style SHALL be applied to every row whose ID is in `selected_ids`.
Rows not in `selected_ids` receive no explicit style. The list SHALL support multi-row
highlighting simultaneously.

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

#### Scenario: Right-clicking a row opens the context menu

- **WHEN** the user right-clicks a torrent row
- **THEN** the context menu is opened for that torrent at the current cursor position

#### Scenario: All selected rows highlighted simultaneously

- **WHEN** `selected_ids` contains multiple IDs
- **THEN** every matching row in the list is rendered with the `selected_row` container style

### Requirement: Delete torrent confirmation modal

When the user activates the Delete action (toolbar button or context menu), a modal overlay
dialog SHALL appear centered over the torrent list. The dialog adapts its content based on the
number of selected torrents:

**Single selection** (`selected_ids.len() == 1`):

- Title: `Delete "<torrent name>"?`
- Body: "This cannot be undone."

**Bulk selection** (`selected_ids.len() > 1`):

- Title: `Remove N selected torrents?` (where N is the count)
- Body: "This cannot be undone."

Both variants include:

- A **checkbox** labelled "Also delete local data" that controls whether the torrent's downloaded
  files are removed from disk.
- A right-aligned button row with **Cancel** (`m3_tonal_button`) and **Confirm Delete** (danger
  pill button).

The dialog SHALL block interaction with the underlying list via an `opaque` wrapping on the
overlay layer. The toolbar SHALL remain unchanged while the modal is open.

The `confirming_delete` state field SHALL change from `Option<(i64, bool)>` to
`Option<(Vec<i64>, bool)>` to support bulk targets. On confirm, the `torrent-remove` RPC is
dispatched with all IDs in the `Vec<i64>`.

#### Scenario: Modal appears on delete click (single selection)

- **WHEN** exactly one torrent is selected and the user clicks Delete
- **THEN** the confirmation modal title reads `Delete "<torrent name>"?`

#### Scenario: Modal appears on delete click (bulk selection)

- **WHEN** multiple torrents are selected and the user clicks Delete
- **THEN** the confirmation modal title reads `Remove N selected torrents?`

#### Scenario: Cancel dismisses the modal without action

- **WHEN** the confirmation modal is open and the user clicks Cancel
- **THEN** the modal is dismissed and no RPC call is issued

#### Scenario: Confirm Delete removes all targeted torrents

- **WHEN** the confirmation modal is open and the user clicks Confirm Delete
- **THEN** a `torrent-remove` RPC call is issued with all IDs in `confirming_delete`

#### Scenario: Also delete local data checkbox defaults to unchecked

- **WHEN** the confirmation modal first opens
- **THEN** the "Also delete local data" checkbox is unchecked

#### Scenario: Cancel uses tonal button style

- **WHEN** the delete confirmation modal is shown
- **THEN** the Cancel button renders with the tonal wash style

#### Scenario: Confirm Delete uses danger pill style

- **WHEN** the delete confirmation modal is shown
- **THEN** the Confirm Delete button renders with a destructive pill background
