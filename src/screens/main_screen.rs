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

//! `MainScreen` is the parent Elm component that owns two children:
//! - [`torrent_list::TorrentListScreen`] — the list, toolbar, and add dialog
//! - [`inspector::InspectorScreen`] — the tabbed detail panel
//!
//! # Layout
//!
//! When no torrent is selected the list fills the full width.
//! When a torrent is selected the content area is split horizontally:
//!   - list:      `FillPortion(3)` (top 3/4)
//!   - inspector: `FillPortion(1)` (bottom 1/4)
//!
//! # Message routing
//!
//! Child messages are wrapped:
//! - `Message::List(TorrentListMessage)` -> delegated to `torrent_list::update`
//! - `Message::Inspector(InspectorMessage)` -> delegated to `inspector::update`
//! - `Message::Disconnect` -> escalated to `app::update`
//! - `Message::OpenSettingsClicked` -> escalated to `app::update`

use std::time::Duration;

use iced::widget::{column, container};
use iced::{Element, Length, Subscription, Task};

use crate::app::ThemeMode;
use crate::rpc::{RpcWork, SessionData, TorrentBandwidthArgs, TransmissionCredentials};
use crate::screens::inspector::{self, InspectorOptionsState, InspectorScreen};
use crate::screens::torrent_list::{self, TorrentListScreen};

// -- Message ------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum Message {
    List(torrent_list::Message),
    Inspector(inspector::Message),
    /// Escalated from `List(Disconnect)` — handled by `app::update`.
    Disconnect,
    /// Settings gear icon — handled by `app::update`.
    OpenSettingsClicked,
    /// Toolbar turtle-mode button pressed — escalated to `app::update`.
    TurtleModeToggled,
    /// Fresh session data from a periodic `session-get` poll — escalated to `app::update`.
    SessionDataLoaded(SessionData),
}

// -- State --------------------------------------------------------------------

#[derive(Debug)]
pub struct MainScreen {
    pub list: TorrentListScreen,
    pub inspector: InspectorScreen,
    /// Host:port label used in the loading splash.
    pub connect_label: String,
    /// Optional profile name shown in the loading splash (None for quick connect).
    pub profile_name: Option<String>,
    /// Daemon poll interval in seconds, from GeneralSettings.
    pub refresh_interval: u8,
    /// Counts torrent-poll ticks to drive the periodic `session-get` refresh.
    session_poll_ticks: u8,
}

impl MainScreen {
    /// Construct with explicit connect label and optional profile name for the
    /// loading splash that is shown while the first torrent list is fetched.
    pub fn new_with_label(
        credentials: TransmissionCredentials,
        session_id: String,
        profile_name: Option<String>,
        _active_id: Option<uuid::Uuid>,
        refresh_interval: u8,
    ) -> Self {
        let connect_label = format!("{}:{}", credentials.host, credentials.port);
        MainScreen {
            list: TorrentListScreen::new(credentials, session_id),
            inspector: InspectorScreen::new(),
            connect_label,
            profile_name,
            refresh_interval,
            session_poll_ticks: 0,
        }
    }

    /// Subscription: tick at the configured refresh interval + serialized RPC worker
    /// + conditional dialog keyboard handler.
    pub fn subscription(&self) -> Subscription<Message> {
        let interval = Duration::from_secs(self.refresh_interval.max(1) as u64);
        let tick = iced::time::every(interval).map(|_| Message::List(torrent_list::Message::Tick));
        let worker = Subscription::run(torrent_list::rpc_worker_stream).map(Message::List);
        let dialog_kb = self.list.dialog_subscription().map(Message::List);
        let cursor = self.list.cursor_subscription().map(Message::List);
        let modifiers =
            torrent_list::TorrentListScreen::modifiers_subscription().map(Message::List);
        Subscription::batch([tick, worker, dialog_kb, cursor, modifiers])
    }

    /// Route messages to the appropriate child; intercept cross-cutting concerns.
    pub fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            // Intercept Disconnect before it reaches the list.
            Message::List(torrent_list::Message::Disconnect) => Task::done(Message::Disconnect),

            // Settings message bubbles up to app::update.
            Message::List(torrent_list::Message::OpenSettingsClicked) => {
                Task::done(Message::OpenSettingsClicked)
            }

            // Turtle-mode toggle button: bubble to app so it can do the session-set.
            Message::List(torrent_list::Message::TurtleModeToggled) => {
                Task::done(Message::TurtleModeToggled)
            }

            // Successful session-get response: bubble to app to update AppState.
            Message::List(torrent_list::Message::SessionDataLoaded(Ok(data))) => {
                Task::done(Message::SessionDataLoaded(data))
            }
            // Error on session-get poll: ignore silently (transient network blip).
            Message::List(torrent_list::Message::SessionDataLoaded(Err(e))) => {
                tracing::debug!(error = %e, "periodic session-get failed; ignoring");
                Task::none()
            }

