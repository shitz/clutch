## Context

Clutch is an iced 0.14 desktop app following the Elm architecture. All state lives in `AppState`; the UI is a single `Screen` enum. There is currently no persistence layer — connection credentials are entered fresh every launch, and toggling the theme is ephemeral. The RPC layer (`src/rpc.rs`) is stable and reusable for the "Test Connection" probe.

Current screen routing:

```
Screen::Connection(ConnectionScreen)
Screen::Main(MainScreen)
```

## Goals / Non-Goals

**Goals:**

- Persist connection profiles and preferences across launches (TOML + OS keyring).
- Let users manage multiple Transmission daemons and switch between them without restarting.
- Expose per-profile settings (name, host, port, credentials) in a Settings screen.
- Add a general preferences section (theme, refresh interval) to the same Settings screen.
- Auto-connect to the last-used profile on startup.

**Non-Goals:**

- Continuous OS dark/light mode polling (system theme is sampled at startup and on-demand when the user selects "System" in Settings; no background watching).
- Profile sync, cloud backup, or import/export.
- Per-profile custom refresh intervals (one global interval).
- Windows/Linux keyring fallbacks beyond what `keyring` crate ships by default.

## Decisions

### 1. Config format: TOML at XDG config dir

**Decision**: `~/.config/clutch/config.toml` via `dirs::config_dir()`.

Rationale: TOML is idiomatic in the Rust ecosystem (`serde` + `toml` crates, zero friction). The `dirs` crate gives the correct OS-specific path on macOS (`~/Library/Application Support/clutch/config.toml`) and Linux.

Passwords are **never** written to this file; all other profile fields are.

Schema:

```toml
last_connected = "uuid-string"   # optional

[general]
theme = "System"                 # "Light" | "Dark" | "System"
refresh_interval = 5             # seconds, 1–30

[[profiles]]
id = "uuid-v4"
name = "Home NAS"
host = "192.168.1.10"
port = 9091
username = "admin"               # optional
```

### 2. Keyring keying: UUID per profile

**Decision**: Keyring service = `"clutch"`, account = profile UUID string.

Rationale: Keying by UUID means renames and host changes never orphan a stored password. UUIDs are generated with `uuid::Uuid::new_v4()` at profile creation and are opaque to the user. On profile delete, `keyring::Entry::new("clutch", &uuid_str).delete_password()` removes the secret cleanly.

Alternatives considered: keying by `host:port` would break on host edits; keying by profile name would break on renames.

### 3. Settings as a full Screen variant

**Decision**: Add `Screen::Settings(SettingsScreen)` to the `Screen` enum.

Rationale: The Connections tab contains a master-detail layout that is complex enough to be its own Elm component. Using `iced_aw::Modal` over `MainScreen` would require threading Settings messages through `MainScreen`, coupling them inappropriately. A dedicated screen is architecturally clean, and feels like a modern "Settings page" rather than a popup.

Navigation: toolbar button (gear icon) in `MainScreen` pushes `Screen::Settings`. Closing the Settings screen returns to `Screen::Main` (or `Screen::Connection` if the user has no active profile).

### 4. ProfileStore loaded synchronously at startup; handler triggered via `Task::done`

**Decision**: `AppState::new()` calls `ProfileStore::load_sync()` (a synchronous disk read) to obtain the initial store, then immediately re-emits it as `Message::ProfilesLoaded` using `Task::done(Message::ProfilesLoaded(initial))` — no second disk read.

Rationale: `AppState::new()` runs before the iced event loop starts, so there is no UI frame to block. Reading synchronously guarantees the correct theme is applied from the very first drawn frame (no flash of wrong theme). Using `Task::done` — which schedules an immediate message on the first event-loop tick — preserves the full `ProfilesLoaded` initialization flow (auto-connect probe, launchpad rebuild) without hitting disk a second time.

### 5. Unsaved-change guard via dirty flag + `stack!` + `opaque` overlay

**Decision**: `SettingsScreen` tracks `dirty: bool` and `confirm_discard: Option<PendingNavigation>`. Any pending navigation (tab switch, profile switch, close) checks `dirty` first. If `true`, `confirm_discard` is set and the overlay is rendered via `iced::widget::stack!` with the dialog wrapped in `iced::widget::opaque`.

