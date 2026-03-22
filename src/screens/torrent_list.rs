//! Torrent list sub-screen — toolbar, sticky header, scrollable rows and the
//! add-torrent modal dialog.
//!
//! This is a self-contained Elm component:
//! - [`TorrentListScreen`] — all mutable state for the list and RPC worker
//! - [`Message`] — every event the list can handle
//! - [`update`] — pure state transition (async work in [`Task`])
//! - [`view`] — produces the widget tree
//!
//! The `Disconnect` message is not handled here; it is intercepted by the
//! parent `MainScreen` and escalated to the application level.
//!
//! # RPC Worker
//!
//! The serialized RPC worker stream is defined in this module and exposed as
//! [`rpc_worker_stream`]. It must be returned from a `Subscription::run` call
//! in the parent screen so the subscription ID is stable across redraws.

use base64::Engine as _;
use iced::futures::SinkExt as _;
use iced::widget::{
    Space, button, checkbox, column, container, progress_bar, row, scrollable, stack, text,
    text_input,
};
use iced::{Element, Length, Task};
use tokio::sync::mpsc;

use crate::rpc::{AddPayload, RpcWork, TorrentData, TransmissionCredentials};

// ── Column layout constants ───────────────────────────────────────────────────

const COL_NAME: u16 = 5;
const COL_STATUS: u16 = 2;
const COL_PROGRESS: u16 = 3;

// ── Status mapping ────────────────────────────────────────────────────────────

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

// ── File-size formatting (u64, for add-dialog preview) ────────────────────────

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

// ── Add-torrent dialog types ──────────────────────────────────────────────────

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

// ── Message ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    // Polling
    Tick,
    TorrentsUpdated(Result<Vec<TorrentData>, String>),
    SessionIdRotated(String),
    // Worker startup
    RpcWorkerReady(mpsc::Sender<RpcWork>),
    // Row selection
    TorrentSelected(i64),
    // Toolbar actions
    PauseClicked,
    ResumeClicked,
    DeleteClicked,
    DeleteLocalDataToggled(bool),
    DeleteConfirmed,
    DeleteCancelled,
    ActionCompleted(Result<(), String>),
    // Add-torrent dialog
    AddTorrentClicked,
    TorrentFileRead(Result<FileReadResult, String>),
    AddLinkClicked,
    AddDialogMagnetChanged(String),
    AddDialogDestinationChanged(String),
    AddConfirmed,
    AddCancelled,
    AddCompleted(Result<(), String>),
    // Escalated to parent — intercepted by MainScreen before reaching update()
    Disconnect,
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct TorrentListScreen {
    pub session_id: String,
    pub credentials: TransmissionCredentials,
    pub torrents: Vec<TorrentData>,
    /// Efficiency guard: `true` while an RPC result is pending.
    pub is_loading: bool,
    pub sender: Option<mpsc::Sender<RpcWork>>,
    pub error: Option<String>,
    pub selected_id: Option<i64>,
    pub confirming_delete: Option<(i64, bool)>,
    pub add_dialog: AddDialogState,
}

impl TorrentListScreen {
    pub fn new(credentials: TransmissionCredentials, session_id: String) -> Self {
        TorrentListScreen {
            session_id,
            credentials,
            torrents: Vec::new(),
            is_loading: false,
            error: None,
            selected_id: None,
            confirming_delete: None,
            add_dialog: AddDialogState::Hidden,
            sender: None,
        }
    }

    /// Return the currently selected torrent, if any.
    pub fn selected_torrent(&self) -> Option<&TorrentData> {
        let id = self.selected_id?;
        self.torrents.iter().find(|t| t.id == id)
    }

    fn enqueue(&self, work: RpcWork) {
        if let Some(tx) = &self.sender {
            if let Err(e) = tx.try_send(work) {
                tracing::error!("RPC work queue full, dropping work item: {e}");
            }
        } else {
            tracing::warn!("RPC worker not ready yet, dropping work item");
        }
    }

    fn enqueue_torrent_get(&self) {
        self.enqueue(RpcWork::TorrentGet {
            url: self.credentials.rpc_url(),
            credentials: self.credentials.clone(),
            session_id: self.session_id.clone(),
        });
    }
}

// ── RPC worker stream ─────────────────────────────────────────────────────────