            // Intercept each Tick to drive the periodic session-get poll.
            Message::List(torrent_list::Message::Tick) => {
                let threshold = (10_u8 / self.refresh_interval.max(1)).max(1);
                self.session_poll_ticks = self.session_poll_ticks.saturating_add(1);
                if self.session_poll_ticks >= threshold {
                    self.session_poll_ticks = 0;
                    self.list
                        .enqueue(RpcWork::SessionGet(self.list.params.clone()));
                }
                torrent_list::update(&mut self.list, torrent_list::Message::Tick).map(Message::List)
            }

            // Intercept TorrentSelected so we can reset the inspector tab.
            Message::List(torrent_list::Message::TorrentSelected(id)) => {
                let prev_single = self.list.selected_torrent().map(|t| t.id);
                let prev_count = self.list.selected_ids.len();
                let task = torrent_list::update(
                    &mut self.list,
                    torrent_list::Message::TorrentSelected(id),
                )
                .map(Message::List);
                let new_count = self.list.selected_ids.len();
                let new_single = self.list.selected_torrent().map(|t| t.id);

                if new_count == 1 && new_single != prev_single {
                    // Transitioned to a different single selection.
                    self.inspector.active_tab = inspector::ActiveTab::General;
                    self.inspector.pending_wanted.clear();
                    self.inspector.bulk_options = Default::default();
                    if let Some(torrent) = self.list.selected_torrent() {
                        self.inspector.options = InspectorOptionsState::from_torrent(torrent);
                    }
                } else if new_count > 1 && prev_count <= 1 {
                    // Transitioned into multi-select from none/single.
                    self.inspector.bulk_options = Default::default();
                }
                // A Ctrl/Cmd-click within an existing multi-select does NOT reset bulk_options.
                task
            }

            // Intercept FileWantedSettled to update pending_wanted in the inspector.
            // On success: trigger an immediate poll; pending entries are reconciled and
            // removed in the List catch-all once TorrentsUpdated confirms the new state.
            // On failure: revert the optimistic UI immediately.
            Message::List(torrent_list::Message::FileWantedSettled(success, indices)) => {
                if success {
                    self.list.enqueue_torrent_get();
                    Task::none()
                } else {
                    inspector::update(
                        &mut self.inspector,
                        inspector::Message::FileWantedSetSuccess { indices },
                    )
                    .map(Message::Inspector)
                }
            }

            // Intercept file-wanted toggles from the inspector to enqueue RPC work.
            Message::Inspector(inspector::Message::FileWantedToggled {
                torrent_id,
                file_index,
                wanted,
            }) => {
                self.inspector.pending_wanted.insert(file_index, wanted);
                self.list.enqueue(RpcWork::SetFileWanted {
                    params: self.list.params.clone(),
                    torrent_id,
                    file_indices: vec![file_index as i64],
                    wanted,
                });
                Task::none()
            }

            // Intercept Options toggle/submit messages to immediately apply each change via RPC.

            // Download limit toggle: update state + send download_limited + download_limit.
            Message::Inspector(inspector::Message::OptionsDownloadLimitToggled(v)) => {
                self.inspector.options.download_limited = v;
                let Some(torrent_id) = self.list.selected_torrent().map(|t| t.id) else {
                    return Task::none();
                };
                let dl_limit: u64 = self
                    .inspector
                    .options
                    .download_limit_val
                    .parse()
                    .unwrap_or(0);
                let args = TorrentBandwidthArgs {
                    download_limited: Some(v),
                    download_limit: Some(dl_limit),
                    ..Default::default()
                };
                self.list.enqueue(RpcWork::TorrentSetBandwidth {
                    params: self.list.params.clone(),
                    ids: vec![torrent_id],
                    args,
                });
                Task::none()
            }

            // Upload limit toggle: update state + send upload_limited + upload_limit.
            Message::Inspector(inspector::Message::OptionsUploadLimitToggled(v)) => {
                self.inspector.options.upload_limited = v;
                let Some(torrent_id) = self.list.selected_torrent().map(|t| t.id) else {
                    return Task::none();
                };
                let ul_limit: u64 = self.inspector.options.upload_limit_val.parse().unwrap_or(0);
                let args = TorrentBandwidthArgs {
                    upload_limited: Some(v),
                    upload_limit: Some(ul_limit),
                    ..Default::default()
                };
                self.list.enqueue(RpcWork::TorrentSetBandwidth {
                    params: self.list.params.clone(),
                    ids: vec![torrent_id],
                    args,
                });
                Task::none()
            }

