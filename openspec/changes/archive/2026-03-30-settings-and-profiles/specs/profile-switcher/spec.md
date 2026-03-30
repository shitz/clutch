## ADDED Requirements

### Requirement: Profile switcher dropdown replaces disconnect button

The main-screen toolbar SHALL replace the existing Disconnect button with a profile switcher dropdown. The dropdown SHALL display the name of the currently active profile as its label, followed by a chevron indicator (▾). Clicking it SHALL open a menu.

#### Scenario: Dropdown shows active profile name

- **WHEN** the user is connected to a profile named "Seedbox"
- **THEN** the toolbar button shows "Seedbox ▾"

### Requirement: Dropdown menu contents

The dropdown menu SHALL list all saved connection profiles. Below the profile list, a visual divider SHALL separate the profile entries from three fixed actions: **Add New Connection…**, a settings shortcut labeled **Manage Connections…**, and a **Disconnect** item at the bottom.

#### Scenario: Dropdown shows all profiles

- **WHEN** the user opens the profile switcher
- **THEN** all saved profiles are listed by name

#### Scenario: Divider separates profiles from actions

- **WHEN** the dropdown is open
- **THEN** a visual separator appears between the last profile and the "Add New Connection…" item

#### Scenario: Manage Connections opens Settings screen

- **WHEN** the user clicks "Manage Connections…"
- **THEN** the app navigates to the Settings screen with the Connections tab active

#### Scenario: Disconnect returns to connection screen

- **WHEN** the user clicks "Disconnect"
- **THEN** the current session is dropped
- **THEN** the app navigates to the connection screen
- **THEN** no profile is deleted; all saved profiles remain intact

### Requirement: Switching profiles via dropdown

Clicking a profile in the dropdown SHALL immediately drop the current connection state, connect to the selected profile's daemon, and refresh the torrent list. No explicit disconnect step SHALL be required.

#### Scenario: Profile switch updates torrent list

- **WHEN** the user selects a different profile from the dropdown
- **THEN** the current torrent list is cleared
- **THEN** the app probes the new daemon using the selected profile's credentials (including keyring password)
- **THEN** on success, the torrent list is populated with the new daemon's torrents
- **THEN** the toolbar label updates to the new profile's name
- **THEN** `last_connected` is updated to the new profile's UUID

#### Scenario: Profile switch fails

- **WHEN** the user selects a profile and the probe fails
- **THEN** an inline error is shown in the toolbar or list area
- **THEN** the app remains on the main screen (does not navigate to connection screen)

### Requirement: Add New Connection from dropdown

Clicking **Add New Connection…** in the dropdown SHALL navigate to `Screen::Connection`. A **Cancel** button SHALL be visible on the connection screen in this context. Cancelling SHALL return the user to the main screen without changing the active connection.

#### Scenario: Connection screen shown with Cancel button

- **WHEN** the user clicks "Add New Connection…"
- **THEN** the app navigates to the connection screen
- **THEN** a Cancel button is visible alongside the Connect button

#### Scenario: Successful new connection from connection screen

- **WHEN** the user fills in the connection form and clicks Connect
- **THEN** the connection probe is performed
- **THEN** on success, the new profile is saved and becomes the active profile
- **THEN** the app transitions to the torrent list

#### Scenario: Cancel returns to torrent list

- **WHEN** the user clicks Cancel on the connection screen
- **THEN** no profile is saved
- **THEN** the app returns to the torrent list with the previous connection unchanged

### Requirement: Dropdown inaccessible during profile switch

While a profile switch probe is in-flight, the profile switcher dropdown SHALL be disabled to prevent concurrent switch attempts.

#### Scenario: Dropdown disabled during switch

- **WHEN** a profile switch probe is in-flight
- **THEN** the dropdown button is non-interactive
- **THEN** the button label shows a loading indicator or the target profile name with a spinner
