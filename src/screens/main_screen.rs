//! Main screen — the torrent list shown after a successful connection.
//!
//! Displays a sticky column header and a scrollable list of torrent rows, each
//! showing the torrent name, status, and a progress bar. A toolbar row at the
//! top holds action buttons (disabled in v0.1) and a Disconnect button.
//!
//! # Non-blocking invariant
//!
//! All RPC calls are issued via `Task::perform()`. The `update()` method
//! only mutates in-memory state and returns immediately.
//!
//! # Polling
//!
//! A 5-second subscription fires `Message::Tick`. Ticks are silently dropped
//! when `is_loading` is `true`, ensuring at most one RPC call is in-flight at
//! a time.

use std::time::Duration;

use iced::widget::{button, column, container, progress_bar, row, scrollable, text, Space};
use iced::{Element, Length, Subscription, Task};

use crate::app::Message;
use crate::rpc::{TorrentData, TransmissionCredentials};

// ── Column layout constants ───────────────────────────────────────────────────

/// Relative width of the Name column. Used in both the header and data rows to
/// keep alignment consistent across the sticky/scrollable split.
const COL_NAME: u16 = 5;

/// Relative width of the Status column.
const COL_STATUS: u16 = 2;

/// Relative width of the Progress column.
const COL_PROGRESS: u16 = 3;

// ── Status mapping ────────────────────────────────────────────────────────────

/// Convert a Transmission status integer to a human-readable label.
///
/// Values are defined in the Transmission RPC spec:
/// <https://github.com/transmission/transmission/blob/main/docs/rpc-spec.md>
fn status_label(status: i32) -> &'static str {
    match status {
        0 => "Stopped",
        1 => "Queued (check)",
        2 => "Checking",
        3 => "Queued",
        4 => "Downloading",
        5 => "Queued (seed)",
        6 => "Seeding",
        _ => "Unknown",
    }
}

// ── MainScreen ────────────────────────────────────────────────────────────────

/// State for the torrent list screen.
#[derive(Debug)]
pub struct MainScreen {
    /// Active Transmission session ID. Updated on `SessionIdRotated`.
    pub session_id: String,
    /// Credentials used to reach the daemon; passed to every RPC call.
    pub credentials: TransmissionCredentials,
    /// The most recently fetched list of torrents.
    pub torrents: Vec<TorrentData>,
    /// Guard flag: `true` while an RPC call is in-flight.
    ///
    /// When `true`, incoming [`Message::Tick`]s are silently dropped to prevent
    /// concurrent RPC requests, which could cause session-id race conditions.
    pub is_loading: bool,
    /// Human-readable error from the last failed poll, if any.
    pub error: Option<String>,
}

impl MainScreen {
    /// Create a new main screen from a successful connection result.
    pub fn new(credentials: TransmissionCredentials, session_id: String) -> Self {
        MainScreen {
            session_id,
            credentials,
            torrents: Vec::new(),
            is_loading: false,
            error: None,
        }
    }

    /// Render the toolbar, sticky header, and scrollable torrent rows.
    ///
    /// The header row lives outside the `scrollable` widget so it remains
    /// visible while the list scrolls. `FillPortion` weights must match
    /// between header cells and data cells — both use the `COL_*` constants.
    pub fn view(&self) -> Element<'_, Message> {
        let toolbar = row![
            button("Add").style(iced::widget::button::secondary),
            button("Pause").style(iced::widget::button::secondary),
            button("Resume").style(iced::widget::button::secondary),
            button("Delete").style(iced::widget::button::secondary),
            Space::new(),
            button("Disconnect").on_press(Message::Disconnect),
        ]
        .spacing(8);

        // Sticky header — outside the scrollable.
        // Uses the same FillPortion weights as the data rows so columns align.
        let header = row![
            text("Name").width(Length::FillPortion(COL_NAME)),
            text("Status").width(Length::FillPortion(COL_STATUS)),
            text("Progress").width(Length::FillPortion(COL_PROGRESS)),
        ]
        .padding(4);

        // Data rows inside the scrollable.
        let rows = self.torrents.iter().map(|t| {
            row![
                // WordOrGlyph breaks long dot-separated filenames at glyph boundary
                // preventing overflow into adjacent columns.
                text(&t.name)
                    .width(Length::FillPortion(COL_NAME))
                    .wrapping(text::Wrapping::WordOrGlyph),
                text(status_label(t.status)).width(Length::FillPortion(COL_STATUS)),
                // .length() sets the main axis (width) of the horizontal bar.
                // .girth() sets the cross axis (height) explicitly.
                progress_bar(0.0..=1.0, t.percent_done as f32)
                    .length(Length::FillPortion(COL_PROGRESS))
                    .girth(14.0),
            ]
            .padding(4)
            .align_y(iced::Center)
            .into()
        });

        let list = scrollable(column(rows).spacing(2));

        container(
            column![toolbar, header, list]
                .spacing(4)
                .padding(8)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .into()
    }

