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

use iced::widget::{Space, column, container, progress_bar, row, scrollable, text};
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
    Options,
}

// ── Message ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    TabSelected(ActiveTab),
    FileWantedToggled {
        torrent_id: i64,
        file_index: usize,
        wanted: bool,
    },
    AllFilesWantedToggled {
        torrent_id: i64,
        file_count: usize,
        wanted: bool,
    },
    /// Emitted when the SetFileWanted RPC completes (success or failure).
    /// Removes the given indices from `pending_wanted`.
    FileWantedSetSuccess {
        indices: Vec<usize>,
    },
    // ── Options tab messages ──────────────────────────────────────────────
    OptionsDownloadLimitToggled(bool),
    OptionsDownloadLimitChanged(String),
    OptionsDownloadLimitSubmitted,
    OptionsUploadLimitToggled(bool),
    OptionsUploadLimitChanged(String),
    OptionsUploadLimitSubmitted,
    OptionsRatioModeChanged(u8),
    OptionsRatioLimitChanged(String),
    OptionsRatioLimitSubmitted,
    OptionsHonorGlobalToggled(bool),
}

// ── InspectorOptionsState ────────────────────────────────────────────────────

/// Local draft for the per-torrent Options tab.
/// Reset whenever a new torrent is selected.
#[derive(Debug, Default, Clone)]
pub struct InspectorOptionsState {
    pub download_limited: bool,
    pub download_limit_val: String,
    pub upload_limited: bool,
    pub upload_limit_val: String,
    /// 0 = Global, 1 = Custom, 2 = Unlimited
    pub ratio_mode: u8,
    pub ratio_limit_val: String,
    pub honors_session_limits: bool,
}

impl InspectorOptionsState {
    /// Populate from fresh torrent data.
    pub fn from_torrent(t: &TorrentData) -> Self {
        Self {
            download_limited: t.download_limited,
            download_limit_val: t.download_limit.to_string(),
            upload_limited: t.upload_limited,
            upload_limit_val: t.upload_limit.to_string(),
            ratio_mode: t.seed_ratio_mode,
            ratio_limit_val: format!("{:.2}", t.seed_ratio_limit),
            honors_session_limits: t.honors_session_limits,
        }
    }
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct InspectorScreen {
    pub active_tab: ActiveTab,
    /// Optimistic file-wanted overrides keyed by file index.
    /// Entries are inserted when the user toggles a checkbox and removed
    /// when the corresponding `torrent-set` RPC completes (or fails).
    pub pending_wanted: std::collections::HashMap<usize, bool>,
    /// Draft state for the Options tab.
    pub options: InspectorOptionsState,
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
        Message::FileWantedToggled {
            file_index, wanted, ..
        } => {
            state.pending_wanted.insert(file_index, wanted);
            Task::none()
        }
        Message::AllFilesWantedToggled {
            file_count, wanted, ..
        } => {
            for i in 0..file_count {
                state.pending_wanted.insert(i, wanted);
            }
            Task::none()
        }
        Message::FileWantedSetSuccess { indices } => {
            for i in &indices {
                state.pending_wanted.remove(i);
            }
            Task::none()
        }
        // ── Options tab ───────────────────────────────────────────────────────
        Message::OptionsDownloadLimitToggled(v) => {
            state.options.download_limited = v;
            Task::none()
        }
        Message::OptionsDownloadLimitChanged(v) => {
            if v.is_empty() || v.chars().all(|c| c.is_ascii_digit()) {
                state.options.download_limit_val = v;
            }
            Task::none()
        }
        Message::OptionsUploadLimitToggled(v) => {
            state.options.upload_limited = v;
            Task::none()
        }
        Message::OptionsUploadLimitChanged(v) => {
            if v.is_empty() || v.chars().all(|c| c.is_ascii_digit()) {
                state.options.upload_limit_val = v;
            }
            Task::none()
        }
        Message::OptionsRatioModeChanged(v) => {
            state.options.ratio_mode = v;
            Task::none()
        }
        Message::OptionsRatioLimitChanged(v) => {
            // Ratio allows digits and at most one decimal point.
            let dot_count = v.chars().filter(|c| *c == '.').count();
            if v.is_empty() || (v.chars().all(|c| c.is_ascii_digit() || c == '.') && dot_count <= 1)
            {
                state.options.ratio_limit_val = v;
            }
            Task::none()
        }
        Message::OptionsHonorGlobalToggled(v) => {
            state.options.honors_session_limits = v;
            Task::none()
        }
        // Submit messages are intercepted by main_screen; nothing to update here.
        Message::OptionsDownloadLimitSubmitted
        | Message::OptionsUploadLimitSubmitted
        | Message::OptionsRatioLimitSubmitted => Task::none(),
    }
}

