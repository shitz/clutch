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

//! Startup flow and app-level routing helpers for the root application state.

use secrecy::ExposeSecret;
use uuid::Uuid;

use iced::Task;

use crate::auth::{AuthDialog, PendingAction};
use crate::profile::{ConnectionProfile, ProfileStore, resolve_theme_config};
use crate::rpc::worker::RpcWork;
use crate::rpc::{self, SessionData, SessionSetArgs, TransmissionCredentials};
use crate::screens::connection::{self, ConnectionScreen};
use crate::screens::main_screen::{self, MainScreen};
use crate::screens::settings::{SettingsScreen, SettingsTab};
use crate::screens::torrent_list;

use super::{AppState, Message, Screen};

/// Handle startup-only messages before normal screen dispatch begins.
pub(super) fn handle_startup_message(
    state: &mut AppState,
    message: &Message,
) -> Option<Task<Message>> {
    match message {
        Message::ProfilesLoaded(store) => Some(apply_loaded_profiles(state, store.clone())),
        Message::AutoConnectResult(result) => Some(apply_auto_connect_result(state, result)),
        _ => None,
    }
}

/// Handle top-level messages that reroute screens or trigger app-wide side effects.
pub(super) fn handle_global_message(
    state: &mut AppState,
    message: &Message,
) -> Option<Task<Message>> {
    match message {
        Message::Main(main_screen::Message::Disconnect) => {
            tracing::info!("Disconnecting; returning to connection launchpad");
            state.active_profile = None;
            if let Some(tray) = &state.tray {
                tray.items.set_connected(false);
                tray.items.reset_speeds();
            }
            Some(show_connection_launchpad(state))
        }
        Message::Main(main_screen::Message::OpenSettingsClicked) => {
            open_settings(state, SettingsTab::General);
            Some(Task::none())
        }
        Message::Main(main_screen::Message::TurtleModeToggled) => {
            // Optimistically flip the tray checkmark now, before the async
            // session-set round-trip completes. SessionDataLoaded will confirm
            // (or correct) the state once the next poll arrives.
            if let Some(tray) = &state.tray {
                tray.items
                    .turtle_mode
                    .set_text(if !state.alt_speed_enabled {
                        "● Turtle Mode"
                    } else {
                        "○ Turtle Mode"
                    });
            }
            Some(toggle_turtle_mode(state))
        }
        Message::Main(main_screen::Message::SessionDataLoaded(data)) => {
            state.alt_speed_enabled = data.alt_speed_enabled;
            if let Some(tray) = &state.tray {
                tray.items.set_turtle_active(data.alt_speed_enabled);
            }
            Some(Task::none())
        }
        Message::Main(main_screen::Message::ProfilePathUsed(path)) => {
            let Some(profile_id) = state.active_profile else {
                return Some(Task::none());
            };
            let Some(profile) = state
                .profiles
                .profiles
                .iter_mut()
                .find(|p| p.id == profile_id)
            else {
                return Some(Task::none());
            };
            // Prepend, dedup, cap at 10.
            profile.recent_download_paths.retain(|p| p != path);
            profile.recent_download_paths.insert(0, path.clone());
            profile.recent_download_paths.truncate(10);
            let updated_paths = profile.recent_download_paths.clone();
            // Propagate into the live TorrentListScreen so the view stays consistent.
            if let Screen::Main(main) = &mut state.screen {
                main.list.recent_download_paths = updated_paths;
            }
            let snapshot = state.profiles.clone();
            Some(Task::perform(async move { snapshot.save().await }, |_| {
                Message::Noop
            }))
        }
        // Update tray speed labels on every poll; return None so main still processes it.
        Message::Main(main_screen::Message::List(torrent_list::Message::TorrentsUpdated(Ok(
            torrents,
        )))) => {
            if let Some(tray) = &state.tray {
                let dl: i64 = torrents.iter().map(|t| t.rate_download).sum();
                let ul: i64 = torrents.iter().map(|t| t.rate_upload).sum();
                tray.items
                    .speed_down
                    .set_text(format!("↓  {}", crate::format::format_speed(dl)));
                tray.items
                    .speed_up
                    .set_text(format!("↑  {}", crate::format::format_speed(ul)));
            }
            None
        }
        // Hide the main window instead of exiting when the close button is clicked.
        Message::WindowCloseRequested(id) => {
            state.main_window_id = Some(*id);
            Some(iced::window::set_mode(*id, iced::window::Mode::Hidden))
        }
        // Restore or surface the main window from the tray / behind other windows.
        Message::TrayAction(crate::tray::TrayAction::ShowWindow) => {
            let task = if let Some(id) = state.main_window_id {
                // Window was hidden via the close button; restore and focus.
                Task::batch([
                    iced::window::set_mode(id, iced::window::Mode::Windowed),
                    iced::window::gain_focus(id),
                ])
            } else {
                // Window is visible but may be behind other windows.
                // Resolve the main window ID at runtime and bring it to front.
                iced::window::oldest().then(|opt_id| match opt_id {
                    Some(id) => iced::window::gain_focus(id),
                    None => Task::none(),
                })
            };
            Some(task)
        }
        Message::TrayAction(crate::tray::TrayAction::PauseAll) => {
            if let Screen::Main(main) = &mut state.screen {
                let ids: Vec<i64> = main.list.torrents.iter().map(|t| t.id).collect();
                if !ids.is_empty() {
                    main.list.enqueue(RpcWork::TorrentStop {
                        params: main.list.params.clone(),
                        ids,
                    });
                }
            }
            Some(Task::none())
        }
        Message::TrayAction(crate::tray::TrayAction::ResumeAll) => {
            if let Screen::Main(main) = &mut state.screen {
                let ids: Vec<i64> = main.list.torrents.iter().map(|t| t.id).collect();
                if !ids.is_empty() {
                    main.list.enqueue(RpcWork::TorrentStart {
                        params: main.list.params.clone(),
                        ids,
                    });
                }
            }
            Some(Task::none())
        }
        Message::TrayAction(crate::tray::TrayAction::ToggleTurtle) => Some(Task::done(
            Message::Main(main_screen::Message::TurtleModeToggled),
        )),
        Message::TrayAction(crate::tray::TrayAction::Exit) => Some(iced::exit()),
        Message::Connection(connection::Message::ManageProfilesClicked) => {
            open_settings(state, SettingsTab::Connections);
            Some(Task::none())
        }
        Message::Connection(connection::Message::ConnectProfile(profile_id)) => {
            Some(connect_saved_profile(state, *profile_id))
        }
        _ => None,
    }
}

