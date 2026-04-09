## ADDED Requirements

### Requirement: Multi-select state model

`TorrentListScreen` SHALL track selection via a `selected_ids: HashSet<i64>` set and a
`selection_anchor: Option<i64>`. The deprecated `selected_id: Option<i64>` field is removed.

`selected_torrent()` SHALL continue to exist and SHALL return `Some(&TorrentData)` only when
`selected_ids.len() == 1`; it returns `None` for empty or multi-selection.

#### Scenario: Empty selection by default

- **WHEN** a new `TorrentListScreen` is constructed
- **THEN** `selected_ids` is empty and `selection_anchor` is `None`

#### Scenario: Single selection via plain click

- **WHEN** the user clicks a torrent row with no modifier keys held
- **THEN** `selected_ids` contains exactly that torrent's ID
- **THEN** `selection_anchor` is set to that torrent's ID

#### Scenario: selected_torrent returns None for multi-selection

- **WHEN** `selected_ids.len() > 1`
- **THEN** `selected_torrent()` returns `None`

### Requirement: Keyboard modifier tracking

`TorrentListScreen` SHALL maintain a `modifiers: iced::keyboard::Modifiers` field that
mirrors the current state of keyboard modifier keys (Shift, Ctrl, Cmd, Alt). A new
`modifiers_subscription()` method SHALL return an always-active subscription that maps
`iced::keyboard::Event::ModifiersChanged(m)` to `Message::ModifiersChanged(m)`.

`main_screen::subscription()` SHALL merge this subscription alongside the existing tick,
worker, dialog-keyboard, and cursor subscriptions.

#### Scenario: ModifiersChanged updates stored modifiers

- **WHEN** the OS emits a `ModifiersChanged` keyboard event with Shift held
- **THEN** `TorrentListScreen::modifiers.shift()` returns `true`

#### Scenario: Modifier state clears on key release

- **WHEN** the Shift key is released
- **THEN** the next `ModifiersChanged` event updates `modifiers` so that `shift()` returns `false`

### Requirement: Plain-click selection semantics

When no modifier key is held and the user clicks a torrent row, the selection SHALL be reset
to contain only the clicked torrent's ID, and `selection_anchor` SHALL be updated.

#### Scenario: Plain click replaces multi-selection

- **WHEN** two torrents are in `selected_ids` and the user plain-clicks a third torrent
- **THEN** `selected_ids` contains only the third torrent's ID

#### Scenario: Plain click on the already-selected torrent deselects it

- **WHEN** `selected_ids` contains exactly one ID and the user plain-clicks that same row
- **THEN** `selected_ids` becomes empty

### Requirement: Ctrl/Cmd-click toggle semantics

When Ctrl (Linux/Windows) or Cmd (macOS) is held and the user clicks a torrent row, the
clicked torrent's ID SHALL be toggled in `selected_ids`. All other selected IDs SHALL remain
unchanged. `selection_anchor` SHALL be updated to the clicked ID.

#### Scenario: Ctrl/Cmd-click adds a torrent to the selection

- **WHEN** `selected_ids` = {A} and the user Ctrl/Cmd-clicks torrent B
- **THEN** `selected_ids` = {A, B}

#### Scenario: Ctrl/Cmd-click removes an already-selected torrent

- **WHEN** `selected_ids` = {A, B} and the user Ctrl/Cmd-clicks torrent A
- **THEN** `selected_ids` = {B}

### Requirement: Shift-click range selection semantics

When Shift is held and the user clicks a torrent row, the system SHALL select all torrents
between `selection_anchor` and the clicked torrent (inclusive) in the current visible
(sorted + filtered) order. The existing selection outside that range is preserved.

The range is computed over a `visible_torrents()` helper that returns the current sorted and
filtered slice. The anchor is stored as an **ID**, not an index. At Shift-click time the
anchor ID is resolved to its **current** position in `visible_torrents()`, so that live poll
updates that shift row order between the anchor-click and the Shift-click are handled
correctly. If the anchor ID is no longer visible, the Shift-click SHALL fall back to
plain-click behaviour (select only the clicked row, reset anchor).

#### Scenario: Shift-click from anchor to later row selects the range

- **WHEN** `selection_anchor` points to the torrent currently at row index 2 and the user
  Shift-clicks the torrent at row index 5
- **THEN** `selected_ids` contains the IDs of the torrents at rows 2, 3, 4, and 5

#### Scenario: Range is resolved against the live sort order

- **WHEN** a poll tick reorders the list so that the anchor torrent shifts from index 2 to
  index 8, and the user then Shift-clicks the torrent now at index 5