/// The serialized RPC worker subscription stream.
///
/// Emits [`Message::RpcWorkerReady`] once on startup with the channel sender.
/// Processes work items one-at-a-time and emits result messages back into the
/// iced update cycle, guaranteeing at most one in-flight HTTP connection.
pub fn rpc_worker_stream() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(32, async |mut output| {
        let (tx, mut rx) = mpsc::channel::<RpcWork>(32);
        let _ = output.send(Message::RpcWorkerReady(tx)).await;
        loop {
            let Some(work) = rx.recv().await else {
                std::future::pending::<()>().await;
                unreachable!()
            };
            let (new_sid, result) = crate::rpc::execute_work(work).await;
            if let Some(new_id) = new_sid {
                let _ = output.send(Message::SessionIdRotated(new_id)).await;
            }
            let msg = match result {
                crate::rpc::RpcResult::TorrentsLoaded(r) => {
                    Message::TorrentsUpdated(r.map_err(|e| e.to_string()))
                }
                crate::rpc::RpcResult::ActionDone(r) => {
                    Message::ActionCompleted(r.map_err(|e| e.to_string()))
                }
                crate::rpc::RpcResult::TorrentAdded(r) => {
                    Message::AddCompleted(r.map_err(|e| e.to_string()))
                }
            };
            let _ = output.send(msg).await;
        }
    })
}

// ── Elm functions ─────────────────────────────────────────────────────────────

