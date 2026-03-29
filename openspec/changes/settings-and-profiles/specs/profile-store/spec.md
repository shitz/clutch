## ADDED Requirements

### Requirement: Profile data model

A `ConnectionProfile` SHALL contain: a UUID `id` (generated at creation, never shown to the user), a human-readable `name` string, `host` string, `port` u16, and an optional `username` string. The password SHALL NOT be stored in this struct; it is always fetched from the OS keyring on demand.

#### Scenario: Profile created with stable UUID

- **WHEN** a new profile is created
- **THEN** a UUID v4 is assigned as its `id`
- **THEN** the UUID is used as the keyring account key for that profile's password
- **THEN** the UUID does not change if the profile is renamed or its host/port is updated

### Requirement: Config file persistence

The `ProfileStore` SHALL persist all profiles (excluding passwords) to a TOML file at the OS-appropriate config directory (e.g. `~/.config/clutch/config.toml` on Linux, `~/Library/Application Support/clutch/config.toml` on macOS). The file SHALL also store `last_connected` (profile UUID) and the `[general]` section (theme, refresh_interval).

#### Scenario: Config file written after profile save

- **WHEN** the user saves a profile in the Settings screen
- **THEN** the config file is updated within the same async task
- **THEN** the app continues without blocking the UI thread

#### Scenario: Config file does not exist on first launch

- **WHEN** no config file is present
- **THEN** `ProfileStore::load` returns an empty store with default general settings
- **THEN** the app shows the connection screen

#### Scenario: Config file is corrupt or unparseable

- **WHEN** the config file exists but cannot be parsed as valid TOML
- **THEN** the parse error is logged to stderr
- **THEN** the app treats the result as an empty store (does not overwrite the corrupt file)
- **THEN** the app shows the connection screen

### Requirement: Password storage in OS keyring

Each profile's password SHALL be stored in the OS keyring with service name `"clutch"` and account equal to the profile's UUID string. Passwords SHALL be read from the keyring when displaying the profile detail form and when initiating a connection. Passwords SHALL be written to the keyring when the user saves a profile that has a non-empty password field.

#### Scenario: Password stored on profile save

- **WHEN** the user saves a profile with a non-empty password
- **THEN** `keyring::Entry::new("clutch", &uuid_str).set_password(pw)` is called
- **THEN** no password appears in the config file

#### Scenario: Password deleted on profile deletion

- **WHEN** the user confirms deletion of a profile
- **THEN** `keyring::Entry::new("clutch", &uuid_str).delete_password()` is called
- **THEN** the profile is removed from the config file

#### Scenario: Deleting the last_connected profile clears last_connected

- **WHEN** the user confirms deletion of a profile whose UUID matches `last_connected`
- **THEN** `last_connected` is set to `None` in the in-memory store
- **THEN** the config file is rewritten without a `last_connected` value

#### Scenario: Keyring unavailable

- **WHEN** the OS keyring is unavailable (headless, permissions, etc.)
- **THEN** the error is logged to stderr
- **THEN** the profile is still saved (without a stored password)
- **THEN** the UI does not crash; connection proceeds without credentials

### Requirement: Startup auto-connect

On launch, the app SHALL attempt to connect to the `last_connected` profile. If the connection succeeds, the app SHALL transition directly to the torrent list. If it fails, or if no `last_connected` profile exists, the app SHALL show the connection screen.

#### Scenario: Auto-connect succeeds

- **WHEN** the app launches and `last_connected` is set
- **THEN** the stored profile credentials (including keyring password) are used to probe the daemon
- **THEN** on success the app transitions to the torrent list without showing the connection screen

#### Scenario: Auto-connect fails

- **WHEN** the app launches and the `last_connected` daemon is unreachable
- **THEN** the connection screen is shown
- **THEN** the failed profile's fields are pre-filled in the connection form

#### Scenario: No profiles on launch

- **WHEN** the app launches and the profile store is empty
- **THEN** the connection screen is shown with default field values

### Requirement: last_connected updated on successful connection

The `ProfileStore` SHALL update `last_connected` to the active profile's UUID each time the user successfully connects to a daemon.

#### Scenario: last_connected persisted after connect

- **WHEN** a connection probe succeeds for a named profile
- **THEN** `last_connected` is set to that profile's UUID in the in-memory store
- **THEN** the config file is rewritten to reflect the updated `last_connected`
