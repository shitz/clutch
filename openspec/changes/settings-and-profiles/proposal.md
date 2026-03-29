## Why

Clutch currently requires users to re-enter connection details every launch and can only connect to a single Transmission daemon. Users who manage multiple daemons (e.g., home NAS, seedbox) have no way to switch between them, and there is no persistent application settings store for preferences like theme and refresh rate.

## What Changes

- Introduce a persistent config file (`~/.config/clutch/config.toml`) storing connection profiles and app preferences.
- Store per-profile passwords securely in the OS keyring via the `keyring` crate (UUIDs as keyring keys, never exposed to users).
- Replace the toolbar "Disconnect" button with a profile switcher dropdown that lists saved profiles and provides quick access to add/manage connections.
- Add a Settings screen (a new `Screen` variant) with two tabs: **General** and **Connections**.
  - General tab: theme selector (Light / Dark / System-at-startup) and daemon refresh interval (1–30 s).
  - Connections tab: master-detail UI for creating, editing, testing, and deleting connection profiles.
- Extend the `ThemeMode` enum with a `System` variant that reads the OS preference once at startup.
- Extend the connection screen to always prompt for a profile name before saving (guest/one-time connections removed).
- Auto-connect on startup to the last successfully used profile; fall back to the connection screen if it fails or no profiles exist.
- "Add New Connection…" opens a modal sheet over the main screen rather than navigating away.
- "Test Connection" button in the profile detail form validates credentials before saving.
- Unsaved-change guard prompts users before switching profiles or tabs within the Settings screen.

## Capabilities

### New Capabilities

- `profile-store`: Persistent storage of connection profiles (TOML config + keyring passwords). Covers CRUD operations, UUID-keyed keyring integration, and the startup auto-connect flow.
- `settings-screen`: Full-screen Settings UI with General and Connections tabs, including the master-detail profile editor, Test Connection, Save/Revert logic, and unsaved-change guard dialogs.
- `profile-switcher`: Toolbar dropdown that shows the active profile, lists saved profiles for one-click switching, and has shortcuts to "Add New Connection…" and "Manage Connections…".

### Modified Capabilities

- `connection-screen`: The save-profile checkbox and profile-name input are added. Guest (unsaved) connections are removed — a profile is always created on connect.

## Impact

- **New dependencies**: `keyring` (OS keyring), `serde`/`toml` (config persistence), `dirs` (XDG config path), `uuid` (profile IDs), `dark-light` (system theme detection at startup).
- **`src/app.rs`**: `AppState` gains `profiles: ProfileStore`, `active_profile: Option<Uuid>`, and refined `ThemeMode`. `Screen` enum gains `Settings(SettingsScreen)`. New messages for profile switching and settings navigation.
- **`src/rpc.rs`**: No structural changes; `TransmissionCredentials` is reused as the profile's runtime credential view.
- **`src/screens/`**: New `settings.rs` module; `torrent_list.rs` toolbar modified for the profile dropdown; `connection.rs` gains profile-save fields.
- **`src/profile.rs`** (new): `ConnectionProfile`, `ProfileStore`, load/save/migrate logic.
