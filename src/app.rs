// Copyright 2026 The clutch authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
use secrecy::{ExposeSecret, SecretString};

use crate::auth::{AuthDialog, PendingAction};
use crate::crypto;
use crate::profile::{ProfileStore, resolve_theme_config};
use crate::screens::connection::{self, ConnectionScreen};
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
    AutoConnectResult(Result<crate::rpc::SessionData, String>),

    // -- Connection screen (delegated) --
    Connection(connection::Message),

    // -- Main screen (delegated) --
    Main(main_screen::Message),

    // -- Settings screen (delegated) --
    Settings(settings::Message),

    // -- Auth dialog --
    AuthSetupPassphraseChanged(String),
    AuthSetupConfirmChanged(String),
    AuthUnlockPassphraseChanged(String),
    SubmitSetupPassphrase,
    SubmitUnlockPassphrase,
    DismissAuthDialog,
    /// Tab pressed while an auth dialog is active — cycles within the dialog.
    AuthTabKeyPressed {
        shift: bool,
    },
    /// Enter pressed while an auth dialog is active — triggers primary CTA.
    AuthEnterPressed,
    /// Returned when the async passphrase hash + encryption task completes.
    SetupPassphraseComplete {
        passphrase: String,
        hash: String,
        profile_id: Uuid,
        encrypted_password: String,
    },
    /// Returned when the async passphrase verify task completes.
    UnlockPassphraseResult {
        passphrase: String,
        valid: bool,
    },
    /// Returned when an async encrypt-only task completes (already-unlocked save).
    EncryptPasswordReady {
        profile_id: Uuid,
        encrypted_password: String,
    },

    /// Fire-and-forget: result of a background save; no state change needed.
    Noop,
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
    /// Master passphrase held in memory for the session (never written to disk).
    /// Wrapped in `SecretString` so the memory is zeroized on drop.
    pub unlocked_passphrase: Option<SecretString>,
    /// Active auth dialog, rendered as a modal overlay over the current screen.
    pub active_dialog: Option<AuthDialog>,
    /// Whether Turtle Mode (alternative speed limits) is currently active on the daemon.
    pub alt_speed_enabled: bool,
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
            unlocked_passphrase: None,
            active_dialog: None,
            alt_speed_enabled: false,
        };
        // Re-emit the already-loaded store via Task::done so ProfilesLoaded
        // runs on the first event-loop tick (auto-connect, launchpad rebuild)
        // without hitting disk a second time.
        let task = Task::done(Message::ProfilesLoaded(initial));
        (state, task)
    }

    /// Return the active `iced::Theme` for the `.theme()` application callback.
    pub fn current_theme(&self) -> Theme {
        crate::theme::clutch_theme(self.theme == ThemeMode::Dark)
    }
}

// ── Elm functions ─────────────────────────────────────────────────────────────

/// If the connected profile has any configured bandwidth or seeding settings, push them
/// to the daemon via `session-set` on connect. Returns `None` when all values are default.
fn make_push_bandwidth_task(
    url: &str,
    creds: &crate::rpc::TransmissionCredentials,
    session_id: &str,
    profile: &crate::profile::ConnectionProfile,
) -> Option<Task<Message>> {
    let has_anything = profile.speed_limit_down_enabled
        || profile.speed_limit_down != 0
        || profile.speed_limit_up_enabled
        || profile.speed_limit_up != 0
        || profile.alt_speed_down != 0
        || profile.alt_speed_up != 0
        || profile.ratio_limit_enabled
        || profile.ratio_limit != 0.0;
    if !has_anything {
        return None;
    }
    let url = url.to_owned();
    let creds = creds.clone();
    let sid = session_id.to_owned();
    let args = crate::rpc::SessionSetArgs {
        speed_limit_down_enabled: Some(profile.speed_limit_down_enabled),
        speed_limit_down: Some(profile.speed_limit_down),
        speed_limit_up_enabled: Some(profile.speed_limit_up_enabled),
        speed_limit_up: Some(profile.speed_limit_up),
        alt_speed_down: if profile.alt_speed_down != 0 {
            Some(profile.alt_speed_down)
        } else {
            None
        },
        alt_speed_up: if profile.alt_speed_up != 0 {
            Some(profile.alt_speed_up)
        } else {
            None
        },
        seed_ratio_limited: Some(profile.ratio_limit_enabled),
        seed_ratio_limit: Some(profile.ratio_limit),
        ..Default::default()
    };
    Some(Task::perform(
        async move { crate::rpc::session_set(&url, &creds, &sid, &args).await },
        |_| Message::Noop,
    ))
}

