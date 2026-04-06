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

//! The auth dialog is rendered as a modal overlay on top of the current screen.
//! Two flows are supported:
//!
//! 1. **Setup** — first-time passphrase creation: derives an Argon2id PHC hash
//!    and encrypts the pending profile password on a background thread.
//! 2. **Unlock** — session unlock: verifies the entered passphrase against the
//!    stored PHC hash, then executes the deferred [`PendingAction`].

use uuid::Uuid;

use iced::widget::{Space, button, column, container, row, stack, text, text_input};
use iced::{Element, Length, Task};
use secrecy::SecretString;

use crate::app::{AppState, Message};
use crate::crypto;
use crate::screens::connection;

// ── Stable widget IDs for auth dialog inputs ────────────────────────────────────

pub fn unlock_input_id() -> iced::widget::Id {
    iced::widget::Id::new("auth_unlock")
}

pub fn setup_passphrase_id() -> iced::widget::Id {
    iced::widget::Id::new("auth_setup_passphrase")
}

pub fn setup_confirm_id() -> iced::widget::Id {
    iced::widget::Id::new("auth_setup_confirm")
}

// ── Types ─────────────────────────────────────────────────────────────────────

/// Action to execute after the unlock passphrase dialog succeeds.
#[derive(Debug, Clone)]
pub enum PendingAction {
    ConnectToProfile(Uuid),
    SavePassword { profile_id: Uuid, password: String },
}

/// Active passphrase dialog — rendered as a modal overlay.
#[derive(Debug, Clone)]
pub enum AuthDialog {
    SetupPassphrase {
        pending_profile_id: Uuid,
        pending_password: String,
        passphrase_input: String,
        confirm_input: String,
        error: Option<String>,
        /// Set to `true` while the background hash+encrypt task is running,
        /// preventing duplicate submissions via double-click.
        is_processing: bool,
    },
    Unlock {
        pending_action: PendingAction,
        passphrase_input: String,
        error: Option<String>,
        /// Set to `true` while the background verify task is running.
        is_processing: bool,
    },
}

// ── State machine ─────────────────────────────────────────────────────────────

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

// ── View helpers ──────────────────────────────────────────────────────────────

/// Wrap `base` in the active auth dialog modal overlay, or return `base` unchanged.
pub fn view_overlay<'a>(
    dialog: Option<&'a AuthDialog>,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    match dialog {
        Some(AuthDialog::SetupPassphrase {
            passphrase_input,
            confirm_input,
            error,
            is_processing,
            ..
        }) => stack![
            base,
            view_setup_passphrase(
                passphrase_input,
                confirm_input,
                error.as_deref(),
                *is_processing
            )
        ]
        .into(),
        Some(AuthDialog::Unlock {
            passphrase_input,
            error,
            is_processing,
            ..
        }) => stack![
            base,
            view_unlock(passphrase_input, error.as_deref(), *is_processing)
        ]
        .into(),
        None => base,
    }
}

fn view_setup_passphrase<'a>(
    passphrase_input: &'a str,
    confirm_input: &'a str,
    error: Option<&'a str>,
    is_processing: bool,
) -> Element<'a, Message> {
    let error_row: Element<'_, Message> = if let Some(err) = error {
        text(err)
            .size(13)
            .style(|t: &iced::Theme| iced::widget::text::Style {
                color: Some(t.extended_palette().danger.base.color),
            })
            .into()
    } else {
        Space::new().into()
    };

    let card = container(
        column![
            text("Create master passphrase").size(18),
            text(
                "This passphrase protects your saved passwords. \
                 You will enter it once per app session."
            )
            .size(13)
            .style(|t: &iced::Theme| iced::widget::text::Style {
                color: Some(t.palette().text.scale_alpha(0.7)),
            }),
            text_input("Enter passphrase", passphrase_input)
                .id(setup_passphrase_id())
                .on_input_maybe((!is_processing).then_some(Message::AuthSetupPassphraseChanged))
                .secure(true)
                .padding([12, 16])
                .style(crate::theme::m3_text_input),
            text_input("Confirm passphrase", confirm_input)
                .id(setup_confirm_id())
                .on_input_maybe((!is_processing).then_some(Message::AuthSetupConfirmChanged))
                .secure(true)
                .padding([12, 16])
                .style(crate::theme::m3_text_input),
            error_row,
            row![
                button(text("Cancel").size(14))
                    .on_press_maybe((!is_processing).then_some(Message::DismissAuthDialog))
                    .padding([10, 20])
                    .style(crate::theme::m3_tonal_button),
                Space::new().width(Length::Fill),
                button(
                    text(if is_processing {
                        "Creating…"
                    } else {
                        "Create"
                    })
                    .size(14)
                )
                .on_press_maybe((!is_processing).then_some(Message::SubmitSetupPassphrase))
                .padding([10, 20])
                .style(crate::theme::m3_primary_button),
            ]
            .width(Length::Fill)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(12),
    )
    .padding(28)
    .max_width(420)
    .style(|t: &iced::Theme| {
        let is_dark = t.extended_palette().background.base.color.r < 0.5;
        iced::widget::container::Style {
            background: Some(iced::Background::Color(if is_dark {
                crate::theme::CARD_SURFACE_DARK
            } else {
                crate::theme::CARD_SURFACE_LIGHT
            })),
            border: iced::Border {
                radius: 12.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    });

    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_: &iced::Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(iced::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.5,
            })),
            ..Default::default()
        })
        .into()
}