pub fn update(state: &mut TorrentListScreen, msg: Message) -> Task<Message> {
    match msg {
        // ── Polling ───────────────────────────────────────────────────────────
        Message::Tick => {
            if state.is_loading {
                tracing::debug!("Tick skipped: RPC call already in-flight");
                return Task::none();
            }
            tracing::debug!("Tick: queuing torrent-get");
            state.is_loading = true;
            state.enqueue_torrent_get();
            Task::none()
        }

        Message::TorrentsUpdated(Ok(torrents)) => {
            tracing::info!(count = torrents.len(), "Torrent list refreshed");
            state.torrents = torrents;
            state.is_loading = false;
            state.error = None;
            Task::none()
        }

        Message::TorrentsUpdated(Err(err)) => {
            tracing::error!(error = %err, "torrent-get failed");
            state.is_loading = false;
            state.error = Some(err);
            Task::none()
        }

        Message::SessionIdRotated(new_id) => {
            tracing::debug!(%new_id, "Persistent session ID updated after rotation");
            state.session_id = new_id;
            Task::none()
        }

        Message::RpcWorkerReady(tx) => {
            tracing::debug!("RPC worker ready, accepting work");
            state.sender = Some(tx);
            Task::none()
        }

        // ── Row selection ─────────────────────────────────────────────────────
        Message::TorrentSelected(id) => {
            state.selected_id = if state.selected_id == Some(id) {
                None
            } else {
                Some(id)
            };
            Task::none()
        }

        // ── Toolbar actions ───────────────────────────────────────────────────
        Message::PauseClicked => {
            if let Some(id) = state.selected_id {
                tracing::info!(id, "Pausing torrent");
                state.is_loading = true;
                state.enqueue(RpcWork::TorrentStop {
                    url: state.credentials.rpc_url(),
                    credentials: state.credentials.clone(),
                    session_id: state.session_id.clone(),
                    id,
                });
            }
            Task::none()
        }

        Message::ResumeClicked => {
            if let Some(id) = state.selected_id {
                tracing::info!(id, "Resuming torrent");
                state.is_loading = true;
                state.enqueue(RpcWork::TorrentStart {
                    url: state.credentials.rpc_url(),
                    credentials: state.credentials.clone(),
                    session_id: state.session_id.clone(),
                    id,
                });
            }
            Task::none()
        }

        Message::DeleteClicked => {
            if let Some(id) = state.selected_id {
                state.confirming_delete = Some((id, false));
            }
            Task::none()
        }

        Message::DeleteLocalDataToggled(val) => {
            if let Some((id, _)) = state.confirming_delete {
                state.confirming_delete = Some((id, val));
            }
            Task::none()
        }

        Message::DeleteCancelled => {
            state.confirming_delete = None;
            Task::none()
        }

        Message::DeleteConfirmed => {
            if let Some((id, delete_local_data)) = state.confirming_delete.take() {
                tracing::info!(id, delete_local_data, "Deleting torrent");
                state.is_loading = true;
                state.enqueue(RpcWork::TorrentRemove {
                    url: state.credentials.rpc_url(),
                    credentials: state.credentials.clone(),
                    session_id: state.session_id.clone(),
                    id,
                    delete_local_data,
                });
            }
            Task::none()
        }

        Message::ActionCompleted(Ok(())) => {
            tracing::info!("Torrent action completed, refreshing list");
            state.enqueue_torrent_get();
            Task::none()
        }

        Message::ActionCompleted(Err(err)) => {
            tracing::error!(error = %err, "Torrent action failed");
            state.is_loading = false;
            state.error = Some(err);
            Task::none()
        }

        // ── Add-torrent dialog ────────────────────────────────────────────────
        Message::AddTorrentClicked => Task::perform(
            async {
                let handle = rfd::AsyncFileDialog::new()
                    .add_filter("Torrent", &["torrent"])
                    .pick_file()
                    .await;
                let Some(handle) = handle else {
                    return Err("cancelled".to_owned());
                };
                let bytes = handle.read().await;
                let b64 = base64::prelude::BASE64_STANDARD.encode(&bytes);
                let torrent = lava_torrent::torrent::v1::Torrent::read_from_bytes(&bytes)
                    .map_err(|e| e.to_string())?;
                let files = match &torrent.files {
                    Some(files) => files
                        .iter()
                        .map(|f| TorrentFileInfo {
                            path: f.path.to_string_lossy().into_owned(),
                            size_bytes: f.length as u64,
                        })
                        .collect(),
                    None => vec![TorrentFileInfo {
                        path: torrent.name.clone(),
                        size_bytes: torrent.length as u64,
                    }],
                };
                Ok(FileReadResult {
                    metainfo_b64: b64,
                    files,
                })
            },
            Message::TorrentFileRead,
        ),

        Message::TorrentFileRead(Ok(result)) => {
            state.add_dialog = AddDialogState::AddFile {
                metainfo_b64: result.metainfo_b64,
                files: result.files,
                destination: String::new(),
                error: None,
            };
            Task::none()
        }

        Message::TorrentFileRead(Err(err)) => {
            if err != "cancelled" {
                tracing::error!(error = %err, "Failed to read torrent file");
                state.error = Some(format!("Could not open torrent file: {err}"));
            }
            Task::none()
        }

        Message::AddLinkClicked => {
            state.add_dialog = AddDialogState::AddLink {
                magnet: String::new(),
                destination: String::new(),
                error: None,
            };
            Task::none()
        }

        Message::AddDialogMagnetChanged(val) => {
            if let AddDialogState::AddLink { magnet, .. } = &mut state.add_dialog {
                *magnet = val;
            }
            Task::none()
        }

        Message::AddDialogDestinationChanged(val) => {
            match &mut state.add_dialog {
                AddDialogState::AddLink { destination, .. } => *destination = val,
                AddDialogState::AddFile { destination, .. } => *destination = val,
                AddDialogState::Hidden => {}
            }
            Task::none()
        }

        Message::AddCancelled => {
            state.add_dialog = AddDialogState::Hidden;
            Task::none()
        }

        Message::AddConfirmed => {
            let (payload, download_dir) = match &state.add_dialog {
                AddDialogState::AddLink {
                    magnet,
                    destination,
                    ..
                } => {
                    if magnet.trim().is_empty() {
                        return Task::none();
                    }
                    (
                        AddPayload::Magnet(magnet.clone()),
                        Some(destination.clone()),
                    )
                }
                AddDialogState::AddFile {
                    metainfo_b64,
                    destination,
                    ..
                } => (
                    AddPayload::Metainfo(metainfo_b64.clone()),
                    Some(destination.clone()),
                ),
                AddDialogState::Hidden => return Task::none(),
            };
            state.is_loading = true;
            tracing::info!("Submitting torrent-add");
            state.enqueue(RpcWork::TorrentAdd {
                url: state.credentials.rpc_url(),
                credentials: state.credentials.clone(),
                session_id: state.session_id.clone(),
                payload,
                download_dir,
            });
            Task::none()
        }

        Message::AddCompleted(Ok(())) => {
            tracing::info!("torrent-add succeeded, refreshing list");
            state.add_dialog = AddDialogState::Hidden;
            state.is_loading = true;
            state.enqueue_torrent_get();
            Task::none()
        }

        Message::AddCompleted(Err(err)) => {
            tracing::error!(error = %err, "torrent-add failed");
            state.is_loading = false;
            match &mut state.add_dialog {
                AddDialogState::AddLink { error, .. } => *error = Some(err),
                AddDialogState::AddFile { error, .. } => *error = Some(err),
                AddDialogState::Hidden => state.error = Some(err),
            }
            Task::none()
        }

        // Disconnect is intercepted by the parent before reaching here.
        Message::Disconnect => Task::none(),
    }
}

