## ADDED Requirements

### Requirement: Settings is a top-level screen

The app SHALL provide a Settings screen as a dedicated `Screen` variant (not a floating modal over the torrent list). It SHALL be reachable via a gear icon button in the main-screen toolbar. The Settings screen SHALL be closed via an **X icon button** in the top-left of the header. Closing the Settings screen SHALL restore the live torrent list without reconnecting or re-fetching data.

#### Scenario: Navigate to Settings from main screen

- **WHEN** the user clicks the gear icon in the main-screen toolbar
- **THEN** the app transitions to the Settings screen
- **THEN** the torrent list is no longer visible

#### Scenario: Close Settings returns to torrent list non-destructively

- **WHEN** the user closes the Settings screen with no unsaved changes
- **THEN** the app returns to the torrent list exactly as it was (no reconnect, no refetch)
- **THEN** the torrent list scroll position and sort order are preserved

### Requirement: Settings tabbed layout

The Settings screen SHALL display a tab bar with at least two tabs: **General** and **Connections**. The tab bar SHALL use the same Material Design 3 tabbed style used elsewhere in the app.

#### Scenario: General tab shown by default

- **WHEN** the Settings screen first opens
- **THEN** the General tab is active

#### Scenario: Tab switching

- **WHEN** the user clicks the Connections tab
- **THEN** the Connections tab content replaces the General tab content

### Requirement: General tab — theme selector

The General tab SHALL include a theme selector with three options: **Light**, **Dark**, and **System**. Selecting an option SHALL immediately update the app's active theme. The selection SHALL be persisted to the config file when the user saves settings.

#### Scenario: Theme selection takes effect immediately

- **WHEN** the user selects "Dark" in the theme selector
- **THEN** the app theme switches to the Material dark theme without requiring a save

#### Scenario: System theme resolved at startup

- **WHEN** the app launches with theme set to "System"
- **THEN** the OS dark/light preference is read once
- **THEN** the app uses the corresponding Material theme for the session
- **THEN** no further OS preference polling occurs

#### Scenario: System theme selected at runtime triggers immediate detection

- **WHEN** the user selects "System" in the theme selector during an active session
- **THEN** `dark_light::detect()` is called once immediately
- **THEN** the resulting Light or Dark theme is applied without requiring an app restart

#### Scenario: System theme unavailable defaults to Light

- **WHEN** the OS returns an unknown or error result for dark mode detection
- **THEN** the app defaults to the Light theme

### Requirement: General tab — refresh interval

The General tab SHALL include a numeric text input for the daemon refresh interval. The input SHALL only accept integer values between 1 and 30 (inclusive), representing seconds. Out-of-range values SHALL prevent saving with an inline validation message.

#### Scenario: Valid refresh interval accepted

- **WHEN** the user enters a value between 1 and 30 and saves
- **THEN** the refresh interval is updated in the config file
- **THEN** the polling subscription uses the new interval

#### Scenario: Out-of-range value blocked

- **WHEN** the user enters 0 or a value greater than 30
- **THEN** an inline validation error is shown
- **THEN** the Save button is disabled until corrected

### Requirement: Connections tab — master-detail layout

The Connections tab SHALL display a two-column master-detail layout. The left column SHALL show a scrollable list of saved profile names. The right column SHALL show the editable fields for the currently selected profile. If no profile is selected or the list is empty, the right column SHALL show a placeholder message.

#### Scenario: Profile selected shows detail

- **WHEN** the user clicks a profile name in the left column
- **THEN** the right column populates with that profile's Name, Host, Port, Username, and masked Password

#### Scenario: Empty state placeholder

- **WHEN** the profile list is empty
- **THEN** the right column shows "No connections. Click '+' to add a new Transmission daemon."

#### Scenario: Active profile visually indicated

- **WHEN** a profile is the currently connected profile
- **THEN** it is highlighted in the left column list

### Requirement: Connections tab — add and delete profiles

The bottom of the left column SHALL have two action buttons: `[+]` to create a new blank profile and `[🗑]` (or `[-]`) to delete the currently selected profile. The delete button SHALL be disabled when the selected profile is the currently active (connected) profile.

#### Scenario: Add creates blank profile

- **WHEN** the user clicks `[+]`
- **THEN** a new profile entry is added to the list with a default name (e.g. "New Profile")
- **THEN** the new profile is immediately selected and its detail form is shown
- **THEN** default values are pre-filled: Host = "localhost", Port = 9091

#### Scenario: Delete disabled for active profile

- **WHEN** the selected profile in the left column is the currently connected profile
- **THEN** the `[🗑]` button is disabled
- **THEN** no deletion can be initiated for that profile

#### Scenario: Delete prompts for confirmation

- **WHEN** the user clicks `[🗑]` for a named profile
- **THEN** a confirmation dialog appears: "Are you sure you want to delete '<name>'? This cannot be undone."
- **THEN** the profile is not deleted until the user confirms

