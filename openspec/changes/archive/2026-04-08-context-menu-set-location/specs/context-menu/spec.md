## ADDED Requirements

### Requirement: Global cursor position tracking

The application SHALL maintain the current mouse cursor position by subscribing to
`iced::event::listen_with` and emitting `Message::CursorMoved(Point)` on every
`iced::mouse::Event::CursorMoved` event. The cursor position SHALL be stored in
`TorrentListScreen::last_cursor_position` and updated on every mouse move without triggering a
layout recalculation or re-render.

#### Scenario: Cursor position is updated on mouse move

- **WHEN** the user moves the mouse anywhere in the application window
- **THEN** `last_cursor_position` is updated to the new `Point` and `Task::none()` is returned

#### Scenario: Cursor tracking does not cause visible re-renders

- **WHEN** `CursorMoved` is handled
- **THEN** no widget in the view tree is rebuilt or re-measured as a result

### Requirement: Right-click opens context menu

The context menu SHALL be opened by right-clicking a torrent row. When
`Message::TorrentRightClicked(id)` is received, the application SHALL store
`context_menu: Some((id, last_cursor_position))` in `TorrentListScreen`. The context menu
SHALL remain open until explicitly dismissed.

#### Scenario: Right-click on a torrent row opens the context menu

- **WHEN** the user right-clicks a torrent row
- **THEN** `context_menu` is set to `Some((torrent_id, cursor_point))`
- **THEN** the context menu overlay is rendered anchored at `cursor_point`

#### Scenario: Opening a new context menu replaces the previous one

- **WHEN** the user right-clicks a different torrent row while a context menu is already open
- **THEN** `context_menu` is updated to reflect the newly right-clicked torrent and position

### Requirement: Floating context menu overlay

When `context_menu` is `Some`, the torrent list view SHALL render a three-layer `stack`:

1. The normal torrent list view (layer 0).
2. A fully transparent `mouse_area` spanning `Length::Fill` (layer 1) whose `.on_press`
   emits `Message::DismissContextMenu`.
3. A Material 3 card menu (layer 2) positioned at the cursor point via container padding.

The menu card SHALL contain four items in order: **Start**, **Pause**, **Delete**, and
**Set Data Location**. The actions SHALL be rendered as individual buttons using
`theme::icon_button` or equivalent action-label buttons consistent with the design language.

Bottom-edge mitigation: if the anchor `y` coordinate exceeds `window_height − 150 px`, the
menu SHALL be rendered 150 px above the cursor instead of below.

#### Scenario: Context menu renders at correct position

- **WHEN** the context menu is open with anchor point `(x, y)` where `y ≤ window_height − 150`
- **THEN** the menu card is positioned at `(x, y)` relative to the top-left of the window

#### Scenario: Context menu is repositioned near the bottom edge

- **WHEN** the context menu anchor `y > window_height − 150`
- **THEN** the menu card top edge is placed at `y − 150` instead of `y`

#### Scenario: Torrent list is still visible beneath the open menu

- **WHEN** the context menu is open
- **THEN** the torrent list rows remain visible in the background (layer 0)

### Requirement: Click-away dismissal

Clicking anywhere outside the context menu card SHALL dismiss it. The transparent `mouse_area`
on layer 1 catches this click-away event.

#### Scenario: Clicking outside the menu dismisses it

- **WHEN** the context menu is open and the user clicks anywhere not on the menu card
- **THEN** `context_menu` becomes `None` and the overlay is removed from the view

#### Scenario: Dismissal does not change the selected torrent

- **WHEN** the context menu is dismissed via click-away
- **THEN** the currently selected torrent in the list remains unchanged

### Requirement: Start and Pause actions in context menu

The context menu SHALL always display both **Start** and **Pause** actions. The action that is
inapplicable to the right-clicked torrent's current Transmission status SHALL be rendered without
an `.on_press` handler (visually inactive/disabled), using the same visual treatment applied to
disabled interactive elements throughout the application. The applicable action SHALL be fully
interactive.

A torrent is considered startable when its status is Stopped (0) or any error/queued state where
resumption is possible. A torrent is considered pausable when its status indicates active transfer
or an active queue position (downloading, seeding, or their queued variants: 3, 4, 5, 6).

#### Scenario: Start is active when torrent is stopped

- **WHEN** the context menu is shown for a torrent with status = 0 (Stopped)
- **THEN** the Start button has an `.on_press` handler and is interactable
- **THEN** the Pause button has no `.on_press` handler and appears disabled

#### Scenario: Pause is active when torrent is downloading or seeding

- **WHEN** the context menu is shown for a torrent with status = 4 (Downloading) or 6 (Seeding)
- **THEN** the Pause button has an `.on_press` handler and is interactable
- **THEN** the Start button has no `.on_press` handler and appears disabled

#### Scenario: Start action dispatches torrent-start RPC

- **WHEN** the user clicks the active Start button
- **THEN** a `torrent-start` RPC call is dispatched for the context-menu torrent
- **THEN** the context menu is dismissed

#### Scenario: Pause action dispatches torrent-stop RPC

- **WHEN** the user clicks the active Pause button
- **THEN** a `torrent-stop` RPC call is dispatched for the context-menu torrent
- **THEN** the context menu is dismissed

### Requirement: Delete action in context menu

The **Delete** action in the context menu SHALL always be active (enabled). Clicking it SHALL
dismiss the context menu and open the same delete-confirmation dialog that the toolbar delete
button opens, pre-targeting the right-clicked torrent.

#### Scenario: Delete opens the confirmation dialog

- **WHEN** the user clicks Delete in the context menu
- **THEN** the context menu is dismissed
- **THEN** the delete confirmation dialog opens targeting the right-clicked torrent

### Requirement: Inspector General tab — Data Path and Error fields

The **General** tab of the inspector panel SHALL display two additional rows:

1. **Data Path** — the absolute path on the daemon's filesystem where the torrent data is stored,
   sourced from `TorrentData::download_dir`. Displayed after "Upload Speed" in the right column.
2. **Error** — the daemon-reported error for this torrent, sourced from `TorrentData::error` and
   `TorrentData::error_string`. Displayed after "Ratio" in the left column. When `error == 0`
   the value SHALL read "none"; otherwise the value SHALL be `error_string` (or
   `"error {code}"` when `error_string` is empty).

#### Scenario: Data Path shown in General tab

- **WHEN** the user opens the inspector for a torrent with `downloadDir = "/data/torrents"`
- **THEN** the General tab shows the row `Data Path   /data/torrents`

#### Scenario: Error row shows "none" for healthy torrents

- **WHEN** `error == 0` for the selected torrent
- **THEN** the Error row reads "none"

#### Scenario: Error row shows the daemon message for errored torrents

- **WHEN** `error != 0` and `error_string = "disk full"`
- **THEN** the Error row reads "disk full"

#### Scenario: Error row falls back to error code when string is empty

- **WHEN** `error != 0` and `error_string` is empty
- **THEN** the Error row reads "error {code}"