pub fn view(state: &TorrentListScreen) -> Element<'_, Message> {
    // ── Toolbar ───────────────────────────────────────────────────────────────
    let toolbar: Element<Message> = if let Some((del_id, del_local)) = state.confirming_delete {
        let name = state
            .torrents
            .iter()
            .find(|t| t.id == del_id)
            .map(|t| t.name.as_str())
            .unwrap_or("this torrent");
        row![
            text(format!("Delete \"{}\"?", name)),
            checkbox(del_local)
                .label("Delete local data")
                .on_toggle(Message::DeleteLocalDataToggled),
            button("Confirm Delete")
                .on_press(Message::DeleteConfirmed)
                .style(iced::widget::button::danger),
            button("Cancel")
                .on_press(Message::DeleteCancelled)
                .style(iced::widget::button::secondary),
            Space::new(),
            button("Disconnect").on_press(Message::Disconnect),
        ]
        .spacing(8)
        .into()
    } else {
        let selected = state
            .selected_id
            .and_then(|id| state.torrents.iter().find(|t| t.id == id));
        let can_pause = selected.is_some_and(|t| matches!(t.status, 3..=6));
        let can_resume = selected.is_some_and(|t| t.status == 0);
        let can_delete = state.selected_id.is_some();

        let pause_btn = {
            let b = button("Pause").style(iced::widget::button::secondary);
            if can_pause {
                b.on_press(Message::PauseClicked)
            } else {
                b
            }
        };
        let resume_btn = {
            let b = button("Resume").style(iced::widget::button::secondary);
            if can_resume {
                b.on_press(Message::ResumeClicked)
            } else {
                b
            }
        };
        let delete_btn = {
            let b = button("Delete").style(iced::widget::button::secondary);
            if can_delete {
                b.on_press(Message::DeleteClicked)
            } else {
                b
            }
        };
        row![
            button("Add Torrent")
                .on_press(Message::AddTorrentClicked)
                .style(iced::widget::button::secondary),
            button("Add Link")
                .on_press(Message::AddLinkClicked)
                .style(iced::widget::button::secondary),
            pause_btn,
            resume_btn,
            delete_btn,
            Space::new(),
            button("Disconnect").on_press(Message::Disconnect),
        ]
        .spacing(8)
        .into()
    };

    // ── Inline error banner ───────────────────────────────────────────────────
    let error_row: Element<Message> = if let Some(err) = &state.error {
        text(format!("⚠ {err}")).into()
    } else {
        Space::new().into()
    };

    // ── Sticky header ─────────────────────────────────────────────────────────
    let header = row![
        text("Name").width(Length::FillPortion(COL_NAME)),
        text("Status").width(Length::FillPortion(COL_STATUS)),
        text("Progress").width(Length::FillPortion(COL_PROGRESS)),
    ]
    .padding(4);

    // ── Data rows ─────────────────────────────────────────────────────────────
    let rows = state.torrents.iter().map(|t| {
        let row_content = row![
            text(&t.name)
                .width(Length::FillPortion(COL_NAME))
                .wrapping(text::Wrapping::WordOrGlyph),
            text(status_label(t.status)).width(Length::FillPortion(COL_STATUS)),
            progress_bar(0.0..=1.0, t.percent_done as f32)
                .length(Length::FillPortion(COL_PROGRESS))
                .girth(14.0),
        ]
        .padding(4)
        .align_y(iced::Center);

        button(row_content)
            .on_press(Message::TorrentSelected(t.id))
            .style(if state.selected_id == Some(t.id) {
                iced::widget::button::primary
            } else {
                iced::widget::button::text
            })
            .width(Length::Fill)
            .into()
    });

    let list = scrollable(column(rows).spacing(2));

    let main_content: Element<Message> = container(
        column![toolbar, error_row, header, list]
            .spacing(4)
            .padding(8)
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .into();

    // ── Modal overlay ─────────────────────────────────────────────────────────
    match &state.add_dialog {
        AddDialogState::Hidden => main_content,
        dialog_state => stack![main_content, view_add_dialog(dialog_state)].into(),
    }
}

