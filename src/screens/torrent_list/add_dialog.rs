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

//! Add-torrent dialog state and rendering helpers.

use std::collections::VecDeque;

use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};
use iced_aw::widget::DropDown;
use iced_aw::widget::drop_down::Alignment as DropDownAlignment;

use crate::theme::{
    CheckState, ICON_ARROW_DROP_DOWN, MATERIAL_ICONS, m3_checkbox, m3_tristate_checkbox,
};

use super::Message;

// ── Focus IDs ─────────────────────────────────────────────────────────────────

/// Stable widget ID for the magnet URI field.
pub fn add_magnet_id() -> iced::widget::Id {
    iced::widget::Id::new("add_magnet")
}

/// Stable widget ID for the destination folder field.
pub fn add_destination_id() -> iced::widget::Id {
    iced::widget::Id::new("add_destination")
}

/// A single file entry parsed from a `.torrent` file, shown in the add dialog.
#[derive(Debug, Clone)]
pub struct TorrentFileInfo {
    pub path: String,
    pub size_bytes: u64,
}

/// Result of reading and parsing a `.torrent` file.
#[derive(Debug, Clone)]
pub struct FileReadResult {
    pub metainfo_b64: String,
    pub files: Vec<TorrentFileInfo>,
}

/// State of the add-torrent modal dialog.
#[derive(Debug, Clone)]
pub enum AddDialogState {
    Hidden,
    AddLink {
        magnet: String,
        destination: String,
        error: Option<String>,
    },
    AddFile {
        metainfo_b64: String,
        files: Vec<TorrentFileInfo>,
        selected: Vec<bool>,
        destination: String,
        error: Option<String>,
        /// Remaining torrents yet to be shown (FIFO queue).
        pending_torrents: VecDeque<FileReadResult>,
        /// Whether the recent-paths dropdown overlay is open.
        is_dropdown_open: bool,
        /// Total number of torrents in the original batch (for N-of-M display).
        total_count: usize,
    },
}

/// Format a file size for the add-dialog preview (uses u64).
fn format_file_size(bytes: u64) -> String {
    const GIB: u64 = 1 << 30;
    const MIB: u64 = 1 << 20;
    const KIB: u64 = 1 << 10;
    if bytes >= GIB {
        format!("{:.1} GB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.0} MB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.0} KB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Render the add-torrent dialog for the current `AddDialogState`.
///
/// `recent_paths` is the active profile's `recent_download_paths`, used to
/// populate the destination dropdown overlay.
pub fn view_add_dialog<'a>(
    state: &'a AddDialogState,
    recent_paths: &'a [String],
) -> Element<'a, Message> {
    let (title_str, input_area, cancel_row): (String, Element<'_, Message>, Element<'_, Message>) =
        match state {
            AddDialogState::AddLink {
                magnet,
                destination,
                error,
            } => {
                let dest_row = build_destination_row(destination, recent_paths, false);
                let input: Element<Message> = column![
                    text("Magnet link"),
                    text_input("magnet:?xt=…", magnet)
                        .id(add_magnet_id())
                        .on_input(Message::AddDialogMagnetChanged)
                        .padding([12, 16])
                        .style(crate::theme::m3_text_input),
                    text("Destination folder (leave empty for default)"),
                    dest_row,
                    text("File list unavailable for magnet links.").style(|t: &iced::Theme| {
                        iced::widget::text::Style {
                            color: Some(t.palette().text.scale_alpha(0.5)),
                        }
                    }),
                ]
                .spacing(6)
                .into();
                let area: Element<Message> = if let Some(err) = error {
                    column![input, text(format!("⚠ {err}"))].spacing(4).into()
                } else {
                    input
                };
                let cancel = single_cancel_row();
                ("Add Link".to_owned(), area, cancel)
            }
            AddDialogState::AddFile {
                metainfo_b64: _,
                files,
                selected,
                destination,
                error,
                pending_torrents,
                is_dropdown_open,
                total_count,
            } => {
                // Tri-state header checkbox
                let checked_count = selected.iter().filter(|&&v| v).count();
                let aggregate = if checked_count == selected.len() {
                    CheckState::Checked
                } else if checked_count == 0 {
                    CheckState::Unchecked
                } else {
                    CheckState::Mixed
                };
                let header = m3_tristate_checkbox(aggregate, "Select All", |next| match next {
                    CheckState::Unchecked => Message::AddDialogDeselectAll,
                    _ => Message::AddDialogSelectAll,
                });

                let file_rows: Vec<Element<'_, Message>> = files
                    .iter()
                    .enumerate()
                    .map(|(i, f)| {
                        let is_selected = selected.get(i).copied().unwrap_or(true);
                        row![
                            m3_checkbox(is_selected, "", move |_| {
                                Message::AddDialogFileToggled(i)
                            }),
                            text(&f.path)
                                .width(Length::Fill)
                                .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
                            text(format_file_size(f.size_bytes)),
                        ]
                        .spacing(8)
                        .align_y(iced::alignment::Vertical::Center)
                        .into()
                    })
                    .collect();
                let file_list: Element<Message> =
                    scrollable(column(file_rows).spacing(2)).height(200).into();

                let dest_row = build_destination_row(destination, recent_paths, *is_dropdown_open);

                let input: Element<Message> = column![
                    text("Destination folder (leave empty for default)"),
                    dest_row,
                    text("Files"),
                    header,
                    file_list,
                ]
                .spacing(6)
                .into();
                let area: Element<Message> = if let Some(err) = error {
                    column![input, text(format!("⚠ {err}"))].spacing(4).into()
                } else {
                    input
                };

                let has_pending = !pending_torrents.is_empty();
                let cancel = if has_pending {
                    multi_cancel_row()
                } else {
                    single_cancel_row()
                };

                // N-of-M title
                let current = *total_count - pending_torrents.len();
                let title = if *total_count > 1 {
                    format!("Add Torrent ({current} of {total_count})")
                } else {
                    "Add Torrent".to_owned()
                };

                (title, area, cancel)
            }
            AddDialogState::Hidden => unreachable!("hidden dialog should not be rendered"),
        };

    let dialog = container(
        column![text(title_str).size(18), input_area, cancel_row,]
            .spacing(12)
            .padding(20)
            .width(500),
    )
    .style(|theme: &iced::Theme| container::Style {
        background: Some(iced::Background::Color(theme.palette().background)),
        border: iced::Border {
            color: theme.palette().text.scale_alpha(0.2),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    });

    container(dialog)
        .style(crate::theme::dialog_scrim(0.45))
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill)
        .into()
}

// ── Cancel button rows ────────────────────────────────────────────────────────

/// Single "Cancel" button row (M = 1).
fn single_cancel_row() -> Element<'static, Message> {
    row![
        iced::widget::Space::new().width(iced::Length::Fill),
        button("Cancel")
            .on_press(Message::AddCancelled)
            .padding([10, 24])
            .style(crate::theme::m3_tonal_button),
        button("Add")
            .on_press(Message::AddConfirmed)
            .padding([10, 24])
            .style(crate::theme::m3_primary_button),
    ]
    .spacing(8)
    .width(iced::Length::Fill)
    .into()
}