            // Ratio mode segmented control: update state + immediately enqueue RPC.
            Message::Inspector(inspector::Message::OptionsRatioModeChanged(v)) => {
                self.inspector.options.ratio_mode = v;
                let Some(torrent_id) = self.list.selected_torrent().map(|t| t.id) else {
                    return Task::none();
                };
                let ratio_limit: f64 = self
                    .inspector
                    .options
                    .ratio_limit_val
                    .parse()
                    .unwrap_or(0.0);
                let args = TorrentBandwidthArgs {
                    seed_ratio_mode: Some(v),
                    seed_ratio_limit: Some(ratio_limit),
                    ..Default::default()
                };
                self.list.enqueue(RpcWork::TorrentSetBandwidth {
                    params: self.list.params.clone(),
                    ids: vec![torrent_id],
                    args,
                });
                Task::none()
            }

            // Honor global limits toggle: update state + send full bandwidth args so that
            // per-torrent limits take effect immediately when the torrent opts out of globals.
            Message::Inspector(inspector::Message::OptionsHonorGlobalToggled(v)) => {
                self.inspector.options.honors_session_limits = v;
                let Some(torrent_id) = self.list.selected_torrent().map(|t| t.id) else {
                    return Task::none();
                };
                let dl_limit: u64 = self
                    .inspector
                    .options
                    .download_limit_val
                    .parse()
                    .unwrap_or(0);
                let ul_limit: u64 = self.inspector.options.upload_limit_val.parse().unwrap_or(0);
                let args = TorrentBandwidthArgs {
                    honors_session_limits: Some(v),
                    download_limited: Some(self.inspector.options.download_limited),
                    download_limit: Some(dl_limit),
                    upload_limited: Some(self.inspector.options.upload_limited),
                    upload_limit: Some(ul_limit),
                    ..Default::default()
                };
                self.list.enqueue(RpcWork::TorrentSetBandwidth {
                    params: self.list.params.clone(),
                    ids: vec![torrent_id],
                    args,
                });
                Task::none()
            }

            // Text submit: apply limit only if the corresponding toggle is enabled.
            Message::Inspector(inspector::Message::OptionsDownloadLimitSubmitted) => {
                let Some(torrent_id) = self.list.selected_torrent().map(|t| t.id) else {
                    return Task::none();
                };
                if !self.inspector.options.download_limited {
                    return Task::none();
                }
                let dl_limit: u64 = self
                    .inspector
                    .options
                    .download_limit_val
                    .parse()
                    .unwrap_or(0);
                let args = TorrentBandwidthArgs {
                    download_limited: Some(true),
                    download_limit: Some(dl_limit),
                    ..Default::default()
                };
                self.list.enqueue(RpcWork::TorrentSetBandwidth {
                    params: self.list.params.clone(),
                    ids: vec![torrent_id],
                    args,
                });
                Task::none()
            }

            Message::Inspector(inspector::Message::OptionsUploadLimitSubmitted) => {
                let Some(torrent_id) = self.list.selected_torrent().map(|t| t.id) else {
                    return Task::none();
                };
                if !self.inspector.options.upload_limited {
                    return Task::none();
                }
                let ul_limit: u64 = self.inspector.options.upload_limit_val.parse().unwrap_or(0);
                let args = TorrentBandwidthArgs {
                    upload_limited: Some(true),
                    upload_limit: Some(ul_limit),
                    ..Default::default()
                };
                self.list.enqueue(RpcWork::TorrentSetBandwidth {
                    params: self.list.params.clone(),
                    ids: vec![torrent_id],
                    args,
                });
                Task::none()
            }

            Message::Inspector(inspector::Message::OptionsRatioLimitSubmitted) => {
                let Some(torrent_id) = self.list.selected_torrent().map(|t| t.id) else {
                    return Task::none();
                };
                // Only apply if in Custom mode (mode 1).
                if self.inspector.options.ratio_mode != 1 {
                    return Task::none();
                }
                let ratio_limit: f64 = self
                    .inspector
                    .options
                    .ratio_limit_val
                    .parse()
                    .unwrap_or(0.0);
                let args = TorrentBandwidthArgs {
                    seed_ratio_mode: Some(1),
                    seed_ratio_limit: Some(ratio_limit),
                    ..Default::default()
                };
                self.list.enqueue(RpcWork::TorrentSetBandwidth {
                    params: self.list.params.clone(),
                    ids: vec![torrent_id],
                    args,
                });
                Task::none()
            }

