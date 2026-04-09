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

//! Widget-tree orchestration and row rendering for the torrent list screen.

use iced::widget::rule;
use iced::widget::{
    Space, button, column, container, mouse_area, opaque, progress_bar, row, scrollable, stack,
    text,
};
use iced::{Alignment, Element, Length};

use crate::format::{format_eta, format_size, format_speed};
use crate::theme::{MATERIAL_ICONS, m3_filter_chip, progress_bar_style};

use super::add_dialog::{AddDialogState, view_add_dialog};
use super::columns::{
    SCROLLBAR_WIDTH, W_ETA, W_PROGRESS, W_RATIO, W_SIZE, W_SPEED_DOWN, W_SPEED_UP, W_STATUS,
    view_column_header,
};
use super::dialogs::{view_delete_dialog, view_set_location_dialog};
use super::filters::{count_filters, display_torrents};
use super::toolbar::view_normal_toolbar;
use super::{Message, StatusFilter, TorrentListScreen};

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

/// Build a single M3 filter chip element.
///
/// The checkmark glyph always occupies a fixed-width slot to prevent label
/// jitter when the chip toggles between selected and unselected states.
fn filter_chip(
    label: &str,
    count: u32,
    is_selected: bool,
    on_press: Message,
) -> Element<'_, Message> {
    let check = text(if is_selected { "\u{e876}" } else { "" })
        .font(MATERIAL_ICONS)
        .size(16)
        .width(Length::Fixed(18.0))
        .align_x(Alignment::Center);

    let count_style = move |theme: &iced::Theme| iced::widget::text::Style {
        color: Some(
            theme
                .palette()
                .text
                .scale_alpha(if is_selected { 0.80 } else { 0.60 }),
        ),
    };

    let content = row![
        check,
        text(label).size(13),
        text(format!("{count}")).size(11).style(count_style),
    ]
    .spacing(4)
    .align_y(Alignment::Center);

    button(content)
        .padding([6, 12])
        .on_press(on_press)
        .style(move |theme, status| m3_filter_chip(theme, status, is_selected))
        .into()
}

/// Render the torrent list screen, including overlays owned by the list itself.
pub fn view(
    state: &TorrentListScreen,
    _theme_mode: crate::app::ThemeMode,
    alt_speed_enabled: bool,
) -> Element<'_, Message> {
    // ── Toolbar ───────────────────────────────────────────────────────────────
    let toolbar = view_normal_toolbar(state, alt_speed_enabled);

    // ── Inline error banner ───────────────────────────────────────────────────
    let error_row: Element<Message> = if let Some(err) = &state.error {
        text(format!("⚠ {err}")).into()
    } else {
        Space::new().into()
    };

    let counts = count_filters(&state.torrents);

    // ── Filter chips row ──────────────────────────────────────────────────────
    let is_all_selected = state.filters.len() == StatusFilter::all().len();
    let chips_row = row![
        filter_chip(
            "All",
            state.torrents.len() as u32,
            is_all_selected,
            Message::FilterAllClicked,
        ),
        Space::new().width(Length::Fixed(4.0)),
        filter_chip(
            "Downloading",
            counts.downloading,
            state.filters.contains(&StatusFilter::Downloading),
            Message::FilterToggled(StatusFilter::Downloading),
        ),
        filter_chip(
            "Seeding",
            counts.seeding,
            state.filters.contains(&StatusFilter::Seeding),
            Message::FilterToggled(StatusFilter::Seeding),
        ),
        filter_chip(
            "Paused",
            counts.paused,
            state.filters.contains(&StatusFilter::Paused),
            Message::FilterToggled(StatusFilter::Paused),
        ),
        filter_chip(
            "Active",
            counts.active,
            state.filters.contains(&StatusFilter::Active),
            Message::FilterToggled(StatusFilter::Active),
        ),
        filter_chip(
            "Error",
            counts.error,
            state.filters.contains(&StatusFilter::Error),
            Message::FilterToggled(StatusFilter::Error),
        ),
    ]
    .spacing(6)
    .padding([4, 0]);

    // ── Sticky header ─────────────────────────────────────────────────────────
    let header = view_column_header(state);
    let display = display_torrents(
        &state.torrents,
        state.sort_column,
        state.sort_dir,
        &state.filters,
    );

    let is_list_empty = state.initial_load_done && state.torrents.is_empty();
    let is_filter_empty =
        state.initial_load_done && !state.torrents.is_empty() && display.is_empty();

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
        let is_ctx_target = state.context_menu.is_some_and(|(id, _)| id == t.id);
        let row_elem: Element<Message> = if is_selected || is_ctx_target {
            container(row_content)
                .style(crate::theme::selected_row)
                .width(Length::Fill)
                .into()
        } else {
            row_content.into()
        };

        mouse_area(
            button(row_elem)
                .on_press(Message::TorrentSelected(t.id))
                .style(iced::widget::button::text)
                .width(Length::Fill)
                .padding(0),
        )
        .on_right_press(Message::TorrentRightClicked(t.id))
        .into()
    });

    let list: Element<Message> = if is_list_empty {
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
    } else if is_filter_empty {
        // Filter placeholder — no torrents match the active chips
        container(
            text("No torrents match the selected filters.")
                .size(14)
                .style(|t: &iced::Theme| iced::widget::text::Style {
                    color: Some(t.palette().text.scale_alpha(0.5)),
                }),
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
        column![
            toolbar,
            error_row,
            chips_row,
            header,
            rule::horizontal(1),
            list
        ]
        .spacing(4)
        .padding([8, 16])
        .width(Length::Fill)
        .height(Length::Fill),
    )
    .into();

    // ── Modal overlay ─────────────────────────────────────────────────────────
    let after_add: Element<Message> = match &state.add_dialog {
        AddDialogState::Hidden => main_content,
        dialog_state => stack![main_content, view_add_dialog(dialog_state)].into(),
    };

    let after_delete: Element<Message> = if let Some((del_id, del_local)) = state.confirming_delete
    {
        let name = state
            .torrents
            .iter()
            .find(|t| t.id == del_id)
            .map(|t| t.name.as_str())
            .unwrap_or("this torrent");
        stack![after_add, opaque(view_delete_dialog(name, del_local))].into()
    } else {
        after_add
    };

    // ── Set Data Location dialog ──────────────────────────────────────────────
    let after_set_location: Element<Message> = if let Some(dlg) = &state.set_location_dialog {
        stack![after_delete, opaque(view_set_location_dialog(dlg))].into()
    } else {
        after_delete
    };

    after_set_location
}
