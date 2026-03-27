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
use iced::widget::rule;
use iced::widget::tooltip;
use iced::widget::{
    Space, button, checkbox, column, container, progress_bar, row, scrollable, stack, text,
    text_input,
};
use iced::{Alignment, Element, Length, Task};
use tokio::sync::mpsc;

use crate::format::{format_eta, format_size, format_speed};
use crate::rpc::{AddPayload, RpcWork, TorrentData, TransmissionCredentials};
use crate::theme::{
    ICON_ADD, ICON_DARK_MODE, ICON_DELETE, ICON_DOWNLOAD, ICON_LIGHT_MODE, ICON_LINK, ICON_LOGOUT,
    ICON_PAUSE, ICON_PLAY, ICON_UPLOAD, icon, progress_bar_style,
};

// ── Column layout ─────────────────────────────────────────────────────────────

// Fixed pixel widths for narrow numeric columns (design D9).
const W_STATUS: f32 = 90.0;
const W_SIZE: f32 = 80.0;
const W_SPEED_DOWN: f32 = 90.0;
const W_SPEED_UP: f32 = 90.0;
const W_ETA: f32 = 80.0;
const W_RATIO: f32 = 64.0;
const W_PROGRESS: f32 = 130.0;
const SCROLLBAR_WIDTH: f32 = 14.0;
// Name column uses Length::Fill.

// ── Sort state ────────────────────────────────────────────────────────────────

/// Column that the torrent list is currently sorted by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    Name,
    Status,
    Size,
    SpeedDown,
    SpeedUp,
    Eta,
    Ratio,
    Progress,
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortDir {
    #[default]
    Asc,
    Desc,
}

/// Apply a single-column sort over a slice of torrents.
///
/// Returns references in the requested order; the backing slice is unchanged.
pub fn sort_torrents(torrents: &[TorrentData], col: SortColumn, dir: SortDir) -> Vec<&TorrentData> {
    let mut sorted: Vec<&TorrentData> = torrents.iter().collect();
    sorted.sort_by(|a, b| {
        let ord = match col {
            SortColumn::Name => a.name.cmp(&b.name),
            SortColumn::Status => a.status.cmp(&b.status),
            SortColumn::Size => a.total_size.cmp(&b.total_size),
            SortColumn::SpeedDown => a.rate_download.cmp(&b.rate_download),
            SortColumn::SpeedUp => a.rate_upload.cmp(&b.rate_upload),
            SortColumn::Eta => {
                // -1 = unknown; sort unknown ETAs to the end.
                let ea = if a.eta < 0 { i64::MAX } else { a.eta };
                let eb = if b.eta < 0 { i64::MAX } else { b.eta };
                ea.cmp(&eb)
            }
            SortColumn::Ratio => {
                let ra = a.upload_ratio.max(0.0);
                let rb = b.upload_ratio.max(0.0);
                ra.partial_cmp(&rb).unwrap_or(std::cmp::Ordering::Equal)
            }
            SortColumn::Progress => a
                .percent_done
                .partial_cmp(&b.percent_done)
                .unwrap_or(std::cmp::Ordering::Equal),
        };
        if dir == SortDir::Desc {
            ord.reverse()
        } else {
            ord
        }
    });
    sorted
}

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
    // Escalated to app — intercepted by app::update via MainScreen
    ThemeToggled,
    // Column sort
    ColumnHeaderClicked(SortColumn),
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
    /// Active sort column (if any).
    pub sort_column: Option<SortColumn>,
    /// Sort direction (Asc when `sort_column` is None, irrelevant).
    pub sort_dir: SortDir,
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
            sort_column: None,
            sort_dir: SortDir::Asc,
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
        // ThemeToggled is intercepted by app::update, never reaches here.
        Message::ThemeToggled => Task::none(),

        // ── Column sort ───────────────────────────────────────────────────────
        Message::ColumnHeaderClicked(col) => {
            match &state.sort_column {
                // Same column: cycle Asc → Desc → None
                Some(current) if *current == col => match state.sort_dir {
                    SortDir::Asc => state.sort_dir = SortDir::Desc,
                    SortDir::Desc => state.sort_column = None,
                },
                // Different column or no sort: start ascending on the clicked column
                _ => {
                    state.sort_column = Some(col);
                    state.sort_dir = SortDir::Asc;
                }
            }
            Task::none()
        }
    }
}

