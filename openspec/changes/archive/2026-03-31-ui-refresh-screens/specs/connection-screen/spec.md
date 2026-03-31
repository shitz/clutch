## MODIFIED Requirements

### Requirement: Connection form fields

The screen SHALL display input fields for Host (text), Port (numeric text), Username (text, optional), and Password (text, masked, optional). Host SHALL default to `localhost` and Port SHALL default to `9091`.

#### Scenario: Default values pre-filled

- **WHEN** the connection screen is first shown
- **THEN** Host field contains `localhost` and Port field contains `9091`

#### Scenario: Username and Password are optional

- **WHEN** the user leaves Username and Password blank and clicks Connect
- **THEN** the connection attempt proceeds without credentials

### Requirement: Connect action

The screen SHALL provide a "Connect" button styled as a primary pill button (fully rounded, brand primary color). Clicking it SHALL initiate a connection probe using the entered credentials. While the probe is in-flight, the button SHALL be disabled to prevent duplicate submissions.

#### Scenario: Button disabled during connection attempt

- **WHEN** the user clicks Connect
- **THEN** the Connect button becomes disabled until the attempt completes or fails

#### Scenario: Successful connection transitions to torrent list

- **WHEN** the connection probe succeeds
- **THEN** the app transitions to the torrent list screen

#### Scenario: Connect button is a pill shape

- **WHEN** the connection screen quick-connect tab is shown
- **THEN** the Connect button uses the primary pill button style (fully rounded ends, brand primary color)

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

### Requirement: Saved Profiles and Quick Connect tab navigation

The connection screen SHALL present two modes — Saved Profiles and Quick Connect — as a segmented control component (M3 segmented button style) rather than flat underline tabs. The segmented control SHALL use the `segmented_control` helper from `src/theme.rs`. Each segment SHALL be labeled "Saved Profiles" and "Quick Connect". The active segment SHALL be visually highlighted with the brand primary color.

#### Scenario: Segmented control shown on connection screen

- **WHEN** the connection screen is rendered
- **THEN** a segmented control with "Saved Profiles" and "Quick Connect" segments is visible

#### Scenario: Switching between Saved Profiles and Quick Connect

- **WHEN** the user clicks the inactive segment
- **THEN** the view switches to the corresponding mode and the clicked segment is highlighted

### Requirement: Saved profile rows styled as selectable cards with action bar

Each saved profile in the Saved Profiles tab SHALL be rendered as a selectable row. The selected profile SHALL be highlighted using the `selected_row` container style (18 % alpha primary wash). Clicking a profile row SHALL select it without immediately initiating a connection. Below the profile list an action bar SHALL contain two buttons: "Manage Profiles" (`m3_tonal_button`) on the left and "Connect" (`m3_primary_button`) on the right. The Connect button initiates the connection using the currently selected profile. The first profile in the list SHALL be pre-selected when the Saved Profiles tab is opened.

The profile list SHALL be scrollable with a maximum height of 300 px to accommodate many profiles without overflowing the screen.

#### Scenario: Profile rows are selectable

- **WHEN** the Saved Profiles tab is shown and profiles exist
- **THEN** each profile row is rendered as a selectable container, with the first profile visually selected by default

#### Scenario: Clicking a row selects it without connecting

- **WHEN** the user clicks a profile row
- **THEN** the row becomes selected (tonal wash highlight) and no connection is initiated

#### Scenario: Connect button connects using selected profile

- **WHEN** the user clicks the Connect action bar button
- **THEN** a connection attempt is initiated using the currently selected profile's credentials

#### Scenario: Action bar has Manage Profiles and Connect buttons

- **WHEN** the Saved Profiles tab is shown
- **THEN** an action bar below the profile list shows "Manage Profiles" (tonal) and "Connect" (primary) buttons

### Requirement: Connection screen layout

The connection screen SHALL use a fixed 80 px top margin before the Clutch logo. The logo handle SHALL be loaded once into connection state (via `Handle::from_memory`) and reused each frame to avoid repeated decoding. The tab segmented control SHALL be centered in a fixed-width (400 px) container.

#### Scenario: Logo is shown at a consistent position

- **WHEN** the connection screen is rendered
- **THEN** the logo appears with approximately 80 px of space above it, centered horizontally