    /// Polling subscription: fires [`Message::Tick`] every 5 seconds.
    ///
    /// Interval is 5 s for v0.1. The spec targets 1–2 s for v1.0; this will be
    /// revisited once user actions (pause/resume) are in-flight and the
    /// concurrency model is hardened.
    pub fn subscription(&self) -> Subscription<Message> {
        iced::time::every(Duration::from_secs(5)).map(|_| Message::Tick)
    }

    /// Handle a message directed at the main screen.
    ///
    /// Returns immediately — all async work is inside `Task::perform()`.
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // 5.6 – Guard: ignore ticks while a request is already in-flight.
            Message::Tick => {
                if self.is_loading {
                    tracing::debug!("Tick skipped: RPC call already in-flight");
                    return Task::none();
                }
                tracing::debug!("Tick: firing torrent-get");
                self.is_loading = true;
                self.fire_torrent_get()
            }

            // 5.7 – Replace torrent list and clear the loading flag.
            Message::TorrentsUpdated(Ok(torrents)) => {
                tracing::info!(count = torrents.len(), "Torrent list refreshed");
                self.torrents = torrents;
                self.is_loading = false;
                self.error = None;
                Task::none()
            }

            Message::TorrentsUpdated(Err(err)) => {
                tracing::error!(error = %err, "torrent-get failed");
                self.is_loading = false;
                self.error = Some(err);
                Task::none()
            }

            // 5.8 – Session rotated: persist new id and retry.
            Message::SessionIdRotated(new_id) => {
                tracing::debug!(%new_id, "Session ID rotated, retrying torrent-get");
                self.session_id = new_id;
                self.fire_torrent_get()
            }

            _ => Task::none(),
        }
    }

    /// Issue an async `torrent-get` RPC call.
    ///
    /// Wraps `rpc::torrent_get` in a `Task::perform`, mapping the result to
    /// either `TorrentsUpdated` or `SessionIdRotated` on a 409 rotation.
    fn fire_torrent_get(&self) -> Task<Message> {
        let url = self.credentials.rpc_url();
        let creds = self.credentials.clone();
        let sid = self.session_id.clone();

        Task::perform(
            async move {
                crate::rpc::torrent_get(&url, &creds, &sid).await
            },
            |result| match result {
                Ok(torrents) => Message::TorrentsUpdated(Ok(torrents)),
                Err(crate::rpc::RpcError::SessionRotated(new_id)) => {
                    Message::SessionIdRotated(new_id)
                }
                Err(e) => Message::TorrentsUpdated(Err(e.to_string())),
            },
        )
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::TorrentData;

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

    fn make_torrent(id: i64, name: &str) -> TorrentData {
        TorrentData { id, name: name.to_owned(), status: 6, percent_done: 1.0 }
    }

    /// 6.1 – Tick when is_loading=true: no command, state unchanged.
    #[test]
    fn tick_ignored_when_loading() {
        let mut screen = make_screen();
        screen.is_loading = true;
        let cmd = screen.update(Message::Tick);
        // Command::none() has no futures; the screen state should be unchanged.
        assert!(screen.is_loading); // still true — no change
        drop(cmd);
    }

    /// 6.2 – Tick when is_loading=false: is_loading becomes true and a command is returned.
    #[test]
    fn tick_fires_when_not_loading() {
        let mut screen = make_screen();
        screen.is_loading = false;
        let cmd = screen.update(Message::Tick);
        assert!(screen.is_loading, "is_loading should be set to true after Tick");
        drop(cmd);
    }

    /// 6.3 – TorrentsUpdated(Ok) replaces torrents and clears is_loading.
    #[test]
    fn torrents_updated_ok_replaces_and_clears_loading() {
        let mut screen = make_screen();
        screen.is_loading = true;
        let new_data = vec![make_torrent(1, "Ubuntu ISO"), make_torrent(2, "Arch Linux")];
        let _ = screen.update(Message::TorrentsUpdated(Ok(new_data)));
        assert!(!screen.is_loading);
        assert_eq!(screen.torrents.len(), 2);
        assert_eq!(screen.torrents[0].name, "Ubuntu ISO");
    }

    /// 6.4 – TorrentsUpdated(Err) clears is_loading and sets error.
    #[test]
    fn torrents_updated_err_clears_loading_and_sets_error() {
        let mut screen = make_screen();
        screen.is_loading = true;
        let _ = screen.update(Message::TorrentsUpdated(Err("timeout".to_owned())));
        assert!(!screen.is_loading);
        assert_eq!(screen.error.as_deref(), Some("timeout"));
    }

    /// 6.5 – SessionIdRotated updates the stored session id.
    #[test]
    fn session_id_rotated_updates_id() {
        let mut screen = make_screen();
        let _ = screen.update(Message::SessionIdRotated("new-id-xyz".to_owned()));
        assert_eq!(screen.session_id, "new-id-xyz");
    }
}