            // ── Bulk Options intercepts (multi-select mode) ───────────────────
            Message::Inspector(inspector::Message::BulkDownloadLimitToggled(v)) => {
                let ids: Vec<i64> = self.list.selected_ids.iter().copied().collect();
                let task = inspector::update(
                    &mut self.inspector,
                    inspector::Message::BulkDownloadLimitToggled(v),
                )
                .map(Message::Inspector);
                if !ids.is_empty() {
                    let args = TorrentBandwidthArgs {
                        download_limited: Some(v),
                        download_limit: if v {
                            self.inspector.bulk_options.download_limit_val.parse().ok()
                        } else {
                            None
                        },
                        ..Default::default()
                    };
                    self.list.enqueue(RpcWork::TorrentSetBandwidth {
                        params: self.list.params.clone(),
                        ids,
                        args,
                    });
                }
                task
            }

            Message::Inspector(inspector::Message::BulkUploadLimitToggled(v)) => {
                let ids: Vec<i64> = self.list.selected_ids.iter().copied().collect();
                let task = inspector::update(
                    &mut self.inspector,
                    inspector::Message::BulkUploadLimitToggled(v),
                )
                .map(Message::Inspector);
                if !ids.is_empty() {
                    let args = TorrentBandwidthArgs {
                        upload_limited: Some(v),
                        upload_limit: if v {
                            self.inspector.bulk_options.upload_limit_val.parse().ok()
                        } else {
                            None
                        },
                        ..Default::default()
                    };
                    self.list.enqueue(RpcWork::TorrentSetBandwidth {
                        params: self.list.params.clone(),
                        ids,
                        args,
                    });
                }
                task
            }

            Message::Inspector(inspector::Message::BulkRatioModeChanged(v)) => {
                let ids: Vec<i64> = self.list.selected_ids.iter().copied().collect();
                let task = inspector::update(
                    &mut self.inspector,
                    inspector::Message::BulkRatioModeChanged(v),
                )
                .map(Message::Inspector);
                if !ids.is_empty() {
                    let args = TorrentBandwidthArgs {
                        seed_ratio_mode: Some(v),
                        ..Default::default()
                    };
                    self.list.enqueue(RpcWork::TorrentSetBandwidth {
                        params: self.list.params.clone(),
                        ids,
                        args,
                    });
                }
                task
            }

            Message::Inspector(inspector::Message::BulkHonorGlobalToggled(v)) => {
                let ids: Vec<i64> = self.list.selected_ids.iter().copied().collect();
                let task = inspector::update(
                    &mut self.inspector,
                    inspector::Message::BulkHonorGlobalToggled(v),
                )
                .map(Message::Inspector);
                if !ids.is_empty() {
                    let args = TorrentBandwidthArgs {
                        honors_session_limits: Some(v),
                        ..Default::default()
                    };
                    self.list.enqueue(RpcWork::TorrentSetBandwidth {
                        params: self.list.params.clone(),
                        ids,
                        args,
                    });
                }
                task
            }

            Message::Inspector(inspector::Message::BulkDownloadLimitSubmitted) => {
                let ids: Vec<i64> = self.list.selected_ids.iter().copied().collect();
                if ids.is_empty()
                    || !self
                        .inspector
                        .bulk_options
                        .download_limited
                        .unwrap_or(false)
                {
                    return inspector::update(
                        &mut self.inspector,
                        inspector::Message::BulkDownloadLimitSubmitted,
                    )
                    .map(Message::Inspector);
                }
                let limit = self
                    .inspector
                    .bulk_options
                    .download_limit_val
                    .parse()
                    .unwrap_or(0);
                let args = TorrentBandwidthArgs {
                    download_limited: Some(true),
                    download_limit: Some(limit),
                    ..Default::default()
                };
                self.list.enqueue(RpcWork::TorrentSetBandwidth {
                    params: self.list.params.clone(),
                    ids,
                    args,
                });
                Task::none()
            }

            Message::Inspector(inspector::Message::BulkUploadLimitSubmitted) => {
                let ids: Vec<i64> = self.list.selected_ids.iter().copied().collect();
                if ids.is_empty() || !self.inspector.bulk_options.upload_limited.unwrap_or(false) {
                    return inspector::update(
                        &mut self.inspector,
                        inspector::Message::BulkUploadLimitSubmitted,
                    )
                    .map(Message::Inspector);
                }
                let limit = self
                    .inspector
                    .bulk_options
                    .upload_limit_val
                    .parse()
                    .unwrap_or(0);
                let args = TorrentBandwidthArgs {
                    upload_limited: Some(true),
                    upload_limit: Some(limit),
                    ..Default::default()
                };
                self.list.enqueue(RpcWork::TorrentSetBandwidth {
                    params: self.list.params.clone(),
                    ids,
                    args,
                });
                Task::none()
            }

