## ADDED Requirements

### Requirement: Torrent row selection

The torrent list SHALL support single-torrent selection by clicking a row. Clicking a selected row
SHALL deselect it. At most one torrent SHALL be selected at a time. The selected row SHALL be
visually distinguished from unselected rows.

#### Scenario: Clicking an unselected row selects it

- **WHEN** the user clicks a torrent row that is not currently selected
- **THEN** that row becomes selected and is visually highlighted

#### Scenario: Clicking the selected row deselects it

- **WHEN** the user clicks the torrent row that is currently selected
- **THEN** the row becomes deselected and no row is highlighted

#### Scenario: Only one row selected at a time

- **WHEN** the user clicks a second torrent row while another is already selected
- **THEN** the previously selected row becomes deselected and the clicked row becomes selected

## MODIFIED Requirements

### Requirement: Toolbar buttons visible but disabled

The main screen SHALL render toolbar action buttons for Pause, Resume, and Delete. Button enabled
state SHALL be derived from the current selection and the selected torrent's status:

- **Pause** SHALL be enabled when a torrent with status Downloading (4), Seeding (6),
  QueuedForDownload (3), or QueuedForSeeding (5) is selected.
- **Resume** SHALL be enabled when a torrent with status Stopped (0) is selected.
- **Delete** SHALL be enabled when any torrent is selected.

All action buttons SHALL be disabled when no torrent is selected.

#### Scenario: No selection — all action buttons disabled

- **WHEN** no torrent row is selected
- **THEN** Pause, Resume, and Delete buttons are all disabled and non-interactive

#### Scenario: Active torrent selected — Pause enabled

- **WHEN** a torrent with status Downloading or Seeding is selected
- **THEN** the Pause button is enabled and Resume is disabled

#### Scenario: Stopped torrent selected — Resume enabled

- **WHEN** a torrent with status Stopped is selected
- **THEN** the Resume button is enabled and Pause is disabled

#### Scenario: Any torrent selected — Delete enabled

- **WHEN** any torrent is selected regardless of status
- **THEN** the Delete button is enabled
