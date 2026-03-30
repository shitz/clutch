## Implemented Design

The connection screen is a **two-tab launchpad** — not a form that always creates a profile.
Every launch of the app shows this screen first; once connected it is replaced by the torrent list.

---

## Requirement: Tabbed layout with Saved Profiles and Quick Connect

The connection screen SHALL display two tabs: **Saved Profiles** and **Quick Connect**.

- If saved profiles exist, the **Saved Profiles** tab is active by default.
- If no saved profiles exist, the **Quick Connect** tab is active by default.

#### Scenario: Default tab — profiles present

- **GIVEN** one or more saved profiles exist
- **WHEN** the app launches
- **THEN** the Saved Profiles tab is shown

#### Scenario: Default tab — no profiles

- **GIVEN** no saved profiles exist
- **WHEN** the app launches
- **THEN** the Quick Connect tab is shown

#### Scenario: Tab switching

- **WHEN** the user clicks the inactive tab
- **THEN** the tab content switches
- **THEN** any previous inline error is cleared

---

## Requirement: Saved Profiles tab — profile cards

The Saved Profiles tab SHALL show a clickable card for each saved profile. Clicking a card
immediately starts a connection probe using that profile's credentials (loaded from the OS
keyring at the moment of the click).

#### Scenario: Profile card shows name and address

- **WHEN** the Saved Profiles tab is visible
- **THEN** each card displays `<name>  —  <host>:<port>`

#### Scenario: Clicking a card starts the connection probe

- **WHEN** the user clicks a profile card
- **THEN** an in-progress label ("Connecting…") replaces the card text
- **THEN** all other cards are non-interactive until the probe completes or fails

#### Scenario: Probe succeeds — transition to torrent list

- **WHEN** the `session-get` probe returns a session ID
- **THEN** the app transitions to `Screen::Main`

#### Scenario: Probe fails — inline error

- **WHEN** the probe returns an error
- **THEN** an inline warning message is shown below the tab content
- **THEN** the profile cards become interactive again

#### Scenario: Empty saved-profiles state

- **WHEN** the Saved Profiles tab is shown but the profile list is empty
- **THEN** a "No saved profiles yet." placeholder is shown
- **THEN** a "⚙ Manage / Add Profile…" button is shown

---

## Requirement: Saved Profiles tab — manage profiles link

The Saved Profiles tab SHALL include a **⚙ Manage / Add Profile…** button at the bottom of
the card list. Clicking it navigates to `Screen::Settings` opened on the Connections tab.

#### Scenario: Navigate to Settings from connection launchpad

- **WHEN** the user clicks "⚙ Manage / Add Profile…"
- **THEN** the app transitions to the Settings screen with the Connections tab active

---

## Requirement: Quick Connect tab — ephemeral credentials form

The Quick Connect tab SHALL show a form with Host, Port, Username (optional), and Password
(optional) fields. Submitting the form starts a connection probe. **No profile is saved** —
credentials are held in memory only for the duration of the session.

Default field values: Host = `"localhost"`, Port = `"9091"`, username and password empty.

#### Scenario: Successful quick connect

- **WHEN** the user fills in credentials and clicks Connect
- **AND** the `session-get` probe succeeds
- **THEN** the app transitions to `Screen::Main`
- **THEN** no `ConnectionProfile` is created or persisted
- **THEN** `ConnectSuccess.profile_id` is `None`

#### Scenario: Quick connect with invalid port

- **WHEN** the user enters a non-numeric value in the Port field and clicks Connect
- **THEN** an inline error "Invalid port number." is shown
- **THEN** no probe is fired

#### Scenario: Connect button disabled while probe in-flight

- **WHEN** a probe is in-flight
- **THEN** the Connect button shows "Connecting…" and is non-interactive

#### Scenario: Quick connect failure shows error

- **WHEN** the probe returns an error
- **THEN** a warning with the error message is shown inline
- **THEN** the form fields remain editable

---

## REMOVED Requirements

### Requirement: Profile Name field on connection screen

**Reason removed**: The connection screen is now a launchpad. Quick Connect is explicitly
ephemeral — there is no profile name field because no profile is created. Profile management
is handled exclusively in Settings > Connections.

### Requirement: Guest connection (one-time connect without saving)

**Reason removed (previous entry superseded)**: The concept was re-introduced as
**Quick Connect** — an ephemeral tab. The original "REMOVED" notice referred to the
opt-in save checkbox; the ephemeral idea itself shipped as Quick Connect.