/// Stash the current main screen (if any) and switch to Settings.
fn open_settings(state: &mut AppState, tab: SettingsTab) {
    if let Screen::Main(_) = &state.screen {
        if let Screen::Main(m) = std::mem::replace(
            &mut state.screen,
            Screen::Settings(SettingsScreen::new(
                &state.profiles,
                state.active_profile,
                tab,
            )),
        ) {
            state.stashed_main = Some(m);
        }
    } else {
        state.screen = Screen::Settings(SettingsScreen::new(
            &state.profiles,
            state.active_profile,
            tab,
        ));
    }
}

pub fn update(state: &mut AppState, message: Message) -> Task<Message> {
    // ── Startup ───────────────────────────────────────────────────────────────

    if let Message::Noop = message {
        return Task::none();
    }

    if let Message::ProfilesLoaded(store) = &message {
        let store = store.clone();
        state.theme = resolve_theme_config(store.general.theme);
        state.profiles = store;

        // Only rebuild the connection launchpad when we are still sitting on
        // the connection screen. If we are already on Main (e.g. a background
        // save fired ProfilesLoaded after a successful connect) we must not
        // navigate away.
        if matches!(state.screen, Screen::Connection(_)) {
            let conn = ConnectionScreen::new_launchpad(&state.profiles.profiles);
            let focus = conn.initial_focus_task().map(Message::Connection);
            state.screen = Screen::Connection(conn);
            return focus;
        }
        return Task::none();
    }

    if let Message::AutoConnectResult(result) = &message {
        match result {
            Ok(info) => {
                let id = state.profiles.last_connected.expect("set before probe");
                let profile = state.profiles.get(id).expect("profile must exist");
                let creds = profile.credentials(
                    state
                        .unlocked_passphrase
                        .as_ref()
                        .map(|s| s.expose_secret().as_str()),
                );
                let sid = info.session_id.clone();
                let profile_name = profile.name.clone();
                tracing::info!(profile = %profile_name, "Auto-connect succeeded");
                state.alt_speed_enabled = info.alt_speed_enabled;
                // Push the profile's stored bandwidth settings to the daemon.
                let push_task = make_push_bandwidth_task(&creds.rpc_url(), &creds, &sid, profile);
                state.screen = Screen::Main(MainScreen::new_with_label(
                    creds,
                    sid,
                    Some(profile_name),
                    Some(id),
                    state.profiles.general.refresh_interval,
                ));
                if let Some(t) = push_task {
                    return t;
                }
            }
            Err(err) => {
                tracing::warn!(error = %err, "Auto-connect failed; showing connection launchpad");
                let conn = ConnectionScreen::new_launchpad(&state.profiles.profiles);
                let focus = conn.initial_focus_task().map(Message::Connection);
                state.screen = Screen::Connection(conn);
                return focus;
            }
        }
        return Task::none();
    }

    // ── Global intercepts ─────────────────────────────────────────────────────

    match &message {
        Message::Main(main_screen::Message::Disconnect) => {
            tracing::info!("Disconnecting; returning to connection launchpad");
            state.active_profile = None;
            let conn = ConnectionScreen::new_launchpad(&state.profiles.profiles);
            let focus = conn.initial_focus_task().map(Message::Connection);
            state.screen = Screen::Connection(conn);
            return focus;
        }
        Message::Main(main_screen::Message::OpenSettingsClicked) => {
            open_settings(state, SettingsTab::General);
            return Task::none();
        }
        Message::Main(main_screen::Message::TurtleModeToggled) => {
            let new_val = !state.alt_speed_enabled;
            state.alt_speed_enabled = new_val;
            if let Screen::Main(main) = &state.screen {
                let params = main.list.params.clone();
                let args = crate::rpc::SessionSetArgs {
                    alt_speed_enabled: Some(new_val),
                    ..Default::default()
                };
                return Task::perform(
                    async move {
                        crate::rpc::session_set(
                            &params.url,
                            &params.credentials,
                            &params.session_id,
                            &args,
                        )
                        .await
                    },
                    |_| Message::Noop,
                );
            }
            return Task::none();
        }
        Message::Main(main_screen::Message::SessionDataLoaded(data)) => {
            state.alt_speed_enabled = data.alt_speed_enabled;
            return Task::none();
        }
        Message::Connection(connection::Message::ManageProfilesClicked) => {
            open_settings(state, SettingsTab::Connections);
            return Task::none();
        }
        Message::Connection(connection::Message::ConnectProfile(id)) => {
            let id = *id;
            let Some(profile) = state.profiles.get(id) else {
                return Task::none();
            };
            // If the profile has an encrypted password and the passphrase is not
            // yet unlocked, show the unlock dialog before connecting.
            if profile.encrypted_password.is_some() && state.unlocked_passphrase.is_none() {
                state.active_dialog = Some(AuthDialog::Unlock {
                    pending_action: PendingAction::ConnectToProfile(id),
                    passphrase_input: String::new(),
                    error: None,
                    is_processing: false,
                });
                return iced::widget::operation::focus(crate::auth::unlock_input_id());
            }
            // Passphrase available (or no password required) — build creds and connect.
            let creds = profile.credentials(
                state
                    .unlocked_passphrase
                    .as_ref()
                    .map(|s| s.expose_secret().as_str()),
            );
            return Task::done(Message::Connection(connection::Message::ConnectWithCreds {
                profile_id: id,
                creds,
            }));
        }
        _ => {}
    }

    // ── Auth dialog messages ──────────────────────────────────────────────────

    if let Some(task) = crate::auth::handle_message(state, &message) {
        // Keep the settings screen's cached hash in sync (e.g. after first-time
        // passphrase setup while the settings screen is open).
        if let Screen::Settings(settings) = &mut state.screen {
            settings.master_passphrase_hash = state.profiles.master_passphrase_hash.clone();
        }
        return task;
    }

    // ── Per-screen dispatch ───────────────────────────────────────────────────

    match &mut state.screen {
        Screen::Connection(conn) => {
            match message {
                Message::Connection(msg) => {
                    let (task, opt_success) = conn.update(msg);
                    if let Some(success) = opt_success {
                        state.alt_speed_enabled = success.alt_speed_enabled;
                        if let Some(id) = success.profile_id {
                            // Saved profile: persist last_connected.
                            state.active_profile = Some(id);
                            state.profiles.last_connected = Some(id);
                            let profile_name = state.profiles.get(id).map(|p| p.name.clone());
                            let push_task = state.profiles.get(id).and_then(|p| {
                                make_push_bandwidth_task(
                                    &success.creds.rpc_url(),
                                    &success.creds,
                                    &success.session_id,
                                    p,
                                )
                            });
                            let profiles_snap = state.profiles.clone();
                            let save_task =
                                Task::perform(async move { profiles_snap.save().await }, |_| {
                                    Message::Noop
                                });
                            state.screen = Screen::Main(MainScreen::new_with_label(
                                success.creds,
                                success.session_id,
                                profile_name,
                                Some(id),
                                state.profiles.general.refresh_interval,
                            ));
                            tracing::info!(profile_id = %id, "Connected via saved profile");
                            let base = save_task.chain(task.map(Message::Connection));
                            return if let Some(t) = push_task {
                                base.chain(t)
                            } else {
                                base
                            };
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
                    task.map(Message::Connection)
                }
                _ => Task::none(),
            }
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
                                store.adopt_from(&state.profiles);
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
                                store.adopt_from(&state.profiles);
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
                                store.adopt_from(&state.profiles);
                                store.last_connected = Some(profile_id);
                                state.profiles = store;
                                state.active_profile = Some(profile_id);
                                let profile = state.profiles.get(profile_id).expect("just saved");
                                let creds = profile.credentials(
                                    state
                                        .unlocked_passphrase
                                        .as_ref()
                                        .map(|s| s.expose_secret().as_str()),
                                );
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
                            settings::SettingsResult::ActiveProfileBandwidthSaved {
                                profile_id,
                                mut store,
                            } => {
                                // Only bandwidth/seeding settings changed — no reconnect.
                                // Just update the store and push new limits to the daemon.
                                store.adopt_from(&state.profiles);
                                state.profiles = store;
                                let push_task = state.profiles.get(profile_id).and_then(|p| {
                                    state.stashed_main.as_ref().map(|m| {
                                        make_push_bandwidth_task(
                                            &m.list.params.url,
                                            &m.list.params.credentials,
                                            &m.list.params.session_id,
                                            p,
                                        )
                                    })
                                });
                                let snap = state.profiles.clone();
                                let save = Task::perform(async move { snap.save().await }, |_| {
                                    Message::Noop
                                });
                                let base = task.map(Message::Settings).chain(save);
                                return if let Some(Some(push)) = push_task {
                                    base.chain(push)
                                } else {
                                    base
                                };
                            }
                            settings::SettingsResult::Closed(mut store) => {
                                store.adopt_from(&state.profiles);
                                state.profiles = store;
                                // Restore the stashed main screen if available —
                                // this avoids a reconnect/refetch.
                                if let Some(main) = state.stashed_main.take() {
                                    state.screen = Screen::Main(main);
                                    return Task::none();
                                }
                                // Fallback: re-open main from active profile.
                                if let Some(id) = state.active_profile
                                    && let Some(profile) = state.profiles.get(id)
                                {
                                    let creds = profile.credentials(
                                        state
                                            .unlocked_passphrase
                                            .as_ref()
                                            .map(|s| s.expose_secret().as_str()),
                                    );
                                    state.screen = Screen::Main(MainScreen::new_with_label(
                                        creds,
                                        String::new(),
                                        state.profiles.get(id).map(|p| p.name.clone()),
                                        Some(id),
                                        state.profiles.general.refresh_interval,
                                    ));
                                    return Task::none();
                                }
                                state.active_profile = None;
                                let conn =
                                    ConnectionScreen::new_launchpad(&state.profiles.profiles);
                                let focus = conn.initial_focus_task().map(Message::Connection);
                                state.screen = Screen::Connection(conn);
                                return focus;
                            }
                            settings::SettingsResult::SaveWithPassword {
                                profile_id,
                                password,
                                mut store,
                            } => {
                                store.adopt_from(&state.profiles);
                                state.profiles = store;
                                match &state.profiles.master_passphrase_hash {
                                    None => {
                                        state.active_dialog = Some(AuthDialog::SetupPassphrase {
                                            pending_profile_id: profile_id,
                                            pending_password: password,
                                            passphrase_input: String::new(),
                                            confirm_input: String::new(),
                                            error: None,
                                            is_processing: false,
                                        });
                                        return task.map(Message::Settings).chain(
                                            iced::widget::operation::focus(
                                                crate::auth::setup_passphrase_id(),
                                            ),
                                        );
                                    }
                                    Some(_) if state.unlocked_passphrase.is_none() => {
                                        state.active_dialog = Some(AuthDialog::Unlock {
                                            pending_action: PendingAction::SavePassword {
                                                profile_id,
                                                password,
                                            },
                                            passphrase_input: String::new(),
                                            error: None,
                                            is_processing: false,
                                        });
                                        return task.map(Message::Settings).chain(
                                            iced::widget::operation::focus(
                                                crate::auth::unlock_input_id(),
                                            ),
                                        );
                                    }
                                    Some(_) => {
                                        // Passphrase already unlocked — encrypt immediately.
                                        let passphrase = state
                                            .unlocked_passphrase
                                            .as_ref()
                                            .unwrap()
                                            .expose_secret()
                                            .to_owned();
                                        let encrypt_task = Task::perform(
                                            async move {
                                                let creds =
                                                    tokio::task::spawn_blocking(move || {
                                                        crypto::encrypt_password(
                                                            &passphrase,
                                                            &password,
                                                        )
                                                    })
                                                    .await
                                                    .expect("encrypt task panicked");
                                                (profile_id, creds)
                                            },
                                            |(pid, ep)| Message::EncryptPasswordReady {
                                                profile_id: pid,
                                                encrypted_password: ep,
                                            },
                                        );
                                        return task.map(Message::Settings).chain(encrypt_task);
                                    }
                                }
                            }
                            settings::SettingsResult::TestConnectionWithId { profile_id } => {
                                // If the passphrase is locked, prompt for it before testing.
                                if let Some(profile) = state.profiles.get(profile_id)
                                    && profile.encrypted_password.is_some()
                                    && state.unlocked_passphrase.is_none()
                                {
                                    state.active_dialog = Some(AuthDialog::Unlock {
                                        pending_action: PendingAction::TestConnectionFromSettings {
                                            profile_id,
                                        },
                                        passphrase_input: String::new(),
                                        error: None,
                                        is_processing: false,
                                    });
                                    return task.map(Message::Settings).chain(
                                        iced::widget::operation::focus(
                                            crate::auth::unlock_input_id(),
                                        ),
                                    );
                                }
                                // Passphrase available (or no password required).
                                let passphrase = state
                                    .unlocked_passphrase
                                    .as_ref()
                                    .map(|s| s.expose_secret().as_str());
                                if let Some(profile) = state.profiles.get(profile_id) {
                                    let creds = profile.credentials(passphrase);
                                    let url = creds.rpc_url();
                                    let probe = Task::perform(
                                        async move {
                                            crate::rpc::session_get(&url, &creds, "")
                                                .await
                                                .map_err(|e| e.to_string())
                                        },
                                        |r| {
                                            Message::Settings(
                                                settings::Message::TestConnectionResult(r),
                                            )
                                        },
                                    );
                                    return task.map(Message::Settings).chain(probe);
                                }
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

/// Render the current screen, wrapping it in the auth dialog overlay if needed.
pub fn view(state: &AppState) -> Element<'_, Message> {
    let base: Element<'_, Message> = match &state.screen {
        Screen::Connection(conn) => conn.view().map(Message::Connection),
        Screen::Main(main) => main
            .view(state.theme, state.alt_speed_enabled)
            .map(Message::Main),
        Screen::Settings(settings) => settings.view().map(Message::Settings),
    };
    crate::auth::view_overlay(state.active_dialog.as_ref(), base)
}

/// Return active subscriptions.
pub fn subscription(state: &AppState) -> Subscription<Message> {
    let dialog_active = state.active_dialog.is_some();

    match &state.screen {
        Screen::Connection(_) => {
            iced::keyboard::listen()
                .with(dialog_active)
                .filter_map(|(dialog_active, event)| {
                    use iced::keyboard::{Event, Key, key::Named};
                    if let Event::KeyPressed { key, modifiers, .. } = event {
                        match key.as_ref() {
                            Key::Named(Named::Tab) if dialog_active => {
                                Some(Message::AuthTabKeyPressed {
                                    shift: modifiers.shift(),
                                })
                            }
                            Key::Named(Named::Enter)
                                if dialog_active && !modifiers.control() && !modifiers.alt() =>
                            {
                                Some(Message::AuthEnterPressed)
                            }
                            Key::Named(Named::Tab) => {
                                Some(Message::Connection(connection::Message::TabKeyPressed {
                                    shift: modifiers.shift(),
                                }))
                            }
                            Key::Named(Named::Enter)
                                if !modifiers.control() && !modifiers.alt() =>
                            {
                                Some(Message::Connection(connection::Message::EnterPressed))
                            }
                            _ => None,
                        }
                    } else {
                        None
                    }
                })
        }

        Screen::Settings(_) => {
            iced::keyboard::listen()
                .with(dialog_active)
                .filter_map(|(dialog_active, event)| {
                    use iced::keyboard::{Event, Key, key::Named};
                    if let Event::KeyPressed { key, modifiers, .. } = event {
                        match key.as_ref() {
                            Key::Named(Named::Tab) if dialog_active => {
                                Some(Message::AuthTabKeyPressed {
                                    shift: modifiers.shift(),
                                })
                            }
                            Key::Named(Named::Enter)
                                if dialog_active && !modifiers.control() && !modifiers.alt() =>
                            {
                                Some(Message::AuthEnterPressed)
                            }
                            Key::Named(Named::Tab) => {
                                Some(Message::Settings(settings::Message::TabKeyPressed {
                                    shift: modifiers.shift(),
                                }))
                            }
                            Key::Named(Named::Enter)
                                if !modifiers.control() && !modifiers.alt() =>
                            {
                                Some(Message::Settings(settings::Message::EnterPressed))
                            }
                            _ => None,
                        }
                    } else {
                        None
                    }
                })
        }

        Screen::Main(main) => main.subscription().map(Message::Main),
    }
}