fn view_unlock<'a>(
    passphrase_input: &'a str,
    error: Option<&'a str>,
    is_processing: bool,
) -> Element<'a, Message> {
    let error_row: Element<'_, Message> = if let Some(err) = error {
        text(err)
            .size(13)
            .style(|t: &iced::Theme| iced::widget::text::Style {
                color: Some(t.extended_palette().danger.base.color),
            })
            .into()
    } else {
        Space::new().into()
    };

    let card = container(
        column![
            text("Enter master passphrase").size(18),
            text("Enter your master passphrase to unlock your saved credentials.")
                .size(13)
                .style(|t: &iced::Theme| iced::widget::text::Style {
                    color: Some(t.palette().text.scale_alpha(0.7)),
                }),
            text_input("Master passphrase", passphrase_input)
                .id(unlock_input_id())
                .on_input_maybe((!is_processing).then_some(Message::AuthUnlockPassphraseChanged))
                .secure(true)
                .padding([12, 16])
                .style(crate::theme::m3_text_input),
            error_row,
            row![
                button(text("Cancel").size(14))
                    .on_press_maybe((!is_processing).then_some(Message::DismissAuthDialog))
                    .padding([10, 20])
                    .style(crate::theme::m3_tonal_button),
                Space::new().width(Length::Fill),
                button(
                    text(if is_processing {
                        "Verifying…"
                    } else {
                        "Unlock"
                    })
                    .size(14)
                )
                .on_press_maybe((!is_processing).then_some(Message::SubmitUnlockPassphrase))
                .padding([10, 20])
                .style(crate::theme::m3_primary_button),
            ]
            .width(Length::Fill)
            .align_y(iced::Alignment::Center),
        ]
        .spacing(12),
    )
    .padding(28)
    .max_width(420)
    .style(|t: &iced::Theme| {
        let is_dark = t.extended_palette().background.base.color.r < 0.5;
        iced::widget::container::Style {
            background: Some(iced::Background::Color(if is_dark {
                crate::theme::CARD_SURFACE_DARK
            } else {
                crate::theme::CARD_SURFACE_LIGHT
            })),
            border: iced::Border {
                radius: 12.0.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    });

    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_: &iced::Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(iced::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.5,
            })),
            ..Default::default()
        })
        .into()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use secrecy::ExposeSecret;

    use super::*;
    use crate::app::AppState;

    fn make_state() -> AppState {
        AppState::new().0
    }

    fn setup_dialog(profile_id: Uuid) -> AuthDialog {
        AuthDialog::SetupPassphrase {
            pending_profile_id: profile_id,
            pending_password: "hunter2".to_owned(),
            passphrase_input: String::new(),
            confirm_input: String::new(),
            error: None,
            is_processing: false,
        }
    }

    fn unlock_dialog() -> AuthDialog {
        AuthDialog::Unlock {
            pending_action: PendingAction::ConnectToProfile(Uuid::new_v4()),
            passphrase_input: String::new(),
            error: None,
            is_processing: false,
        }
    }

    // ── DismissAuthDialog ─────────────────────────────────────────────────────

    #[test]
    fn dismiss_clears_setup_dialog() {
        let mut state = make_state();
        state.active_dialog = Some(setup_dialog(Uuid::new_v4()));
        let result = handle_message(&mut state, &Message::DismissAuthDialog);
        assert!(result.is_some(), "message should be consumed");
        assert!(state.active_dialog.is_none());
    }

    #[test]
    fn dismiss_clears_unlock_dialog() {
        let mut state = make_state();
        state.active_dialog = Some(unlock_dialog());
        handle_message(&mut state, &Message::DismissAuthDialog);
        assert!(state.active_dialog.is_none());
    }

    #[test]
    fn dismiss_when_no_dialog_still_consumed() {
        let mut state = make_state();
        let result = handle_message(&mut state, &Message::DismissAuthDialog);
        assert!(result.is_some(), "DismissAuthDialog is always consumed");
    }

    // ── AuthSetupPassphraseChanged ────────────────────────────────────────────

    #[test]
    fn setup_passphrase_changed_updates_input() {
        let mut state = make_state();
        state.active_dialog = Some(setup_dialog(Uuid::new_v4()));
        handle_message(
            &mut state,
            &Message::AuthSetupPassphraseChanged("abc".to_owned()),
        );
        let Some(AuthDialog::SetupPassphrase {
            passphrase_input, ..
        }) = &state.active_dialog
        else {
            panic!("dialog should still be set");
        };
        assert_eq!(passphrase_input, "abc");
    }

    #[test]
    fn setup_passphrase_changed_clears_error() {
        let mut state = make_state();
        state.active_dialog = Some(AuthDialog::SetupPassphrase {
            pending_profile_id: Uuid::new_v4(),
            pending_password: String::new(),
            passphrase_input: "old".to_owned(),
            confirm_input: String::new(),
            error: Some("previous error".to_owned()),
            is_processing: false,
        });
        handle_message(
            &mut state,
            &Message::AuthSetupPassphraseChanged("new".to_owned()),
        );
        let Some(AuthDialog::SetupPassphrase { error, .. }) = &state.active_dialog else {
            panic!();
        };
        assert!(error.is_none(), "error should be cleared on input change");
    }

    // ── AuthSetupConfirmChanged ───────────────────────────────────────────────

    #[test]
    fn confirm_changed_updates_input() {
        let mut state = make_state();
        state.active_dialog = Some(setup_dialog(Uuid::new_v4()));
        handle_message(
            &mut state,
            &Message::AuthSetupConfirmChanged("xyz".to_owned()),
        );
        let Some(AuthDialog::SetupPassphrase { confirm_input, .. }) = &state.active_dialog else {
            panic!();
        };
        assert_eq!(confirm_input, "xyz");
    }

    #[test]
    fn confirm_changed_clears_error() {
        let mut state = make_state();
        state.active_dialog = Some(AuthDialog::SetupPassphrase {
            pending_profile_id: Uuid::new_v4(),
            pending_password: String::new(),
            passphrase_input: String::new(),
            confirm_input: String::new(),
            error: Some("mismatch".to_owned()),
            is_processing: false,
        });
        handle_message(
            &mut state,
            &Message::AuthSetupConfirmChanged("x".to_owned()),
        );
        let Some(AuthDialog::SetupPassphrase { error, .. }) = &state.active_dialog else {
            panic!();
        };
        assert!(error.is_none());
    }

    // ── AuthUnlockPassphraseChanged ───────────────────────────────────────────

    #[test]
    fn unlock_passphrase_changed_updates_input() {
        let mut state = make_state();
        state.active_dialog = Some(unlock_dialog());
        handle_message(
            &mut state,
            &Message::AuthUnlockPassphraseChanged("secret".to_owned()),
        );
        let Some(AuthDialog::Unlock {
            passphrase_input, ..
        }) = &state.active_dialog
        else {
            panic!();
        };
        assert_eq!(passphrase_input, "secret");
    }

    #[test]
    fn unlock_passphrase_changed_clears_error() {
        let mut state = make_state();
        state.active_dialog = Some(AuthDialog::Unlock {
            pending_action: PendingAction::ConnectToProfile(Uuid::new_v4()),
            passphrase_input: String::new(),
            error: Some("Incorrect passphrase".to_owned()),
            is_processing: false,
        });
        handle_message(
            &mut state,
            &Message::AuthUnlockPassphraseChanged("x".to_owned()),
        );
        let Some(AuthDialog::Unlock { error, .. }) = &state.active_dialog else {
            panic!();
        };
        assert!(error.is_none());
    }

    // ── SubmitSetupPassphrase ─────────────────────────────────────────────────

    #[test]
    fn submit_setup_passphrase_mismatch_sets_error() {
        let mut state = make_state();
        state.active_dialog = Some(AuthDialog::SetupPassphrase {
            pending_profile_id: Uuid::new_v4(),
            pending_password: "pw".to_owned(),
            passphrase_input: "aaa".to_owned(),
            confirm_input: "bbb".to_owned(),
            error: None,
            is_processing: false,
        });
        handle_message(&mut state, &Message::SubmitSetupPassphrase);
        let Some(AuthDialog::SetupPassphrase { error, .. }) = &state.active_dialog else {
            panic!();
        };
        assert_eq!(error.as_deref(), Some("Passphrases do not match"));
    }

    #[test]
    fn submit_setup_passphrase_empty_sets_error() {
        let mut state = make_state();
        state.active_dialog = Some(AuthDialog::SetupPassphrase {
            pending_profile_id: Uuid::new_v4(),
            pending_password: "pw".to_owned(),
            passphrase_input: String::new(),
            confirm_input: String::new(),
            error: None,
            is_processing: false,
        });
        handle_message(&mut state, &Message::SubmitSetupPassphrase);
        let Some(AuthDialog::SetupPassphrase { error, .. }) = &state.active_dialog else {
            panic!();
        };
        assert_eq!(error.as_deref(), Some("Passphrase cannot be empty"));
    }

    #[test]
    fn submit_setup_passphrase_valid_returns_task() {
        let mut state = make_state();
        state.active_dialog = Some(AuthDialog::SetupPassphrase {
            pending_profile_id: Uuid::new_v4(),
            pending_password: "hunter2".to_owned(),
            passphrase_input: "correct".to_owned(),
            confirm_input: "correct".to_owned(),
            error: None,
            is_processing: false,
        });
        let result = handle_message(&mut state, &Message::SubmitSetupPassphrase);
        assert!(result.is_some(), "should return an async task");
        // Dialog not yet cleared — cleared only on SetupPassphraseComplete.
        assert!(state.active_dialog.is_some());
    }

    // ── SubmitUnlockPassphrase ────────────────────────────────────────────────

    #[test]
    fn submit_unlock_empty_passphrase_sets_error() {
        let mut state = make_state();
        state.active_dialog = Some(unlock_dialog());
        handle_message(&mut state, &Message::SubmitUnlockPassphrase);
        let Some(AuthDialog::Unlock { error, .. }) = &state.active_dialog else {
            panic!();
        };
        assert_eq!(error.as_deref(), Some("Passphrase cannot be empty"));
    }

    #[test]
    fn submit_unlock_non_empty_passphrase_returns_task() {
        let mut state = make_state();
        state.profiles.master_passphrase_hash = Some("$argon2id$fakehash".to_owned());
        state.active_dialog = Some(AuthDialog::Unlock {
            pending_action: PendingAction::ConnectToProfile(Uuid::new_v4()),
            passphrase_input: "somepassphrase".to_owned(),
            error: None,
            is_processing: false,
        });
        let result = handle_message(&mut state, &Message::SubmitUnlockPassphrase);
        assert!(result.is_some(), "should dispatch async verify task");
    }

    // ── UnlockPassphraseResult ────────────────────────────────────────────────

    #[test]
    fn unlock_result_invalid_sets_error() {
        let mut state = make_state();
        state.active_dialog = Some(unlock_dialog());
        handle_message(
            &mut state,
            &Message::UnlockPassphraseResult {
                passphrase: "wrong".to_owned(),
                valid: false,
            },
        );
        let Some(AuthDialog::Unlock { error, .. }) = &state.active_dialog else {
            panic!();
        };
        assert_eq!(error.as_deref(), Some("Incorrect passphrase"));
    }

    #[test]
    fn unlock_result_valid_clears_dialog_and_sets_passphrase() {
        let mut state = make_state();
        state.active_dialog = Some(unlock_dialog());
        handle_message(
            &mut state,
            &Message::UnlockPassphraseResult {
                passphrase: "correct".to_owned(),
                valid: true,
            },
        );
        assert!(state.active_dialog.is_none(), "dialog should be cleared");
        assert_eq!(
            state
                .unlocked_passphrase
                .as_ref()
                .map(|s| s.expose_secret().as_str()),
            Some("correct")
        );
    }

    // ── SetupPassphraseComplete ───────────────────────────────────────────────

    #[test]
    fn setup_passphrase_complete_updates_state() {
        let mut state = make_state();
        let profile_id = Uuid::new_v4();
        state.active_dialog = Some(setup_dialog(profile_id));
        // Add a profile so encrypted_password can be set.
        state
            .profiles
            .profiles
            .push(crate::profile::ConnectionProfile {
                id: profile_id,
                name: "Test".to_owned(),
                host: "localhost".to_owned(),
                port: 9091,
                username: None,
                encrypted_password: None,
                speed_limit_down: 0,
                speed_limit_down_enabled: false,
                speed_limit_up: 0,
                speed_limit_up_enabled: false,
                alt_speed_down: 0,
                alt_speed_up: 0,
                ratio_limit: 0.0,
                ratio_limit_enabled: false,
            });

        let result = handle_message(
            &mut state,
            &Message::SetupPassphraseComplete {
                passphrase: "my_passphrase".to_owned(),
                hash: "$argon2id$fakehash".to_owned(),
                profile_id,
                encrypted_password: "salt$nonce$ciphertext".to_owned(),
            },
        );
        assert!(result.is_some());
        assert!(state.active_dialog.is_none(), "dialog cleared");
        assert_eq!(
            state
                .unlocked_passphrase
                .as_ref()
                .map(|s| s.expose_secret().as_str()),
            Some("my_passphrase")
        );
        assert_eq!(
            state.profiles.master_passphrase_hash.as_deref(),
            Some("$argon2id$fakehash")
        );
        let profile = state
            .profiles
            .profiles
            .iter()
            .find(|p| p.id == profile_id)
            .unwrap();
        assert_eq!(
            profile.encrypted_password.as_deref(),
            Some("salt$nonce$ciphertext")
        );
    }

    // ── EncryptPasswordReady ──────────────────────────────────────────────────

    #[test]
    fn encrypt_password_ready_sets_encrypted_password_on_profile() {
        let mut state = make_state();
        let profile_id = Uuid::new_v4();
        state
            .profiles
            .profiles
            .push(crate::profile::ConnectionProfile {
                id: profile_id,
                name: "Test".to_owned(),
                host: "localhost".to_owned(),
                port: 9091,
                username: None,
                encrypted_password: None,
                speed_limit_down: 0,
                speed_limit_down_enabled: false,
                speed_limit_up: 0,
                speed_limit_up_enabled: false,
                alt_speed_down: 0,
                alt_speed_up: 0,
                ratio_limit: 0.0,
                ratio_limit_enabled: false,
            });

        handle_message(
            &mut state,
            &Message::EncryptPasswordReady {
                profile_id,
                encrypted_password: "salt$nonce$ct".to_owned(),
            },
        );

        let profile = state
            .profiles
            .profiles
            .iter()
            .find(|p| p.id == profile_id)
            .unwrap();
        assert_eq!(profile.encrypted_password.as_deref(), Some("salt$nonce$ct"));
    }

    #[test]
    fn encrypt_password_ready_unknown_profile_is_noop() {
        let mut state = make_state();
        // No profiles — should not panic.
        let result = handle_message(
            &mut state,
            &Message::EncryptPasswordReady {
                profile_id: Uuid::new_v4(),
                encrypted_password: "x$y$z".to_owned(),
            },
        );
        assert!(result.is_some(), "message is consumed");
    }

    // ── Non-auth messages ─────────────────────────────────────────────────────

    #[test]
    fn non_auth_message_returns_none() {
        let mut state = make_state();
        let result = handle_message(&mut state, &Message::Noop);
        assert!(result.is_none(), "non-auth messages must not be consumed");
    }

    // ── AuthTabKeyPressed ─────────────────────────────────────────────────────

    /// Tab with no active dialog is not consumed (returns None).
    #[test]
    fn auth_tab_not_consumed_when_no_dialog() {
        let mut state = make_state();
        let result = handle_message(&mut state, &Message::AuthTabKeyPressed { shift: false });
        assert!(result.is_none(), "no dialog → message must not be consumed");
    }

    /// Tab with Unlock dialog is consumed but is a no-op (single input field).
    #[test]
    fn auth_tab_noop_in_unlock_dialog() {
        let mut state = make_state();
        state.active_dialog = Some(unlock_dialog());
        let result = handle_message(&mut state, &Message::AuthTabKeyPressed { shift: false });
        assert!(result.is_some(), "Unlock → must be consumed");
        // Dialog itself must be untouched.
        assert!(
            matches!(state.active_dialog, Some(AuthDialog::Unlock { .. })),
            "dialog must not be cleared"
        );
    }

    /// Shift-Tab with Unlock dialog is also consumed as a no-op.
    #[test]
    fn auth_shift_tab_noop_in_unlock_dialog() {
        let mut state = make_state();
        state.active_dialog = Some(unlock_dialog());
        let result = handle_message(&mut state, &Message::AuthTabKeyPressed { shift: true });
        assert!(result.is_some());
    }

    /// Tab with Setup dialog is consumed (two-input ring — returns a focus task).
    #[test]
    fn auth_tab_active_in_setup_dialog() {
        let mut state = make_state();
        state.active_dialog = Some(setup_dialog(Uuid::new_v4()));
        let result = handle_message(&mut state, &Message::AuthTabKeyPressed { shift: false });
        assert!(result.is_some(), "Setup → must be consumed");
        // Dialog must still be open.
        assert!(
            matches!(
                state.active_dialog,
                Some(AuthDialog::SetupPassphrase { .. })
            ),
            "dialog must remain open"
        );
    }

    /// Shift-Tab with Setup dialog is also consumed.
    #[test]
    fn auth_shift_tab_active_in_setup_dialog() {
        let mut state = make_state();
        state.active_dialog = Some(setup_dialog(Uuid::new_v4()));
        let result = handle_message(&mut state, &Message::AuthTabKeyPressed { shift: true });
        assert!(result.is_some());
    }

    // ── AuthEnterPressed ──────────────────────────────────────────────────────

    /// Enter with no dialog is not consumed.
    #[test]
    fn auth_enter_not_consumed_when_no_dialog() {
        let mut state = make_state();
        let result = handle_message(&mut state, &Message::AuthEnterPressed);
        assert!(result.is_none());
    }

    /// Enter in Unlock dialog while not processing dispatches the submit task.
    #[test]
    fn auth_enter_dispatches_unlock_submit_when_ready() {
        let mut state = make_state();
        state.active_dialog = Some(unlock_dialog());
        let result = handle_message(&mut state, &Message::AuthEnterPressed);
        assert!(result.is_some(), "must be consumed");
        // Not yet processing — the submit task is returned, no state mutation yet.
        assert!(
            matches!(
                state.active_dialog,
                Some(AuthDialog::Unlock {
                    is_processing: false,
                    ..
                })
            ),
            "is_processing must not flip before task executes"
        );
    }

    /// Enter in Unlock dialog while processing is ignored (prevents double-submit).
    #[test]
    fn auth_enter_noop_in_unlock_while_processing() {
        let mut state = make_state();
        state.active_dialog = Some(AuthDialog::Unlock {
            pending_action: PendingAction::ConnectToProfile(Uuid::new_v4()),
            passphrase_input: "x".to_owned(),
            error: None,
            is_processing: true,
        });
        let result = handle_message(&mut state, &Message::AuthEnterPressed);
        assert!(result.is_some(), "still consumed");
        assert!(
            matches!(
                state.active_dialog,
                Some(AuthDialog::Unlock {
                    is_processing: true,
                    ..
                })
            ),
            "is_processing must not be reset"
        );
    }

    /// Enter in Setup dialog while not processing dispatches the submit task.
    #[test]
    fn auth_enter_dispatches_setup_submit_when_ready() {
        let mut state = make_state();
        state.active_dialog = Some(setup_dialog(Uuid::new_v4()));
        let result = handle_message(&mut state, &Message::AuthEnterPressed);
        assert!(result.is_some());
        assert!(
            matches!(
                state.active_dialog,
                Some(AuthDialog::SetupPassphrase { .. })
            ),
            "dialog must remain open until async task completes"
        );
    }

    /// Enter in Setup dialog while processing is ignored.
    #[test]
    fn auth_enter_noop_in_setup_while_processing() {
        let mut state = make_state();
        state.active_dialog = Some(AuthDialog::SetupPassphrase {
            pending_profile_id: Uuid::new_v4(),
            pending_password: "pw".to_owned(),
            passphrase_input: "good".to_owned(),
            confirm_input: "good".to_owned(),
            error: None,
            is_processing: true,
        });
        let result = handle_message(&mut state, &Message::AuthEnterPressed);
        assert!(result.is_some());
        assert!(
            matches!(
                state.active_dialog,
                Some(AuthDialog::SetupPassphrase {
                    is_processing: true,
                    ..
                })
            ),
            "is_processing must not be reset"
        );
    }
}
