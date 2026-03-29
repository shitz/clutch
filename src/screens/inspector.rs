//! Detail inspector sub-screen.
//!
//! Displays per-torrent detail in a tabbed panel using `iced_aw::Tabs`.
//! `view()` accepts an immutable reference to the currently selected
//! `TorrentData` — all data arrives via the polling subscription; the
//! inspector owns no RPC state.
//!
//! # Architecture
//!
//! This module is a self-contained Elm component:
//! - [`InspectorScreen`] — state (active tab only)
//! - [`Message`] — messages that can be dispatched to this component
//! - [`update`] — pure state transition
//! - [`view`] — renders the panel for the given torrent

use iced::widget::{Space, button};
use iced::widget::{column, container, progress_bar, row, scrollable, text};
use iced::{Element, Length, Task};

use crate::format::{format_ago, format_eta, format_size, format_speed};
use crate::rpc::TorrentData;

// ── ActiveTab ─────────────────────────────────────────────────────────────────

/// The currently visible inspector tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ActiveTab {
    #[default]
    General,
    Files,
    Trackers,
    Peers,
}

// ── Message ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    TabSelected(ActiveTab),
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct InspectorScreen {
    pub active_tab: ActiveTab,
}

impl InspectorScreen {
    pub fn new() -> Self {
        Self::default()
    }
}

// ── Elm functions ─────────────────────────────────────────────────────────────

pub fn update(state: &mut InspectorScreen, msg: Message) -> Task<Message> {
    match msg {
        Message::TabSelected(tab) => {
            state.active_tab = tab;
            Task::none()
        }
    }
}

pub fn view<'a>(state: &InspectorScreen, torrent: &'a TorrentData) -> Element<'a, Message> {
    // ── Material tab bar ──────────────────────────────────────────────────────
    // Each tab is a plain button (transparent background). The active tab gets
    // primary-color text; inactive tabs use muted text. A 2 px underline bar
    // is stacked beneath the active tab label.

    let tabs: &[(ActiveTab, &str)] = &[
        (ActiveTab::General, "General"),
        (ActiveTab::Files, "Files"),
        (ActiveTab::Trackers, "Trackers"),
        (ActiveTab::Peers, "Peers"),
    ];

    let tab_buttons: Vec<Element<'_, Message>> = tabs
        .iter()
        .map(|(tab, label)| {
            let is_active = state.active_tab == *tab;
            let btn = button(text(*label).size(13).width(Length::Fill))
                .on_press(Message::TabSelected(*tab))
                .style(if is_active {
                    crate::theme::tab_active
                } else {
                    crate::theme::tab_inactive
                })
                .width(Length::Fill)
                .padding([6, 16]);

            let col: Element<'_, Message> = if is_active {
                // Stack the button with a 2 px underline below it.
                column![
                    btn,
                    container(Space::new().width(Length::Fill).height(2.0))
                        .style(crate::theme::tab_underline)
                        .width(Length::Fill),
                ]
                .width(Length::Fill)
                .into()
            } else {
                column![btn, Space::new().width(Length::Fill).height(2.0),]
                    .width(Length::Fill)
                    .into()
            };

            container(col).width(Length::FillPortion(1)).into()
        })
        .collect();

    let tab_bar = row(tab_buttons).spacing(0);

    let content = match state.active_tab {
        ActiveTab::General => view_general(torrent),
        ActiveTab::Files => view_files(torrent),
        ActiveTab::Trackers => view_trackers(torrent),
        ActiveTab::Peers => view_peers(torrent),
    };

    container(
        column![tab_bar, content]
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .style(crate::theme::inspector_surface)
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn tab_content_wrap<'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

// ── General tab ───────────────────────────────────────────────────────────────

fn view_general(torrent: &TorrentData) -> Element<'_, Message> {
    let ratio_str = if torrent.upload_ratio < 0.0 {
        "—".to_owned()
    } else {
        format!("{:.2}", torrent.upload_ratio)
    };

    tab_content_wrap(
        scrollable(
            container(
                column![
                    info_row("Name", torrent.name.clone()),
                    info_row("Total Size", format_size(torrent.total_size)),
                    info_row("Downloaded", format_size(torrent.downloaded_ever)),
                    info_row("Uploaded", format_size(torrent.uploaded_ever)),
                    info_row("Ratio", ratio_str),
                    info_row("ETA", format_eta(torrent.eta)),
                    info_row("Download Speed", format_speed(torrent.rate_download)),
                    info_row("Upload Speed", format_speed(torrent.rate_upload)),
                ]
                .spacing(4),
            )
            .padding([8, 16])
            .width(Length::Fill),
        )
        .into(),
    )
}

fn info_row<'a>(label: &'a str, value: impl ToString) -> Element<'a, Message> {
    row![
        text(label.to_owned())
            .width(120)
            .style(|t: &iced::Theme| iced::widget::text::Style {
                color: Some(t.palette().text.scale_alpha(0.65)),
            }),
        text(value.to_string()),
    ]
    .spacing(16)
    .into()
}

