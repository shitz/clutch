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

//! Modal overlay rendering for the auth dialog.

use iced::widget::{Space, button, column, container, row, stack, text, text_input};
use iced::{Element, Length};

use crate::app::Message;

use super::{AuthDialog, setup_confirm_id, setup_passphrase_id, unlock_input_id};

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
    .style(crate::theme::auth_dialog_card);

    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(crate::theme::dialog_scrim(0.5))
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
    .style(crate::theme::auth_dialog_card);

    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(crate::theme::dialog_scrim(0.5))
        .into()
}
