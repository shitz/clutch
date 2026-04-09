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

//! Toolbar rendering for the torrent list screen.

use iced::widget::tooltip;
use iced::widget::{Space, row, text};
use iced::{Alignment, Element, Length};

use crate::theme::{
    ICON_ADD, ICON_DELETE, ICON_LINK, ICON_LOGOUT, ICON_PAUSE, ICON_PLAY, ICON_SETTINGS,
    ICON_SPEED, active_icon_button, icon, icon_button,
};

use super::{Message, TorrentListScreen};

/// Render the primary torrent-list toolbar and its action groups.
pub(crate) fn view_normal_toolbar(
    state: &TorrentListScreen,
    alt_speed_enabled: bool,
) -> Element<'_, Message> {
    let selected = state
        .selected_id
        .and_then(|id| state.torrents.iter().find(|torrent| torrent.id == id));
    let can_pause = selected.is_some_and(|torrent| matches!(torrent.status, 3..=6));
    let can_resume = selected.is_some_and(|torrent| torrent.status == 0);
    let can_delete = state.selected_id.is_some();

    let group1: Element<Message> = row![
        tooltip(
            icon_button(icon(ICON_ADD)).on_press(Message::AddTorrentClicked),
            text("Add torrent from file"),
            tooltip::Position::Bottom,
        )
        .gap(6)
        .style(crate::theme::m3_tooltip),
        tooltip(
            icon_button(icon(ICON_LINK)).on_press(Message::AddLinkClicked),
            text("Add torrent from magnet link"),
            tooltip::Position::Bottom,
        )
        .gap(6)
        .style(crate::theme::m3_tooltip),
    ]
    .spacing(4)
    .into();

    let pause_btn = {
        let button = icon_button(icon(ICON_PAUSE));
        if can_pause {
            button.on_press(Message::PauseClicked)
        } else {
            button
        }
    };
    let resume_btn = {
        let button = icon_button(icon(ICON_PLAY));
        if can_resume {
            button.on_press(Message::ResumeClicked)
        } else {
            button
        }
    };
    let delete_btn = {
        let button = icon_button(icon(ICON_DELETE));
        if can_delete {
            button.on_press(Message::DeleteClicked)
        } else {
            button
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
            active_icon_button(icon(ICON_SPEED), alt_speed_enabled)
                .on_press(Message::TurtleModeToggled),
            text("Turtle Mode"),
            tooltip::Position::Bottom,
        )
        .gap(6)
        .style(crate::theme::m3_tooltip),
        tooltip(
            icon_button(icon(ICON_SETTINGS)).on_press(Message::OpenSettingsClicked),
            text("Settings"),
            tooltip::Position::Bottom,
        )
        .gap(6)
        .style(crate::theme::m3_tooltip),
        tooltip(
            icon_button(icon(ICON_LOGOUT)).on_press(Message::Disconnect),
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
