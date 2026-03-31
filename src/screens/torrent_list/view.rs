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


use iced::widget::rule;
use iced::widget::tooltip;
use iced::widget::{
    Space, button, checkbox, column, container, progress_bar, row, scrollable, stack, text,
};
use iced::{Alignment, Element, Length};

use crate::format::{format_eta, format_size, format_speed};
use crate::rpc::TorrentData;
use crate::theme::{
    ICON_ADD, ICON_DELETE, ICON_DOWNLOAD, ICON_LINK, ICON_LOGOUT, ICON_PAUSE, ICON_PLAY,
    ICON_SETTINGS, ICON_UPLOAD, icon, progress_bar_style,
};

use super::add_dialog::{AddDialogState, view_add_dialog};
use super::sort::{SortColumn, SortDir, sort_torrents};
use super::{Message, TorrentListScreen};

// Fixed pixel widths for narrow numeric columns.
const W_STATUS: f32 = 90.0;
const W_SIZE: f32 = 80.0;
const W_SPEED_DOWN: f32 = 90.0;
const W_SPEED_UP: f32 = 90.0;
const W_ETA: f32 = 80.0;
const W_RATIO: f32 = 64.0;
const W_PROGRESS: f32 = 130.0;
const SCROLLBAR_WIDTH: f32 = 14.0;

fn status_label(status: i32) -> &'static str {
    match status {
        0 => "Stopped",
        1 => "Queued (check)",
        2 => "Checking",
        3 => "Queued",
        4 => "Downloading",
        5 => "Queued (seed)",
        6 => "Seeding",
        _ => "Unknown",
    }
}

