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

use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};

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
        destination: String,
        error: Option<String>,
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

pub fn view_add_dialog(state: &AddDialogState) -> Element<'_, Message> {
    let (title_str, input_area): (&str, Element<'_, Message>) = match state {
        AddDialogState::AddLink {
            magnet,
            destination,
            error,
        } => {
            let input: Element<Message> = column![
                text("Magnet link"),
                text_input("magnet:?xt=…", magnet)
                    .id(add_magnet_id())
                    .on_input(Message::AddDialogMagnetChanged)
                    .padding([12, 16])
                    .style(crate::theme::m3_text_input),
                text("Destination folder (leave empty for default)"),
                text_input("/path/to/downloads", destination)
                    .id(add_destination_id())
                    .on_input(Message::AddDialogDestinationChanged)
                    .padding([12, 16])
                    .style(crate::theme::m3_text_input),
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
            ("Add Link", area)
        }
        AddDialogState::AddFile {
            files,
            destination,
            error,
            ..
        } => {
            let file_rows = files.iter().map(|f| {
                row![
                    text(&f.path).width(Length::Fill),
                    text(format_file_size(f.size_bytes)),
                ]
                .spacing(8)
                .into()
            });
            let file_list: Element<Message> =
                scrollable(column(file_rows).spacing(2)).height(200).into();

            let input: Element<Message> = column![
                text("Destination folder (leave empty for default)"),
                text_input("/path/to/downloads", destination)
                    .id(add_destination_id())
                    .on_input(Message::AddDialogDestinationChanged)
                    .padding([12, 16])
                    .style(crate::theme::m3_text_input),
                text("Files"),
                file_list,
            ]
            .spacing(6)
            .into();
            let area: Element<Message> = if let Some(err) = error {
                column![input, text(format!("⚠ {err}"))].spacing(4).into()
            } else {
                input
            };
            ("Add Torrent", area)
        }
        AddDialogState::Hidden => unreachable!("hidden dialog should not be rendered"),
    };

    let dialog = container(
        column![
            text(title_str).size(18),
            input_area,
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
            .width(iced::Length::Fill),
        ]
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
        .style(|_theme: &iced::Theme| container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgba(
                0.0, 0.0, 0.0, 0.45,
            ))),
            ..Default::default()
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill)
        .into()
}
