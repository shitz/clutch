## MODIFIED Requirements

### Requirement: Connect action

The screen SHALL provide a "Connect" button styled as a primary filled pill
(`m3_primary_button`). In the Quick Connect tab, clicking it initiates a connection
probe using the entered credentials and disables the button for the duration. In the
Saved Profiles tab, clicking Connect in the action bar connects using the currently
selected profile. While the probe is in-flight, the button SHALL be disabled to prevent
duplicate submissions.

Pressing **Enter** in the Quick Connect form SHALL trigger the same action as clicking
Connect, provided the button is not currently disabled.

When the selected profile has an `encrypted_password` and `AppState.unlocked_passphrase`
is `None`, clicking Connect (or pressing Enter) SHALL NOT initiate a connection
immediately. Instead it SHALL trigger the passphrase unlock dialog (defined in the
`credential-encryption` capability). The connection SHALL resume automatically after
successful unlock.

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

- **WHEN** the user clicks Connect (or presses Enter) on a profile that has an
  `encrypted_password`
- **AND** `unlocked_passphrase` is `None`
- **THEN** the passphrase unlock dialog is shown instead of initiating an immediate
  connection attempt

#### Scenario: Connection proceeds after successful unlock

- **WHEN** the user successfully unlocks the passphrase via the unlock dialog
- **THEN** the connection attempt proceeds automatically with the decrypted password

### Requirement: Connection form fields

The screen SHALL display input fields for Host (text), Port (numeric text), Username
(text, optional), and Password (text, masked, optional). Host SHALL default to
`localhost` and Port SHALL default to `9091`.

Each field SHALL have a stable `text_input::Id` so the keyboard navigation subscription
can move focus between them. The Tab ring order SHALL be: Host → Port → Username →
Password → (wrap to Host).

When the Quick Connect tab is shown, the first empty field in the Tab ring SHALL be
automatically focused. Because Host and Port are pre-filled with defaults, the auto-
focus target is typically the Username field; if the user has also typed a username it
falls through to Password, and if all four are filled the focus lands on Host.

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