            Message::Inspector(inspector::Message::BulkRatioLimitSubmitted) => {
                let ids: Vec<i64> = self.list.selected_ids.iter().copied().collect();
                if ids.is_empty() || self.inspector.bulk_options.ratio_mode != Some(1) {
                    return inspector::update(
                        &mut self.inspector,
                        inspector::Message::BulkRatioLimitSubmitted,
                    )
                    .map(Message::Inspector);
                }
                let ratio: f64 = self
                    .inspector
                    .bulk_options
                    .ratio_limit_val
                    .parse()
                    .unwrap_or(0.0);
                let args = TorrentBandwidthArgs {
                    seed_ratio_mode: Some(1),
                    seed_ratio_limit: Some(ratio),
                    ..Default::default()
                };
                self.list.enqueue(RpcWork::TorrentSetBandwidth {
                    params: self.list.params.clone(),
                    ids,
                    args,
                });
                Task::none()
            }

            Message::Inspector(inspector::Message::AllFilesWantedToggled {
                torrent_id,
                file_count,
                wanted,
            }) => {
                for i in 0..file_count {
                    self.inspector.pending_wanted.insert(i, wanted);
                }
                let file_indices: Vec<i64> = (0..file_count as i64).collect();
                self.list.enqueue(RpcWork::SetFileWanted {
                    params: self.list.params.clone(),
                    torrent_id,
                    file_indices,
                    wanted,
                });
                Task::none()
            }

            Message::List(msg) => {
                let task = torrent_list::update(&mut self.list, msg).map(Message::List);
                // After every list update, remove any pending file-wanted entries that are
                // already confirmed by the latest polled file_stats.  This is the primary
                // mechanism for clearing pending_wanted after a successful torrent-set RPC.
                if !self.inspector.pending_wanted.is_empty()
                    && let Some(torrent) = self.list.selected_torrent()
                {
                    let file_stats = &torrent.file_stats;
                    self.inspector
                        .pending_wanted
                        .retain(|i, wanted| file_stats.get(*i).map(|s| s.wanted) != Some(*wanted));
                }
                task
            }

            Message::Inspector(msg) => {
                inspector::update(&mut self.inspector, msg).map(Message::Inspector)
            }

            // Already escalated; app::update handles these.
            Message::Disconnect
            | Message::OpenSettingsClicked
            | Message::TurtleModeToggled
            | Message::SessionDataLoaded(_) => Task::none(),
        }
    }

    /// Compose the list and (when a torrent is selected) the inspector panel.
    pub fn view(&self, theme_mode: ThemeMode, alt_speed_enabled: bool) -> Element<'_, Message> {
        // Show splash until the first torrent-list response arrives.
        if !self.list.initial_load_done {
            let mut label = format!("Connecting to {}\u{2026}", self.connect_label);
            if let Some(name) = &self.profile_name {
                label = format!("Connecting to {} ({})\u{2026}", self.connect_label, name);
            }
            return column![
                container(iced::widget::text(label).size(16))
                    .width(Length::Fill)
                    .height(Length::FillPortion(2))
                    .center_x(Length::Fill)
                    .center_y(Length::Fill),
                container(
                    iced::widget::image(iced::widget::image::Handle::from_bytes(
                        crate::theme::ICON_512_BYTES,
                    ))
                    .width(Length::Fixed(72.0))
                    .content_fit(iced::ContentFit::ScaleDown),
                )
                .width(Length::Fill)
                .height(Length::FillPortion(1))
                .center_x(Length::Fill)
                .center_y(Length::Fill),
            ]
            .into();
        }

        let list_elem =
            torrent_list::view(&self.list, theme_mode, alt_speed_enabled).map(Message::List);

        let selected_count = self.list.selected_ids.len();
        let content: Element<Message> = if selected_count == 0 {
            list_elem
        } else {
            let torrent = self.list.selected_torrent();
            let inspector_elem =
                inspector::view(&self.inspector, torrent, selected_count).map(Message::Inspector);
            column![
                container(list_elem)
                    .height(Length::FillPortion(3))
                    .width(Length::Fill),
                container(inspector_elem)
                    .height(Length::FillPortion(2))
                    .width(Length::Fill)
                    .style(crate::theme::m3_card),
            ]
            .into()
        };

        // Lift the context menu overlay to this level so it can draw over the
        // inspector panel (the torrent-list container alone is not tall enough).
        if let Some(overlay) = torrent_list::view_context_menu_overlay(&self.list) {
            iced::widget::stack![content, overlay.map(Message::List)].into()
        } else {
            content
        }
    }
}

