## 1. Dependencies and Module Scaffolding

- [x] 1.1 Add `keyring`, `toml`, `serde`, `dirs`, `uuid` (with v4 feature), and `dark-light` to `Cargo.toml`
- [x] 1.2 Create `src/profile.rs` with `ConnectionProfile` and `ProfileStore` structs (serde derives, no logic yet)
- [x] 1.3 Create `src/screens/settings.rs` as an empty module with placeholder `SettingsScreen` struct
- [x] 1.4 Declare `mod profile` in `src/main.rs` (or `lib.rs`) and `mod settings` in `src/screens/mod.rs`

## 2. Profile Store — Data Layer

- [x] 2.1 Implement `ProfileStore::load() -> Result<ProfileStore>` — reads TOML from `dirs::config_dir()/clutch/config.toml`, returns default on missing file, logs and returns default on parse error
- [x] 2.2 Implement `ProfileStore::save(&self) -> Result<()>` — serializes to TOML and writes atomically to config file (write to tmp, rename)
- [x] 2.3 Implement `ProfileStore::get_password(id: Uuid) -> Option<String>` — fetches from keyring, logs errors, returns `None` on failure
- [x] 2.4 Implement `ProfileStore::set_password(id: Uuid, password: &str) -> Result<()>` — stores in keyring, logs and returns error on failure
- [x] 2.5 Implement `ProfileStore::delete_password(id: Uuid)` — deletes keyring entry, logs (does not fail if missing)
- [x] 2.6 Write unit tests for `load()` with: missing file, valid file, corrupt file

## 3. App State — Integrate Profile Store

- [x] 3.1 Add `ThemeMode::System` variant; implement `resolve_theme_mode()` that calls `dark_light::detect()` and defaults to `Light` on error; call it at startup and also whenever the user selects "System" in Settings at runtime
- [x] 3.2 Add `profiles: ProfileStore`, `active_profile: Option<Uuid>` to `AppState`
- [x] 3.3 Add `Screen::Settings(SettingsScreen)` to the `Screen` enum
- [x] 3.4 Add `Message::ProfilesLoaded(ProfileStore)` and `Message::AutoConnectResult(Result<SessionInfo, String>)` variants
- [x] 3.5 Update `app::update` to handle `ProfilesLoaded`: store profiles, resolve `ThemeMode::System`, fire auto-connect task if `last_connected` is set
- [x] 3.6 Update `app::update` to handle `AutoConnectResult`: on success transition to `Screen::Main`; on failure show `Screen::Connection` with last profile pre-filled
- [x] 3.7 Issue `Task::perform(ProfileStore::load, Message::ProfilesLoaded)` from `AppState::new()` (or `main()`)

## 4. Connection Screen — Profile Save

- [x] 4.1 Add `profile_name: String` and `profile_name_manually_edited: bool` fields to `ConnectionScreen`
- [x] 4.2 Add `Message::ProfileNameChanged(String)` and render the Profile Name text input
- [x] 4.3 Implement auto-mirroring logic: when `HostChanged` fires and `!profile_name_manually_edited`, set `profile_name` to the new host value
- [x] 4.4 Disable the Connect button when `profile_name.is_empty()`
- [x] 4.5 On `SessionProbeResult(Ok)`: create a `ConnectionProfile` and call `ProfileStore::save` + `set_password` before transitioning to `MainScreen`
- [x] 4.6 Update `ConnectionScreen::new(profile: Option<&ConnectionProfile>, return_to_main: bool)` to accept a profile for pre-filling and a flag that shows a Cancel button (used by auto-connect failure path and "Add New Connection" flow)

## 5. Profile Switcher — Toolbar Dropdown

- [x] 5.1 Add `switching_profile: bool` field to `MainScreen` (or `TorrentListScreen`)
- [x] 5.2 Replace the Disconnect button in the toolbar with a dropdown button labeled with the active profile name + "▾"
- [x] 5.3 Add `Message::ProfileSwitchDropdownToggled`, `Message::ProfileSelected(Uuid)`, `Message::AddNewConnectionClicked`, `Message::ManageConnectionsClicked`, and `Message::DisconnectClicked` to the main-screen message set
- [x] 5.4 Implement dropdown menu rendering: profile list, divider, "Add New Connection…", "Manage Connections…", "Disconnect"
- [x] 5.5 Handle `ProfileSelected(id)`: clear torrent list, load credentials from store + keyring, fire session probe; disable dropdown while in-flight
- [x] 5.6 On successful profile switch probe: update `AppState::active_profile`, call `ProfileStore::save` to persist `last_connected`
- [x] 5.7 On failed profile switch probe: show inline error in the toolbar area, re-enable dropdown
- [x] 5.8 Handle `ManageConnectionsClicked`: transition to `Screen::Settings` with the Connections tab pre-selected
- [x] 5.9 Handle `AddNewConnectionClicked`: transition to `Screen::Connection` with `return_to_main = true`
- [x] 5.10 Handle `DisconnectClicked`: drop current session, transition to `Screen::Connection`

## 6. Settings Screen — Structure

- [x] 6.1 Define `SettingsScreen` struct with fields: `active_tab: SettingsTab { General | Connections }`, `dirty: bool`, `confirm_discard: Option<PendingNavigation>`, general fields (theme draft, refresh_interval draft), connection fields (profile list, selected_profile_id, draft fields)
- [x] 6.2 Define `SettingsScreen::Message` enum covering all settings interactions
- [x] 6.3 Implement `SettingsScreen::new(profiles: &ProfileStore, theme: ThemeMode, active_tab: SettingsTab)` constructor
- [x] 6.4 Implement `SettingsScreen::view()` with tab bar (Material MD3 style), General content, Connections content, and close button
- [x] 6.5 Implement `SettingsScreen::update()` dispatching to General and Connections handlers

