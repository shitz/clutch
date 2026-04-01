## ADDED Requirements

### Requirement: Connection form fields

The screen SHALL display input fields for Host (text), Port (numeric text), Username (text,
optional), and Password (text, masked, optional). Host SHALL default to `localhost` and Port SHALL
default to `9091`.

Each field SHALL have a stable widget ID so the keyboard navigation subscription can move focus
between them. The Tab ring order SHALL be: Host → Port → Username → Password → (wrap to Host).

When the Quick Connect tab is shown, the first empty field in the Tab ring SHALL be automatically
focused. Because Host and Port are pre-filled with defaults, the auto-focus target is typically the
Username field; if the user has also typed a username it falls through to Password, and if all four
are filled the focus lands on Host.

#### Scenario: Default values pre-filled

- **WHEN** the connection screen is first shown
- **THEN** Host field contains `localhost` and Port field contains `9091`

#### Scenario: Username and Password are optional

- **WHEN** the user leaves Username and Password blank and clicks Connect
- **THEN** the connection attempt proceeds without credentials

#### Scenario: Tab ring cycles through Quick Connect fields

- **WHEN** the Quick Connect tab is active
- **THEN** pressing Tab advances focus Host → Port → Username → Password → Host

#### Scenario: First empty field is focused on Quick Connect tab activation

- **WHEN** the Quick Connect tab becomes active (tab switch or screen open)
- **THEN** the first empty text input in the ring is automatically focused

### Requirement: Connect action

The screen SHALL provide a "Connect" button styled as a primary filled pill (`m3_primary_button`). In
the Quick Connect tab, clicking it initiates a connection probe using the entered credentials and
disables the button for the duration. In the Saved Profiles tab, clicking Connect in the action bar
connects using the currently selected profile. While the probe is in-flight, the button SHALL be
disabled to prevent duplicate submissions.

Pressing **Enter** in the Quick Connect form SHALL trigger the same action as clicking Connect,
provided the button is not currently disabled.

When the selected profile has an `encrypted_password` and `AppState.unlocked_passphrase` is `None`,
clicking Connect (or pressing Enter) SHALL NOT initiate a connection immediately. Instead it SHALL
trigger the passphrase unlock dialog (defined in the `credential-encryption` capability). The
connection SHALL resume automatically after successful unlock.

#### Scenario: Button disabled during connection attempt

- **WHEN** the user clicks Connect or presses Enter
- **THEN** the Connect button becomes disabled until the attempt completes or fails

#### Scenario: Successful connection transitions to torrent list

- **WHEN** the connection probe succeeds
- **THEN** the app transitions to the torrent list screen

#### Scenario: Enter triggers Connect in Quick Connect tab

- **WHEN** the Quick Connect tab is active
- **AND** the Connect button is enabled
- **AND** the user presses Enter (no Ctrl or Alt modifier)
- **THEN** the connection probe is initiated as if Connect was clicked

#### Scenario: Enter is ignored during in-flight probe

- **WHEN** a connection probe is already in-flight
- **AND** the user presses Enter
- **THEN** no duplicate probe is started

#### Scenario: Encrypted profile without unlocked passphrase triggers unlock dialog

- **WHEN** the user clicks Connect (or presses Enter) on a profile that has an `encrypted_password`
- **AND** `unlocked_passphrase` is `None`
- **THEN** the passphrase unlock dialog is shown instead of initiating an immediate connection attempt

#### Scenario: Connection proceeds after successful unlock

- **WHEN** the user successfully unlocks the passphrase via the unlock dialog
- **THEN** the connection attempt proceeds automatically with the decrypted password

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

## ADDED Requirements

### Requirement: Saved Profiles and Quick Connect tab navigation

The connection screen SHALL present two modes — Saved Profiles and Quick Connect — as a segmented control component (M3 style) using the `segmented_control` helper from `src/theme.rs`. The segmented control SHALL be centered in a fixed-width (400 px) container.

#### Scenario: Segmented control shown on connection screen

- **WHEN** the connection screen is rendered
- **THEN** a segmented control with "Saved Profiles" and "Quick Connect" segments is visible

### Requirement: Saved profile rows as selectable cards with action bar

Each saved profile in the Saved Profiles tab SHALL be rendered as a selectable row highlighted with the `selected_row` style (18 % alpha primary wash) when active. Clicking a row selects it without connecting. Below the profile list an action bar SHALL contain "Manage Profiles" (`m3_tonal_button`) and "Connect" (`m3_primary_button`). Connect initiates the connection using the selected profile. The first profile SHALL be pre-selected on open. The profile list SHALL be scrollable with a maximum height of 300 px.

#### Scenario: Profile rows are selectable

- **WHEN** the Saved Profiles tab is shown and profiles exist
- **THEN** each profile row is selectable, with the first profile pre-selected by default

#### Scenario: Clicking a row selects it without connecting

- **WHEN** the user clicks a profile row
- **THEN** the row becomes selected and no connection is initiated

#### Scenario: Action bar has Manage Profiles and Connect buttons

- **WHEN** the Saved Profiles tab is shown
- **THEN** an action bar below the list shows "Manage Profiles" (tonal) and "Connect" (primary) buttons

### Requirement: Connection screen layout

The connection screen SHALL place the Clutch logo with a fixed 80 px top margin. The logo handle SHALL be loaded once into connection state (via `Handle::from_memory`) and reused each frame.

#### Scenario: Logo is shown at a consistent position

- **WHEN** the connection screen is rendered
- **THEN** the logo appears with approximately 80 px of space above it, centered horizontally
