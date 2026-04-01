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
use crate::rpc::TransmissionCredentials;
use crate::screens::inspector::{self, InspectorScreen};
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
        }
    }

    /// Subscription: tick at the configured refresh interval + serialized RPC worker
    /// + conditional dialog keyboard handler.
    pub fn subscription(&self) -> Subscription<Message> {
        let interval = Duration::from_secs(self.refresh_interval.max(1) as u64);
        let tick = iced::time::every(interval).map(|_| Message::List(torrent_list::Message::Tick));
        let worker = Subscription::run(torrent_list::rpc_worker_stream).map(Message::List);
        let dialog_kb = self.list.dialog_subscription().map(Message::List);
        Subscription::batch([tick, worker, dialog_kb])
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

            // Intercept TorrentSelected so we can reset the inspector tab.
            Message::List(torrent_list::Message::TorrentSelected(id)) => {
                let prev = self.list.selected_id;
                let task = torrent_list::update(
                    &mut self.list,
                    torrent_list::Message::TorrentSelected(id),
                )
                .map(Message::List);
                // Reset to General whenever a *new* torrent is selected.
                if self.list.selected_id != prev && self.list.selected_id.is_some() {
                    self.inspector.active_tab = inspector::ActiveTab::General;
                }
                task
            }

            Message::List(msg) => torrent_list::update(&mut self.list, msg).map(Message::List),

            Message::Inspector(msg) => {
                inspector::update(&mut self.inspector, msg).map(Message::Inspector)
            }

            // Already escalated; app::update handles these.
            Message::Disconnect | Message::OpenSettingsClicked => Task::none(),
        }
    }

    /// Compose the list and (when a torrent is selected) the inspector panel.
    pub fn view(&self, theme_mode: ThemeMode) -> Element<'_, Message> {
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

        let list_elem = torrent_list::view(&self.list, theme_mode).map(Message::List);

        match self.list.selected_torrent() {
            None => list_elem,
            Some(torrent) => {
                let inspector_elem =
                    inspector::view(&self.inspector, torrent).map(Message::Inspector);
                column![
                    container(list_elem)
                        .height(Length::FillPortion(3))
                        .width(Length::Fill),
                    container(inspector_elem)
                        .height(Length::FillPortion(1))
                        .width(Length::Fill)
                        .style(crate::theme::m3_card),
                ]
                .into()
            }
        }
    }
}

// -- Tests --------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::TorrentData;
    use crate::screens::inspector::ActiveTab;
    use crate::screens::torrent_list::Message as TLMsg;

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
        assert_eq!(screen.list.selected_id, Some(1));
        assert_eq!(screen.inspector.active_tab, ActiveTab::General);

        let _ = screen.update(Message::Inspector(inspector::Message::TabSelected(
            ActiveTab::Files,
        )));
        assert_eq!(screen.inspector.active_tab, ActiveTab::Files);

        // Select a different torrent — tab should reset.
        let _ = screen.update(Message::List(TLMsg::TorrentSelected(2)));
        assert_eq!(screen.list.selected_id, Some(2));
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

        // Clicking the same row deselects.
        let _ = screen.update(Message::List(TLMsg::TorrentSelected(1)));
        assert_eq!(screen.list.selected_id, None);
        assert_eq!(
            screen.inspector.active_tab,
            ActiveTab::Peers,
            "tab should stay on Peers after deselection"
        );
    }
}
