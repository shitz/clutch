## ADDED Requirements

### Requirement: Download queue settings in Connections tab

The Connections settings tab SHALL display a "Queueing" card directly below the "Bandwidth"
card. The card SHALL contain:

- A toggle labelled "Limit simultaneous downloads" bound to
  `download_queue_enabled` in the settings draft.
- A numeric text input for the max simultaneous download count, bound to
  `download_queue_size`, enabled only when the toggle is on.
- A toggle labelled "Limit simultaneous seeding" bound to `seed_queue_enabled` in the
  settings draft.
- A numeric text input for the max simultaneous seed count, bound to `seed_queue_size`,
  enabled only when the toggle is on.

The card SHALL be populated from `SessionData` when the settings screen is opened, and the
values SHALL be saved to the daemon via `session-set` when the user saves the settings.

#### Scenario: Queueing card appears below Bandwidth card

- **WHEN** the user opens the Connections settings tab
- **THEN** a "Queueing" card is visible below the "Bandwidth" card

#### Scenario: Download queue size input disabled when toggle is off

- **WHEN** the "Limit simultaneous downloads" toggle is off
- **THEN** the download queue size text input SHALL be non-interactive (no `.on_input` handler)

#### Scenario: Download queue size input enabled when toggle is on

- **WHEN** the "Limit simultaneous downloads" toggle is on
- **THEN** the download queue size text input SHALL accept user input

#### Scenario: Seed queue size input disabled when toggle is off

- **WHEN** the "Limit simultaneous seeding" toggle is off
- **THEN** the seed queue size text input SHALL be non-interactive

#### Scenario: Queue settings saved to daemon on save

- **WHEN** the user enables download queue with size 3 and clicks Save
- **THEN** a `session-set` RPC is dispatched with
  `download-queue-enabled: true` and `download-queue-size: 3`

#### Scenario: Queue settings populated from session on open

- **WHEN** `SessionData` reports `download_queue_enabled: true` and `download_queue_size: 4`
- **THEN** the download queue toggle is checked and the size input shows `4`
