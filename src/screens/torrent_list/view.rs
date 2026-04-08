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
    Space, button, checkbox, column, container, mouse_area, opaque, progress_bar, row, scrollable,
    stack, text,
};
use iced::{Alignment, Element, Length};

use crate::format::{format_eta, format_size, format_speed};
use crate::rpc::TorrentData;
use crate::theme::{
    ICON_ADD, ICON_DELETE, ICON_DOWNLOAD, ICON_FOLDER, ICON_LINK, ICON_LOGOUT, ICON_PAUSE,
    ICON_PLAY, ICON_SETTINGS, ICON_SPEED, ICON_UPLOAD, MATERIAL_ICONS, icon, m3_filter_chip,
    progress_bar_style,
};

use super::add_dialog::{AddDialogState, view_add_dialog};
use super::sort::{SortColumn, SortDir, sort_torrents};
use super::{Message, SetLocationDialog, StatusFilter, TorrentListScreen, matching_filters};

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

pub fn view(
    state: &TorrentListScreen,
    theme_mode: crate::app::ThemeMode,
    alt_speed_enabled: bool,
) -> Element<'_, Message> {
    // ── Toolbar ───────────────────────────────────────────────────────────────
    let toolbar = view_normal_toolbar(state, theme_mode, alt_speed_enabled);

    // ── Inline error banner ───────────────────────────────────────────────────
    let error_row: Element<Message> = if let Some(err) = &state.error {
        text(format!("⚠ {err}")).into()
    } else {
        Space::new().into()
    };

    // ── Count pass: tally per-bucket counts over the full torrent list ────────
    let mut count_downloading: u32 = 0;
    let mut count_seeding: u32 = 0;
    let mut count_paused: u32 = 0;
    let mut count_active: u32 = 0;
    let mut count_error: u32 = 0;
    for t in &state.torrents {
        for f in matching_filters(t) {
            match f {
                StatusFilter::Downloading => count_downloading += 1,
                StatusFilter::Seeding => count_seeding += 1,
                StatusFilter::Paused => count_paused += 1,
                StatusFilter::Active => count_active += 1,
                StatusFilter::Error => count_error += 1,
            }
        }
    }

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
            count_downloading,
            state.filters.contains(&StatusFilter::Downloading),
            Message::FilterToggled(StatusFilter::Downloading),
        ),
        filter_chip(
            "Seeding",
            count_seeding,
            state.filters.contains(&StatusFilter::Seeding),
            Message::FilterToggled(StatusFilter::Seeding),
        ),
        filter_chip(
            "Paused",
            count_paused,
            state.filters.contains(&StatusFilter::Paused),
            Message::FilterToggled(StatusFilter::Paused),
        ),
        filter_chip(
            "Active",
            count_active,
            state.filters.contains(&StatusFilter::Active),
            Message::FilterToggled(StatusFilter::Active),
        ),
        filter_chip(
            "Error",
            count_error,
            state.filters.contains(&StatusFilter::Error),
            Message::FilterToggled(StatusFilter::Error),
        ),
    ]
    .spacing(6)
    .padding([4, 0]);

    // ── Sticky header ─────────────────────────────────────────────────────────
    let header = view_column_header(state);

    // ── Data rows (sorted, then filtered against active chips) ────────────────
    let sorted: Vec<&TorrentData> = match state.sort_column {
        Some(col) => sort_torrents(&state.torrents, col, state.sort_dir),
        None => state.torrents.iter().collect(),
    };
    let display: Vec<&TorrentData> = sorted
        .into_iter()
        .filter(|t| {
            matching_filters(t)
                .iter()
                .any(|f| state.filters.contains(f))
        })
        .collect();

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

    // ── Context menu overlay ──────────────────────────────────────────────────
    // Rendered at the main_screen level via `view_context_menu_overlay` so it
    // can draw over the inspector panel. Nothing to do here.
    after_set_location
}

/// Menu width in logical pixels — used for right-edge mitigation.
const MENU_WIDTH: f32 = 220.0;

/// Builds a single M3 context-menu row with a fixed-width icon container and
/// an edge-to-edge hover highlight. Pass `on_press = None` to render disabled.
fn menu_item<'a>(
    icon_glyph: char,
    label: &'a str,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    // Force every icon glyph into an identical 24-px bounding box so that
    // narrow glyphs (play triangle) align with wide ones (trash square).
    let icon_widget = text(icon_glyph)
        .font(MATERIAL_ICONS)
        .size(18)
        .width(Length::Fixed(24.0))
        .align_x(Alignment::Center);

    let content = row![icon_widget, text(label).size(14)]
        .spacing(12)
        .align_y(Alignment::Center);

    let btn = button(content).width(Length::Fill).padding([8, 16]);

    if let Some(msg) = on_press {
        btn.on_press(msg).style(crate::theme::m3_menu_item).into()
    } else {
        btn.style(crate::theme::m3_menu_item_disabled).into()
    }
}