- **THEN** `selected_ids` contains the IDs of the torrents at rows 5, 6, 7, and 8
  (the range between the anchor's current position and the click)

#### Scenario: Shift-click falls back to plain click when anchor is not visible

- **WHEN** `selection_anchor` holds an ID that is currently filtered out and the user
  Shift-clicks a row
- **THEN** only the clicked row is selected and `selection_anchor` is updated to it

#### Scenario: Shift-click with no anchor behaves like a plain click

- **WHEN** `selection_anchor` is `None` and the user Shift-clicks a row
- **THEN** only that row is selected and `selection_anchor` is set to it

#### Scenario: Shift-click range respects the current filter

- **WHEN** a status filter is active and the user Shift-clicks across a range
- **THEN** only visible (non-filtered-out) torrents in the range are included in the selection

### Requirement: Multi-row visual highlight

The torrent list view SHALL apply the `selected_row` container style to **every** row whose
ID is in `selected_ids`. Rows not in `selected_ids` receive no explicit style.

#### Scenario: All selected rows are highlighted

- **WHEN** `selected_ids` = {A, B, C}
- **THEN** rows A, B, and C are rendered with the `selected_row` container style

#### Scenario: Unselected rows have no highlight

- **WHEN** a torrent's ID is not in `selected_ids`
- **THEN** no `selected_row` style is applied to that row

### Requirement: Toolbar action guards use aggregate selection

The toolbar's Start (Resume) and Pause buttons SHALL be enabled based on the aggregate
state of all torrents in `selected_ids`:

- **Start** is enabled if _any_ torrent in `selected_ids` is in a startable state (status 0,
  or any error state where status ≠ 3/4/5/6).
- **Pause** is enabled if _any_ torrent in `selected_ids` is in a pausable state (status
  3, 4, 5, or 6).
- **Delete** is enabled if `selected_ids` is non-empty.

When `selected_ids` is empty all three action buttons are disabled.

#### Scenario: Start enabled when any selected torrent is stopped

- **WHEN** `selected_ids` contains one stopped and one downloading torrent
- **THEN** the Start toolbar button is enabled

#### Scenario: Pause enabled when any selected torrent is active

- **WHEN** `selected_ids` contains one stopped and one seeding torrent
- **THEN** the Pause toolbar button is enabled

#### Scenario: All action buttons disabled with empty selection

- **WHEN** `selected_ids` is empty
- **THEN** Start, Pause, and Delete toolbar buttons are all disabled

### Requirement: Bulk RPC dispatch for toolbar actions

Clicking an enabled Start, Pause, or Delete toolbar action SHALL dispatch the corresponding
RPC with the full list of selected IDs.

#### Scenario: Resume dispatches torrent-start for all selected IDs

- **WHEN** the user clicks the Resume toolbar button with multiple torrents selected
- **THEN** a single `torrent-start` RPC call is dispatched with `ids` containing all IDs in
  `selected_ids`

#### Scenario: Pause dispatches torrent-stop for all selected IDs

- **WHEN** the user clicks the Pause toolbar button with multiple torrents selected
- **THEN** a single `torrent-stop` RPC call is dispatched with `ids` containing all IDs in
  `selected_ids`

#### Scenario: Delete opens confirmation dialog with all selected IDs

- **WHEN** the user clicks the Delete toolbar button with multiple torrents selected
- **THEN** the delete confirmation dialog opens targeting all IDs in `selected_ids`

### Requirement: Cmd+A / Ctrl+A selects all visible torrents

When the user presses Cmd+A (macOS) or Ctrl+A (Linux/Windows) while the torrent list is
the active context, the system SHALL select all torrents currently visible in the filtered
and sorted list. Torrents hidden by the active status filter SHALL NOT be selected.
`selection_anchor` SHALL be set to the first visible torrent's ID (or cleared if the list
contains no visible torrents). Any prior selection is replaced.

This shortcut SHALL be inactive while the add-torrent dialog is open.

#### Scenario: Cmd+A selects all visible torrents

- **WHEN** the torrent list shows 12 torrents and the user presses Cmd+A (or Ctrl+A)
- **THEN** `selected_ids` contains all 12 visible torrent IDs

#### Scenario: Select-all respects the active filter

- **WHEN** a status filter is active that hides 4 torrents, leaving 8 visible, and the user
  presses Cmd+A
- **THEN** `selected_ids` contains only the 8 visible torrent IDs

#### Scenario: Select-all is a no-op when no torrents are visible

- **WHEN** the filtered list is empty and the user presses Cmd+A
- **THEN** `selected_ids` remains empty

#### Scenario: Cmd+A is ignored while the add-torrent dialog is open

- **WHEN** the add-torrent dialog is open and the user presses Cmd+A
- **THEN** `selected_ids` is unchanged (the shortcut is consumed by the dialog context)