/// Forward connection-screen messages and reconcile successful connects into `AppState`.
pub(super) fn handle_connection_message(state: &mut AppState, message: Message) -> Task<Message> {
    let (task, opt_success) = match (&mut state.screen, message) {
        (Screen::Connection(connection), Message::Connection(msg)) => connection.update(msg),
        _ => return Task::none(),
    };

    let Some(success) = opt_success else {
        return task.map(Message::Connection);
    };

    state.alt_speed_enabled = success.alt_speed_enabled;
    if let Some(tray) = &state.tray {
        tray.items.set_connected(true);
        tray.items.set_turtle_active(success.alt_speed_enabled);
    }

    if let Some(profile_id) = success.profile_id {
        state.active_profile = Some(profile_id);
        state.profiles.last_connected = Some(profile_id);

        let profile_name = state
            .profiles
            .get(profile_id)
            .map(|profile| profile.name.clone());
        let push_task = state.profiles.get(profile_id).and_then(|profile| {
            make_push_bandwidth_task(
                &success.creds.rpc_url(),
                &success.creds,
                &success.session_id,
                profile,
            )
        });
        let profiles_snapshot = state.profiles.clone();

        state.screen = Screen::Main(Box::new(MainScreen::new_with_label(
            success.creds,
            success.session_id,
            profile_name,
            Some(profile_id),
            state.profiles.general.refresh_interval,
        )));
        // Seed the list with the profile's recent download paths so the add-dialog
        // dropdown is immediately populated without waiting for a ProfilePathUsed event.
        if let Screen::Main(main) = &mut state.screen
            && let Some(profile) = state.profiles.get(profile_id)
        {
            main.list.recent_download_paths = profile.recent_download_paths.clone();
        }
        tracing::info!(profile_id = %profile_id, "Connected via saved profile");

        let save_task = Task::perform(async move { profiles_snapshot.save().await }, |_| {
            Message::Noop
        });
        let base = save_task.chain(task.map(Message::Connection));
        return if let Some(push_task) = push_task {
            base.chain(push_task)
        } else {
            base
        };
    }

    state.active_profile = None;
    state.screen = Screen::Main(Box::new(MainScreen::new_with_label(
        success.creds,
        success.session_id,
        None,
        None,
        state.profiles.general.refresh_interval,
    )));
    tracing::info!("Connected via quick connect (ephemeral)");
    task.map(Message::Connection)
}

/// Build a `session-set` task when a profile stores non-default bandwidth settings.
pub(super) fn make_push_bandwidth_task(
    url: &str,
    credentials: &TransmissionCredentials,
    session_id: &str,
    profile: &ConnectionProfile,
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
    let credentials = credentials.clone();
    let session_id = session_id.to_owned();
    let args = SessionSetArgs {
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
        async move { rpc::session_set(&url, &credentials, &session_id, &args).await },
        |_| Message::Noop,
    ))
}

