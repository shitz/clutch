//! View logic for the settings screen.

use iced::widget::{
    Space, button, column, container, opaque, row, scrollable, stack, text, text_input,
};
use iced::{Alignment, Color, Element, Length};

use crate::profile::ThemeConfig;

use super::draft::TestResult;
use super::state::SettingsScreen;
use super::{Message, SettingsTab};

impl SettingsScreen {
    pub fn view(&self) -> Element<'_, Message> {
        let main_content = column![
            self.view_header(),
            self.view_tab_bar(),
            match self.active_tab {
                SettingsTab::General => self.view_general_tab(),
                SettingsTab::Connections => self.view_connections_tab(),
            },
        ]
        .spacing(0);

        // Overlay layers.
        if let Some(id) = self.confirm_delete_id {
            let name = self
                .profiles
                .iter()
                .find(|p| p.id == id)
                .map(|p| p.name.as_str())
                .unwrap_or("this profile");
            let title = format!("Delete \"{}\"?", name);
            let dialog = self.view_overlay_dialog(
                title,
                "This cannot be undone. The saved password will also be removed from the system keyring.",
                vec![
                    ("Cancel", Message::DeleteCancelled, false),
                    ("Delete", Message::DeleteConfirmed, true),
                ],
            );
            return stack![main_content, opaque(dialog)].into();
        }

        if self.confirm_discard.is_some() {
            let dialog = self.view_overlay_dialog(
                "You have unsaved changes".to_owned(),
                "Do you want to save your changes or discard them?",
                vec![
                    ("Cancel", Message::GuardCancel, false),
                    ("Discard", Message::GuardDiscard, false),
                    ("Save", Message::GuardSave, true),
                ],
            );
            return stack![main_content, opaque(dialog)].into();
        }

