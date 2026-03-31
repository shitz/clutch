//! View logic for the settings screen.

use iced::widget::{
    Space, button, column, container, opaque, row, scrollable, stack, text, text_input, tooltip,
};
use iced::{Alignment, ContentFit, Element, Length};

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
                SettingsTab::About => self.view_about_tab(),
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
                "This cannot be undone. The saved password will also be removed.",
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
            tooltip(
                crate::theme::icon_button(crate::theme::icon(crate::theme::ICON_CLOSE))
                    .on_press(Message::CloseClicked),
                text("Close"),
                tooltip::Position::Right,
            )
            .gap(6)
            .style(crate::theme::m3_tooltip),
            text("Settings").size(20),
            Space::new().width(Length::Fill),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .padding([8, 16])
        .into()
    }

    fn view_tab_bar(&self) -> Element<'_, Message> {
        // Center the segmented control and give each segment equal width.
        let ctrl = crate::theme::segmented_control(
            &[
                ("General", SettingsTab::General),
                ("Connections", SettingsTab::Connections),
                ("About", SettingsTab::About),
            ],
            self.active_tab,
            Message::TabClicked,
            true,
            false,
        );
        container(container(ctrl).width(Length::Fixed(400.0)))
            .width(Length::Fill)
            .padding([4, 16])
            .center_x(Length::Fill)
            .into()
    }

    // ── General tab ──────────────────────────────────────────────────────────

    fn view_general_tab(&self) -> Element<'_, Message> {
        let theme_control = container(crate::theme::segmented_control(
            &[
                ("Light", ThemeConfig::Light),
                ("Dark", ThemeConfig::Dark),
                ("System", ThemeConfig::System),
            ],
            self.theme_draft,
            Message::ThemeConfigChanged,
            true,
            false,
        ))
        .width(Length::Fixed(320.0));

        let theme_row = row![text("Theme").width(160), theme_control,]
            .spacing(8)
            .align_y(Alignment::Center);

        let interval_row = row![
            text("Refresh interval (s)").width(160),
            text_input("1", &self.refresh_interval_draft)
                .on_input(Message::RefreshIntervalChanged)
                .width(80)
                .padding([12, 16])
                .style(crate::theme::m3_text_input),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let appearance_card = container(
            column![
                text("Appearance").size(13).style(|t: &iced::Theme| {
                    iced::widget::text::Style {
                        color: Some(t.palette().text.scale_alpha(0.6)),
                    }
                }),
                theme_row,
            ]
            .spacing(12),
        )
        .style(crate::theme::m3_card)
        .padding(16)
        .width(Length::Fill);

        let behaviour_card = container(
            column![
                text("Behaviour").size(13).style(|t: &iced::Theme| {
                    iced::widget::text::Style {
                        color: Some(t.palette().text.scale_alpha(0.6)),
                    }
                }),
                interval_row,
            ]
            .spacing(12),
        )
        .style(crate::theme::m3_card)
        .padding(16)
        .width(Length::Fill);

        let form = column![appearance_card, behaviour_card]
            .spacing(16)
            .padding([24, 16]);

        let save_enabled = self.general_validation_error.is_none();
        let save_btn = {
            let b = button(
                row![
                    crate::theme::icon(crate::theme::ICON_SAVE),
                    text("Save").size(14),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([10, 20])
            .style(crate::theme::m3_primary_button);
            if save_enabled {
                b.on_press(Message::GeneralSaveClicked)
            } else {
                b
            }
        };
        let revert_btn = {
            let b = button(
                row![
                    crate::theme::icon(crate::theme::ICON_UNDO),
                    text("Undo").size(14),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([10, 20])
            .style(crate::theme::m3_tonal_button);
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

        let action_row = row![Space::new().width(Length::Fill), revert_btn, save_btn,]
            .spacing(8)
            .padding(iced::Padding {
                top: 8.0,
                right: 16.0,
                bottom: 8.0,
                left: 16.0,
            });

        column![
            form,
            Space::new().height(Length::Fill),
            status_col.padding(iced::Padding {
                top: 0.0,
                right: 24.0,
                bottom: 4.0,
                left: 24.0,
            }),
            action_row,
        ]
        .height(Length::Fill)
        .into()
    }

    // ── Connections tab ───────────────────────────────────────────────────────

    fn view_connections_tab(&self) -> Element<'_, Message> {
        let left = container(self.view_profile_list())
            .style(crate::theme::m3_card)
            .padding(8)
            .width(220)
            .height(Length::Fill);
        let right = container(self.view_profile_detail())
            .style(crate::theme::m3_card)
            .padding(8)
            .width(Length::Fill)
            .height(Length::Fill);

        row![left, right]
            .spacing(8)
            .height(Length::Fill)
            .padding(16)
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
                    .style(move |t: &iced::Theme, _s| {
                        let p = t.extended_palette();
                        if is_selected {
                            button::Style {
                                background: Some(iced::Background::Color(p.primary.base.color)),
                                text_color: p.primary.base.text,
                                border: iced::Border {
                                    color: iced::Color::TRANSPARENT,
                                    width: 0.0,
                                    radius: 100.0.into(),
                                },
                                shadow: iced::Shadow::default(),
                                snap: false,
                            }
                        } else {
                            button::Style {
                                background: None,
                                text_color: p.background.base.text,
                                border: iced::Border {
                                    color: iced::Color::TRANSPARENT,
                                    width: 0.0,
                                    radius: 100.0.into(),
                                },
                                shadow: iced::Shadow::default(),
                                snap: false,
                            }
                        }
                    })
                    .width(Length::Fill)
                    .padding([8, 12])
                    .into()
            })
            .collect();

        let list = scrollable(column(items).spacing(2).padding([4, 6])).height(Length::Fill);

        let delete_enabled = self.selected_profile_id.is_some()
            && self.selected_profile_id != self.active_profile_id;

        column![
            list,
            row![
                tooltip(
                    crate::theme::icon_button(crate::theme::icon(crate::theme::ICON_ADD))
                        .on_press(Message::AddProfileClicked),
                    text("Add profile"),
                    tooltip::Position::Top,
                )
                .gap(6)
                .style(crate::theme::m3_tooltip),
                {
                    let b = crate::theme::icon_button(crate::theme::icon(crate::theme::ICON_TRASH));
                    let b = if delete_enabled {
                        b.on_press(Message::DeleteProfileClicked)
                    } else {
                        b
                    };
                    tooltip(b, text("Delete profile"), tooltip::Position::Top)
                        .gap(6)
                        .style(crate::theme::m3_tooltip)
                },
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
            let b = button(
                row![
                    crate::theme::icon(crate::theme::ICON_SAVE),
                    text("Save").size(14),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([10, 20])
            .style(crate::theme::m3_primary_button);
            if can_save {
                b.on_press(Message::SaveClicked)
            } else {
                b
            }
        };
        let revert_btn = {
            let b = button(
                row![
                    crate::theme::icon(crate::theme::ICON_UNDO),
                    text("Undo").size(14),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([10, 20])
            .style(crate::theme::m3_tonal_button);
            if self.dirty {
                b.on_press(Message::RevertClicked)
            } else {
                b
            }
        };

        let test_btn = {
            let b = button(
                row![
                    crate::theme::icon(crate::theme::ICON_LINK),
                    text("Test Connection").size(14),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([10, 20])
            .style(crate::theme::m3_tonal_button);
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
                text_input("Name", &draft.name)
                    .on_input(Message::DraftNameChanged)
                    .padding([12, 16])
                    .style(crate::theme::m3_text_input)
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![
                text("Host").width(120),
                text_input("localhost", &draft.host)
                    .on_input(Message::DraftHostChanged)
                    .padding([12, 16])
                    .style(crate::theme::m3_text_input)
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![
                text("Port").width(120),
                text_input("9091", &draft.port)
                    .on_input(Message::DraftPortChanged)
                    .width(100)
                    .padding([12, 16])
                    .style(crate::theme::m3_text_input)
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![
                text("Username").width(120),
                text_input("optional", &draft.username)
                    .on_input(Message::DraftUsernameChanged)
                    .padding([12, 16])
                    .style(crate::theme::m3_text_input)
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![
                text("Password").width(120),
                text_input(
                    if draft.has_saved_password && !draft.password_changed {
                        "••••••••"
                    } else {
                        "optional"
                    },
                    &draft.password
                )
                .on_input(Message::DraftPasswordChanged)
                .secure(true)
                .padding([12, 16])
                .style(crate::theme::m3_text_input)
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![Space::new().width(Length::Fill), test_status, test_btn]
                .spacing(12)
                .align_y(Alignment::Center),
        ]
        .spacing(12)
        .padding(24);

        let action_row = row![Space::new().width(Length::Fill), revert_btn, save_btn,]
            .spacing(8)
            .padding(iced::Padding {
                top: 8.0,
                right: 16.0,
                bottom: 8.0,
                left: 16.0,
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
                    .padding([10, 24])
                    .style(if primary {
                        |t: &iced::Theme, s| {
                            let p = t.extended_palette();
                            let (bg, hover) = (p.danger.base.color, p.danger.strong.color);
                            let bg = match s {
                                button::Status::Hovered | button::Status::Pressed => hover,
                                _ => bg,
                            };
                            button::Style {
                                background: Some(iced::Background::Color(bg)),
                                text_color: p.danger.base.text,
                                border: iced::Border {
                                    radius: 100.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }
                        }
                    } else {
                        crate::theme::m3_tonal_button
                    })
                    .into()
            })
            .collect();

        let card = container(
            column![
                text(title).size(18),
                text(body).size(13),
                row![Space::new().width(Length::Fill),]
                    .extend(buttons)
                    .spacing(8)
                    .width(Length::Fill),
            ]
            .spacing(16),
        )
        .padding(28)
        .max_width(360.0)
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
                    color: iced::Color::from_rgba8(0, 0, 0, 0.35),
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
                background: Some(iced::Background::Color(iced::Color::from_rgba8(
                    0, 0, 0, 0.70,
                ))),
                ..Default::default()
            })
            .into()
    }

    fn view_about_tab(&self) -> Element<'_, Message> {
        container(
            column![
                iced::widget::image(iced::widget::image::Handle::from_bytes(
                    include_bytes!("../../../assets/Clutch_Logo.png").as_ref(),
                ))
                .width(Length::Fixed(240.0))
                .content_fit(ContentFit::ScaleDown),
                text(format!("Version {}", env!("CARGO_PKG_VERSION")))
                    .size(15)
                    .style(|t: &iced::Theme| iced::widget::text::Style {
                        color: Some(t.palette().text.scale_alpha(0.7)),
                    }),
                text("© 2026 The Clutch Authors")
                    .size(13)
                    .style(|t: &iced::Theme| iced::widget::text::Style {
                        color: Some(t.palette().text.scale_alpha(0.5)),
                    }),
            ]
            .spacing(12)
            .align_x(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }
}