/// Resolve a profile into runtime credentials using the unlocked passphrase, if present.
pub(super) fn profile_credentials(
    state: &AppState,
    profile: &ConnectionProfile,
) -> TransmissionCredentials {
    profile.credentials(
        state
            .unlocked_passphrase
            .as_ref()
            .map(|passphrase| passphrase.expose_secret().as_str()),
    )
}

/// Probe a saved profile by issuing a background `session-get` request.
pub(super) fn probe_profile(state: &AppState, profile_id: Uuid) -> Task<Message> {
    let Some(profile) = state.profiles.get(profile_id) else {
        return Task::none();
    };

    let credentials = profile_credentials(state, profile);
    let url = credentials.rpc_url();
    Task::perform(
        async move {
            rpc::session_get(&url, &credentials, "")
                .await
                .map_err(|error| error.to_string())
        },
        Message::AutoConnectResult,
    )
}

/// Rebuild the connection launchpad and return any initial focus task it requires.
pub(super) fn show_connection_launchpad(state: &mut AppState) -> Task<Message> {
    let connection = ConnectionScreen::new_launchpad(&state.profiles.profiles);
    let focus = connection.initial_focus_task().map(Message::Connection);
    state.screen = Screen::Connection(connection);
    focus
}

/// Stash the current main screen, if any, and switch to the settings screen.
pub(super) fn open_settings(state: &mut AppState, tab: SettingsTab) {
    if let Screen::Main(_) = &state.screen {
        if let Screen::Main(main) = std::mem::replace(
            &mut state.screen,
            Screen::Settings(SettingsScreen::new(
                &state.profiles,
                state.active_profile,
                tab,
            )),
        ) {
            state.stashed_main = Some(main);
        }
    } else {
        state.screen = Screen::Settings(SettingsScreen::new(
            &state.profiles,
            state.active_profile,
            tab,
        ));
    }
}

fn apply_loaded_profiles(state: &mut AppState, store: ProfileStore) -> Task<Message> {
    state.theme = resolve_theme_config(store.general.theme);
    state.profiles = store;

    if matches!(state.screen, Screen::Connection(_)) {
        return show_connection_launchpad(state);
    }

    Task::none()
}

fn apply_auto_connect_result(
    state: &mut AppState,
    result: &Result<SessionData, String>,
) -> Task<Message> {
    match result {
        Ok(info) => {
            let profile_id = state.profiles.last_connected.expect("set before probe");
            let profile = state.profiles.get(profile_id).expect("profile must exist");
            let credentials = profile_credentials(state, profile);
            let session_id = info.session_id.clone();
            let profile_name = profile.name.clone();
            tracing::info!(profile = %profile_name, "Auto-connect succeeded");
            state.alt_speed_enabled = info.alt_speed_enabled;
            if let Some(tray) = &state.tray {
                tray.items.set_connected(true);
                tray.items.set_turtle_active(info.alt_speed_enabled);
            }

            let push_task = make_push_bandwidth_task(
                &credentials.rpc_url(),
                &credentials,
                &session_id,
                profile,
            );
            state.screen = Screen::Main(Box::new(MainScreen::new_with_label(
                credentials,
                session_id,
                Some(profile_name),
                Some(profile_id),
                state.profiles.general.refresh_interval,
            )));

            if let Some(push_task) = push_task {
                push_task
            } else {
                Task::none()
            }
        }
        Err(error) => {
            tracing::warn!(error = %error, "Auto-connect failed; showing connection launchpad");
            show_connection_launchpad(state)
        }
    }
}

fn connect_saved_profile(state: &mut AppState, profile_id: Uuid) -> Task<Message> {
    let Some(profile) = state.profiles.get(profile_id) else {
        return Task::none();
    };
    let has_encrypted_password = profile.encrypted_password.is_some();

    if has_encrypted_password && state.unlocked_passphrase.is_none() {
        state.active_dialog = Some(AuthDialog::Unlock {
            pending_action: PendingAction::ConnectToProfile(profile_id),
            passphrase_input: String::new(),
            error: None,
            is_processing: false,
        });
        return iced::widget::operation::focus(crate::auth::unlock_input_id());
    }

    let Some(profile) = state.profiles.get(profile_id) else {
        return Task::none();
    };
    let credentials = profile_credentials(state, profile);
    Task::done(Message::Connection(connection::Message::ConnectWithCreds {
        profile_id,
        creds: credentials,
    }))
}

fn toggle_turtle_mode(state: &mut AppState) -> Task<Message> {
    let new_value = !state.alt_speed_enabled;
    state.alt_speed_enabled = new_value;

    if let Screen::Main(main) = &state.screen {
        let params = main.list.params.clone();
        let args = SessionSetArgs {
            alt_speed_enabled: Some(new_value),
            ..Default::default()
        };
        return Task::perform(
            async move {
                rpc::session_set(&params.url, &params.credentials, &params.session_id, &args).await
            },
            |_| Message::Noop,
        );
    }

    Task::none()
}