        main_content.into()
    }

    fn view_header(&self) -> Element<'_, Message> {
        row![
            button(crate::theme::icon(crate::theme::ICON_CLOSE))
                .on_press(Message::CloseClicked)
                .style(iced::widget::button::text),
            text("Settings").size(20),
            Space::new().width(Length::Fill),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .padding([8, 16])
        .into()
    }

    fn view_tab_bar(&self) -> Element<'_, Message> {
        let tabs: &[(SettingsTab, &str)] = &[
            (SettingsTab::General, "General"),
            (SettingsTab::Connections, "Connections"),
        ];
        let buttons: Vec<Element<'_, Message>> = tabs
            .iter()
            .map(|(tab, label)| {
                let is_active = self.active_tab == *tab;
                let btn = button(text(*label).size(15).width(Length::Fill))
                    .on_press(Message::TabClicked(*tab))
                    .style(if is_active {
                        crate::theme::tab_active
                    } else {
                        crate::theme::tab_inactive
                    })
                    .width(Length::Fill)
                    .padding([6, 16]);
                let col: Element<'_, Message> = if is_active {
                    column![
                        btn,
                        container(Space::new().width(Length::Fill).height(2.0))
                            .style(crate::theme::tab_underline)
                            .width(Length::Fill),
                    ]
                    .width(Length::Fill)
                    .into()
                } else {
                    column![btn, Space::new().width(Length::Fill).height(2.0)]
                        .width(Length::Fill)
                        .into()
                };
                container(col).width(Length::FillPortion(1)).into()
            })
            .collect();
        row(buttons).spacing(0).into()
    }

    // ── General tab ──────────────────────────────────────────────────────────

    fn view_general_tab(&self) -> Element<'_, Message> {
        let theme_row = row![
            text("Theme").width(160),
            button("Light")
                .on_press(Message::ThemeConfigChanged(ThemeConfig::Light))
                .style(if self.theme_draft == ThemeConfig::Light {
                    iced::widget::button::primary
                } else {
                    iced::widget::button::secondary
                }),
            button("Dark")
                .on_press(Message::ThemeConfigChanged(ThemeConfig::Dark))
                .style(if self.theme_draft == ThemeConfig::Dark {
                    iced::widget::button::primary
                } else {
                    iced::widget::button::secondary
                }),
            button("System")
                .on_press(Message::ThemeConfigChanged(ThemeConfig::System))
                .style(if self.theme_draft == ThemeConfig::System {
                    iced::widget::button::primary
                } else {
                    iced::widget::button::secondary
                }),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let interval_row = row![
            text("Refresh interval (s)").width(160),
            text_input("1", &self.refresh_interval_draft)
                .on_input(Message::RefreshIntervalChanged)
                .width(80),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let form = column![theme_row, interval_row].spacing(16).padding(24);

        let save_enabled = self.general_validation_error.is_none();
        let save_btn = {
            let b = button(crate::theme::icon(crate::theme::ICON_SAVE))
                .style(iced::widget::button::primary)
                .padding([4, 8]);
            if save_enabled {
                b.on_press(Message::GeneralSaveClicked)
            } else {
                b
            }
        };
        let revert_btn = {
            let b = button(crate::theme::icon(crate::theme::ICON_UNDO))
                .style(iced::widget::button::secondary)
                .padding([4, 8]);
            if self.general_dirty {
                b.on_press(Message::GeneralRevertClicked)
            } else {
                b
            }
        };

        let mut status_col = column![].spacing(8);
        if let Some(err) = &self.general_validation_error {
            status_col = status_col.push(text(err.as_str()).style(|t: &iced::Theme| {
                iced::widget::text::Style {
                    color: Some(t.extended_palette().danger.base.color),
                }
            }));
        }
        if self.general_saved {
            status_col =
                status_col.push(text("\u{2713} Settings saved").style(|t: &iced::Theme| {
                    iced::widget::text::Style {
                        color: Some(t.extended_palette().success.base.color),
                    }
                }));
        }

        let action_row = row![revert_btn, save_btn]
            .spacing(4)
            .padding(iced::Padding {
                top: 8.0,
                right: 8.0,
                bottom: 8.0,
                left: 24.0,
            });

        column![
            form,
            Space::new().height(Length::Fill),
            status_col.padding(iced::Padding {
                top: 0.0,
                right: 24.0,
                bottom: 8.0,
                left: 24.0,
            }),
            action_row,
        ]
        .height(Length::Fill)
        .into()
    }

    // ── Connections tab ───────────────────────────────────────────────────────

    fn view_connections_tab(&self) -> Element<'_, Message> {
        let left = self.view_profile_list();
        let right = self.view_profile_detail();

        row![
            container(left).width(220).height(Length::Fill),
            container(right).width(Length::Fill).height(Length::Fill),
        ]
        .spacing(0)
        .height(Length::Fill)
        .into()
    }

    fn view_profile_list(&self) -> Element<'_, Message> {
        let items: Vec<Element<'_, Message>> = self
            .profiles
            .iter()
            .map(|p| {
                let is_selected = self.selected_profile_id == Some(p.id);
                let is_active = self.active_profile_id == Some(p.id);
                let label = if is_active {
                    format!("● {}", p.name)
                } else {
                    p.name.clone()
                };
                button(text(label).width(Length::Fill))
                    .on_press(Message::ProfileListClicked(p.id))
                    .style(if is_selected {
                        iced::widget::button::primary
                    } else {
                        iced::widget::button::text
                    })
                    .width(Length::Fill)
                    .padding([6, 12])
                    .into()
            })
            .collect();

        let list = scrollable(column(items).spacing(2)).height(Length::Fill);

        let delete_enabled = self.selected_profile_id.is_some()
            && self.selected_profile_id != self.active_profile_id;

        let del_btn = {
            let b = button(crate::theme::icon(crate::theme::ICON_TRASH))
                .style(iced::widget::button::secondary)
                .padding([4, 8]);
            if delete_enabled {
                b.on_press(Message::DeleteProfileClicked)
            } else {
                b
            }
        };

        column![
            list,
            row![
                button(crate::theme::icon(crate::theme::ICON_ADD))
                    .on_press(Message::AddProfileClicked)
                    .style(iced::widget::button::secondary)
                    .padding([4, 8]),
                del_btn,
            ]
            .spacing(4)
            .padding([8, 8]),
        ]
        .into()
    }

    fn view_profile_detail(&self) -> Element<'_, Message> {
        let Some(draft) = &self.draft else {
            return container(
                text(if self.profiles.is_empty() {
                    "No connections. Click '+' to add a new Transmission daemon."
                } else {
                    "Select a connection profile or create a new one."
                })
                .size(14),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .padding(24)
            .into();
        };

        let can_save = self.dirty && self.draft_is_saveable();
        let save_btn = {
            let b = button(crate::theme::icon(crate::theme::ICON_SAVE))
                .style(iced::widget::button::primary)
                .padding([4, 8]);
            if can_save {
                b.on_press(Message::SaveClicked)
            } else {
                b
            }
        };
        let revert_btn = {
            let b = button(crate::theme::icon(crate::theme::ICON_UNDO))
                .style(iced::widget::button::secondary)
                .padding([4, 8]);
            if self.dirty {
                b.on_press(Message::RevertClicked)
            } else {
                b
            }
        };

        let test_btn = {
            let b = button("Test Connection").style(iced::widget::button::secondary);
            if draft.testing {
                b
            } else {
                b.on_press(Message::TestConnectionClicked)
            }
        };

        let test_status: Element<'_, Message> = if draft.testing {
            text("Testing connection\u{2026}").into()
        } else {
            match &draft.test_result {
                Some(TestResult::Success) => text("\u{2713} Connection test successful!")
                    .style(|t: &iced::Theme| iced::widget::text::Style {
                        color: Some(t.extended_palette().success.base.color),
                    })
                    .into(),
                Some(TestResult::Failure(e)) => {
                    text(format!("\u{2717} Connection test failed: {e}"))
                        .style(|t: &iced::Theme| iced::widget::text::Style {
                            color: Some(t.extended_palette().danger.base.color),
                        })
                        .into()
                }
                None => Space::new().into(),
            }
        };

        let form = column![
            row![
                text("Profile Name").width(120),
                text_input("Name", &draft.name).on_input(Message::DraftNameChanged)
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![
                text("Host").width(120),
                text_input("localhost", &draft.host).on_input(Message::DraftHostChanged)
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![
                text("Port").width(120),
                text_input("9091", &draft.port)
                    .on_input(Message::DraftPortChanged)
                    .width(100)
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![
                text("Username").width(120),
                text_input("optional", &draft.username).on_input(Message::DraftUsernameChanged)
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![
                text("Password").width(120),
                text_input("optional", &draft.password)
                    .on_input(Message::DraftPasswordChanged)
                    .secure(true)
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![test_btn, test_status]
                .spacing(8)
                .align_y(Alignment::Center),
        ]
        .spacing(12)
        .padding(24);

        let action_row = row![revert_btn, save_btn]
            .spacing(4)
            .padding(iced::Padding {
                top: 8.0,
                right: 8.0,
                bottom: 8.0,
                left: 24.0,
            });

        column![form, Space::new().height(Length::Fill), action_row]
            .height(Length::Fill)
            .into()
    }

    // ── Overlay dialog ───────────────────────────────────────────────────────

    fn view_overlay_dialog<'a>(
        &'a self,
        title: String,
        body: &'a str,
        actions: Vec<(&'a str, Message, bool)>,
    ) -> Element<'a, Message> {
        let buttons: Vec<Element<'_, Message>> = actions
            .into_iter()
            .map(|(label, msg, primary)| {
                button(label)
                    .on_press(msg)
                    .style(if primary {
                        iced::widget::button::danger
                    } else {
                        iced::widget::button::secondary
                    })
                    .into()
            })
            .collect();

        let card = container(
            column![
                text(title).size(18),
                text(body).size(13),
                row(buttons).spacing(8),
            ]
            .spacing(16),
        )
        .padding(28)
        .style(|t: &iced::Theme| {
            let p = t.extended_palette();
            container::Style {
                background: Some(iced::Background::Color(p.background.base.color)),
                border: iced::Border {
                    radius: 12.0.into(),
                    width: 1.0,
                    color: p.background.strong.color,
                },
                shadow: iced::Shadow {
                    color: Color::from_rgba8(0, 0, 0, 0.35),
                    offset: iced::Vector::new(0.0, 4.0),
                    blur_radius: 16.0,
                },
                ..Default::default()
            }
        });

        container(card)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_: &iced::Theme| container::Style {
                background: Some(iced::Background::Color(Color::from_rgba8(0, 0, 0, 0.70))),
                ..Default::default()
            })
            .into()
    }
}
