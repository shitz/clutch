//! Main screen — composes the torrent list and detail inspector.
//!
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
//! - `Message::List(TorrentListMessage)` → delegated to `torrent_list::update`
//! - `Message::Inspector(InspectorMessage)` → delegated to `inspector::update`
//! - `Message::Disconnect` → escalated via `Task::done` to `app::update`
//!
//! `TorrentSelected` is intercepted before delegation so the inspector tab can
//! be reset to `General` whenever a different torrent is selected.

use std::time::Duration;

use iced::widget::{column, container};
use iced::{Element, Length, Subscription, Task};

use crate::rpc::TransmissionCredentials;
use crate::screens::inspector::{self, InspectorScreen};
use crate::screens::torrent_list::{self, TorrentListScreen};

// ── Message ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    List(torrent_list::Message),
    Inspector(inspector::Message),
    /// Escalated from `List(Disconnect)` — handled by `app::update`.
    Disconnect,
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct MainScreen {
    pub list: TorrentListScreen,
    pub inspector: InspectorScreen,
}

impl MainScreen {
    pub fn new(credentials: TransmissionCredentials, session_id: String) -> Self {
        MainScreen {
            list: TorrentListScreen::new(credentials, session_id),
            inspector: InspectorScreen::new(),
        }
    }

    /// Subscription: 1-second tick + serialized RPC worker.
    pub fn subscription(&self) -> Subscription<Message> {
        let tick = iced::time::every(Duration::from_secs(1))
            .map(|_| Message::List(torrent_list::Message::Tick));
        let worker = Subscription::run(torrent_list::rpc_worker_stream).map(Message::List);
        Subscription::batch([tick, worker])
    }

    /// Route messages to the appropriate child; intercept cross-cutting concerns.
    pub fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            // Intercept Disconnect before it reaches the list.
            Message::List(torrent_list::Message::Disconnect) => Task::done(Message::Disconnect),

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

            // Already escalated; app::update handles it.
            Message::Disconnect => Task::none(),
        }
    }

    /// Compose the list and (when a torrent is selected) the inspector panel.
    pub fn view(&self) -> Element<'_, Message> {
        let list_elem = torrent_list::view(&self.list).map(Message::List);

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
                        .width(Length::Fill),
                ]
                .into()
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::TorrentData;
    use crate::screens::inspector::ActiveTab;
    use crate::screens::torrent_list::Message as TLMsg;

    fn make_screen() -> MainScreen {
        MainScreen::new(
            TransmissionCredentials {
                host: "localhost".to_owned(),
                port: 9091,
                username: None,
                password: None,
            },
            "test-session-id".to_owned(),
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

    /// 12.3 – Selecting a torrent resets inspector.active_tab to General.
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

    /// 12.3b – Selecting the same torrent (deselects) does not reset the tab.
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

    /// 12.4 – No selection means inspector not rendered.
    #[test]
    fn no_selection_means_inspector_hidden() {
        let screen = make_screen();
        assert!(screen.list.selected_torrent().is_none());
    }

    /// 12.4b – With a selection, selected_torrent() returns Some.
    #[test]
    fn with_selection_inspector_is_shown() {
        let mut screen = make_screen();
        screen.list.torrents = vec![make_torrent(3)];
        let _ = screen.update(Message::List(TLMsg::TorrentSelected(3)));
        assert!(screen.list.selected_torrent().is_some());
    }
}