/// "Cancel This" + "Cancel All" button row (M > 1).
fn multi_cancel_row() -> Element<'static, Message> {
    row![
        iced::widget::Space::new().width(iced::Length::Fill),
        button("Cancel All")
            .on_press(Message::AddCancelAll)
            .padding([10, 24])
            .style(crate::theme::m3_tonal_button),
        button("Cancel This")
            .on_press(Message::AddCancelThis)
            .padding([10, 24])
            .style(crate::theme::m3_tonal_button),
        button("Add")
            .on_press(Message::AddConfirmed)
            .padding([10, 24])
            .style(crate::theme::m3_primary_button),
    ]
    .spacing(8)
    .width(iced::Length::Fill)
    .into()
}

// ── Destination combobox ──────────────────────────────────────────────────────

/// Build the destination field combobox: a `text_input` next to a ▼ toggle button,
/// wrapped in an `iced_aw::DropDown` that shows recent paths as a menu overlay.
fn build_destination_row<'a>(
    destination: &'a str,
    recent_paths: &'a [String],
    is_open: bool,
) -> Element<'a, Message> {
    // ── Overlay ────────────────────────────────────────────────────────────────
    let mut overlay_col = column![].spacing(2).padding([4, 0]);
    for path in recent_paths {
        let path_clone = path.clone();
        let btn = button(text(path).size(13))
            .width(Length::Fill)
            .padding([6, 12])
            .style(crate::theme::m3_menu_item)
            .on_press(Message::AddDialogRecentPathSelected(path_clone));
        overlay_col = overlay_col.push(btn);
    }
    let overlay = container(scrollable(overlay_col))
        .style(crate::theme::m3_menu_card)
        .max_height(200.0)
        .width(Length::Fill);

    // ── Toggle button ─────────────────────────────────────────────────────────
    let drop_icon = text(ICON_ARROW_DROP_DOWN).font(MATERIAL_ICONS).size(24);
    let toggle_btn = if recent_paths.is_empty() {
        button(drop_icon)
            .padding([10, 8])
            .style(crate::theme::m3_tonal_button)
        // no on_press — disabled state
    } else {
        button(drop_icon)
            .padding([10, 8])
            .on_press(Message::AddDialogToggleDropdown)
            .style(crate::theme::m3_tonal_button)
    };

    // ── DropDown wraps only the text_input so the overlay matches its width ──
    let input_underlay = text_input("/path/to/downloads", destination)
        .id(add_destination_id())
        .on_input(Message::AddDialogDestinationChanged)
        .padding([12, 16])
        .style(crate::theme::m3_text_input);

    let dropdown = DropDown::new(input_underlay, overlay, is_open)
        .alignment(DropDownAlignment::Bottom)
        .on_dismiss(Message::AddDialogDismissDropdown);

    row![dropdown, toggle_btn]
        .spacing(4)
        .align_y(iced::Alignment::Center)
        .into()
}