pub fn view(state: &TorrentListScreen, theme_mode: crate::app::ThemeMode) -> Element<'_, Message> {
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

        let theme_icon = if theme_mode == crate::app::ThemeMode::Dark {
            ICON_LIGHT_MODE
        } else {
            ICON_DARK_MODE
        };
        let theme_hint = if theme_mode == crate::app::ThemeMode::Dark {
            "Switch to light mode"
        } else {
            "Switch to dark mode"
        };
        // Determine which secondary button style to use — dim in dark mode.
        let sec_style: fn(
            &iced::Theme,
            iced::widget::button::Status,
        ) -> iced::widget::button::Style = if theme_mode == crate::app::ThemeMode::Dark {
            crate::theme::dim_secondary
        } else {
            iced::widget::button::secondary
        };

        // ── Group 1: Add actions ──────────────────────────────────────────────
        let group1: Element<Message> = row![
            tooltip(
                button(icon(ICON_ADD))
                    .on_press(Message::AddTorrentClicked)
                    .style(iced::widget::button::primary)
                    .padding([4, 6]),
                text("Add torrent from file"),
                tooltip::Position::Bottom,
            )
            .gap(6)
            .style(container::rounded_box),
            tooltip(
                button(icon(ICON_LINK))
                    .on_press(Message::AddLinkClicked)
                    .style(sec_style)
                    .padding([4, 6]),
                text("Add torrent from magnet link"),
                tooltip::Position::Bottom,
            )
            .gap(6)
            .style(container::rounded_box),
        ]
        .spacing(4)
        .into();

        // ── Group 2: Torrent actions ──────────────────────────────────────────
        let pause_btn = {
            let b = button(icon(ICON_PAUSE)).style(sec_style).padding([4, 6]);
            if can_pause {
                b.on_press(Message::PauseClicked)
            } else {
                b
            }
        };
        let resume_btn = {
            let b = button(icon(ICON_PLAY)).style(sec_style).padding([4, 6]);
            if can_resume {
                b.on_press(Message::ResumeClicked)
            } else {
                b
            }
        };
        let delete_btn = {
            let b = button(icon(ICON_DELETE)).style(sec_style).padding([4, 6]);
            if can_delete {
                b.on_press(Message::DeleteClicked)
            } else {
                b
            }
        };
        let group2: Element<Message> = row![
            tooltip(pause_btn, text("Pause"), tooltip::Position::Bottom)
                .gap(6)
                .style(container::rounded_box),
            tooltip(resume_btn, text("Resume"), tooltip::Position::Bottom)
                .gap(6)
                .style(container::rounded_box),
            tooltip(delete_btn, text("Delete"), tooltip::Position::Bottom)
                .gap(6)
                .style(container::rounded_box),
        ]
        .spacing(4)
        .into();

        // ── Group 3: Global / right-aligned ───────────────────────────────────
        let group3: Element<Message> = row![
            tooltip(
                button(icon(theme_icon))
                    .on_press(Message::ThemeToggled)
                    .style(sec_style)
                    .padding([4, 6]),
                text(theme_hint),
                tooltip::Position::Bottom,
            )
            .gap(6)
            .style(container::rounded_box),
            tooltip(
                button(icon(ICON_LOGOUT))
                    .on_press(Message::Disconnect)
                    .style(sec_style)
                    .padding([4, 6]),
                text("Disconnect from daemon"),
                tooltip::Position::Bottom,
            )
            .gap(6)
            .style(container::rounded_box),
        ]
        .spacing(4)
        .into();

        row![
            group1,
            Space::new().width(16),
            group2,
            Space::new().width(Length::Fill),
            group3
        ]
        .align_y(Alignment::Center)
        .spacing(0)
        .into()
    };

    // ── Inline error banner ───────────────────────────────────────────────────
    let error_row: Element<Message> = if let Some(err) = &state.error {
        text(format!("⚠ {err}")).into()
    } else {
        Space::new().into()
    };

    // ── Sticky header ─────────────────────────────────────────────────────────
    let header_row = row![
        container(
            tooltip(
                col_header_btn(
                    "NAME",
                    SortColumn::Name,
                    &state.sort_column,
                    state.sort_dir,
                    Alignment::Start,
                )
                .width(Length::Fill),
                text("Sort by name"),
                tooltip::Position::Bottom,
            )
            .gap(6)
            .style(container::rounded_box),
        )
        .width(Length::Fill),
        container(
            tooltip(
                col_header_btn(
                    "STATUS",
                    SortColumn::Status,
                    &state.sort_column,
                    state.sort_dir,
                    Alignment::Start,
                )
                .width(Length::Fixed(W_STATUS)),
                text("Sort by status"),
                tooltip::Position::Bottom,
            )
            .gap(6)
            .style(container::rounded_box),
        )
        .width(Length::Fixed(W_STATUS)),
        container(
            tooltip(
                col_header_btn(
                    "SIZE",
                    SortColumn::Size,
                    &state.sort_column,
                    state.sort_dir,
                    Alignment::End,
                )
                .width(Length::Fixed(W_SIZE)),
                text("Total size"),
                tooltip::Position::Bottom,
            )
            .gap(6)
            .style(container::rounded_box),
        )
        .width(Length::Fixed(W_SIZE)),
        container(
            tooltip(
                col_header_icon_btn(
                    ICON_DOWNLOAD,
                    SortColumn::SpeedDown,
                    &state.sort_column,
                    state.sort_dir,
                    Alignment::End,
                )
                .width(Length::Fixed(W_SPEED_DOWN)),
                text("Download speed"),
                tooltip::Position::Bottom,
            )
            .gap(6)
            .style(container::rounded_box),
        )
        .width(Length::Fixed(W_SPEED_DOWN)),
        container(
            tooltip(
                col_header_icon_btn(
                    ICON_UPLOAD,
                    SortColumn::SpeedUp,
                    &state.sort_column,
                    state.sort_dir,
                    Alignment::End,
                )
                .width(Length::Fixed(W_SPEED_UP)),
                text("Upload speed"),
                tooltip::Position::Bottom,
            )
            .gap(6)
            .style(container::rounded_box),
        )
        .width(Length::Fixed(W_SPEED_UP)),
        container(
            tooltip(
                col_header_btn(
                    "ETA",
                    SortColumn::Eta,
                    &state.sort_column,
                    state.sort_dir,
                    Alignment::End,
                )
                .width(Length::Fixed(W_ETA)),
                text("Estimated time remaining"),
                tooltip::Position::Bottom,
            )
            .gap(6)
            .style(container::rounded_box),
        )
        .width(Length::Fixed(W_ETA)),
        container(
            tooltip(
                col_header_btn(
                    "RATIO",
                    SortColumn::Ratio,
                    &state.sort_column,
                    state.sort_dir,
                    Alignment::End,
                )
                .width(Length::Fixed(W_RATIO)),
                text("Upload/download ratio"),
                tooltip::Position::Bottom,
            )
            .gap(6)
            .style(container::rounded_box),
        )
        .width(Length::Fixed(W_RATIO)),
        container(
            tooltip(
                col_header_btn(
                    "PROGRESS",
                    SortColumn::Progress,
                    &state.sort_column,
                    state.sort_dir,
                    Alignment::End,
                )
                .width(Length::Fixed(W_PROGRESS)),
                text("Percent complete"),
                tooltip::Position::Bottom,
            )
            .gap(6)
            .style(container::rounded_box),
        )
        .width(Length::Fixed(W_PROGRESS)),
    ]
    .spacing(16);

    let header = container(header_row).padding(iced::Padding {
        top: 0.0,
        bottom: 0.0,
        left: 0.0,
        right: SCROLLBAR_WIDTH + 2.0,
    });

    // ── Data rows ─────────────────────────────────────────────────────────────
    let display: Vec<&TorrentData> = match state.sort_column {
        Some(col) => sort_torrents(&state.torrents, col, state.sort_dir),
        None => state.torrents.iter().collect(),
    };

    let rows = display.into_iter().map(|t| {
        let ratio_str = if t.upload_ratio < 0.0 {
            "—".to_owned()
        } else {
            format!("{:.2}", t.upload_ratio)
        };

        let row_content = row![
            text(&t.name)
                .width(Length::Fill)
                .align_x(Alignment::Start)
                .wrapping(text::Wrapping::WordOrGlyph),
            text(status_label(t.status))
                .width(Length::Fixed(W_STATUS))
                .align_x(Alignment::Start),
            text(format_size(t.total_size))
                .width(Length::Fixed(W_SIZE))
                .align_x(Alignment::End),
            text(format_speed(t.rate_download))
                .width(Length::Fixed(W_SPEED_DOWN))
                .align_x(Alignment::End),
            text(format_speed(t.rate_upload))
                .width(Length::Fixed(W_SPEED_UP))
                .align_x(Alignment::End),
            text(format_eta(t.eta))
                .width(Length::Fixed(W_ETA))
                .align_x(Alignment::End),
            text(ratio_str)
                .width(Length::Fixed(W_RATIO))
                .align_x(Alignment::End),
            container(
                row![
                    progress_bar(0.0..=1.0, t.percent_done as f32)
                        .style(progress_bar_style(t.status))
                        .length(Length::Fill)
                        .girth(10.0),
                    text(format!("{:.0}%", t.percent_done * 100.0))
                        .size(11)
                        .width(Length::Fixed(34.0))
                        .align_x(Alignment::End),
                ]
                .spacing(4)
                .align_y(iced::Center),
            )
            .width(Length::Fixed(W_PROGRESS))
            .align_x(Alignment::Start),
        ]
        .spacing(16)
        .width(Length::Fill)
        .padding([8, 0])
        .align_y(iced::Center);

        let is_selected = state.selected_id == Some(t.id);
        let row_elem: Element<Message> = if is_selected {
            container(row_content)
                .style(crate::theme::selected_row)
                .width(Length::Fill)
                .into()
        } else {
            row_content.into()
        };

        button(row_elem)
            .on_press(Message::TorrentSelected(t.id))
            .style(iced::widget::button::text)
            .width(Length::Fill)
            .padding(0)
            .into()
    });

    let list = scrollable(container(column(rows).spacing(2)).padding(iced::Padding {
        top: 0.0,
        bottom: 0.0,
        left: 0.0,
        right: SCROLLBAR_WIDTH + 2.0,
    }))
    .direction(iced::widget::scrollable::Direction::Vertical(
        iced::widget::scrollable::Scrollbar::new()
            .width(SCROLLBAR_WIDTH)
            .scroller_width(SCROLLBAR_WIDTH)
            .margin(0),
    ));

    let main_content: Element<Message> = container(
        column![toolbar, error_row, header, rule::horizontal(1), list]
            .spacing(4)
            .padding([8, 16])
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

/// Column-header button: label aligns based on parameter, sort chevron sits next to it.
///
/// The text is styled muted + smaller to differentiate from data rows.
fn col_header_btn(
    label: &'static str,
    col: SortColumn,
    active: &Option<SortColumn>,
    dir: SortDir,
    alignment: Alignment,
) -> iced::widget::Button<'static, Message> {
    let chevron = chevron_indicator(col, active, dir);
    let label_elem = text(label)
        .size(11)
        .width(Length::Fill)
        .align_x(alignment)
        .style(|t: &iced::Theme| iced::widget::text::Style {
            color: Some(t.palette().text.scale_alpha(0.55)),
        });
    let chevron_elem = text(chevron)
        .size(14)
        .width(Length::Fixed(14.0))
        .align_x(Alignment::Center)
        .style(|t: &iced::Theme| iced::widget::text::Style {
            color: Some(t.palette().text.scale_alpha(0.55)),
        });

    let content: Element<'static, Message> = if alignment == Alignment::End {
        row![chevron_elem, label_elem]
            .width(Length::Fill)
            .align_y(Alignment::Center)
            .into()
    } else {
        row![label_elem, chevron_elem]
            .width(Length::Fill)
            .align_y(Alignment::Center)
            .into()
    };

    button(content)
        .on_press(Message::ColumnHeaderClicked(col))
        .style(iced::widget::button::text)
        .padding([2, 0])
}

