## ADDED Requirements

### Requirement: Connection form fields

The screen SHALL display input fields for Host (text), Port (numeric text), Username (text, optional), and Password (text, masked, optional). Host SHALL default to `localhost` and Port SHALL default to `9091`.

#### Scenario: Default values pre-filled

- **WHEN** the connection screen is first shown
- **THEN** Host field contains `localhost` and Port field contains `9091`

#### Scenario: Username and Password are optional

- **WHEN** the user leaves Username and Password blank and clicks Connect
- **THEN** the connection attempt proceeds without credentials

### Requirement: Connect action

The screen SHALL provide a "Connect" button. Clicking it SHALL initiate a connection probe using the entered credentials. While the probe is in-flight, the button SHALL be disabled to prevent duplicate submissions.

#### Scenario: Button disabled during connection attempt

- **WHEN** the user clicks Connect
- **THEN** the Connect button becomes disabled until the attempt completes or fails

#### Scenario: Successful connection transitions to torrent list

- **WHEN** the connection probe succeeds
- **THEN** the app transitions to the torrent list screen

### Requirement: Inline error display on failure

When a connection attempt fails, the screen SHALL remain visible and display an inline error message describing the failure. The connection form fields SHALL remain populated with the values the user entered.

#### Scenario: Failed connection shows error and retains input

- **WHEN** a connection attempt fails (refused, timeout, or auth error)
- **THEN** an error message appears on the connection screen
- **THEN** all previously entered field values remain unchanged
- **THEN** the Connect button is re-enabled

#### Scenario: Authentication failure shows distinct message

- **WHEN** the server responds with 401 Unauthorized
- **THEN** the error message indicates authentication failure (distinct from a connectivity error)

### Requirement: Error logged to console

All connection errors SHALL be logged to stdout/stderr in addition to being shown in the UI.

#### Scenario: Console log on failure

- **WHEN** a connection attempt fails
- **THEN** the error details are printed to the console
