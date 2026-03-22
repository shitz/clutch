//! Detail inspector sub-screen.
//!
//! Displays per-torrent detail in a tabbed panel. `view()` accepts an
//! immutable reference to the currently selected `TorrentData` — all data
//! arrives via the polling subscription; the inspector owns no RPC state.
//!
//! # Architecture
//!
//! This module is a self-contained Elm component:
//! - [`InspectorScreen`] — state (active tab only)
//! - [`Message`] — messages that can be dispatched to this component
//! - [`update`] — pure state transition
//! - [`view`] — renders the panel for the given torrent

use iced::widget::{button, column, container, progress_bar, row, scrollable, text};
use iced::{Element, Length, Task};

use crate::rpc::TorrentData;

// ── ActiveTab ─────────────────────────────────────────────────────────────────

/// The currently visible inspector tab.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
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
    let tab_bar = row![
        tab_button("General", ActiveTab::General, state.active_tab),
        tab_button("Files", ActiveTab::Files, state.active_tab),
        tab_button("Trackers", ActiveTab::Trackers, state.active_tab),
        tab_button("Peers", ActiveTab::Peers, state.active_tab),
    ]
    .spacing(4);

    let content: Element<'a, Message> = match state.active_tab {
        ActiveTab::General => view_general(torrent),
        ActiveTab::Files => view_files(torrent),
        ActiveTab::Trackers => view_trackers(torrent),
        ActiveTab::Peers => view_peers(torrent),
    };

    container(
        column![tab_bar, content]
            .spacing(8)
            .padding(8)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn tab_button(label: &'static str, tab: ActiveTab, active: ActiveTab) -> Element<'static, Message> {
    let btn = button(text(label));
    let btn = if tab == active {
        btn.style(iced::widget::button::primary)
    } else {
        btn.style(iced::widget::button::secondary)
    };
    btn.on_press(Message::TabSelected(tab)).into()
}

// ── General tab ───────────────────────────────────────────────────────────────

fn view_general(torrent: &TorrentData) -> Element<'_, Message> {
    let ratio_str = if torrent.upload_ratio < 0.0 {
        "—".to_owned()
    } else {
        format!("{:.2}", torrent.upload_ratio)
    };

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
    .spacing(4)
    .into()
}

fn info_row(label: &'static str, value: String) -> Element<'static, Message> {
    row![
        text(label).width(Length::FillPortion(2)),
        text(value).width(Length::FillPortion(3)),
    ]
    .into()
}

// ── Files tab ─────────────────────────────────────────────────────────────────

fn view_files(torrent: &TorrentData) -> Element<'_, Message> {
    if torrent.files.is_empty() {
        return text("No file information available.").into();
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

    scrollable(column(rows).spacing(2)).into()
}

// ── Trackers tab ──────────────────────────────────────────────────────────────

fn view_trackers(torrent: &TorrentData) -> Element<'_, Message> {
    if torrent.tracker_stats.is_empty() {
        return text("No tracker information available.").into();
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

    scrollable(column(rows).spacing(6)).into()
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
        return text(msg).into();
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

    column![scrollable(column(rows).spacing(6)), footer,]
        .spacing(4)
        .into()
}

// ── Formatting helpers ────────────────────────────────────────────────────────

/// Format a byte count as a human-readable string (e.g. `"1.00 GiB"`).
///
/// Returns `"—"` for negative values (sentinel for unavailable).
fn format_size(bytes: i64) -> String {
    if bytes < 0 {
        return "—".to_owned();
    }
    let bytes = bytes as u64;
    const GIB: u64 = 1 << 30;
    const MIB: u64 = 1 << 20;
    const KIB: u64 = 1 << 10;
    if bytes >= GIB {
        format!("{:.2} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.2} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.2} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Format a bytes-per-second rate (e.g. `"1.00 MiB/s"`).
fn format_speed(bps: i64) -> String {
    if bps < 0 {
        return "—".to_owned();
    }
    let bps = bps as u64;
    const MIB: u64 = 1 << 20;
    const KIB: u64 = 1 << 10;
    if bps >= MIB {
        format!("{:.2} MiB/s", bps as f64 / MIB as f64)
    } else if bps >= KIB {
        format!("{:.2} KiB/s", bps as f64 / KIB as f64)
    } else {
        format!("{bps} B/s")
    }
}

/// Format an ETA in seconds to a human-readable duration string.
///
/// Returns `"—"` when `secs` is negative (Transmission sentinel for unknown).
fn format_eta(secs: i64) -> String {
    if secs < 0 {
        return "—".to_owned();
    }
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        let m = secs / 60;
        let s = secs % 60;
        format!("{m}m {s}s")
    } else {
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        format!("{h}h {m}m")
    }
}

/// Format a Unix timestamp as a relative "time ago" string.
fn format_ago(unix_secs: i64) -> String {
    if unix_secs <= 0 {
        return "Never".to_owned();
    }
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let diff = now.saturating_sub(unix_secs);
    if diff < 60 {
        format!("{diff}s ago")
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else {
        format!("{}d ago", diff / 86400)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_size_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn format_size_kib() {
        assert_eq!(format_size(1024), "1.00 KiB");
        assert_eq!(format_size(2048), "2.00 KiB");
    }

    #[test]
    fn format_size_mib() {
        assert_eq!(format_size(1 << 20), "1.00 MiB");
    }

    #[test]
    fn format_size_gib() {
        assert_eq!(format_size(1 << 30), "1.00 GiB");
    }

    #[test]
    fn format_size_negative_sentinel() {
        assert_eq!(format_size(-1), "—");
    }

    #[test]
    fn format_speed_bps() {
        assert_eq!(format_speed(0), "0 B/s");
        assert_eq!(format_speed(512), "512 B/s");
    }

    #[test]
    fn format_speed_kibps() {
        assert_eq!(format_speed(1024), "1.00 KiB/s");
    }

    #[test]
    fn format_speed_mibps() {
        assert_eq!(format_speed(1 << 20), "1.00 MiB/s");
    }

    #[test]
    fn format_eta_unknown() {
        assert_eq!(format_eta(-1), "—");
    }

    #[test]
    fn format_eta_zero() {
        assert_eq!(format_eta(0), "0s");
    }

    #[test]
    fn format_eta_seconds() {
        assert_eq!(format_eta(45), "45s");
    }

    #[test]
    fn format_eta_minutes() {
        assert_eq!(format_eta(90), "1m 30s");
        assert_eq!(format_eta(3599), "59m 59s");
    }

    #[test]
    fn format_eta_hours() {
        assert_eq!(format_eta(3600), "1h 0m");
        assert_eq!(format_eta(7200), "2h 0m");
    }

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