// -- Tests --------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::{RpcWork, TorrentData};
    use crate::screens::inspector::ActiveTab;
    use crate::screens::torrent_list::Message as TLMsg;
    use tokio::sync::mpsc;

    fn make_screen() -> MainScreen {
        MainScreen::new_with_label(
            TransmissionCredentials {
                host: "localhost".to_owned(),
                port: 9091,
                username: None,
                password: None,
            },
            "test-session-id".to_owned(),
            None,
            None,
            1,
        )
    }

    fn make_torrent(id: i64) -> TorrentData {
        TorrentData {
            id,
            name: format!("torrent-{id}"),
            status: 4,
            percent_done: 0.5,
            ..Default::default()
        }
    }

    /// Selecting a torrent resets inspector.active_tab to General.
    #[test]
    fn torrent_selected_resets_inspector_tab() {
        let mut screen = make_screen();
        screen.list.torrents = vec![make_torrent(1), make_torrent(2)];

        let _ = screen.update(Message::List(TLMsg::TorrentSelected(1)));
        assert!(screen.list.selected_ids.contains(&1));
        assert_eq!(screen.inspector.active_tab, ActiveTab::General);

        let _ = screen.update(Message::Inspector(inspector::Message::TabSelected(
            ActiveTab::Files,
        )));
        assert_eq!(screen.inspector.active_tab, ActiveTab::Files);

        // Select a different torrent — tab should reset.
        let _ = screen.update(Message::List(TLMsg::TorrentSelected(2)));
        assert!(screen.list.selected_ids.contains(&2));
        assert_eq!(
            screen.inspector.active_tab,
            ActiveTab::General,
            "tab should reset to General on new selection"
        );
    }

    /// Selecting the same torrent (deselects) does not reset the tab.
    #[test]
    fn deselecting_torrent_does_not_reset_tab() {
        let mut screen = make_screen();
        screen.list.torrents = vec![make_torrent(1)];

        let _ = screen.update(Message::List(TLMsg::TorrentSelected(1)));
        let _ = screen.update(Message::Inspector(inspector::Message::TabSelected(
            ActiveTab::Peers,
        )));
        assert_eq!(screen.inspector.active_tab, ActiveTab::Peers);

        // Clicking a different row clears the first selection (plain-click replaces).
        let _ = screen.update(Message::List(TLMsg::TorrentSelected(1)));
        assert!(
            screen.list.selected_ids.contains(&1),
            "clicking the same row re-selects (plain click always replaces)"
        );
        assert_eq!(
            screen.inspector.active_tab,
            ActiveTab::Peers,
            "tab should stay on Peers after re-selecting the same torrent"
        );
    }

    // ── Bandwidth hierarchy helpers ───────────────────────────────────────────

    /// Create a screen with a live mpsc sender so enqueued work can be inspected.
    fn make_screen_with_sender() -> (MainScreen, mpsc::Receiver<RpcWork>) {
        let mut screen = make_screen();
        let (tx, rx) = mpsc::channel(16);
        screen.list.sender = Some(tx);
        (screen, rx)
    }

    /// A torrent with non-default bandwidth fields for testing.
    fn make_bandwidth_torrent(id: i64) -> TorrentData {
        TorrentData {
            id,
            name: format!("torrent-{id}"),
            status: 4,
            percent_done: 0.5,
            download_limited: true,
            download_limit: 800,
            upload_limited: true,
            upload_limit: 200,
            honors_session_limits: true,
            ..Default::default()
        }
    }

    /// Drain all work items from the receiver without blocking.
    fn drain(rx: &mut mpsc::Receiver<RpcWork>) -> Vec<RpcWork> {
        let mut items = Vec::new();
        while let Ok(w) = rx.try_recv() {
            items.push(w);
        }
        items
    }

    // ── Honor global limits ───────────────────────────────────────────────────

    /// Toggling "Honor Global Limits" off must include the current per-torrent
    /// download/upload limit state so the daemon can enforce them immediately.
    #[test]
    fn honor_global_toggle_off_sends_full_bandwidth_state() {
        let (mut screen, mut rx) = make_screen_with_sender();
        screen.list.torrents = vec![make_bandwidth_torrent(1)];
        let _ = screen.update(Message::List(TLMsg::TorrentSelected(1)));

        let _ = screen.update(Message::Inspector(
            inspector::Message::OptionsHonorGlobalToggled(false),
        ));

        let work = drain(&mut rx);
        let bandwidth_items: Vec<_> = work
            .into_iter()
            .filter_map(|w| {
                if let RpcWork::TorrentSetBandwidth { ids, args, .. } = w {
                    Some((ids, args))
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(bandwidth_items.len(), 1);
        let (ids, args) = &bandwidth_items[0];
        assert!(ids.contains(&1));
        assert_eq!(args.honors_session_limits, Some(false));
        assert_eq!(args.download_limited, Some(true));
        assert_eq!(args.download_limit, Some(800));
        assert_eq!(args.upload_limited, Some(true));
        assert_eq!(args.upload_limit, Some(200));
    }

    /// Toggling "Honor Global Limits" back on also includes full bandwidth state.
    #[test]
    fn honor_global_toggle_on_sends_full_bandwidth_state() {
        let (mut screen, mut rx) = make_screen_with_sender();
        let mut torrent = make_bandwidth_torrent(1);
        torrent.honors_session_limits = false;
        screen.list.torrents = vec![torrent];
        let _ = screen.update(Message::List(TLMsg::TorrentSelected(1)));

        let _ = screen.update(Message::Inspector(
            inspector::Message::OptionsHonorGlobalToggled(true),
        ));

        let work = drain(&mut rx);
        let (_, args) = work
            .into_iter()
            .find_map(|w| {
                if let RpcWork::TorrentSetBandwidth { ids, args, .. } = w {
                    Some((ids, args))
                } else {
                    None
                }
            })
            .expect("should enqueue TorrentSetBandwidth");
        assert_eq!(args.honors_session_limits, Some(true));
        assert_eq!(args.download_limited, Some(true));
        assert_eq!(args.upload_limited, Some(true));
    }

    // ── Download / upload limit toggles ──────────────────────────────────────

    /// Enabling the download limit toggle sends `downloadLimited=true` plus the
    /// current value from the text field.
    #[test]
    fn download_limit_toggle_on_sends_correct_args() {
        let (mut screen, mut rx) = make_screen_with_sender();
        let mut torrent = make_bandwidth_torrent(1);
        torrent.download_limited = false;
        screen.list.torrents = vec![torrent];
        let _ = screen.update(Message::List(TLMsg::TorrentSelected(1)));

        // Simulate the user typing a new value before enabling the toggle.
        let _ = screen.update(Message::Inspector(
            inspector::Message::OptionsDownloadLimitChanged("500".to_owned()),
        ));
        let _ = screen.update(Message::Inspector(
            inspector::Message::OptionsDownloadLimitToggled(true),
        ));

        let work = drain(&mut rx);
        let (_, args) = work
            .into_iter()
            .find_map(|w| {
                if let RpcWork::TorrentSetBandwidth { ids, args, .. } = w {
                    Some((ids, args))
                } else {
                    None
                }
            })
            .expect("should enqueue TorrentSetBandwidth");
        assert_eq!(args.download_limited, Some(true));
        assert_eq!(args.download_limit, Some(500));
    }

    /// Disabling the download limit toggle sends `downloadLimited=false`.
    #[test]
    fn download_limit_toggle_off_sends_correct_args() {
        let (mut screen, mut rx) = make_screen_with_sender();
        screen.list.torrents = vec![make_bandwidth_torrent(1)];
        let _ = screen.update(Message::List(TLMsg::TorrentSelected(1)));

        let _ = screen.update(Message::Inspector(
            inspector::Message::OptionsDownloadLimitToggled(false),
        ));

        let work = drain(&mut rx);
        let (_, args) = work
            .into_iter()
            .find_map(|w| {
                if let RpcWork::TorrentSetBandwidth { ids, args, .. } = w {
                    Some((ids, args))
                } else {
                    None
                }
            })
            .expect("should enqueue TorrentSetBandwidth");
        assert_eq!(args.download_limited, Some(false));
    }

    /// Submitting a download limit value with the toggle OFF is a no-op (no RPC).
    #[test]
    fn download_limit_submit_noop_when_toggle_off() {
        let (mut screen, mut rx) = make_screen_with_sender();
        let mut torrent = make_bandwidth_torrent(1);
        torrent.download_limited = false;
        screen.list.torrents = vec![torrent];
        let _ = screen.update(Message::List(TLMsg::TorrentSelected(1)));

        let _ = screen.update(Message::Inspector(
            inspector::Message::OptionsDownloadLimitSubmitted,
        ));

        let work = drain(&mut rx);
        let bandwidth_count = work
            .iter()
            .filter(|w| matches!(w, RpcWork::TorrentSetBandwidth { .. }))
            .count();
        assert_eq!(
            bandwidth_count, 0,
            "should not enqueue RPC when toggle is off"
        );
    }

    /// Submitting an upload limit value with the toggle OFF is a no-op (no RPC).
    #[test]
    fn upload_limit_submit_noop_when_toggle_off() {
        let (mut screen, mut rx) = make_screen_with_sender();
        let mut torrent = make_bandwidth_torrent(1);
        torrent.upload_limited = false;
        screen.list.torrents = vec![torrent];
        let _ = screen.update(Message::List(TLMsg::TorrentSelected(1)));

        let _ = screen.update(Message::Inspector(
            inspector::Message::OptionsUploadLimitSubmitted,
        ));

        let work = drain(&mut rx);
        let bandwidth_count = work
            .iter()
            .filter(|w| matches!(w, RpcWork::TorrentSetBandwidth { .. }))
            .count();
        assert_eq!(
            bandwidth_count, 0,
            "should not enqueue RPC when toggle is off"
        );
    }

    // ── honorsSessionLimits semantics ─────────────────────────────────────────
    // These tests document the precise semantic of the "Honor Global Speed Limits"
    // toggle:
    //   ON  (true)  → torrent respects whatever global limit is currently active on
    //                 the daemon (standard limits when turtle mode is off; alternative
    //                 limits when turtle mode is on).
    //   OFF (false) → torrent ignores ALL session-level limits. Only its own
    //                 per-torrent download/upload caps apply; if those are also
    //                 disabled, the torrent runs uncapped.

    /// Disabling honor on a torrent that has no per-torrent limits sends
    /// `honors_session_limits=false` + `download_limited=false` + `upload_limited=false`.
    /// The torrent will run uncapped: no global limit, no per-torrent limit.
    #[test]
    fn honor_off_no_per_torrent_limits_torrent_runs_uncapped() {
        let (mut screen, mut rx) = make_screen_with_sender();
        // Torrent with no per-torrent limits and honor=true (default state).
        screen.list.torrents = vec![TorrentData {
            id: 1,
            name: "torrent".to_owned(),
            status: 4,
            percent_done: 0.5,
            download_limited: false,
            upload_limited: false,
            honors_session_limits: true,
            ..Default::default()
        }];
        let _ = screen.update(Message::List(TLMsg::TorrentSelected(1)));

        let _ = screen.update(Message::Inspector(
            inspector::Message::OptionsHonorGlobalToggled(false),
        ));

        let (_, args) = drain(&mut rx)
            .into_iter()
            .find_map(|w| {
                if let RpcWork::TorrentSetBandwidth { ids, args, .. } = w {
                    Some((ids, args))
                } else {
                    None
                }
            })
            .expect("should enqueue TorrentSetBandwidth");

        // Both global and per-torrent limits are off → torrent runs uncapped.
        assert_eq!(args.honors_session_limits, Some(false));
        assert_eq!(args.download_limited, Some(false));
        assert_eq!(args.upload_limited, Some(false));
    }

    /// Re-enabling honor subjects the torrent to the active session limit again.
    /// Which limit that is (standard vs alternative) depends on whether turtle mode
    /// is active on the daemon — that is a daemon concern, not tested here.
    /// This test confirms the client sends `honors_session_limits=true` so Transmission
    /// will enforce whichever global limit is currently active.
    #[test]
    fn honor_on_restores_session_limit_compliance() {
        let (mut screen, mut rx) = make_screen_with_sender();
        screen.list.torrents = vec![TorrentData {
            id: 1,
            name: "torrent".to_owned(),
            status: 4,
            percent_done: 0.5,
            download_limited: false,
            upload_limited: false,
            honors_session_limits: false, // currently bypassing global limits
            ..Default::default()
        }];
        let _ = screen.update(Message::List(TLMsg::TorrentSelected(1)));

        let _ = screen.update(Message::Inspector(
            inspector::Message::OptionsHonorGlobalToggled(true),
        ));

        let (_, args) = drain(&mut rx)
            .into_iter()
            .find_map(|w| {
                if let RpcWork::TorrentSetBandwidth { ids, args, .. } = w {
                    Some((ids, args))
                } else {
                    None
                }
            })
            .expect("should enqueue TorrentSetBandwidth");

        // The torrent will comply with whatever global limit the daemon has active
        // (standard or alternative depending on turtle mode state).
        assert_eq!(args.honors_session_limits, Some(true));
    }

    /// When honor is OFF and a per-torrent download limit is also set, disabling honor
    /// does NOT remove the per-torrent limit — the torrent's own cap still applies even
    /// though it ignores global limits.
    #[test]
    fn honor_off_with_per_torrent_limit_torrent_still_capped_by_own_limit() {
        let (mut screen, mut rx) = make_screen_with_sender();
        screen.list.torrents = vec![TorrentData {
            id: 1,
            name: "torrent".to_owned(),
            status: 4,
            percent_done: 0.5,
            download_limited: true,
            download_limit: 300,
            upload_limited: false,
            upload_limit: 0,
            honors_session_limits: true,
            ..Default::default()
        }];
        let _ = screen.update(Message::List(TLMsg::TorrentSelected(1)));

        let _ = screen.update(Message::Inspector(
            inspector::Message::OptionsHonorGlobalToggled(false),
        ));

        let (_, args) = drain(&mut rx)
            .into_iter()
            .find_map(|w| {
                if let RpcWork::TorrentSetBandwidth { ids, args, .. } = w {
                    Some((ids, args))
                } else {
                    None
                }
            })
            .expect("should enqueue TorrentSetBandwidth");

        // Per-torrent download cap is preserved; global limits are bypassed.
        assert_eq!(args.honors_session_limits, Some(false));
        assert_eq!(args.download_limited, Some(true));
        assert_eq!(args.download_limit, Some(300));
        assert_eq!(args.upload_limited, Some(false));
    }
}
