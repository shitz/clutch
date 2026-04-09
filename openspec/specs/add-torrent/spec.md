# add-torrent Specification

## Purpose

TBD - created by archiving change add-torrents. Update Purpose after archive.

## Requirements

### Requirement: Add Torrent toolbar entry points

The main-screen toolbar SHALL provide two buttons: **Add Torrent** and **Add Link**. These are
always visible and do not require a torrent to be selected.

#### Scenario: Add Torrent button clicked

- **WHEN** the user clicks "Add Torrent"
- **THEN** a native file picker dialog filtered to `.torrent` files is opened

#### Scenario: Add Link button clicked

- **WHEN** the user clicks "Add Link"
- **THEN** the add-torrent dialog opens in magnet-input mode

### Requirement: Add-torrent dialog

Both add flows SHALL converge on a single modal dialog overlaid on the main screen. The dialog
SHALL contain:

- A **destination folder** text input (empty by default; an empty value means the daemon uses its
  configured default download directory).
- In magnet mode: a **magnet URI** text input above the destination field.
- A **file list** showing a checkbox, name, and size for each file in the torrent. All checkboxes
  SHALL be checked by default. The file list is only shown in file mode; magnet mode shows a
  static note that file metadata is unavailable.
- An **Add** button and a **Cancel** button.

The dialog SHALL block interaction with the torrent list and action buttons beneath it.

Each text input in the dialog SHALL have a stable widget ID. The Tab ring order SHALL be:

- _Magnet mode_: Magnet URI → Destination → (wrap to Magnet URI)
- _File mode_: Destination → (single-field ring)

When the dialog opens, the first empty text input in the ring SHALL receive automatic focus via a
`focus(id)` Task returned from the `update()` call that transitions to the dialog state.

Pressing **Enter** while the dialog is open SHALL trigger the **Add** action (same as clicking the
Add button), unless the guard conditions are unmet (e.g. empty magnet field).

#### Scenario: Dialog shown after file selection

- **WHEN** the user selects a `.torrent` file in the file picker
- **THEN** the add-torrent dialog opens showing the parsed file list with a checkbox, file name, and size per row
- **THEN** all file checkboxes are checked by default
- **THEN** the destination folder field is empty and receives automatic focus

#### Scenario: Dialog shown after magnet input

- **WHEN** the user clicks "Add Link" and the dialog opens in magnet mode
- **THEN** the dialog shows a magnet URI text input and the destination field
- **THEN** the magnet URI field is empty and receives automatic focus
- **THEN** the file list area displays a note that file metadata is unavailable for magnet links

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

### Requirement: Destination folder input

The destination folder field SHALL accept a free-text path. If the field is left empty, the daemon
SHALL use its own configured default download directory.

#### Scenario: User provides a destination

- **WHEN** the user types a path into the destination field and clicks Add
- **THEN** `torrent-add` is called with `{ "download-dir": "<path>", ... }`

#### Scenario: User leaves destination empty

- **WHEN** the destination field is empty and the user clicks Add
- **THEN** `torrent-add` is called without a `download-dir` field

### Requirement: .torrent file flow

When the user selects a `.torrent` file, the app SHALL parse it locally to extract the file list
before opening the dialog. No RPC call is made at this stage.

#### Scenario: File picked and parsed

- **WHEN** the user selects a `.torrent` file from the file picker
- **THEN** the file is read and parsed locally
- **THEN** the dialog opens with the file list populated

#### Scenario: File picker cancelled

- **WHEN** the user dismisses the file picker without selecting a file
- **THEN** the dialog does not open and the UI state is unchanged

#### Scenario: User confirms the file add

- **WHEN** the user clicks Add in the dialog (file mode)
- **THEN** `torrent-add` is called with `{ "metainfo": "<base64>", ... }`
- **THEN** on success the dialog is dismissed and the torrent list is refreshed immediately

#### Scenario: Add fails

- **WHEN** `torrent-add` returns an error
- **THEN** the dialog remains open and an inline error message is shown within it

### Requirement: Magnet link flow