Rationale: `iced_aw` removed its `Modal` widget in recent versions because `iced` 0.14 introduced first-class layering via `stack!`. `opaque` swallows all pointer and scroll events from underlying layers — exactly what a blocking dialog needs — with zero external dependencies. The overlay state lives inside `SettingsScreen`, keeping the top-level `Screen` enum clean.

### 6. "Add New Connection" reuses `Screen::Connection`

**Decision**: Clicking **Add New Connection…** in the profile switcher dropdown navigates to `Screen::Connection` with a `return_to_main: bool` flag set. When this flag is set, a **Cancel** button is shown on the connection screen; cancelling returns to `Screen::Main` without disconnecting.

Rationale: Building a separate `AddProfileModal` struct with its own connection form would duplicate all of `ConnectionScreen`'s fields, validation, and async probe logic (DRY violation). Reusing the existing screen is simpler and correct. The brief screen transition is acceptable UX.

### 7. In-memory profile edits, explicit Save

**Decision**: The Connections tab stores a `draft: ConnectionProfile` that is a clone of the selected profile. Edits mutate the draft only. `[Save]` writes the draft back to `ProfileStore` and flushes config to disk. `[Revert]` replaces the draft with the canonical profile.

This prevents accidentally corrupting running connections by half-typing a new host.

### 8. System theme detection — startup + on-demand

**Decision**: `dark_light::detect()` (from the `dark-light` crate) is called in two situations:

1. **At startup**, to resolve a persisted `ThemeMode::System` before the first frame is drawn.
2. **At runtime**, when the user selects "System" in the Settings General tab — a one-off detect call is fired immediately so the theme updates without an app restart.

In both cases the detected value resolves to `ThemeMode::Light` or `ThemeMode::Dark` and is applied to `AppState::theme`. There is no background polling.

Fallback: if `dark_light::detect()` returns an error or `Unknown`, default to `ThemeMode::Light`.

### 9. Stash / restore `MainScreen` on Settings open

**Decision**: When the user opens Settings from `Screen::Main`, the live `MainScreen` is moved into `AppState::stashed_main`. Closing Settings without saving restores it directly as `Screen::Main`, without re-probing the daemon or re-fetching the torrent list.

Rationale: Users frequently open Settings to check a profile name or tweak the refresh interval. Tearing down and rebuilding the full `MainScreen` (losing scroll position, sort order, torrent list) on every visit would be disruptive. Stashing is near-zero cost and keeps the UX non-destructive.

### 10. Lazy keychain password loading

**Decision**: When a profile is selected in the Connections tab, the password field is left blank (empty string). The OS keyring is only queried the first time the user clicks **[Test Connection]** — and only if the password field has not already been typed into.

Rationale: Reading from the OS keyring may trigger an unlock dialog or biometric prompt. Doing so on every profile selection would be disruptive when the user is just browsing profile names or host fields. Deferring to Test Connection (an explicit action) matches user intent.

## Risks / Trade-offs

- **Keyring unavailable on headless/CI systems** → `keyring` crate returns an error; the app should log a warning and skip password storage/retrieval gracefully (profile still usable without password if daemon allows it).
- **Config file corruption** → `toml::from_str` failure; treat as "no config" (start fresh), log the parse error. Do not overwrite the corrupt file automatically.
- **Profile switch mid-flight RPC** → Switching profile while a torrent-get is in-flight will cause the response to arrive for the wrong session. Mitigation: on switch, immediately invalidate/clear the torrent list and session-id before the new probe arrives. The RPC serialization queue in `MainScreen` handles ordering.
- **Saving the active profile changes connection params** → If the user edits host/port/credentials of the currently connected profile and saves, the existing session is stale. The app must drop the current session and re-probe with the new credentials automatically on save.

## Migration Plan

1. On first launch after update: config file does not exist → `ProfileStore::load_sync()` returns empty defaults. App shows the connection launchpad with the **Quick Connect** tab selected. No migration needed.
2. Quick Connect is ephemeral — credentials are held in memory only for the duration of the session. If the user wants to persist a connection, they open Settings > Connections and add a profile.

## Resolved Questions

- **Overlay for dialogs**: `iced_aw` removed its `Modal` widget; use `iced::widget::stack!` + `opaque` (Decision 5).
- **Test Connection probe**: fires a bare `session-get` without storing the returned session-id. The probe is purely diagnostic; its side-effects must not corrupt the live session state.
