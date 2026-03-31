## MODIFIED Requirements

### Requirement: Connect action

The screen SHALL provide a "Connect" button styled as a primary filled pill (`m3_primary_button`). In the Quick Connect tab, clicking it initiates a connection probe using the entered credentials and disables the button for the duration. In the Saved Profiles tab, clicking Connect in the action bar connects using the currently selected profile. While the probe is in-flight, the button SHALL be disabled to prevent duplicate submissions.

When the selected profile has an `encrypted_password` and `AppState.unlocked_passphrase` is `None`, clicking Connect SHALL NOT initiate a connection immediately. Instead it SHALL trigger the passphrase unlock dialog (defined in the `credential-encryption` capability). The connection SHALL resume automatically after successful unlock.

#### Scenario: Button disabled during connection attempt

- **WHEN** the user clicks Connect
- **THEN** the Connect button becomes disabled until the attempt completes or fails

#### Scenario: Successful connection transitions to torrent list

- **WHEN** the connection probe succeeds
- **THEN** the app transitions to the torrent list screen

#### Scenario: Encrypted profile without unlocked passphrase triggers unlock dialog

- **WHEN** the user clicks Connect on a profile that has an `encrypted_password`
- **AND** `unlocked_passphrase` is `None`
- **THEN** the passphrase unlock dialog is shown instead of initiating an immediate connection attempt

#### Scenario: Connection proceeds after successful unlock

- **WHEN** the user successfully unlocks the passphrase via the unlock dialog
- **THEN** the connection attempt proceeds automatically with the decrypted password
