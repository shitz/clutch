//! Application-level state, screen routing, and the top-level Elm loop.
//!
//! # Architecture
//!
//! The app follows `iced`'s Elm architecture with free-function style (iced 0.14):
//!
//! ```text
//! view(&State) → Element → user interaction → Message
//!                                                  ↓
//!                                    update(&mut State, Message)
//!                                                  ↓
//!                                          Task<Message>
//! ```
//!
//! # Non-blocking invariant
//!
//! **`update()` must return in microseconds.** All I/O (RPC calls, file I/O)
//! must be performed inside `iced::Task::perform()`. The returned `Task`
//! is executed by the tokio runtime on a background thread; the result arrives
//! back as a new `Message`. Violating this invariant will freeze the UI.

use uuid::Uuid;

use iced::{Element, Subscription, Task, Theme};

use crate::profile::{ProfileStore, resolve_theme_config};
use crate::screens::connection::{ConnectionScreen, ConnectionTab};
use crate::screens::main_screen::{self, MainScreen};
use crate::screens::settings::{self, SettingsScreen, SettingsTab};

// ── Screen ────────────────────────────────────────────────────────────────────

/// Top-level screen router.
///
/// Holds exactly one screen at a time, making illegal UI states unrepresentable.
#[derive(Debug)]
pub enum Screen {
    /// The initial connection form. Shown on startup and after Disconnect.
    Connection(ConnectionScreen),
    /// The main torrent list. Shown after a successful connection.
    Main(MainScreen),
    /// The full-screen settings / profiles editor.
    Settings(SettingsScreen),
}

// ── Message ───────────────────────────────────────────────────────────────────

/// Every event that can occur in the application.
#[derive(Debug, Clone)]
pub enum Message {
    // -- Startup --
    /// Config file loaded from disk.
    ProfilesLoaded(ProfileStore),
    /// Result of the auto-connect probe fired after `ProfilesLoaded`.
    AutoConnectResult(Result<crate::rpc::SessionInfo, String>),

    // -- Connection screen --
    HostChanged(String),
    PortChanged(String),
    UsernameChanged(String),
    PasswordChanged(String),
    ConnectClicked,
    SessionProbeResult(Result<crate::rpc::SessionInfo, String>),
    /// Tab change on the launchpad.
    ConnectionTabSelected(ConnectionTab),
    /// User clicked a saved profile card.
    ConnectProfile(Uuid),
    /// User clicked "Manage / Add Profile" on the launchpad.
    ManageProfilesClicked,
    /// Fire-and-forget: result of a background save; no state change needed.
    Noop,

    // -- Main screen (delegated) --
    Main(main_screen::Message),

    // -- Settings screen (delegated) --
    Settings(settings::Message),
}

// ── Theme mode ────────────────────────────────────────────────────────────────

/// Resolved display theme — always `Light` or `Dark`.
///
/// `System` preference is in [`crate::profile::ThemeConfig`] and gets resolved
/// at startup and on-demand. Only the concrete resolved value is stored here.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
}

// ── App state ─────────────────────────────────────────────────────────────────

/// Root application state.
#[derive(Debug)]
pub struct AppState {
    pub screen: Screen,
    /// Resolved display theme (Light or Dark).
    pub theme: ThemeMode,
    /// Persistent profiles and general settings.
    pub profiles: ProfileStore,
    /// UUID of the currently connected profile.
    pub active_profile: Option<Uuid>,
    /// Stashed main screen while Settings is open, so we can restore it
    /// without a reconnect/refetch when the user closes Settings.
    pub stashed_main: Option<MainScreen>,
}

impl AppState {
    /// Create the initial app state.
    ///
    /// Returns `(state, task)` — the task loads the profile store from disk.
    pub fn new() -> (Self, Task<Message>) {
        // Load synchronously so the correct theme is applied from the very first
        // draw — before the async ProfilesLoaded message can arrive.
        let initial = crate::profile::ProfileStore::load_sync();
        let theme = resolve_theme_config(initial.general.theme);
        let state = AppState {
            screen: Screen::Connection(ConnectionScreen::default()),
            theme,
            profiles: initial.clone(),
            active_profile: None,
            stashed_main: None,
        };
        // Re-emit the already-loaded store via Task::done so ProfilesLoaded
        // runs on the first event-loop tick (auto-connect, launchpad rebuild)
        // without hitting disk a second time.
        let task = Task::done(Message::ProfilesLoaded(initial));
        (state, task)
    }

    /// Return the active `iced::Theme` for the `.theme()` application callback.
    pub fn current_theme(&self) -> Theme {
        match self.theme {
            ThemeMode::Dark => crate::theme::material_dark_theme(),
            ThemeMode::Light => crate::theme::material_light_theme(),
        }
    }
}

// ── Elm functions ─────────────────────────────────────────────────────────────