/// Builds the context menu overlay element when a context menu is open.
///
/// Returns `None` when no context menu is active. The returned element must be
/// placed on top of the **full** application content (including the inspector)
/// so the menu is not clipped by the torrent-list container.
pub fn view_context_menu_overlay(state: &TorrentListScreen) -> Option<Element<'_, Message>> {
    let (torrent_id, point) = state.context_menu?;

    // Bottom-edge mitigation: shift menu up if it would overflow.
    let effective_y = if point.y > state.window_height - 150.0 {
        point.y - 150.0
    } else {
        point.y
    };

    // Right-edge mitigation: anchor to the left of the cursor when there is
    // not enough horizontal space to the right.
    let effective_x = if point.x + MENU_WIDTH > state.window_width {
        (point.x - MENU_WIDTH).max(0.0)
    } else {
        point.x
    };

    let torrent_opt = state.torrents.iter().find(|t| t.id == torrent_id);
    let can_start = torrent_opt.is_some_and(|t| !matches!(t.status, 3..=6));
    let can_pause = torrent_opt.is_some_and(|t| matches!(t.status, 3..=6));

    // M3 menu card: 8px vertical padding, zero horizontal padding so buttons
    // span edge-to-edge with their own horizontal padding inside.
    let menu_card = container(
        column![
            menu_item(
                ICON_PLAY,
                "Start",
                can_start.then_some(Message::ContextMenuStart(torrent_id))
            ),
            menu_item(
                ICON_PAUSE,
                "Pause",
                can_pause.then_some(Message::ContextMenuPause(torrent_id))
            ),
            menu_item(
                ICON_DELETE,
                "Delete",
                Some(Message::ContextMenuDelete(torrent_id))
            ),
            menu_item(
                ICON_FOLDER,
                "Set Data Location",
                Some(Message::OpenSetLocation(torrent_id))
            ),
        ]
        .spacing(0),
    )
    .padding(iced::Padding {
        top: 8.0,
        bottom: 8.0,
        left: 0.0,
        right: 0.0,
    })
    .width(Length::Fixed(MENU_WIDTH))
    .style(crate::theme::m3_menu_card);

    let positioned = container(menu_card)
        .padding(iced::Padding {
            top: effective_y,
            left: effective_x,
            right: 0.0,
            bottom: 0.0,
        })
        .width(Length::Fill)
        .height(Length::Fill);

    let click_away = mouse_area(
        container(Space::new())
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(Message::DismissContextMenu);

    Some(stack![click_away, positioned].into())
}

fn view_delete_dialog(name: &str, del_local: bool) -> Element<'_, Message> {
    let card = container(
        column![
            text(format!("Delete \"{}\"?", name)).size(18),
            text("This cannot be undone.").size(13),
            checkbox(del_local)
                .label("Also delete local data")
                .on_toggle(Message::DeleteLocalDataToggled),
            row![
                Space::new().width(Length::Fill),
                button("Cancel")
                    .on_press(Message::DeleteCancelled)
                    .padding([10, 24])
                    .style(crate::theme::m3_tonal_button),
                button("Confirm Delete")
                    .on_press(Message::DeleteConfirmed)
                    .padding([10, 24])
                    .style(|t: &iced::Theme, s| {
                        let p = t.extended_palette();
                        let bg = match s {
                            button::Status::Hovered | button::Status::Pressed => {
                                p.danger.strong.color
                            }
                            _ => p.danger.base.color,
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
                    }),
            ]
            .spacing(8)
            .width(Length::Fill),
        ]
        .spacing(16),
    )
    .padding(28)
    .max_width(400.0)
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

fn view_set_location_dialog(dlg: &SetLocationDialog) -> Element<'_, Message> {
    use iced::widget::text_input;

    let card = container(
        column![
            text("Set Data Location").size(18),
            text("Destination path on the daemon's filesystem"),
            text_input("/path/to/data", &dlg.path)
                .on_input(Message::SetLocationPathChanged)
                .padding([12, 16])
                .style(crate::theme::m3_text_input),
            checkbox(dlg.move_data)
                .label("Move data to new location")
                .on_toggle(|_| Message::SetLocationMoveToggled),
            row![
                Space::new().width(Length::Fill),
                button("Cancel")
                    .on_press(Message::SetLocationCancel)
                    .padding([10, 24])
                    .style(crate::theme::m3_tonal_button),
                button("Apply")
                    .on_press(Message::SetLocationApply)
                    .padding([10, 24])
                    .style(crate::theme::m3_primary_button),
            ]
            .spacing(8)
            .width(Length::Fill),
        ]
        .spacing(16),
    )
    .padding(28)
    .max_width(440.0)
    .style(crate::theme::m3_card);

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

fn view_normal_toolbar(
    state: &TorrentListScreen,
    _theme_mode: crate::app::ThemeMode,
    alt_speed_enabled: bool,
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
            crate::theme::active_icon_button(icon(ICON_SPEED), alt_speed_enabled)
                .on_press(Message::TurtleModeToggled),
            text("Turtle Mode"),
            tooltip::Position::Bottom,
        )
        .gap(6)
        .style(crate::theme::m3_tooltip),
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
