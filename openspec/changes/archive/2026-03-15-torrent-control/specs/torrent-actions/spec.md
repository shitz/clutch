## ADDED Requirements

### Requirement: Pause a torrent

The app SHALL send a `torrent-stop` RPC request for the selected torrent when the user clicks the
Pause button. On success the torrent list SHALL refresh immediately. On failure an inline error
message SHALL be displayed.

#### Scenario: Pause active torrent

- **WHEN** an active torrent is selected and the user clicks Pause
- **THEN** a `torrent-stop` RPC request is issued for that torrent ID

#### Scenario: List refreshes after successful pause

- **WHEN** the `torrent-stop` request completes successfully
- **THEN** a `torrent-get` poll is issued immediately and the list is updated

#### Scenario: Error shown on pause failure

- **WHEN** the `torrent-stop` request fails
- **THEN** an inline error message is displayed on the main screen

### Requirement: Resume a torrent

The app SHALL send a `torrent-start` RPC request for the selected torrent when the user clicks the
Resume button. On success the torrent list SHALL refresh immediately. On failure an inline error
message SHALL be displayed.

#### Scenario: Resume stopped torrent

- **WHEN** a stopped torrent is selected and the user clicks Resume
- **THEN** a `torrent-start` RPC request is issued for that torrent ID

#### Scenario: List refreshes after successful resume

- **WHEN** the `torrent-start` request completes successfully
- **THEN** a `torrent-get` poll is issued immediately and the list is updated

#### Scenario: Error shown on resume failure

- **WHEN** the `torrent-start` request fails
- **THEN** an inline error message is displayed on the main screen

### Requirement: Delete a torrent — confirmation

Clicking the Delete toolbar button SHALL NOT immediately issue an RPC request. Instead a
confirmation row SHALL appear below the toolbar showing the name of the selected torrent, a
"Delete local data" checkbox (unchecked by default), a **Confirm Delete** button, and a
**Cancel** button. The RPC request SHALL only be issued after the user clicks **Confirm Delete**.

#### Scenario: Delete button opens confirmation row

- **WHEN** a torrent is selected and the user clicks Delete
- **THEN** the confirmation row is shown with the torrent name, an unchecked "Delete local data" checkbox, and Confirm / Cancel buttons
- **AND** no RPC request is issued

#### Scenario: Cancel dismisses confirmation row

- **WHEN** the confirmation row is visible and the user clicks Cancel
- **THEN** the confirmation row is hidden and no RPC request is issued

#### Scenario: Confirm issues torrent-remove without local data

- **WHEN** the confirmation row is visible, the checkbox is unchecked, and the user clicks Confirm Delete
- **THEN** a `torrent-remove` RPC request is issued with `delete-local-data: false`

#### Scenario: Confirm issues torrent-remove with local data

- **WHEN** the confirmation row is visible, the user checks "Delete local data", and clicks Confirm Delete
- **THEN** a `torrent-remove` RPC request is issued with `delete-local-data: true`

#### Scenario: List refreshes after successful delete

- **WHEN** the `torrent-remove` request completes successfully
- **THEN** the deleted torrent no longer appears in the list

#### Scenario: Error shown on delete failure

- **WHEN** the `torrent-remove` request fails
- **THEN** an inline error message is displayed on the main screen

### Requirement: Action does not block polling

Sending an action RPC SHALL NOT start a new background poll while the action or its follow-up
refresh is in-flight. The existing `is_loading` guard SHALL prevent concurrent RPC calls.

#### Scenario: Poll tick ignored while action in-flight

- **WHEN** an action RPC is in-flight and a poll tick fires
- **THEN** the tick is ignored and no duplicate `torrent-get` is issued
