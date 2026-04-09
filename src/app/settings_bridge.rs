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

//! Reconcile `SettingsScreen` results back into the root `AppState`.

use secrecy::ExposeSecret;

use iced::Task;

use crate::auth::{AuthDialog, PendingAction};
use crate::crypto;
use crate::profile::{ProfileStore, resolve_theme_config};
use crate::screens::main_screen::MainScreen;
use crate::screens::settings::{self, SettingsResult};

use super::routing::{
    make_push_bandwidth_task, probe_profile, profile_credentials, show_connection_launchpad,
};
use super::{AppState, Message, Screen};

/// Forward settings messages and apply any returned `SettingsResult` to `AppState`.
pub(super) fn handle_message(state: &mut AppState, message: Message) -> Task<Message> {
    let (task, opt_result) = match (&mut state.screen, message) {
        (Screen::Settings(settings), Message::Settings(msg)) => settings.update(msg),
        _ => return Task::none(),
    };

    let Some(result) = opt_result else {
        return task.map(Message::Settings);
    };

    handle_result(state, task, result)
}

fn handle_result(
    state: &mut AppState,
    task: Task<settings::Message>,
    result: SettingsResult,
) -> Task<Message> {
    match result {
        SettingsResult::GeneralSettingsSaved {
            theme_config,
            mut store,
        } => {
            store.adopt_from(&state.profiles);
            state.profiles = store;
            state.theme = resolve_theme_config(theme_config);
            if let Some(main) = &mut state.stashed_main {
                main.refresh_interval = state.profiles.general.refresh_interval;
            }
            save_profiles_task(task, state.profiles.clone())
        }
        SettingsResult::StoreUpdated(mut store) => {
            store.adopt_from(&state.profiles);
            state.profiles = store;
            save_profiles_task(task, state.profiles.clone())
        }
        SettingsResult::ActiveProfileSaved {
            profile_id,
            mut store,
        } => {
            store.adopt_from(&state.profiles);
            store.last_connected = Some(profile_id);
            state.profiles = store;
            state.active_profile = Some(profile_id);

            task.map(Message::Settings)
                .chain(probe_profile(state, profile_id))
        }
        SettingsResult::ActiveProfileBandwidthSaved {
            profile_id,
            mut store,
        } => {
            store.adopt_from(&state.profiles);
            state.profiles = store;

            let push_task = state.profiles.get(profile_id).and_then(|profile| {
                state.stashed_main.as_ref().and_then(|main| {
                    make_push_bandwidth_task(
                        &main.list.params.url,
                        &main.list.params.credentials,
                        &main.list.params.session_id,
                        profile,
                    )
                })
            });

            let base = save_profiles_task(task, state.profiles.clone());
            if let Some(push_task) = push_task {
                base.chain(push_task)
            } else {
                base
            }
        }
        SettingsResult::Closed(mut store) => {
            store.adopt_from(&state.profiles);
            state.profiles = store;

            if let Some(main) = state.stashed_main.take() {
                state.screen = Screen::Main(main);
                return Task::none();
            }

            if let Some(profile_id) = state.active_profile
                && let Some(profile) = state.profiles.get(profile_id)
            {
                let profile_name = profile.name.clone();
                let credentials = profile_credentials(state, profile);
                state.screen = Screen::Main(Box::new(MainScreen::new_with_label(
                    credentials,
                    String::new(),
                    Some(profile_name),
                    Some(profile_id),
                    state.profiles.general.refresh_interval,
                )));
                return Task::none();
            }

            state.active_profile = None;
            show_connection_launchpad(state)
        }
        SettingsResult::SaveWithPassword {
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
                    focus_setup_input(task)
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
                    focus_unlock_input(task)
                }
                Some(_) => {
                    let passphrase = state
                        .unlocked_passphrase
                        .as_ref()
                        .expect("unlock guard handled earlier")
                        .expose_secret()
                        .to_owned();
                    let encrypt_task = Task::perform(
                        async move {
                            let encrypted_password = tokio::task::spawn_blocking(move || {
                                crypto::encrypt_password(&passphrase, &password)
                            })
                            .await
                            .expect("encrypt task panicked");
                            (profile_id, encrypted_password)
                        },
                        |(profile_id, encrypted_password)| Message::EncryptPasswordReady {
                            profile_id,
                            encrypted_password,
                        },
                    );
                    task.map(Message::Settings).chain(encrypt_task)
                }
            }
        }
        SettingsResult::TestConnectionWithId { profile_id } => {
            if let Some(profile) = state.profiles.get(profile_id)
                && profile.encrypted_password.is_some()
                && state.unlocked_passphrase.is_none()
            {
                state.active_dialog = Some(AuthDialog::Unlock {
                    pending_action: PendingAction::TestConnectionFromSettings { profile_id },
                    passphrase_input: String::new(),
                    error: None,
                    is_processing: false,
                });
                return focus_unlock_input(task);
            }

            let Some(profile) = state.profiles.get(profile_id) else {
                return task.map(Message::Settings);
            };
            let credentials = profile_credentials(state, profile);
            let url = credentials.rpc_url();
            let probe = Task::perform(
                async move {
                    crate::rpc::session_get(&url, &credentials, "")
                        .await
                        .map_err(|error| error.to_string())
                },
                |result| Message::Settings(settings::Message::TestConnectionResult(result)),
            );
            task.map(Message::Settings).chain(probe)
        }
    }
}

fn save_profiles_task(task: Task<settings::Message>, snapshot: ProfileStore) -> Task<Message> {
    task.map(Message::Settings)
        .chain(Task::perform(async move { snapshot.save().await }, |_| {
            Message::Noop
        }))
}

fn focus_setup_input(task: Task<settings::Message>) -> Task<Message> {
    task.map(Message::Settings)
        .chain(iced::widget::operation::focus(
            crate::auth::setup_passphrase_id(),
        ))
}

fn focus_unlock_input(task: Task<settings::Message>) -> Task<Message> {
    task.map(Message::Settings)
        .chain(iced::widget::operation::focus(
            crate::auth::unlock_input_id(),
        ))
}