pub fn view<'a>(state: &'a InspectorScreen, torrent: &'a TorrentData) -> Element<'a, Message> {
    // ── Material tab bar ──────────────────────────────────────────────────────
    // Each tab is a plain button (transparent background). The active tab gets
    // primary-color text; inactive tabs use muted text. A 2 px underline bar
    // is stacked beneath the active tab label.

    let tab_bar = crate::theme::segmented_control(
        &[
            ("General", ActiveTab::General),
            ("Files", ActiveTab::Files),
            ("Trackers", ActiveTab::Trackers),
            ("Peers", ActiveTab::Peers),
            ("Options", ActiveTab::Options),
        ],
        state.active_tab,
        Message::TabSelected,
        true,
        true,
    );

    // Center the tab bar: equal Space on both sides so it sits in the middle.
    let tab_row: Element<'_, Message> = container(
        row![
            Space::new().width(Length::FillPortion(1)),
            container(tab_bar).width(Length::FillPortion(2)),
            Space::new().width(Length::FillPortion(1)),
        ]
        .spacing(0),
    )
    .padding(iced::Padding {
        top: 6.0,
        right: 0.0,
        bottom: 0.0,
        left: 0.0,
    })
    .into();

    let content = match state.active_tab {
        ActiveTab::General => view_general(torrent),
        ActiveTab::Files => view_files(state, torrent),
        ActiveTab::Trackers => view_trackers(torrent),
        ActiveTab::Peers => view_peers(torrent),
        ActiveTab::Options => view_options(&state.options),
    };

    container(
        column![tab_row, content]
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

    let col1 = column![
        info_row("Total Size", format_size(torrent.total_size)),
        info_row("Downloaded", format_size(torrent.downloaded_ever)),
        info_row("Uploaded", format_size(torrent.uploaded_ever)),
        info_row("Ratio", ratio_str),
    ]
    .spacing(4)
    .width(Length::FillPortion(1));

    let col2 = column![
        info_row("ETA", format_eta(torrent.eta)),
        info_row("Download Speed", format_speed(torrent.rate_download)),
        info_row("Upload Speed", format_speed(torrent.rate_upload)),
    ]
    .spacing(4)
    .width(Length::FillPortion(1));

    tab_content_wrap(
        scrollable(
            container(
                column![
                    info_row("Name", torrent.name.clone()),
                    row![col1, col2].spacing(16),
                ]
                .spacing(8),
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

fn view_files<'a>(state: &'a InspectorScreen, torrent: &'a TorrentData) -> Element<'a, Message> {
    if torrent.files.is_empty() {
        return tab_content_wrap(
            container(text("No file information available."))
                .padding([8, 16])
                .into(),
        );
    }

    let torrent_id = torrent.id;
    let file_count = torrent.files.len();

    let effective_wanted: Vec<bool> = (0..file_count)
        .map(|i| {
            state
                .pending_wanted
                .get(&i)
                .copied()
                .unwrap_or_else(|| torrent.file_stats.get(i).map(|s| s.wanted).unwrap_or(true))
        })
        .collect();

    let wanted_count = effective_wanted.iter().filter(|&&v| v).count();
    let aggregate = if wanted_count == file_count {
        crate::theme::CheckState::Checked
    } else if wanted_count == 0 {
        crate::theme::CheckState::Unchecked
    } else {
        crate::theme::CheckState::Mixed
    };

    let header =
        crate::theme::m3_tristate_checkbox(aggregate, "Select All", move |next| match next {
            crate::theme::CheckState::Unchecked => Message::AllFilesWantedToggled {
                torrent_id,
                file_count,
                wanted: false,
            },
            _ => Message::AllFilesWantedToggled {
                torrent_id,
                file_count,
                wanted: true,
            },
        });

    let rows: Vec<Element<'a, Message>> = torrent
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
            let is_wanted = effective_wanted.get(i).copied().unwrap_or(true);
            row![
                crate::theme::m3_checkbox(is_wanted, "", move |wanted| {
                    Message::FileWantedToggled {
                        torrent_id,
                        file_index: i,
                        wanted,
                    }
                }),
                text(&f.name).width(Length::Fill),
                progress_bar(0.0..=1.0, progress)
                    .length(Length::FillPortion(1))
                    .girth(10.0),
            ]
            .spacing(8)
            .align_y(iced::alignment::Vertical::Center)
            .into()
        })
        .collect();

    tab_content_wrap(
        scrollable(
            container(column![header, column(rows).spacing(2)].spacing(4))
                .padding([8, 16])
                .width(Length::Fill),
        )
        .into(),
    )
}

// ── Trackers tab ──────────────────────────────────────────────────────────────

fn view_trackers(torrent: &TorrentData) -> Element<'_, Message> {
    if torrent.tracker_stats.is_empty() {
        return tab_content_wrap(
            container(text("No tracker information available."))
                .padding([8, 16])
                .into(),
        );
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

fn view_options(opts: &InspectorOptionsState) -> Element<'_, Message> {
    use iced::widget::{text_input, toggler};

    let field_w = Length::Fixed(90.0);
    let tog_gap = || Space::new().width(Length::Fixed(8.0));
    let sub_label = |s: &'static str| {
        text(s)
            .size(12)
            .style(|t: &iced::Theme| iced::widget::text::Style {
                color: Some(t.palette().text.scale_alpha(0.45)),
            })
    };

    // ── Left card: Speed Limits + Honor Global ─────────────────────────────
    let dl_row = row![
        toggler(opts.download_limited)
            .on_toggle(Message::OptionsDownloadLimitToggled)
            .width(Length::Shrink),
        tog_gap(),
        text("Limit Download (KB/s)").width(Length::Fill),
        text_input("", &opts.download_limit_val)
            .on_input(Message::OptionsDownloadLimitChanged)
            .on_submit(Message::OptionsDownloadLimitSubmitted)
            .width(field_w)
            .padding([10, 14])
            .style(crate::theme::m3_text_input),
    ]
    .align_y(iced::Center);

    let ul_row = row![
        toggler(opts.upload_limited)
            .on_toggle(Message::OptionsUploadLimitToggled)
            .width(Length::Shrink),
        tog_gap(),
        text("Limit Upload (KB/s)").width(Length::Fill),
        text_input("", &opts.upload_limit_val)
            .on_input(Message::OptionsUploadLimitChanged)
            .on_submit(Message::OptionsUploadLimitSubmitted)
            .width(field_w)
            .padding([10, 14])
            .style(crate::theme::m3_text_input),
    ]
    .align_y(iced::Center);

    let honor_row = row![
        toggler(opts.honors_session_limits)
            .on_toggle(Message::OptionsHonorGlobalToggled)
            .width(Length::Shrink),
        tog_gap(),
        text("Honor Global Speed Limits").width(Length::Fill),
    ]
    .align_y(iced::Center);

    let left_card = container(
        scrollable(column![sub_label("Speed Limits"), dl_row, ul_row, honor_row,].spacing(12))
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(6).scroller_width(6),
            )),
    )
    .style(crate::theme::m3_card)
    .padding([12, 16])
    .width(Length::FillPortion(1));

    // ── Right card: Seeding Ratio (segmented + optional custom) ───────────────
    let ratio_ctrl = crate::theme::segmented_control(
        &[("Global", 0_u8), ("Custom", 1_u8), ("Unlimited", 2_u8)],
        opts.ratio_mode,
        Message::OptionsRatioModeChanged,
        true,
        true,
    );

    let mut right_items: Vec<Element<'_, Message>> = vec![
        text("Seeding Ratio")
            .size(12)
            .style(|t: &iced::Theme| iced::widget::text::Style {
                color: Some(t.palette().text.scale_alpha(0.45)),
            })
            .into(),
        ratio_ctrl,
    ];

    if opts.ratio_mode == 1 {
        let custom_row = row![
            text("Custom ratio").width(Length::Fill),
            text_input("ratio", &opts.ratio_limit_val)
                .on_input(Message::OptionsRatioLimitChanged)
                .on_submit(Message::OptionsRatioLimitSubmitted)
                .width(field_w)
                .padding([10, 14])
                .style(crate::theme::m3_text_input),
        ]
        .align_y(iced::Center);
        right_items.push(custom_row.into());
    }

    let right_card = container(iced::widget::column(right_items).spacing(12))
        .style(crate::theme::m3_card)
        .padding([12, 16])
        .width(Length::FillPortion(1));

    container(row![left_card, right_card].spacing(12).padding([12, 16]))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
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

    // ── 5.5-5.8 – Selective file download (inspector) ────────────────────────

    /// 5.5 – FileWantedToggled inserts the toggled index into `pending_wanted`.
    #[test]
    fn file_wanted_toggled_updates_pending() {
        let mut screen = InspectorScreen::new();
        let _ = update(
            &mut screen,
            Message::FileWantedToggled {
                torrent_id: 1,
                file_index: 2,
                wanted: false,
            },
        );
        assert_eq!(screen.pending_wanted.get(&2), Some(&false));
        assert!(!screen.pending_wanted.contains_key(&0));
    }

    /// 5.6 – FileWantedSetSuccess removes only the specified indices, leaving others intact.
    #[test]
    fn file_wanted_set_success_clears_only_specified_indices() {
        let mut screen = InspectorScreen::new();
        // Seed several pending entries.
        screen.pending_wanted.insert(0, true);
        screen.pending_wanted.insert(1, false);
        screen.pending_wanted.insert(2, true);

        let _ = update(
            &mut screen,
            Message::FileWantedSetSuccess {
                indices: vec![0, 2],
            },
        );

        assert!(
            !screen.pending_wanted.contains_key(&0),
            "index 0 should be cleared"
        );
        assert!(
            !screen.pending_wanted.contains_key(&2),
            "index 2 should be cleared"
        );
        assert_eq!(
            screen.pending_wanted.get(&1),
            Some(&false),
            "index 1 must remain untouched"
        );
    }

    /// 5.7 – AllFilesWantedToggled inserts all indices into `pending_wanted`.
    #[test]
    fn all_files_wanted_toggled_populates_all_indices() {
        let mut screen = InspectorScreen::new();
        let _ = update(
            &mut screen,
            Message::AllFilesWantedToggled {
                torrent_id: 7,
                file_count: 4,
                wanted: false,
            },
        );
        for i in 0..4 {
            assert_eq!(
                screen.pending_wanted.get(&i),
                Some(&false),
                "index {i} should be set to false"
            );
        }
    }

    /// 5.8 – The inspector `Message` enum has no variant that would clear
    /// `pending_wanted` on a background poll. Verified structurally: calling
    /// `TabSelected` (a non-file message) leaves `pending_wanted` unchanged.
    #[test]
    fn poll_does_not_clear_pending_wanted() {
        let mut screen = InspectorScreen::new();
        screen.pending_wanted.insert(5, true);

        // Simulate any non-file-wanted message arriving (e.g. from a background poll path).
        let _ = update(&mut screen, Message::TabSelected(ActiveTab::Files));

        assert_eq!(
            screen.pending_wanted.get(&5),
            Some(&true),
            "pending_wanted must not be cleared by unrelated messages"
        );
    }
}