// ── Files tab ─────────────────────────────────────────────────────────────────

fn view_files(torrent: &TorrentData) -> Element<'_, Message> {
    if torrent.files.is_empty() {
        return tab_content_wrap(text("No file information available.").into());
    }

    let rows: Vec<Element<'_, Message>> = torrent
        .files
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let completed = torrent
                .file_stats
                .get(i)
                .map(|s| s.bytes_completed)
                .unwrap_or(0);
            let progress = if f.length == 0 {
                1.0f32
            } else {
                (completed as f32 / f.length as f32).clamp(0.0, 1.0)
            };
            row![
                text(&f.name).width(Length::Fill),
                progress_bar(0.0..=1.0, progress)
                    .length(Length::FillPortion(1))
                    .girth(10.0),
            ]
            .spacing(8)
            .into()
        })
        .collect();

    tab_content_wrap(
        scrollable(
            container(column(rows).spacing(2))
                .padding([8, 16])
                .width(Length::Fill),
        )
        .into(),
    )
}

// ── Trackers tab ──────────────────────────────────────────────────────────────

fn view_trackers(torrent: &TorrentData) -> Element<'_, Message> {
    if torrent.tracker_stats.is_empty() {
        return tab_content_wrap(text("No tracker information available.").into());
    }

    let rows: Vec<Element<'_, Message>> = torrent
        .tracker_stats
        .iter()
        .map(|t| {
            let seeders = if t.seeder_count < 0 {
                "—".to_owned()
            } else {
                t.seeder_count.to_string()
            };
            let leechers = if t.leecher_count < 0 {
                "—".to_owned()
            } else {
                t.leecher_count.to_string()
            };
            column![
                text(&t.host).wrapping(text::Wrapping::WordOrGlyph),
                row![
                    text(format!("↑{seeders}")).width(80),
                    text(format!("↓{leechers}")).width(80),
                    text(format_ago(t.last_announce_time)),
                ]
                .spacing(8),
            ]
            .spacing(2)
            .into()
        })
        .collect();

    tab_content_wrap(
        scrollable(
            container(column(rows).spacing(6))
                .padding([8, 16])
                .width(Length::Fill),
        )
        .into(),
    )
}

// ── Peers tab ─────────────────────────────────────────────────────────────────

fn view_peers(torrent: &TorrentData) -> Element<'_, Message> {
    let inactive_count = torrent
        .peers
        .iter()
        .filter(|p| p.rate_to_client == 0 && p.rate_to_peer == 0)
        .count();

    let mut active: Vec<_> = torrent
        .peers
        .iter()
        .filter(|p| p.rate_to_client > 0 || p.rate_to_peer > 0)
        .collect();

    if active.is_empty() {
        let msg = if torrent.peers.is_empty() {
            "No peers connected.".to_owned()
        } else {
            format!("{inactive_count} inactive peer(s).")
        };
        return tab_content_wrap(
            container(text(msg))
                .padding([8, 16])
                .width(Length::Fill)
                .into(),
        );
    }

    active.sort_by(|a, b| {
        b.rate_to_client
            .cmp(&a.rate_to_client)
            .then(b.rate_to_peer.cmp(&a.rate_to_peer))
    });

    let rows: Vec<Element<'_, Message>> = active
        .iter()
        .map(|p| {
            column![
                text(&p.address).wrapping(text::Wrapping::WordOrGlyph),
                row![
                    text(format!("↓ {}", format_speed(p.rate_to_client))).width(Length::Fill),
                    text(format!("↑ {}", format_speed(p.rate_to_peer))).width(Length::Fill),
                ]
                .spacing(4),
            ]
            .spacing(2)
            .into()
        })
        .collect();

    let footer: Element<'_, Message> = if inactive_count > 0 {
        text(format!("{inactive_count} inactive peer(s) not shown."))
            .style(|t: &iced::Theme| iced::widget::text::Style {
                color: Some(t.palette().text.scale_alpha(0.5)),
            })
            .into()
    } else {
        text("").into()
    };

    tab_content_wrap(
        column![
            scrollable(
                container(column(rows).spacing(6))
                    .padding([8, 16])
                    .width(Length::Fill)
            ),
            container(footer).padding([0, 16])
        ]
        .spacing(4)
        .into(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 12.2 – TabSelected updates active_tab.
    #[test]
    fn tab_selected_updates_active() {
        let mut screen = InspectorScreen::new();
        assert_eq!(screen.active_tab, ActiveTab::General);

        let _ = update(&mut screen, Message::TabSelected(ActiveTab::Files));
        assert_eq!(screen.active_tab, ActiveTab::Files);

        let _ = update(&mut screen, Message::TabSelected(ActiveTab::Trackers));
        assert_eq!(screen.active_tab, ActiveTab::Trackers);

        let _ = update(&mut screen, Message::TabSelected(ActiveTab::Peers));
        assert_eq!(screen.active_tab, ActiveTab::Peers);

        let _ = update(&mut screen, Message::TabSelected(ActiveTab::General));
        assert_eq!(screen.active_tab, ActiveTab::General);
    }
}