pub fn view(state: &TorrentListScreen, theme_mode: crate::app::ThemeMode) -> Element<'_, Message> {
    // ── Toolbar ───────────────────────────────────────────────────────────────
    let toolbar: Element<Message> = if let Some((del_id, del_local)) = state.confirming_delete {
        let name = state
            .torrents
            .iter()
            .find(|t| t.id == del_id)
            .map(|t| t.name.as_str())
            .unwrap_or("this torrent");
        row![
            text(format!("Delete \"{}\"?", name)).align_y(Alignment::Center),
            checkbox(del_local)
                .label("Delete local data")
                .on_toggle(Message::DeleteLocalDataToggled),
            Space::new(),
            button("Cancel")
                .on_press(Message::DeleteCancelled)
                .padding([10, 20])
                .style(crate::theme::m3_tonal_button),
            button("Confirm Delete")
                .on_press(Message::DeleteConfirmed)
                .padding([10, 20])
                .style(|t: &iced::Theme, s| {
                    let p = t.extended_palette();
                    let bg = match s {
                        iced::widget::button::Status::Hovered
                        | iced::widget::button::Status::Pressed => p.danger.strong.color,
                        _ => p.danger.base.color,
                    };
                    iced::widget::button::Style {
                        background: Some(iced::Background::Color(bg)),
                        text_color: p.danger.base.text,
                        border: iced::Border {
                            radius: 100.0.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                }),
        ]
        .align_y(Alignment::Center)
        .spacing(8)
        .into()
    } else {
        view_normal_toolbar(state, theme_mode)
    };

    // ── Inline error banner ───────────────────────────────────────────────────
    let error_row: Element<Message> = if let Some(err) = &state.error {
        text(format!("⚠ {err}")).into()
    } else {
        Space::new().into()
    };

    // ── Sticky header ─────────────────────────────────────────────────────────
    let header = view_column_header(state);

    // ── Data rows ─────────────────────────────────────────────────────────────
    let display: Vec<&TorrentData> = match state.sort_column {
        Some(col) => sort_torrents(&state.torrents, col, state.sort_dir),
        None => state.torrents.iter().collect(),
    };

    let rows = display.into_iter().map(|t| {
        let ratio_str = if t.upload_ratio < 0.0 {
            "—".to_owned()
        } else {
            format!("{:.2}", t.upload_ratio)
        };

        let row_content = row![
            text(&t.name)
                .width(Length::Fill)
                .align_x(Alignment::Start)
                .wrapping(text::Wrapping::WordOrGlyph),
            text(status_label(t.status))
                .width(Length::Fixed(W_STATUS))
                .align_x(Alignment::Start),
            text(format_size(t.total_size))
                .width(Length::Fixed(W_SIZE))
                .align_x(Alignment::End),
            text(format_speed(t.rate_download))
                .width(Length::Fixed(W_SPEED_DOWN))
                .align_x(Alignment::End),
            text(format_speed(t.rate_upload))
                .width(Length::Fixed(W_SPEED_UP))
                .align_x(Alignment::End),
            text(format_eta(t.eta))
                .width(Length::Fixed(W_ETA))
                .align_x(Alignment::End),
            text(ratio_str)
                .width(Length::Fixed(W_RATIO))
                .align_x(Alignment::End),
            container(
                row![
                    progress_bar(0.0..=1.0, t.percent_done as f32)
                        .style(progress_bar_style(t.status))
                        .length(Length::Fill)
                        .girth(10.0),
                    text(format!("{:.0}%", t.percent_done * 100.0))
                        .size(11)
                        .width(Length::Fixed(34.0))
                        .align_x(Alignment::End),
                ]
                .spacing(4)
                .align_y(iced::Center),
            )
            .width(Length::Fixed(W_PROGRESS))
            .align_x(Alignment::Start),
        ]
        .spacing(16)
        .width(Length::Fill)
        .padding([10, 0])
        .align_y(iced::Center);

        let is_selected = state.selected_id == Some(t.id);
        let row_elem: Element<Message> = if is_selected {
            container(row_content)
                .style(crate::theme::selected_row)
                .width(Length::Fill)
                .into()
        } else {
            row_content.into()
        };

        button(row_elem)
            .on_press(Message::TorrentSelected(t.id))
            .style(iced::widget::button::text)
            .width(Length::Fill)
            .padding(0)
            .into()
    });

    let list: Element<Message> = if state.initial_load_done && state.torrents.is_empty() {
        // Empty state — centered logo with helper text
        container(
            column![
                iced::widget::image(iced::widget::image::Handle::from_bytes(
                    crate::theme::LOGO_BYTES,
                ))
                .width(Length::Fixed(180.0))
                .content_fit(iced::ContentFit::ScaleDown)
                .opacity(0.25),
                text("No torrents. Add one with +")
                    .size(14)
                    .style(|t: &iced::Theme| iced::widget::text::Style {
                        color: Some(t.palette().text.scale_alpha(0.4)),
                    }),
            ]
            .spacing(16)
            .align_x(iced::Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    } else {
        scrollable(container(column(rows).spacing(4)).padding(iced::Padding {
            top: 0.0,
            bottom: 0.0,
            left: 0.0,
            right: SCROLLBAR_WIDTH + 2.0,
        }))
        .direction(iced::widget::scrollable::Direction::Vertical(
            iced::widget::scrollable::Scrollbar::new()
                .width(SCROLLBAR_WIDTH)
                .scroller_width(SCROLLBAR_WIDTH)
                .margin(0),
        ))
        .into()
    };

    let main_content: Element<Message> = container(
        column![toolbar, error_row, header, rule::horizontal(1), list]
            .spacing(4)
            .padding([8, 16])
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .into();

    // ── Modal overlay ─────────────────────────────────────────────────────────
    match &state.add_dialog {
        AddDialogState::Hidden => main_content,
        dialog_state => stack![main_content, view_add_dialog(dialog_state)].into(),
    }
}

fn view_normal_toolbar(
    state: &TorrentListScreen,
    _theme_mode: crate::app::ThemeMode,
) -> Element<'_, Message> {
    let selected = state
        .selected_id
        .and_then(|id| state.torrents.iter().find(|t| t.id == id));
    let can_pause = selected.is_some_and(|t| matches!(t.status, 3..=6));
    let can_resume = selected.is_some_and(|t| t.status == 0);
    let can_delete = state.selected_id.is_some();

    let group1: Element<Message> = row![
        tooltip(
            crate::theme::icon_button(icon(ICON_ADD)).on_press(Message::AddTorrentClicked),
            text("Add torrent from file"),
            tooltip::Position::Bottom,
        )
        .gap(6)
        .style(crate::theme::m3_tooltip),
        tooltip(
            crate::theme::icon_button(icon(ICON_LINK)).on_press(Message::AddLinkClicked),
            text("Add torrent from magnet link"),
            tooltip::Position::Bottom,
        )
        .gap(6)
        .style(crate::theme::m3_tooltip),
    ]
    .spacing(4)
    .into();

    let pause_btn = {
        let b = crate::theme::icon_button(icon(ICON_PAUSE));
        if can_pause {
            b.on_press(Message::PauseClicked)
        } else {
            b
        }
    };
    let resume_btn = {
        let b = crate::theme::icon_button(icon(ICON_PLAY));
        if can_resume {
            b.on_press(Message::ResumeClicked)
        } else {
            b
        }
    };
    let delete_btn = {
        let b = crate::theme::icon_button(icon(ICON_DELETE));
        if can_delete {
            b.on_press(Message::DeleteClicked)
        } else {
            b
        }
    };
    let group2: Element<Message> = row![
        tooltip(pause_btn, text("Pause"), tooltip::Position::Bottom)
            .gap(6)
            .style(crate::theme::m3_tooltip),
        tooltip(resume_btn, text("Resume"), tooltip::Position::Bottom)
            .gap(6)
            .style(crate::theme::m3_tooltip),
        tooltip(delete_btn, text("Delete"), tooltip::Position::Bottom)
            .gap(6)
            .style(crate::theme::m3_tooltip),
    ]
    .spacing(4)
    .into();

    let group3: Element<Message> = row![
        tooltip(
            crate::theme::icon_button(icon(ICON_SETTINGS)).on_press(Message::OpenSettingsClicked),
            text("Settings"),
            tooltip::Position::Bottom,
        )
        .gap(6)
        .style(crate::theme::m3_tooltip),
        tooltip(
            crate::theme::icon_button(icon(ICON_LOGOUT)).on_press(Message::Disconnect),
            text("Disconnect"),
            tooltip::Position::Bottom,
        )
        .gap(6)
        .style(crate::theme::m3_tooltip),
    ]
    .spacing(4)
    .into();

    row![
        group1,
        Space::new().width(16),
        group2,
        Space::new().width(Length::Fill),
        group3
    ]
    .align_y(Alignment::Center)
    .spacing(0)
    .into()
}

fn view_column_header(state: &TorrentListScreen) -> Element<'_, Message> {
    let header_row = row![
        container(
            tooltip(
                col_header_btn(
                    "NAME",
                    SortColumn::Name,
                    &state.sort_column,
                    state.sort_dir,
                    Alignment::Start
                )
                .width(Length::Fill),
                text("Sort by name"),
                tooltip::Position::Bottom,
            )
            .gap(6)
            .style(crate::theme::m3_tooltip),
        )
        .width(Length::Fill),
        container(
            tooltip(
                col_header_btn(
                    "STATUS",
                    SortColumn::Status,
                    &state.sort_column,
                    state.sort_dir,
                    Alignment::Start
                )
                .width(Length::Fixed(W_STATUS)),
                text("Sort by status"),
                tooltip::Position::Bottom,
            )
            .gap(6)
            .style(crate::theme::m3_tooltip),
        )
        .width(Length::Fixed(W_STATUS)),
        container(
            tooltip(
                col_header_btn(
                    "SIZE",
                    SortColumn::Size,
                    &state.sort_column,
                    state.sort_dir,
                    Alignment::End
                )
                .width(Length::Fixed(W_SIZE)),
                text("Total size"),
                tooltip::Position::Bottom,
            )
            .gap(6)
            .style(crate::theme::m3_tooltip),
        )
        .width(Length::Fixed(W_SIZE)),
        container(
            tooltip(
                col_header_icon_btn(
                    ICON_DOWNLOAD,
                    SortColumn::SpeedDown,
                    &state.sort_column,
                    state.sort_dir,
                    Alignment::End
                )
                .width(Length::Fixed(W_SPEED_DOWN)),
                text("Download speed"),
                tooltip::Position::Bottom,
            )
            .gap(6)
            .style(crate::theme::m3_tooltip),
        )
        .width(Length::Fixed(W_SPEED_DOWN)),
        container(
            tooltip(
                col_header_icon_btn(
                    ICON_UPLOAD,
                    SortColumn::SpeedUp,
                    &state.sort_column,
                    state.sort_dir,
                    Alignment::End
                )
                .width(Length::Fixed(W_SPEED_UP)),
                text("Upload speed"),
                tooltip::Position::Bottom,
            )
            .gap(6)
            .style(crate::theme::m3_tooltip),
        )
        .width(Length::Fixed(W_SPEED_UP)),
        container(
            tooltip(
                col_header_btn(
                    "ETA",
                    SortColumn::Eta,
                    &state.sort_column,
                    state.sort_dir,
                    Alignment::End
                )
                .width(Length::Fixed(W_ETA)),
                text("Estimated time remaining"),
                tooltip::Position::Bottom,
            )
            .gap(6)
            .style(crate::theme::m3_tooltip),
        )
        .width(Length::Fixed(W_ETA)),
        container(
            tooltip(
                col_header_btn(
                    "RATIO",
                    SortColumn::Ratio,
                    &state.sort_column,
                    state.sort_dir,
                    Alignment::End
                )
                .width(Length::Fixed(W_RATIO)),
                text("Upload/download ratio"),
                tooltip::Position::Bottom,
            )
            .gap(6)
            .style(crate::theme::m3_tooltip),
        )
        .width(Length::Fixed(W_RATIO)),
        container(
            tooltip(
                col_header_btn(
                    "PROGRESS",
                    SortColumn::Progress,
                    &state.sort_column,
                    state.sort_dir,
                    Alignment::End
                )
                .width(Length::Fixed(W_PROGRESS)),
                text("Percent complete"),
                tooltip::Position::Bottom,
            )
            .gap(6)
            .style(crate::theme::m3_tooltip),
        )
        .width(Length::Fixed(W_PROGRESS)),
    ]
    .spacing(16);

    container(header_row)
        .padding(iced::Padding {
            top: 0.0,
            bottom: 0.0,
            left: 0.0,
            right: SCROLLBAR_WIDTH + 2.0,
        })
        .into()
}

fn col_header_btn(
    label: &'static str,
    col: SortColumn,
    active: &Option<SortColumn>,
    dir: SortDir,
    alignment: Alignment,
) -> iced::widget::Button<'static, Message> {
    let chevron = chevron_indicator(col, active, dir);
    let label_elem = text(label)
        .size(11)
        .width(Length::Fill)
        .align_x(alignment)
        .style(|t: &iced::Theme| iced::widget::text::Style {
            color: Some(t.palette().text.scale_alpha(0.55)),
        });
    let chevron_elem = text(chevron)
        .size(14)
        .width(Length::Fixed(14.0))
        .align_x(Alignment::Center)
        .style(|t: &iced::Theme| iced::widget::text::Style {
            color: Some(t.palette().text.scale_alpha(0.55)),
        });

    let content: Element<'static, Message> = if alignment == Alignment::End {
        row![chevron_elem, label_elem]
            .width(Length::Fill)
            .align_y(Alignment::Center)
            .into()
    } else {
        row![label_elem, chevron_elem]
            .width(Length::Fill)
            .align_y(Alignment::Center)
            .into()
    };

    button(content)
        .on_press(Message::ColumnHeaderClicked(col))
        .style(iced::widget::button::text)
        .padding([2, 0])
}

