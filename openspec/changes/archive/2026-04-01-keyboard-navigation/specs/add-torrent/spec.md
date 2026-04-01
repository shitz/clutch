## MODIFIED Requirements

### Requirement: Add-torrent dialog

Both add flows SHALL converge on a single modal dialog overlaid on the main screen.
The dialog SHALL contain:

- A **destination folder** text input (empty by default; an empty value means the daemon
  uses its configured default download directory).
- In magnet mode: a **magnet URI** text input above the destination field.
- A **file list** showing name and size for each file in the torrent.
- An **Add** button and a **Cancel** button.

The dialog SHALL block interaction with the torrent list and action buttons beneath it.

Each text input in the dialog SHALL have a stable `text_input::Id`. The Tab ring order
SHALL be:

- _Magnet mode_: Magnet URI → Destination → (wrap to Magnet URI)
- _File mode_: Destination → (single-field ring)

When the dialog opens, the first empty text input in the ring SHALL receive automatic
focus via a `text_input::focus(id)` Task returned from the `update()` call that
transitions to the dialog state.

Pressing **Enter** while the dialog is open SHALL trigger the **Add** action (same as
clicking the Add button), unless the appropriate guard conditions for an empty add fail
(e.g., empty magnet field).

#### Scenario: Dialog shown after file selection

- **WHEN** the user selects a `.torrent` file in the file picker
- **THEN** the add-torrent dialog opens showing the parsed file list with file names and
  sizes
- **THEN** the destination folder field is empty and receives automatic focus

#### Scenario: Dialog shown after magnet input

- **WHEN** the user clicks "Add Link" and the dialog opens in magnet mode
- **THEN** the dialog shows a magnet URI text input and the destination field
- **THEN** the magnet URI field is empty and receives automatic focus
- **THEN** the file list area displays a note that file metadata is unavailable for
  magnet links

#### Scenario: User cancels the dialog

- **WHEN** the user clicks Cancel in the dialog
- **THEN** the dialog is dismissed and no RPC call is issued
- **THEN** the torrent list is unchanged

#### Scenario: Enter confirms the Add action

- **WHEN** the add-torrent dialog is open
- **AND** the Add button's guard conditions are met (non-empty magnet URI for magnet mode;
  parsed metainfo present for file mode)
- **AND** the user presses Enter (no Ctrl or Alt modifier)
- **THEN** the Add action is triggered as if the Add button was clicked

#### Scenario: Enter is ignored when guard conditions are unmet

- **WHEN** the add-torrent dialog is open in magnet mode
- **AND** the magnet URI field is empty
- **AND** the user presses Enter
- **THEN** no RPC call is issued

#### Scenario: Tab ring cycles through magnet-mode fields

- **WHEN** the dialog is open in magnet mode
- **THEN** pressing Tab advances focus Magnet URI → Destination → Magnet URI

#### Scenario: Auto-focus magnet field on dialog open (magnet mode)

- **WHEN** the add-link dialog opens
- **THEN** the magnet URI text input is automatically focused

#### Scenario: Auto-focus destination field on dialog open (file mode)

- **WHEN** the add-torrent dialog opens after a file is selected
- **THEN** the destination folder text input is automatically focused
