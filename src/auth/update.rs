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

//! State-machine message handler for the auth dialog.

use iced::Task;
use secrecy::SecretString;

use crate::app::{AppState, Message};
use crate::crypto;
use crate::screens::connection;

use super::{AuthDialog, PendingAction};

/// Handle an auth dialog message.
///
/// Returns `Some(task)` if the message was consumed by the auth state machine,
/// or `None` if it should be forwarded to the per-screen update handler.
pub fn handle_message(state: &mut AppState, message: &Message) -> Option<Task<Message>> {
    match message {
        Message::AuthTabKeyPressed { shift } => {
            match &state.active_dialog {
                Some(AuthDialog::SetupPassphrase { .. }) => {
                    // Two-input ring: passphrase ↔ confirm.
                    // We don't track which is focused, so we always use focus_next /
                    // focus_previous which cycles naturally within the opaque overlay.
                    let task = if *shift {
                        iced::widget::operation::focus_previous()
                    } else {
                        iced::widget::operation::focus_next()
                    };
                    Some(task)
                }
                // Unlock dialog has one input — Tab is a no-op.
                Some(AuthDialog::Unlock { .. }) => Some(Task::none()),
                None => None,
            }
        }
        Message::AuthEnterPressed => match &state.active_dialog {
            Some(AuthDialog::Unlock { is_processing, .. }) if !*is_processing => {
                Some(Task::done(Message::SubmitUnlockPassphrase))
            }
            Some(AuthDialog::SetupPassphrase { is_processing, .. }) if !*is_processing => {
                Some(Task::done(Message::SubmitSetupPassphrase))
            }
            Some(_) => Some(Task::none()),
            None => None,
        },
        Message::DismissAuthDialog => {
            state.active_dialog = None;
            Some(Task::none())
        }
        Message::AuthSetupPassphraseChanged(v) => {
            if let Some(AuthDialog::SetupPassphrase {
                passphrase_input,
                error,
                ..
            }) = &mut state.active_dialog
            {
                *passphrase_input = v.clone();
                *error = None;
            }
            Some(Task::none())
        }
        Message::AuthSetupConfirmChanged(v) => {
            if let Some(AuthDialog::SetupPassphrase {
                confirm_input,
                error,
                ..
            }) = &mut state.active_dialog
            {
                *confirm_input = v.clone();
                *error = None;
            }
            Some(Task::none())
        }
        Message::AuthUnlockPassphraseChanged(v) => {
            if let Some(AuthDialog::Unlock {
                passphrase_input,
                error,
                ..
            }) = &mut state.active_dialog
            {
                *passphrase_input = v.clone();
                *error = None;
            }
            Some(Task::none())
        }
        Message::SubmitSetupPassphrase => {
            let Some(AuthDialog::SetupPassphrase {
                pending_profile_id,
                pending_password,
                passphrase_input,
                confirm_input,
                error,
                is_processing,
            }) = &mut state.active_dialog
            else {
                return Some(Task::none());
            };
            if *is_processing {
                return Some(Task::none());
            }
            if passphrase_input != confirm_input {
                *error = Some("Passphrases do not match".to_owned());
                return Some(Task::none());
            }
            if passphrase_input.is_empty() {
                *error = Some("Passphrase cannot be empty".to_owned());
                return Some(Task::none());
            }
            let profile_id = *pending_profile_id;
            let password = pending_password.clone();
            let passphrase = passphrase_input.clone();
            *is_processing = true;
            let task = Task::perform(
                async move {
                    let passphrase_c = passphrase.clone();
                    let (hash, encrypted) = tokio::task::spawn_blocking(move || {
                        let hash = crypto::hash_passphrase(&passphrase_c);
                        let encrypted = crypto::encrypt_password(&passphrase_c, &password);
                        (hash, encrypted)
                    })
                    .await
                    .expect("passphrase setup task panicked");
                    (passphrase, hash, profile_id, encrypted)
                },
                |(passphrase, hash, pid, ep)| Message::SetupPassphraseComplete {
                    passphrase,
                    hash,
                    profile_id: pid,
                    encrypted_password: ep,
                },
            );
            Some(task)
        }
        Message::SetupPassphraseComplete {
            passphrase,
            hash,
            profile_id,
            encrypted_password,
        } => {
            state.active_dialog = None;
            state.unlocked_passphrase = Some(SecretString::new(passphrase.clone()));
            state.profiles.master_passphrase_hash = Some(hash.clone());
            if let Some(p) = state
                .profiles
                .profiles
                .iter_mut()
                .find(|p| p.id == *profile_id)
            {
                p.encrypted_password = Some(encrypted_password.clone());
            }
            let snap = state.profiles.clone();
            Some(Task::perform(async move { snap.save().await }, |_| {
                Message::Noop
            }))
        }
        Message::SubmitUnlockPassphrase => {
            let Some(AuthDialog::Unlock {
                passphrase_input,
                is_processing,
                ..
            }) = &state.active_dialog
            else {
                return Some(Task::none());
            };
            if *is_processing {
                return Some(Task::none());
            }
            if passphrase_input.is_empty() {
                if let Some(AuthDialog::Unlock { error, .. }) = &mut state.active_dialog {
                    *error = Some("Passphrase cannot be empty".to_owned());
                }
                return Some(Task::none());
            }
            let passphrase = passphrase_input.clone();
            let hash = state
                .profiles
                .master_passphrase_hash
                .clone()
                .unwrap_or_default();
            if let Some(AuthDialog::Unlock { is_processing, .. }) = &mut state.active_dialog {
                *is_processing = true;
            }
            let task = Task::perform(
                async move {
                    let passphrase_c = passphrase.clone();
                    let valid = tokio::task::spawn_blocking(move || {
                        crypto::verify_passphrase(&passphrase_c, &hash)
                    })
                    .await
                    .expect("passphrase verify task panicked");
                    (passphrase, valid)
                },
                |(passphrase, valid)| Message::UnlockPassphraseResult { passphrase, valid },
            );
            Some(task)
        }
        Message::UnlockPassphraseResult { passphrase, valid } => {
            if !valid {
                if let Some(AuthDialog::Unlock {
                    error,
                    is_processing,
                    ..
                }) = &mut state.active_dialog
                {
                    *error = Some("Incorrect passphrase".to_owned());
                    *is_processing = false;
                }
                return Some(Task::none());
            }
            state.unlocked_passphrase = Some(SecretString::new(passphrase.clone()));
            let pending = match state.active_dialog.take() {
                Some(AuthDialog::Unlock { pending_action, .. }) => pending_action,
                _ => return Some(Task::none()),
            };
            match pending {
                PendingAction::ConnectToProfile(id) => {
                    let Some(profile) = state.profiles.get(id) else {
                        return Some(Task::none());
                    };
                    let creds = profile.credentials(Some(passphrase.as_str()));
                    Some(Task::done(Message::Connection(
                        connection::Message::ConnectWithCreds {
                            profile_id: id,
                            creds,
                        },
                    )))
                }
                PendingAction::SavePassword {
                    profile_id,
                    password,
                } => {
                    let pw = passphrase.clone();
                    let task = Task::perform(
                        async move {
                            let encrypted = tokio::task::spawn_blocking(move || {
                                crypto::encrypt_password(&pw, &password)
                            })
                            .await
                            .expect("encrypt task panicked");
                            (profile_id, encrypted)
                        },
                        |(pid, ep)| Message::EncryptPasswordReady {
                            profile_id: pid,
                            encrypted_password: ep,
                        },
                    );
                    Some(task)
                }
                PendingAction::TestConnectionFromSettings { profile_id } => {
                    let Some(profile) = state.profiles.get(profile_id) else {
                        return Some(Task::none());
                    };
                    let creds = profile.credentials(Some(passphrase.as_str()));
                    let url = creds.rpc_url();
                    let probe = Task::perform(
                        async move {
                            crate::rpc::session_get(&url, &creds, "")
                                .await
                                .map_err(|e| e.to_string())
                        },
                        |r| {
                            Message::Settings(
                                crate::screens::settings::Message::TestConnectionResult(r),
                            )
                        },
                    );
                    Some(probe)
                }
            }
        }
        Message::EncryptPasswordReady {
            profile_id,
            encrypted_password,
        } => {
            if let Some(p) = state
                .profiles
                .profiles
                .iter_mut()
                .find(|p| p.id == *profile_id)
            {
                p.encrypted_password = Some(encrypted_password.clone());
            }
            let snap = state.profiles.clone();
            Some(Task::perform(async move { snap.save().await }, |_| {
                Message::Noop
            }))
        }
        _ => None,
    }
}