/// Icon-only column-header button (for download/upload speed).
fn col_header_icon_btn(
    glyph: char,
    col: SortColumn,
    active: &Option<SortColumn>,
    dir: SortDir,
    alignment: Alignment,
) -> iced::widget::Button<'static, Message> {
    let chevron = chevron_indicator(col, active, dir);
    let icon_elem = text(String::from(glyph))
        .font(crate::theme::MATERIAL_ICONS)
        .size(14)
        .width(Length::Fill)
        .align_x(alignment)
        .style(|t: &iced::Theme| iced::widget::text::Style {
            color: Some(t.palette().text.scale_alpha(0.55)),
        });
    let chevron_elem = text(chevron)
        .size(14)
        .width(Length::Fixed(14.0))
        .align_x(Alignment::Center)
        .style(|t: &iced::Theme| iced::widget::text::Style {
            color: Some(t.palette().text.scale_alpha(0.55)),
        });

    let content: Element<'static, Message> = if alignment == Alignment::End {
        row![chevron_elem, icon_elem]
            .width(Length::Fill)
            .align_y(Alignment::Center)
            .into()
    } else {
        row![icon_elem, chevron_elem]
            .width(Length::Fill)
            .align_y(Alignment::Center)
            .into()
    };

    button(content)
        .on_press(Message::ColumnHeaderClicked(col))
        .style(iced::widget::button::text)
        .padding([2, 0])
}

