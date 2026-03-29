//! Add-torrent dialog state and view.

use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::{Element, Length};

use super::Message;

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
                    .on_input(Message::AddDialogMagnetChanged)
                    .padding(6),
                text("Destination folder (leave empty for default)"),
                text_input("/path/to/downloads", destination)
                    .on_input(Message::AddDialogDestinationChanged)
                    .padding(6),
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
                    .on_input(Message::AddDialogDestinationChanged)
                    .padding(6),
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
                button("Add")
                    .on_press(Message::AddConfirmed)
                    .style(iced::widget::button::primary),
                button("Cancel")
                    .on_press(Message::AddCancelled)
                    .style(iced::widget::button::secondary),
            ]
            .spacing(8),
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
