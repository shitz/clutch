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

//! Column-header rendering and shared width constants for the torrent list.

use iced::widget::tooltip;
use iced::widget::{button, container, row, text};
use iced::{Alignment, Element, Length};

use crate::theme::{ICON_DOWNLOAD, ICON_UPLOAD, MATERIAL_ICONS};

use super::sort::{SortColumn, SortDir};
use super::{Message, TorrentListScreen};

pub(crate) const W_STATUS: f32 = 90.0;
pub(crate) const W_SIZE: f32 = 80.0;
pub(crate) const W_SPEED_DOWN: f32 = 90.0;
pub(crate) const W_SPEED_UP: f32 = 90.0;
pub(crate) const W_ETA: f32 = 80.0;
pub(crate) const W_RATIO: f32 = 64.0;
pub(crate) const W_PROGRESS: f32 = 130.0;
pub(crate) const SCROLLBAR_WIDTH: f32 = 14.0;

/// Render the sticky column header row and sort controls.
pub(crate) fn view_column_header(state: &TorrentListScreen) -> Element<'_, Message> {
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
        .style(|theme: &iced::Theme| iced::widget::text::Style {
            color: Some(theme.palette().text.scale_alpha(0.55)),
        });
    let chevron_elem = text(chevron)
        .size(14)
        .width(Length::Fixed(14.0))
        .align_x(Alignment::Center)
        .style(|theme: &iced::Theme| iced::widget::text::Style {
            color: Some(theme.palette().text.scale_alpha(0.55)),
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
        .font(MATERIAL_ICONS)
        .size(14)
        .width(Length::Fill)
        .align_x(alignment)
        .style(|theme: &iced::Theme| iced::widget::text::Style {
            color: Some(theme.palette().text.scale_alpha(0.55)),
        });
    let chevron_elem = text(chevron)
        .size(14)
        .width(Length::Fixed(14.0))
        .align_x(Alignment::Center)
        .style(|theme: &iced::Theme| iced::widget::text::Style {
            color: Some(theme.palette().text.scale_alpha(0.55)),
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
        Some(current) if *current == col => match dir {
            SortDir::Asc => "▴",
            SortDir::Desc => "▾",
        },
        _ => "",
    }
}
