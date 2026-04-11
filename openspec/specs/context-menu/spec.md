## MODIFIED Requirements

### Requirement: Right-click opens context menu

The context menu SHALL be opened by right-clicking a torrent row. When
`Message::TorrentRightClicked(id)` is received, the selection is updated according to the
following rules before the menu is shown:

- If the right-clicked ID **is already** in `selected_ids` → leave `selected_ids` unchanged.
- If the right-clicked ID **is not** in `selected_ids` → clear `selected_ids`, select only the
  right-clicked ID, and reset `selection_anchor`.

After applying selection drift, the application SHALL store
`context_menu: Some((last_cursor_position))`. Context-menu actions operate on the full
`selected_ids` set (not just the right-clicked ID). The context menu SHALL remain open until
explicitly dismissed.

#### Scenario: Right-click on a selected row keeps existing selection

- **WHEN** `selected_ids` = {A, B} and the user right-clicks row A
- **THEN** `selected_ids` remains {A, B}
- **THEN** the context menu opens at the cursor position

#### Scenario: Right-click on an unselected row resets selection

- **WHEN** `selected_ids` = {A, B} and the user right-clicks row C
- **THEN** `selected_ids` = {C} and `selection_anchor` = C
- **THEN** the context menu opens at the cursor position

#### Scenario: Opening a new context menu replaces the previous one

- **WHEN** the user right-clicks a different torrent row while a context menu is already open
- **THEN** the context menu position is updated and selection drift rules are applied again

### Requirement: Start and Pause actions in context menu

The context menu SHALL always display both **Start** and **Pause** actions. The applicable
action is determined by the **aggregate state of all torrents in `selected_ids`**:

- **Start** is enabled if _any_ torrent in `selected_ids` is in a startable state (status 0 or
  any non-active state where status ≠ 3/4/5/6).
- **Pause** is enabled if _any_ torrent in `selected_ids` is in a pausable state (status
  3, 4, 5, or 6).

Both Start and Pause may be simultaneously enabled when the selection contains a mix of active
and stopped torrents.

An inapplicable action SHALL be rendered without an `.on_press` handler (visually disabled).

When Start is clicked, a `torrent-start` RPC SHALL be dispatched with all IDs in `selected_ids`.
When Pause is clicked, a `torrent-stop` RPC SHALL be dispatched with all IDs in `selected_ids`.

#### Scenario: Start is active when any selected torrent is stopped

- **WHEN** `selected_ids` contains one stopped and one seeding torrent
- **THEN** the Start button has an `.on_press` handler

#### Scenario: Pause is active when any selected torrent is active

- **WHEN** `selected_ids` contains one stopped and one seeding torrent
- **THEN** the Pause button has an `.on_press` handler

#### Scenario: Start dispatches torrent-start for all selected IDs

- **WHEN** the user clicks Start in the context menu with multiple torrents selected
- **THEN** a `torrent-start` RPC is dispatched with all IDs in `selected_ids`
- **THEN** the context menu is dismissed

#### Scenario: Pause dispatches torrent-stop for all selected IDs

- **WHEN** the user clicks Pause in the context menu with multiple torrents selected
- **THEN** a `torrent-stop` RPC is dispatched with all IDs in `selected_ids`
- **THEN** the context menu is dismissed

### Requirement: Delete action in context menu

The **Delete** action in the context menu SHALL always be active (enabled) when at least one
torrent is selected. Clicking it SHALL dismiss the context menu and open the delete-confirmation
dialog targeting **all torrents in `selected_ids`**.

#### Scenario: Delete opens the confirmation dialog for all selected torrents

- **WHEN** `selected_ids` = {A, B, C} and the user clicks Delete in the context menu
- **THEN** the context menu is dismissed
- **THEN** the delete confirmation dialog opens for all three torrents (showing bulk text)

### Requirement: Queue movement actions in context menu

The context menu SHALL display a dedicated group of four queue-movement actions: **Move to
Top**, **Move Up**, **Move Down**, and **Move to Bottom**. These actions SHALL always be
rendered (no conditional visibility based on torrent status). Each action SHALL dispatch the
corresponding bulk `queue-move-*` RPC with all IDs in `selected_ids` and SHALL dismiss the
context menu on activation.

#### Scenario: All four queue actions are visible in the context menu

- **WHEN** the user right-clicks a torrent row
- **THEN** the context menu contains "Move to Top", "Move Up", "Move Down", and
  "Move to Bottom" items

#### Scenario: Move to Top dispatches queue-move-top for all selected IDs

- **WHEN** `selected_ids` = {2, 5} and the user clicks "Move to Top"
- **THEN** a `queue-move-top` RPC is dispatched with `ids: [2, 5]`
- **THEN** the context menu is dismissed

#### Scenario: Move Up dispatches queue-move-up for all selected IDs

- **WHEN** `selected_ids` = {3} and the user clicks "Move Up"
- **THEN** a `queue-move-up` RPC is dispatched with `ids: [3]`
- **THEN** the context menu is dismissed

#### Scenario: Move Down dispatches queue-move-down for all selected IDs

- **WHEN** `selected_ids` = {3} and the user clicks "Move Down"
- **THEN** a `queue-move-down` RPC is dispatched with `ids: [3]`
- **THEN** the context menu is dismissed

#### Scenario: Move to Bottom dispatches queue-move-bottom for all selected IDs

- **WHEN** `selected_ids` = {1, 4} and the user clicks "Move to Bottom"
- **THEN** a `queue-move-bottom` RPC is dispatched with `ids: [1, 4]`
- **THEN** the context menu is dismissed