#### Scenario: Delete confirmed removes profile and keyring entry

- **WHEN** the user confirms deletion
- **THEN** the profile is removed from the list and config file
- **THEN** the OS keyring entry for this profile's UUID is deleted
- **THEN** the right column shows the placeholder message

#### Scenario: Delete cancelled leaves profile intact

- **WHEN** the user cancels the deletion dialog
- **THEN** the profile remains unchanged

### Requirement: Profile detail form fields

The right column SHALL contain editable text inputs for: **Profile Name**, **Host**, **Port** (numeric), **Username** (optional), and **Password** (masked, optional). The password field is populated lazily — it is loaded from the OS keyring only the first time the user clicks [Test Connection], not on profile selection, to avoid triggering an unnecessary keychain unlock prompt.

#### Scenario: Password field always masked

- **WHEN** a profile with a saved password is selected
- **THEN** the password field is initially blank (not pre-populated)
- **THEN** the field is displayed with masked characters (dots) while the user types

#### Scenario: Password loaded from keyring on Test Connection

- **WHEN** the user clicks [Test Connection] and has not typed a password
- **THEN** the stored keyring password is fetched once and placed in the field
- **THEN** subsequent Test Connection clicks use the already-loaded value

#### Scenario: Port pre-filled for new profiles

- **WHEN** a new blank profile is created
- **THEN** the Port field is pre-filled with 9091

### Requirement: Test Connection button

The right column SHALL include a **[Test Connection]** button. Clicking it SHALL fire a lightweight `session-get` probe using the current field values (not yet saved). The result SHALL be shown inline. While the probe is in-flight the label "Testing connection…" SHALL be displayed in place of any previous result.

#### Scenario: Test Connection in-flight label

- **WHEN** a test probe is in-flight
- **THEN** the text "Testing connection…" is shown inline next to the button
- **THEN** the [Test Connection] button is disabled

#### Scenario: Test Connection success

- **WHEN** the user clicks [Test Connection] and the daemon responds
- **THEN** a green "✓ Connection test successful!" message appears inline
- **THEN** no changes to the saved profile are made

#### Scenario: Test Connection failure

- **WHEN** the user clicks [Test Connection] and the daemon is unreachable or rejects auth
- **THEN** a red "✗ Connection test failed: <reason>" message appears inline
- **THEN** no changes to the saved profile are made

#### Scenario: Test Connection button disabled while in-flight

- **WHEN** a test probe is in-flight
- **THEN** the [Test Connection] button is disabled until the result arrives

### Requirement: Save and Revert buttons

The right column SHALL show **[Save]** and **[Revert]** buttons. Save SHALL write the draft to the profile store and persist the config file. Revert SHALL reset the draft to the last saved state. Both buttons SHALL only be enabled when there are unsaved changes.

#### Scenario: Save persists changes

- **WHEN** the user edits a profile and clicks [Save]
- **THEN** the profile store is updated in memory
- **THEN** the config file is rewritten asynchronously
- **THEN** if the password field was edited, the keyring entry is updated
- **THEN** the left column reflects the updated profile name
- **THEN** both Save and Revert become disabled (no pending changes)

#### Scenario: Saving the active profile triggers reconnection

- **WHEN** the user edits the profile that is currently connected and clicks [Save]
- **THEN** the current session is dropped
- **THEN** the app immediately re-probes the daemon using the updated credentials
- **THEN** on success the torrent list refreshes; on failure an error is shown in the main screen

#### Scenario: Revert discards draft changes

- **WHEN** the user edits a profile and clicks [Revert]
- **THEN** all field values are reset to the last saved state
- **THEN** both Save and Revert become disabled

### Requirement: Unsaved change guard

When the user has unsaved edits in the profile detail form, any navigation that would discard those edits (switching profiles, switching tabs, or closing Settings) SHALL prompt the user with a confirmation dialog offering **Save** and **Discard** options.

#### Scenario: Profile switch with unsaved changes

- **WHEN** the user has edited a profile and clicks a different profile in the left column
- **THEN** a dialog appears: "You have unsaved changes. Save or discard them?"
- **THEN** clicking Save saves and switches
- **THEN** clicking Discard discards and switches

#### Scenario: Tab switch with unsaved changes

- **WHEN** the user has unsaved changes and clicks the General tab
- **THEN** a dialog appears with Save / Discard options
- **THEN** after resolution, the tab switch proceeds

#### Scenario: Close Settings with unsaved changes

- **WHEN** the user attempts to close Settings with unsaved changes
- **THEN** the same Save / Discard dialog is shown before navigating away

### Requirement: Profile name reflects saved name in left column

When the user saves a profile with a new name, the left column list SHALL update immediately to show the new name.

#### Scenario: Renamed profile reflected in list

- **WHEN** the user changes the Profile Name field and saves
- **THEN** the left column entry updates to the new name without requiring a screen refresh
