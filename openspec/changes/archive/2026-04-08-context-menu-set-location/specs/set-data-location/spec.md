## ADDED Requirements

### Requirement: Set Data Location menu action

The context menu SHALL include a **Set Data Location** action. Clicking it SHALL dismiss the
context menu and open the Set Data Location modal dialog targeting the right-clicked torrent.

#### Scenario: Set Data Location opens the modal dialog

- **WHEN** the user clicks "Set Data Location" in the context menu
- **THEN** the context menu is dismissed
- **THEN** the Set Data Location modal dialog opens pre-filled with the torrent's current
  `downloadDir` value

### Requirement: Set Data Location modal dialog

The Set Data Location dialog SHALL be a centered M3 card overlay rendered on top of the full
screen, following the visual pattern of the Add Torrent dialog. It SHALL contain:

- A text input (styled with `theme::m3_text_input`) for the absolute destination path, prefilled
  with the torrent's current `downloadDir`.
- A checkbox labelled "Move data to new location", defaulting to checked (`true`).
- A "Cancel" button (tonal style) that closes the dialog without dispatching any RPC.
- An "Apply" button (primary style) that dispatches the `torrent-set-location` RPC and closes
  the dialog.

The dialog state SHALL be represented as a dedicated struct containing `torrent_id: i64`,
`path: String` (initialised from `downloadDir`), and `move_data: bool` (default `true`).

#### Scenario: Dialog is prefilled with current download directory

- **WHEN** the Set Data Location dialog opens for a torrent
- **THEN** the path text input contains the torrent's current `downloadDir` value

#### Scenario: Move data checkbox defaults to checked

- **WHEN** the Set Data Location dialog opens
- **THEN** the "Move data to new location" checkbox is checked

#### Scenario: Cancel closes dialog without RPC

- **WHEN** the user clicks "Cancel"
- **THEN** the dialog is dismissed
- **THEN** no RPC call is dispatched

#### Scenario: Apply dispatches torrent-set-location and closes dialog

- **WHEN** the user clicks "Apply" with a non-empty path
- **THEN** a `torrent-set-location` RPC is dispatched with the entered path and the checkbox value
- **THEN** the dialog is dismissed

#### Scenario: Path input is editable

- **WHEN** the dialog is open
- **THEN** the user can edit the path text input to any value

#### Scenario: Checkbox state toggles

- **WHEN** the user clicks the "Move data to new location" checkbox
- **THEN** the checkbox state toggles between checked and unchecked

### Requirement: torrent-set-location RPC dispatch

The application SHALL dispatch a `torrent-set-location` JSON-RPC call through the existing mpsc
worker queue when the user applies the Set Data Location dialog. The call SHALL include the
torrent's id, the destination path, and the move flag. Asynchronous file-move errors SHALL be
surfaced on the next `torrent-get` poll via the torrent's `errorString` field; no additional
error-handling is required for this operation.

#### Scenario: RPC payload is correct

- **WHEN** `torrent-set-location` is dispatched with torrent_id=42, path="/data/new", move=true
- **THEN** the JSON-RPC body is:
  `{"method":"torrent-set-location","arguments":{"ids":[42],"location":"/data/new","move":true}}`

#### Scenario: move=false sends a path-update-only request

- **WHEN** `torrent-set-location` is dispatched with `move_data=false`
- **THEN** the JSON-RPC body contains `"move": false`

#### Scenario: RPC is dispatched through the mpsc worker queue

- **WHEN** the user clicks Apply in the Set Data Location dialog
- **THEN** the RPC call is enqueued via the existing worker channel and not issued directly
  from `update()`