fn col_header_icon_btn(
    glyph: char,
    col: SortColumn,
    active: &Option<SortColumn>,
    dir: SortDir,
    alignment: Alignment,
) -> iced::widget::Button<'static, Message> {
    let chevron = chevron_indicator(col, active, dir);
    let icon_elem = text(String::from(glyph))
        .font(crate::theme::MATERIAL_ICONS)
        .size(14)
        .width(Length::Fill)
        .align_x(alignment)
        .style(|t: &iced::Theme| iced::widget::text::Style {
            color: Some(t.palette().text.scale_alpha(0.55)),
        });
    let chevron_elem = text(chevron)
        .size(14)
        .width(Length::Fixed(14.0))
        .align_x(Alignment::Center)
        .style(|t: &iced::Theme| iced::widget::text::Style {
            color: Some(t.palette().text.scale_alpha(0.55)),
        });

    let content: Element<'static, Message> = if alignment == Alignment::End {
        row![chevron_elem, icon_elem]
            .width(Length::Fill)
            .align_y(Alignment::Center)
            .into()
    } else {
        row![icon_elem, chevron_elem]
            .width(Length::Fill)
            .align_y(Alignment::Center)
            .into()
    };

    button(content)
        .on_press(Message::ColumnHeaderClicked(col))
        .style(iced::widget::button::text)
        .padding([2, 0])
}

fn chevron_indicator(col: SortColumn, active: &Option<SortColumn>, dir: SortDir) -> &'static str {
    match active {
        Some(c) if *c == col => match dir {
            SortDir::Asc => "▴",
            SortDir::Desc => "▾",
        },
        _ => "",
    }
}