When the add-link dialog is open, the user SHALL paste a magnet URI into a text field. The file
list area SHALL display a static note that file metadata is unavailable for magnet links.

#### Scenario: Empty magnet input blocked

- **WHEN** the magnet field is empty and the user clicks Add
- **THEN** no RPC call is issued

#### Scenario: User confirms the magnet add

- **WHEN** the user fills in a magnet URI and clicks Add
- **THEN** `torrent-add` is called with `{ "filename": "<uri>", ... }`
- **THEN** on success the dialog is dismissed and the torrent list is refreshed immediately

#### Scenario: Magnet add fails

- **WHEN** `torrent-add` returns an error for a magnet submission
- **THEN** the dialog remains open and an inline error message is shown within it

### Requirement: Non-blocking add flow

All file I/O, Base64 encoding, torrent parsing, and RPC calls SHALL execute inside
`Task::perform()`. The `update()` function MUST return immediately without waiting.

#### Scenario: UI remains responsive during add

- **WHEN** a `torrent-add` RPC call is in-flight
- **THEN** the UI continues to render
- **THEN** `update()` returns immediately without waiting for the call to complete

### Requirement: Immediate list refresh after add

After `torrent-add` succeeds the system SHALL dismiss the dialog and trigger an immediate
`torrent-get` poll without waiting for the next scheduled tick.

#### Scenario: List updated after successful add

- **WHEN** `AddCompleted(Ok(()))` is received
- **THEN** the add dialog is dismissed
- **THEN** a `torrent-get` poll is issued immediately
- **THEN** the torrent list reflects the newly added torrent on the next `TorrentsUpdated` message

### Requirement: Per-file selection in file add dialog

In file mode, each file row in the add-torrent dialog SHALL include a checkbox. Checkboxes
SHALL default to checked. The user SHALL be able to uncheck individual files before confirming
the add. When the user clicks Add, unchecked file indices SHALL be passed as `files-unwanted`
in the `torrent-add` RPC call so the daemon never starts downloading those files.

#### Scenario: User unchecks a file before adding

- **WHEN** the user unchecks file index _i_ in the add-dialog file list
- **THEN** the checkbox for file _i_ becomes unchecked
- **THEN** all other checkboxes remain unchanged

#### Scenario: torrent-add includes files-unwanted for unchecked files

- **WHEN** the user confirms the add with one or more files unchecked
- **THEN** `torrent-add` is called with `{ "metainfo": "...", "files-unwanted": [<unchecked indices>], ... }`

#### Scenario: torrent-add omits files-unwanted when all files selected

- **WHEN** all file checkboxes are checked when the user confirms
- **THEN** `torrent-add` is called without a `files-unwanted` field

### Requirement: Select All tri-state header in file add dialog

The file add dialog SHALL display a **tri-state checkbox** header row above the file list,
using the `m3_tristate_checkbox` helper from the public `crate::theme` module. Its state SHALL be derived
from the `selected: Vec<bool>` state:

- All selected \u2192 **Checked**
- None selected \u2192 **Unchecked**
- Some selected \u2192 **Mixed**

Clicking the header when Mixed or Unchecked emits `AddDialogSelectAll`; clicking when
Checked emits `AddDialogDeselectAll`.

#### Scenario: Header shows Checked when all files selected

- **WHEN** all entries in `selected` are `true`
- **THEN** the header tri-state checkbox renders as Checked

#### Scenario: Header shows Unchecked when no files selected

- **WHEN** all entries in `selected` are `false`
- **THEN** the header tri-state checkbox renders as Unchecked

#### Scenario: Header shows Mixed when some files selected

- **WHEN** at least one entry in `selected` is `true` and at least one is `false`
- **THEN** the header tri-state checkbox renders as Mixed

#### Scenario: Clicking Mixed or Unchecked header selects all

- **WHEN** the header is Mixed or Unchecked and the user clicks it
- **THEN** all file checkboxes are set to checked

#### Scenario: Clicking Checked header deselects all

- **WHEN** the header is Checked and the user clicks it
- **THEN** all file checkboxes are set to unchecked
