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

//! View rendering for the inspector panel tabs.

use iced::widget::{Space, column, container, progress_bar, row, scrollable, text};
use iced::{Element, Length};

use crate::format::{format_ago, format_eta, format_size, format_speed};
use crate::rpc::TorrentData;

use super::{
    ActiveTab, InspectorBulkOptionsState, InspectorOptionsState, InspectorScreen, Message,
};

/// Render the inspector panel.
///
/// - `torrent`: the single selected torrent data, or `None` when the selection
///   is empty or in multi-select bulk-edit mode.
/// - `selected_count`: the total number of selected torrents (0, 1, or >1).
pub fn view<'a>(
    state: &'a InspectorScreen,
    torrent: Option<&'a TorrentData>,
    selected_count: usize,
) -> Element<'a, Message> {
    // In bulk mode the tab bar is always shown, but only the Options tab is
    // interactive — clicking any other tab is a no-op (mapped to Options).
    let is_bulk = selected_count > 1;

    let tab_bar = crate::theme::segmented_control(
        &[
            ("General", ActiveTab::General),
            ("Files", ActiveTab::Files),
            ("Trackers", ActiveTab::Trackers),
            ("Peers", ActiveTab::Peers),
            ("Options", ActiveTab::Options),
        ],
        if is_bulk {
            ActiveTab::Options
        } else {
            state.active_tab
        },
        move |tab| {
            if is_bulk && tab != ActiveTab::Options {
                // Absorb clicks on disabled tabs in bulk mode.
                Message::TabSelected(ActiveTab::Options)
            } else {
                Message::TabSelected(tab)
            }
        },
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

    let content: Element<'_, Message> = if is_bulk {
        let subtitle: Element<'_, Message> = container(
            text("Editing options for multiple selected torrents")
                .size(12)
                .style(|t: &iced::Theme| iced::widget::text::Style {
                    color: Some(t.palette().text.scale_alpha(0.55)),
                }),
        )
        .padding(iced::Padding {
            top: 4.0,
            right: 16.0,
            bottom: 0.0,
            left: 16.0,
        })
        .into();
        column![subtitle, view_bulk_options(&state.bulk_options)]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        match state.active_tab {
            ActiveTab::General => torrent.map(view_general).unwrap_or_else(view_empty),
            ActiveTab::Files => torrent
                .map(|t| view_files(state, t))
                .unwrap_or_else(view_empty),
            ActiveTab::Trackers => torrent.map(view_trackers).unwrap_or_else(view_empty),
            ActiveTab::Peers => torrent.map(view_peers).unwrap_or_else(view_empty),
            ActiveTab::Options => view_options(&state.options),
        }
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

/// Placeholder panel shown when no torrent tab data is available.
fn view_empty<'a>() -> Element<'a, Message> {
    tab_content_wrap(
        container(text("No torrent selected.").size(14))
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into(),
    )
}

// ── General tab ───────────────────────────────────────────────────────────────

fn view_general(torrent: &TorrentData) -> Element<'_, Message> {
    let ratio_str = if torrent.upload_ratio < 0.0 {
        "—".to_owned()
    } else {
        format!("{:.2}", torrent.upload_ratio)
    };

    let error_str = if torrent.error == 0 {
        "none".to_owned()
    } else if torrent.error_string.is_empty() {
        format!("error {}", torrent.error)
    } else {
        torrent.error_string.clone()
    };

    let col1 = column![
        info_row("Total Size", format_size(torrent.total_size)),
        info_row("Downloaded", format_size(torrent.downloaded_ever)),
        info_row("Uploaded", format_size(torrent.uploaded_ever)),
        info_row("Ratio", ratio_str),
        info_row("Error", error_str),
    ]
    .spacing(4)
    .width(Length::FillPortion(1));

    let col2 = column![
        info_row("ETA", format_eta(torrent.eta)),
        info_row("Download Speed", format_speed(torrent.rate_download)),
        info_row("Upload Speed", format_speed(torrent.rate_upload)),
        info_row("Data Path", torrent.download_dir.clone()),
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

// ── Options tab ───────────────────────────────────────────────────────────────

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

// ── Bulk Options tab ──────────────────────────────────────────────────────────

fn view_bulk_options(opts: &InspectorBulkOptionsState) -> Element<'_, Message> {
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

    let dl_row = row![
        toggler(opts.download_limited.unwrap_or(false))
            .on_toggle(Message::BulkDownloadLimitToggled)
            .width(Length::Shrink),
        tog_gap(),
        text("Limit Download (KB/s)").width(Length::Fill),
        text_input("", &opts.download_limit_val)
            .on_input(Message::BulkDownloadLimitChanged)
            .on_submit(Message::BulkDownloadLimitSubmitted)
            .width(field_w)
            .padding([10, 14])
            .style(crate::theme::m3_text_input),
    ]
    .align_y(iced::Center);

    let ul_row = row![
        toggler(opts.upload_limited.unwrap_or(false))
            .on_toggle(Message::BulkUploadLimitToggled)
            .width(Length::Shrink),
        tog_gap(),
        text("Limit Upload (KB/s)").width(Length::Fill),
        text_input("", &opts.upload_limit_val)
            .on_input(Message::BulkUploadLimitChanged)
            .on_submit(Message::BulkUploadLimitSubmitted)
            .width(field_w)
            .padding([10, 14])
            .style(crate::theme::m3_text_input),
    ]
    .align_y(iced::Center);

    let honor_row = row![
        toggler(opts.honors_session_limits.unwrap_or(false))
            .on_toggle(Message::BulkHonorGlobalToggled)
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

    let ratio_mode_val = opts.ratio_mode.unwrap_or(0);
    let ratio_ctrl = crate::theme::segmented_control(
        &[("Global", 0_u8), ("Custom", 1_u8), ("Unlimited", 2_u8)],
        ratio_mode_val,
        Message::BulkRatioModeChanged,
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

    if ratio_mode_val == 1 {
        let custom_row = row![
            text("Custom ratio").width(Length::Fill),
            text_input("ratio", &opts.ratio_limit_val)
                .on_input(Message::BulkRatioLimitChanged)
                .on_submit(Message::BulkRatioLimitSubmitted)
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
