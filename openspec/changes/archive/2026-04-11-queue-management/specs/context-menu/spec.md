## ADDED Requirements

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