fn view_add_dialog(state: &AddDialogState) -> Element<'_, Message> {
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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::TorrentData;

    fn make_screen() -> TorrentListScreen {
        TorrentListScreen::new(
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
        TorrentData {
            id,
            name: name.to_owned(),
            status: 6,
            percent_done: 1.0,
            ..Default::default()
        }
    }

    fn stopped(id: i64) -> TorrentData {
        TorrentData {
            id,
            name: format!("torrent-{id}"),
            status: 0,
            percent_done: 0.5,
            ..Default::default()
        }
    }

    fn downloading(id: i64) -> TorrentData {
        TorrentData {
            id,
            name: format!("torrent-{id}"),
            status: 4,
            percent_done: 0.5,
            ..Default::default()
        }
    }

    /// 6.1 – Tick when is_loading=true: no command, state unchanged.
    #[test]
    fn tick_ignored_when_loading() {
        let mut screen = make_screen();
        screen.is_loading = true;
        let cmd = update(&mut screen, Message::Tick);
        assert!(screen.is_loading);
        drop(cmd);
    }

    /// 6.2 – Tick when is_loading=false: is_loading becomes true.
    #[test]
    fn tick_fires_when_not_loading() {
        let mut screen = make_screen();
        screen.is_loading = false;
        let cmd = update(&mut screen, Message::Tick);
        assert!(
            screen.is_loading,
            "is_loading should be set to true after Tick"
        );
        drop(cmd);
    }

    /// 6.3 – TorrentsUpdated(Ok) replaces torrents and clears is_loading.
    #[test]
    fn torrents_updated_ok_replaces_and_clears_loading() {
        let mut screen = make_screen();
        screen.is_loading = true;
        let new_data = vec![make_torrent(1, "Ubuntu ISO"), make_torrent(2, "Arch Linux")];
        let _ = update(&mut screen, Message::TorrentsUpdated(Ok(new_data)));
        assert!(!screen.is_loading);
        assert_eq!(screen.torrents.len(), 2);
        assert_eq!(screen.torrents[0].name, "Ubuntu ISO");
    }

    /// 6.4 – TorrentsUpdated(Err) clears is_loading and sets error.
    #[test]
    fn torrents_updated_err_clears_loading_and_sets_error() {
        let mut screen = make_screen();
        screen.is_loading = true;
        let _ = update(
            &mut screen,
            Message::TorrentsUpdated(Err("timeout".to_owned())),
        );
        assert!(!screen.is_loading);
        assert_eq!(screen.error.as_deref(), Some("timeout"));
    }

    /// 6.5 – SessionIdRotated updates the stored session id.
    #[test]
    fn session_id_rotated_updates_id() {
        let mut screen = make_screen();
        let _ = update(
            &mut screen,
            Message::SessionIdRotated("new-id-xyz".to_owned()),
        );
        assert_eq!(screen.session_id, "new-id-xyz");
    }

    // ── 8.x  v0.2 selection and action tests ─────────────────────────────────

    /// 8.1 – TorrentSelected toggles selected_id correctly.
    #[test]
    fn torrent_selected_toggles() {
        let mut screen = make_screen();
        screen.torrents = vec![make_torrent(1, "A"), make_torrent(2, "B")];

        let _ = update(&mut screen, Message::TorrentSelected(1));
        assert_eq!(screen.selected_id, Some(1));

        let _ = update(&mut screen, Message::TorrentSelected(2));
        assert_eq!(screen.selected_id, Some(2));

        let _ = update(&mut screen, Message::TorrentSelected(2));
        assert_eq!(screen.selected_id, None);
    }

    /// 12.1 – selected_torrent() returns the matching TorrentData when selected.
    #[test]
    fn selected_torrent_returns_correct_entry() {
        let mut screen = make_screen();
        screen.torrents = vec![make_torrent(1, "Alpha"), make_torrent(2, "Beta")];
        screen.selected_id = Some(2);
        let t = screen.selected_torrent().expect("should have a selection");
        assert_eq!(t.id, 2);
        assert_eq!(t.name, "Beta");
    }

    /// 12.1b – selected_torrent() returns None when nothing is selected.
    #[test]
    fn selected_torrent_none_when_no_selection() {
        let screen = make_screen();
        assert!(screen.selected_torrent().is_none());
    }

    /// 8.2 – Button enable/disable logic (status conditions).
    #[test]
    fn button_enable_state_logic() {
        for status in [3i32, 4, 5, 6] {
            assert!(
                matches!(status, 3 | 4 | 5 | 6),
                "status {status} should allow pause"
            );
        }
        for status in [0i32, 1, 2] {
            assert!(
                !matches!(status, 3 | 4 | 5 | 6),
                "status {status} should not allow pause"
            );
        }
        assert_eq!(0i32, 0, "status 0 should allow resume");
    }

    /// 8.3 – DeleteClicked sets confirming_delete.
    #[test]
    fn delete_clicked_sets_confirming() {
        let mut screen = make_screen();
        screen.torrents = vec![make_torrent(5, "Arch")];
        screen.selected_id = Some(5);
        let task = update(&mut screen, Message::DeleteClicked);
        assert_eq!(screen.confirming_delete, Some((5, false)));
        assert!(!screen.is_loading);
        drop(task);
    }

    /// 8.3 – DeleteClicked when nothing is selected is a no-op.
    #[test]
    fn delete_clicked_no_selection_is_noop() {
        let mut screen = make_screen();
        let _ = update(&mut screen, Message::DeleteClicked);
        assert_eq!(screen.confirming_delete, None);
    }

    /// 8.4 – DeleteCancelled clears confirming_delete.
    #[test]
    fn delete_cancelled_clears_confirming() {
        let mut screen = make_screen();
        screen.confirming_delete = Some((3, true));
        let _ = update(&mut screen, Message::DeleteCancelled);
        assert_eq!(screen.confirming_delete, None);
    }

    /// 8.5 – DeleteConfirmed fires a task and clears confirming_delete.
    #[test]
    fn delete_confirmed_clears_confirming_and_loads() {
        let mut screen = make_screen();
        screen.confirming_delete = Some((7, true));
        let task = update(&mut screen, Message::DeleteConfirmed);
        assert_eq!(screen.confirming_delete, None);
        assert!(screen.is_loading);
        drop(task);
    }

    /// 8.5 – DeleteConfirmed when confirming_delete is None is a no-op.
    #[test]
    fn delete_confirmed_no_state_is_noop() {
        let mut screen = make_screen();
        let _ = update(&mut screen, Message::DeleteConfirmed);
        assert!(!screen.is_loading);
    }

    /// 8.6 – DeleteLocalDataToggled updates checkbox state.
    #[test]
    fn delete_local_data_toggled_updates_state() {
        let mut screen = make_screen();
        screen.confirming_delete = Some((9, false));
        let _ = update(&mut screen, Message::DeleteLocalDataToggled(true));
        assert_eq!(screen.confirming_delete, Some((9, true)));
        let _ = update(&mut screen, Message::DeleteLocalDataToggled(false));
        assert_eq!(screen.confirming_delete, Some((9, false)));
    }

    /// 8.7 – ActionCompleted(Ok) keeps is_loading=true and fires a poll task.
    #[test]
    fn action_completed_ok_fires_refresh() {
        let mut screen = make_screen();
        screen.is_loading = true;
        let task = update(&mut screen, Message::ActionCompleted(Ok(())));
        assert!(screen.is_loading);
        drop(task);
    }

    /// 8.8 – ActionCompleted(Err) clears is_loading and stores error.
    #[test]
    fn action_completed_err_clears_and_stores_error() {
        let mut screen = make_screen();
        screen.is_loading = true;
        let _ = update(
            &mut screen,
            Message::ActionCompleted(Err("daemon refused".to_owned())),
        );
        assert!(!screen.is_loading);
        assert_eq!(screen.error.as_deref(), Some("daemon refused"));
    }

    /// 8.9 – Poll tick is ignored while is_loading is true.
    #[test]
    fn tick_ignored_while_action_in_flight() {
        let mut screen = make_screen();
        screen.is_loading = true;
        screen.selected_id = Some(1);
        let task = update(&mut screen, Message::Tick);
        assert!(screen.is_loading);
        drop(task);
    }

    #[allow(dead_code)]
    fn _use_helpers() {
        let _ = stopped(1);
        let _ = downloading(1);
    }

    // ── 9.x  v0.3 add-torrent dialog tests ───────────────────────────────────

    /// 9.1 – AddLinkClicked transitions add_dialog to AddLink.
    #[test]
    fn add_link_clicked_opens_add_link_dialog() {
        let mut screen = make_screen();
        let _ = update(&mut screen, Message::AddLinkClicked);
        assert!(matches!(screen.add_dialog, AddDialogState::AddLink { .. }));
    }

    /// 9.2 – AddConfirmed with empty magnet is a no-op.
    #[test]
    fn add_confirmed_empty_magnet_is_noop() {
        let mut screen = make_screen();
        screen.add_dialog = AddDialogState::AddLink {
            magnet: String::new(),
            destination: String::new(),
            error: None,
        };
        let task = update(&mut screen, Message::AddConfirmed);
        assert!(matches!(screen.add_dialog, AddDialogState::AddLink { .. }));
        drop(task);
    }

    /// 9.3 – AddConfirmed with a valid magnet sets is_loading=true.
    #[test]
    fn add_confirmed_valid_magnet_emits_task() {
        let mut screen = make_screen();
        screen.add_dialog = AddDialogState::AddLink {
            magnet: "magnet:?xt=urn:btih:abc123".to_owned(),
            destination: String::new(),
            error: None,
        };
        let task = update(&mut screen, Message::AddConfirmed);
        assert!(
            screen.is_loading,
            "is_loading should be true while torrent-add is in-flight"
        );
        assert!(matches!(screen.add_dialog, AddDialogState::AddLink { .. }));
        drop(task);
    }

    /// 9.4 – AddCancelled resets add_dialog to Hidden.
    #[test]
    fn add_cancelled_closes_dialog() {
        let mut screen = make_screen();
        screen.add_dialog = AddDialogState::AddLink {
            magnet: "magnet:?xt=urn:btih:abc".to_owned(),
            destination: String::new(),
            error: None,
        };
        let _ = update(&mut screen, Message::AddCancelled);
        assert!(matches!(screen.add_dialog, AddDialogState::Hidden));
    }

    /// 9.5 – TorrentFileRead(Ok) opens AddFile dialog with the correct file list.
    #[test]
    fn torrent_file_read_ok_opens_add_file_dialog() {
        let mut screen = make_screen();
        let result = FileReadResult {
            metainfo_b64: "dGVzdA==".to_owned(),
            files: vec![TorrentFileInfo {
                path: "movie.mkv".to_owned(),
                size_bytes: 1_073_741_824,
            }],
        };
        let _ = update(&mut screen, Message::TorrentFileRead(Ok(result)));
        match &screen.add_dialog {
            AddDialogState::AddFile { files, .. } => {
                assert_eq!(files.len(), 1);
                assert_eq!(files[0].path, "movie.mkv");
            }
            other => panic!("expected AddFile, got {other:?}"),
        }
    }

    /// 9.6 – AddCompleted(Ok) clears the dialog and fires an immediate torrent-get.
    #[test]
    fn add_completed_ok_clears_dialog_and_polls() {
        let mut screen = make_screen();
        screen.add_dialog = AddDialogState::AddLink {
            magnet: "magnet:?xt=urn:btih:abc".to_owned(),
            destination: String::new(),
            error: None,
        };
        let task = update(&mut screen, Message::AddCompleted(Ok(())));
        assert!(matches!(screen.add_dialog, AddDialogState::Hidden));
        assert!(screen.is_loading);
        drop(task);
    }

    /// 9.7 – AddCompleted(Err) stores the error inside the dialog without closing it.
    #[test]
    fn add_completed_err_stores_error_in_dialog() {
        let mut screen = make_screen();
        screen.add_dialog = AddDialogState::AddLink {
            magnet: "magnet:?xt=urn:btih:abc".to_owned(),
            destination: String::new(),
            error: None,
        };
        let _ = update(
            &mut screen,
            Message::AddCompleted(Err("daemon error".to_owned())),
        );
        match &screen.add_dialog {
            AddDialogState::AddLink { error, .. } => {
                assert_eq!(error.as_deref(), Some("daemon error"));
            }
            other => panic!("expected AddLink dialog still open, got {other:?}"),
        }
    }
}