fn chevron_indicator(col: SortColumn, active: &Option<SortColumn>, dir: SortDir) -> &'static str {
    match active {
        Some(c) if *c == col => match dir {
            SortDir::Asc => "▴",
            SortDir::Desc => "▾",
        },
        _ => "",
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

    // ── 10.x  v0.5 sort tests ────────────────────────────────────────────────

    fn make_list() -> Vec<TorrentData> {
        vec![
            TorrentData {
                id: 1,
                name: "charlie".into(),
                status: 6,
                total_size: 300,
                rate_download: 30,
                rate_upload: 3,
                eta: 30,
                upload_ratio: 0.3,
                percent_done: 0.3,
                ..Default::default()
            },
            TorrentData {
                id: 2,
                name: "alpha".into(),
                status: 0,
                total_size: 100,
                rate_download: 10,
                rate_upload: 1,
                eta: 10,
                upload_ratio: 0.1,
                percent_done: 0.1,
                ..Default::default()
            },
            TorrentData {
                id: 3,
                name: "bravo".into(),
                status: 4,
                total_size: 200,
                rate_download: 20,
                rate_upload: 2,
                eta: 20,
                upload_ratio: 0.2,
                percent_done: 0.2,
                ..Default::default()
            },
        ]
    }

    /// 10.1 – Empty list returns empty vec for any sort.
    #[test]
    fn sort_empty_list() {
        let torrents: Vec<TorrentData> = vec![];
        assert!(sort_torrents(&torrents, SortColumn::Name, SortDir::Asc).is_empty());
    }

    /// 10.2 – Single-element list is a no-op.
    #[test]
    fn sort_single_element() {
        let torrents = vec![make_torrent(1, "only")];
        let result = sort_torrents(&torrents, SortColumn::Name, SortDir::Asc);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
    }

    /// 10.3 – Ascending sort by Name.
    #[test]
    fn sort_by_name_asc() {
        let list = make_list();
        let result = sort_torrents(&list, SortColumn::Name, SortDir::Asc);
        let names: Vec<&str> = result.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["alpha", "bravo", "charlie"]);
    }

    /// 10.4 – Descending sort by Name.
    #[test]
    fn sort_by_name_desc() {
        let list = make_list();
        let result = sort_torrents(&list, SortColumn::Name, SortDir::Desc);
        let names: Vec<&str> = result.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["charlie", "bravo", "alpha"]);
    }

    /// 10.5 – Ascending sort by Status.
    #[test]
    fn sort_by_status_asc() {
        let list = make_list();
        let result = sort_torrents(&list, SortColumn::Status, SortDir::Asc);
        let ids: Vec<i64> = result.iter().map(|t| t.id).collect();
        // status: 0, 4, 6 → ids 2, 3, 1
        assert_eq!(ids, [2, 3, 1]);
    }

    /// 10.6 – Ascending sort by Size.
    #[test]
    fn sort_by_size_asc() {
        let list = make_list();
        let result = sort_torrents(&list, SortColumn::Size, SortDir::Asc);
        let ids: Vec<i64> = result.iter().map(|t| t.id).collect();
        assert_eq!(ids, [2, 3, 1]);
    }

    /// 10.7 – Ascending sort by SpeedDown.
    #[test]
    fn sort_by_speed_down_asc() {
        let list = make_list();
        let result = sort_torrents(&list, SortColumn::SpeedDown, SortDir::Asc);
        let ids: Vec<i64> = result.iter().map(|t| t.id).collect();
        assert_eq!(ids, [2, 3, 1]);
    }

    /// 10.8 – Ascending sort by SpeedUp.
    #[test]
    fn sort_by_speed_up_asc() {
        let list = make_list();
        let result = sort_torrents(&list, SortColumn::SpeedUp, SortDir::Asc);
        let ids: Vec<i64> = result.iter().map(|t| t.id).collect();
        assert_eq!(ids, [2, 3, 1]);
    }

    /// 10.9 – Ascending sort by ETA.
    #[test]
    fn sort_by_eta_asc() {
        let list = make_list();
        let result = sort_torrents(&list, SortColumn::Eta, SortDir::Asc);
        let ids: Vec<i64> = result.iter().map(|t| t.id).collect();
        assert_eq!(ids, [2, 3, 1]);
    }

    /// 10.10 – Unknown ETA (-1) sorts to end.
    #[test]
    fn sort_by_eta_unknown_last() {
        let mut list = make_list();
        list[0].eta = -1; // id=1 has unknown ETA
        let result = sort_torrents(&list, SortColumn::Eta, SortDir::Asc);
        assert_eq!(result.last().unwrap().id, 1);
    }

    /// 10.11 – Ascending sort by Ratio.
    #[test]
    fn sort_by_ratio_asc() {
        let list = make_list();
        let result = sort_torrents(&list, SortColumn::Ratio, SortDir::Asc);
        let ids: Vec<i64> = result.iter().map(|t| t.id).collect();
        assert_eq!(ids, [2, 3, 1]);
    }

    /// 10.12 – Ascending sort by Progress.
    #[test]
    fn sort_by_progress_asc() {
        let list = make_list();
        let result = sort_torrents(&list, SortColumn::Progress, SortDir::Asc);
        let ids: Vec<i64> = result.iter().map(|t| t.id).collect();
        assert_eq!(ids, [2, 3, 1]);
    }

    /// 10.13 – Descending sort reverses any column (spot-check with Size).
    #[test]
    fn sort_by_size_desc() {
        let list = make_list();
        let result = sort_torrents(&list, SortColumn::Size, SortDir::Desc);
        let ids: Vec<i64> = result.iter().map(|t| t.id).collect();
        assert_eq!(ids, [1, 3, 2]);
    }

    /// 11.1 – ColumnHeaderClicked cycles Unsorted → Asc → Desc → Unsorted.
    #[test]
    fn column_header_clicked_cycles_sort() {
        let mut screen = make_screen();
        assert_eq!(screen.sort_column, None);

        // First click: Asc
        let _ = update(&mut screen, Message::ColumnHeaderClicked(SortColumn::Name));
        assert_eq!(screen.sort_column, Some(SortColumn::Name));
        assert_eq!(screen.sort_dir, SortDir::Asc);

        // Second click: Desc
        let _ = update(&mut screen, Message::ColumnHeaderClicked(SortColumn::Name));
        assert_eq!(screen.sort_column, Some(SortColumn::Name));
        assert_eq!(screen.sort_dir, SortDir::Desc);

        // Third click: clear
        let _ = update(&mut screen, Message::ColumnHeaderClicked(SortColumn::Name));
        assert_eq!(screen.sort_column, None);
    }

    /// 11.2 – Clicking a different column starts Asc on the new column.
    #[test]
    fn column_header_clicked_different_column_resets() {
        let mut screen = make_screen();

        let _ = update(&mut screen, Message::ColumnHeaderClicked(SortColumn::Name));
        assert_eq!(screen.sort_column, Some(SortColumn::Name));
        assert_eq!(screen.sort_dir, SortDir::Asc);

        // Click a different column
        let _ = update(&mut screen, Message::ColumnHeaderClicked(SortColumn::Size));
        assert_eq!(screen.sort_column, Some(SortColumn::Size));
        assert_eq!(screen.sort_dir, SortDir::Asc);
    }
}