## 7. Settings Screen — General Tab

- [x] 7.1 Render theme selector with three options (Light / Dark / System); emit `ThemeDraftChanged(ThemeMode)` on selection
- [x] 7.2 Immediate theme preview: when `ThemeDraftChanged` fires, update `AppState::theme` directly (not gated on Save); if value is `System`, call `resolve_theme_mode()` once immediately
- [x] 7.3 Render refresh interval text input; validate 1–30 range inline; disable Save when invalid
- [x] 7.4 On Save: write updated general settings to `ProfileStore` and trigger `ProfileStore::save` task

## 8. Settings Screen — Connections Tab

- [x] 8.1 Render left column: scrollable profile list with active-profile highlight; `[+]` and `[🗑]` buttons at the bottom
- [x] 8.2 Disable `[🗑]` button when the selected profile is the currently active (connected) profile
- [x] 8.3 Render right column: detail form (Profile Name, Host, Port, Username, Password-masked); placeholder when nothing selected
- [x] 8.4 Implement `[+]` handler: insert a new blank `ConnectionProfile` (UUID v4, default host/port), select it, mark `dirty = true`
- [x] 8.5 Implement `[🗑]` handler: show confirmation overlay via `stack!` + `opaque` ("Are you sure you want to delete '<name>'?")
- [x] 8.6 Implement deletion confirmed: remove profile from store, call `delete_password`, clear `last_connected` if deleted UUID matches it, rewrite config, clear right column
- [x] 8.7 Implement profile field edits: all edits update `draft` fields, set `dirty = true`
- [x] 8.8 Implement `[Save]`: write draft back to profile store, update keyring if password changed, call `ProfileStore::save` task, set `dirty = false`; if saved profile is the active profile, drop session and re-probe with new credentials
- [x] 8.9 Implement `[Revert]`: reset draft from saved profile, set `dirty = false`
- [x] 8.10 Implement profile name propagation to left column list on Save (list reflects new name immediately)

## 9. Test Connection Button

- [x] 9.1 Add `test_result: Option<TestResult { Success | Failure(String) }>` and `testing: bool` to the profile draft state
- [x] 9.2 Render `[Test Connection]` button; disable when `testing = true`
- [x] 9.3 Fire `Task::perform(rpc::probe_session, Message::TestConnectionResult)` using current draft credentials (bare `session-get`, do not store the returned session-id)
- [x] 9.4 On result: set `test_result` and `testing = false`; render ✓ or ✗ inline below the button

## 10. Unsaved Change Guard

- [x] 10.1 Intercept profile-switch, tab-switch, and close-settings messages: if `dirty`, store pending navigation as `confirm_discard` and render the guard overlay
- [x] 10.2 Render guard overlay using `iced::widget::stack!` + `opaque`: semi-transparent backdrop, centered dialog card with Save and Discard buttons
- [x] 10.3 Guard dialog Save: call the Save handler, then execute the pending navigation
- [x] 10.4 Guard dialog Discard: discard draft, set `dirty = false`, execute pending navigation

## 11. Wiring and Integration

- [x] 11.1 Update `app::update` to route `Message::Settings(SettingsMessage)` to `SettingsScreen::update` when `Screen::Settings` is active
- [x] 11.2 Handle settings-close message in `app::update`: return to `Screen::Main` if an active profile exists, else `Screen::Connection`
- [x] 11.3 Pass updated `ProfileStore` back from `SettingsScreen` on close so `AppState` has the latest profiles
- [x] 11.4 Ensure the main-screen subscription uses `AppState::refresh_interval` from `ProfileStore::general` (not a hard-coded constant)
- [x] 11.5 Thread `active_profile` name and UUID down to `TorrentListScreen` for the toolbar dropdown label and deletion guard

## 12. Polish and Edge Cases

- [x] 12.1 Handle "no profiles after deleting all": show placeholder in right column, disable `[🗑]`
- [x] 12.2 Handle profile switch to the currently active profile: no-op (silently skip the probe)
- [x] 12.3 Verify keyring-unavailable path: log warning, continue without stored password
- [x] 12.4 Add `ICON_SETTINGS` (gear) and `ICON_TRASH` (delete) codepoints to `src/theme.rs`
- [x] 12.5 Manual smoke-test: launch with no config → connect → verify profile saved → relaunch → auto-connect → open Settings → edit active profile → save (verify reconnect) → switch profiles via dropdown → disconnect

## 13. UX Polish (implemented)

- [x] 13.1 Loading splash: show "Connecting to host:port (Profile Name)…" from screen creation until first `TorrentsUpdated` response (`initial_load_done` flag on `TorrentListScreen`)
- [x] 13.2 Non-destructive Settings close: stash `MainScreen` in `AppState::stashed_main` when opening Settings; restore on Close without reconnect/refetch
- [x] 13.3 X close button (ICON_CLOSE) in Settings header replacing "← Back" text button
- [x] 13.4 Custom MD3 tab bar with size-15 text and 2 px underline indicator
- [x] 13.5 "✓ Settings saved" success toast on General tab after Save
- [x] 13.6 Test Connection: 5 s timeout, "Testing connection…" in-progress label, "✓ Connection test successful!" / "✗ Connection test failed: …" result labels
- [x] 13.7 Lazy keychain access: password only fetched from OS keyring on Test Connection click (not on profile selection)
- [x] 13.8 Revert/Save icon buttons (ICON_UNDO / ICON_SAVE) on both General and Connections tabs, pinned to bottom-left aligned with form labels
- [x] 13.9 General tab: dirty tracking (`general_dirty`), Revert restores last-saved values and re-applies theme preview
- [x] 13.10 Default `refresh_interval` changed from 5 s to 1 s; subscription uses configured value