pub fn update(state: &mut AppState, message: Message) -> Task<Message> {
    // ── Startup ───────────────────────────────────────────────────────────────

    if let Message::Noop = message {
        return Task::none();
    }

    if let Message::ProfilesLoaded(store) = &message {
        let store = store.clone();
        state.theme = resolve_theme_config(store.general.theme);
        let auto_id = store.last_connected;
        state.profiles = store;

        if let Some(id) = auto_id {
            if let Some(profile) = state.profiles.get(id) {
                tracing::info!(profile = %profile.name, "Auto-connecting to last profile");
                let creds = profile.credentials();
                let url = creds.rpc_url();
                return Task::perform(
                    async move {
                        crate::rpc::session_get(&url, &creds, "")
                            .await
                            .map_err(|e| e.to_string())
                    },
                    Message::AutoConnectResult,
                );
            }
        }
        // Only rebuild the connection launchpad when we are still sitting on
        // the connection screen. If we are already on Main (e.g. a background
        // save fired ProfilesLoaded after a successful connect) we must not
        // navigate away.
        if matches!(state.screen, Screen::Connection(_)) {
            state.screen =
                Screen::Connection(ConnectionScreen::new_launchpad(&state.profiles.profiles));
        }
        return Task::none();
    }

    if let Message::AutoConnectResult(result) = &message {
        match result {
            Ok(info) => {
                let id = state.profiles.last_connected.expect("set before probe");
                let profile = state.profiles.get(id).expect("profile must exist");
                let creds = profile.credentials();
                let sid = info.session_id.clone();
                state.active_profile = Some(id);
                let profile_name = profile.name.clone();
                tracing::info!(profile = %profile_name, "Auto-connect succeeded");
                state.screen = Screen::Main(MainScreen::new_with_label(
                    creds,
                    sid,
                    Some(profile_name),
                    Some(id),
                    state.profiles.general.refresh_interval,
                ));
            }
            Err(err) => {
                tracing::warn!(error = %err, "Auto-connect failed; showing connection launchpad");
                state.screen =
                    Screen::Connection(ConnectionScreen::new_launchpad(&state.profiles.profiles));
            }
        }
        return Task::none();
    }

    // ── Global intercepts ─────────────────────────────────────────────────────

    match &message {
        Message::Main(main_screen::Message::Disconnect) => {
            tracing::info!("Disconnecting; returning to connection launchpad");
            state.active_profile = None;
            state.screen =
                Screen::Connection(ConnectionScreen::new_launchpad(&state.profiles.profiles));
            return Task::none();
        }
        Message::Main(main_screen::Message::OpenSettingsClicked) => {
            // Stash the current main screen so we can restore it on close.
            if let Screen::Main(_) = &state.screen {
                if let Screen::Main(m) = std::mem::replace(
                    &mut state.screen,
                    Screen::Settings(SettingsScreen::new(
                        &state.profiles,
                        state.active_profile,
                        SettingsTab::General,
                    )),
                ) {
                    state.stashed_main = Some(m);
                }
            } else {
                state.screen = Screen::Settings(SettingsScreen::new(
                    &state.profiles,
                    state.active_profile,
                    SettingsTab::General,
                ));
            }
            return Task::none();
        }
        Message::ManageProfilesClicked => {
            // Stash if coming from Main.
            if let Screen::Main(_) = &state.screen {
                if let Screen::Main(m) = std::mem::replace(
                    &mut state.screen,
                    Screen::Settings(SettingsScreen::new(
                        &state.profiles,
                        state.active_profile,
                        SettingsTab::Connections,
                    )),
                ) {
                    state.stashed_main = Some(m);
                }
            } else {
                state.screen = Screen::Settings(SettingsScreen::new(
                    &state.profiles,
                    state.active_profile,
                    SettingsTab::Connections,
                ));
            }
            return Task::none();
        }
        _ => {}
    }

    // ── Per-screen dispatch ───────────────────────────────────────────────────

    match &mut state.screen {
        Screen::Connection(conn) => {
            let (task, opt_success) = conn.update(message);
            if let Some(success) = opt_success {
                if let Some(id) = success.profile_id {
                    // Saved profile: persist last_connected.
                    state.active_profile = Some(id);
                    state.profiles.last_connected = Some(id);
                    let profile_name = state.profiles.get(id).map(|p| p.name.clone());
                    let profiles_snap = state.profiles.clone();
                    let save_task =
                        Task::perform(async move { profiles_snap.save().await }, |_| Message::Noop);
                    state.screen = Screen::Main(MainScreen::new_with_label(
                        success.creds,
                        success.session_id,
                        profile_name,
                        Some(id),
                        state.profiles.general.refresh_interval,
                    ));
                    tracing::info!(profile_id = %id, "Connected via saved profile");
                    return save_task.chain(task);
                } else {
                    // Ephemeral quick connect: nothing is persisted.
                    state.active_profile = None;
                    state.screen = Screen::Main(MainScreen::new_with_label(
                        success.creds,
                        success.session_id,
                        None,
                        None,
                        state.profiles.general.refresh_interval,
                    ));
                    tracing::info!("Connected via quick connect (ephemeral)");
                }
            }
            task
        }

        Screen::Main(main) => match message {
            Message::Main(msg) => main.update(msg).map(Message::Main),
            _ => Task::none(),
        },

        Screen::Settings(settings) => {
            match message {
                Message::Settings(msg) => {
                    let (task, opt_result) = settings.update(msg);
                    if let Some(result) = opt_result {
                        match result {
                            settings::SettingsResult::GeneralSettingsSaved {
                                theme_config,
                                mut store,
                            } => {
                                store.last_connected = state.profiles.last_connected;
                                state.profiles = store;
                                state.theme = resolve_theme_config(theme_config);
                                // Propagate new interval to stashed main (if settings were
                                // opened while main was active).
                                if let Some(m) = &mut state.stashed_main {
                                    m.refresh_interval = state.profiles.general.refresh_interval;
                                }
                                // Re-save so last_connected is not lost (the settings
                                // screen's own save omits it).
                                let snap = state.profiles.clone();
                                return task.map(Message::Settings).chain(Task::perform(
                                    async move { snap.save().await },
                                    |_| Message::Noop,
                                ));
                            }
                            settings::SettingsResult::StoreUpdated(mut store) => {
                                store.last_connected = state.profiles.last_connected;
                                if let Some(lid) = store.last_connected {
                                    if store.get(lid).is_none() {
                                        store.last_connected = None;
                                    }
                                }
                                state.profiles = store;
                                // Re-save so last_connected is not lost.
                                let snap = state.profiles.clone();
                                return task.map(Message::Settings).chain(Task::perform(
                                    async move { snap.save().await },
                                    |_| Message::Noop,
                                ));
                            }
                            settings::SettingsResult::ActiveProfileSaved {
                                profile_id,
                                mut store,
                            } => {
                                // Re-probe using AutoConnectResult so the main screen
                                // loads fresh session info with updated credentials.
                                store.last_connected = Some(profile_id);
                                state.profiles = store;
                                state.active_profile = Some(profile_id);
                                let profile = state.profiles.get(profile_id).expect("just saved");
                                let creds = profile.credentials();
                                let url = creds.rpc_url();
                                let probe = Task::perform(
                                    async move {
                                        crate::rpc::session_get(&url, &creds, "")
                                            .await
                                            .map_err(|e| e.to_string())
                                    },
                                    Message::AutoConnectResult,
                                );
                                return task.map(Message::Settings).chain(probe);
                            }
                            settings::SettingsResult::Closed(mut store) => {
                                store.last_connected = state.profiles.last_connected;
                                if let Some(lid) = store.last_connected {
                                    if store.get(lid).is_none() {
                                        store.last_connected = None;
                                    }
                                }
                                state.profiles = store;
                                // Restore the stashed main screen if available —
                                // this avoids a reconnect/refetch.
                                if let Some(main) = state.stashed_main.take() {
                                    state.screen = Screen::Main(main);
                                    return Task::none();
                                }
                                // Fallback: re-open main from active profile.
                                if let Some(id) = state.active_profile {
                                    if let Some(profile) = state.profiles.get(id) {
                                        let creds = profile.credentials();
                                        state.screen = Screen::Main(MainScreen::new_with_label(
                                            creds,
                                            String::new(),
                                            state.profiles.get(id).map(|p| p.name.clone()),
                                            Some(id),
                                            state.profiles.general.refresh_interval,
                                        ));
                                        return Task::none();
                                    }
                                }
                                state.active_profile = None;
                                state.screen = Screen::Connection(ConnectionScreen::new_launchpad(
                                    &state.profiles.profiles,
                                ));
                            }
                        }
                    }
                    task.map(Message::Settings)
                }
                _ => Task::none(),
            }
        }
    }
}

/// Render the current screen.
pub fn view(state: &AppState) -> Element<'_, Message> {
    match &state.screen {
        Screen::Connection(conn) => conn.view(),
        Screen::Main(main) => main.view(state.theme).map(Message::Main),
        Screen::Settings(settings) => settings.view().map(Message::Settings),
    }
}

/// Return active subscriptions.
pub fn subscription(state: &AppState) -> Subscription<Message> {
    match &state.screen {
        Screen::Connection(_) | Screen::Settings(_) => Subscription::none(),
        Screen::Main(main) => main.subscription().map(Message::Main),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screens::main_screen;
    use crate::screens::torrent_list;

    fn dummy_main_state() -> AppState {
        let creds = crate::rpc::TransmissionCredentials {
            host: "localhost".to_owned(),
            port: 9091,
            username: None,
            password: None,
        };
        let id = Uuid::new_v4();
        AppState {
            screen: Screen::Main(MainScreen::new_with_label(
                creds,
                "sid".to_owned(),
                None,
                None,
                1,
            )),
            theme: ThemeMode::Dark,
            profiles: ProfileStore::default(),
            active_profile: Some(id),
            stashed_main: None,
        }
    }
}
