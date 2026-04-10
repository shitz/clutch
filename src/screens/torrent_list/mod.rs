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

//!
//! # Sub-modules
//!
//! - [`sort`] — Pure column-sort logic (no UI dependencies)
//! - [`add_dialog`] — Add-torrent modal dialog state and view
//! - [`filters`] — Pure status-bucket counting and visibility helpers
//! - [`toolbar`] — Torrent-list toolbar rendering
//! - [`columns`] — Sticky header and sort-indicator rendering
//! - [`dialogs`] — Context-menu and modal overlay rendering
//! - [`view`] — Widget-tree rendering orchestration and row rendering
//! - [`update`] — Elm update function
//! - [`worker`] — Serialized RPC worker subscription stream

pub mod add_dialog;
mod columns;
mod dialogs;
mod filters;
pub mod sort;
mod toolbar;
mod update;
pub mod view;
pub mod worker;

use std::collections::HashSet;

use iced::keyboard::Modifiers;
use tokio::sync::mpsc;

use crate::rpc::{ConnectionParams, RpcWork, SessionData, TorrentData, TransmissionCredentials};

use add_dialog::AddDialogState;
use sort::{SortColumn, SortDir};

// Re-export for parent modules.
pub use add_dialog::FileReadResult;
pub use dialogs::view_context_menu_overlay;
pub use filters::matching_filters;
pub use update::update;
pub use view::view;
pub use worker::rpc_worker_stream;

// ── SetLocationDialog ───────────────────────────────────────────────────────────────

/// State for the "Set Data Location" modal dialog.
#[derive(Debug, Clone)]
pub struct SetLocationDialog {
    pub ids: Vec<i64>,
    /// Absolute path to set (prefilled from the torrent's `downloadDir`).
    pub path: String,
    /// When `true`, the daemon physically moves files to the new location;
    /// when `false`, only the internal path record is updated.
    pub move_data: bool,
}

// ── StatusFilter ──────────────────────────────────────────────────────────────

/// Consolidated status buckets used for client-side filter chips.
///
/// A torrent may match more than one bucket simultaneously (e.g. a torrent
/// actively downloading at 500 KB/s matches both `Downloading` and `Active`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusFilter {
    /// Transmission status 3 (queue download) or 4 (downloading).
    Downloading,
    /// Transmission status 5 (queue seed) or 6 (seeding).
    Seeding,
    /// Transmission status 0 (stopped).
    Paused,
    /// Derived: `rate_download > 0 || rate_upload > 0`.
    Active,
    /// Transmission status 1 (check queue) or 2 (checking).
    Error,
}

impl StatusFilter {
    /// All filter variants in display order.
    pub const fn all() -> [StatusFilter; 5] {
        [
            StatusFilter::Downloading,
            StatusFilter::Seeding,
            StatusFilter::Paused,
            StatusFilter::Active,
            StatusFilter::Error,
        ]
    }
}

use iced::Subscription;

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
    /// Clicking empty space below the torrent rows clears the selection.
    ClearSelection,
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
    TorrentFileRead(Result<std::collections::VecDeque<FileReadResult>, String>),
    AddLinkClicked,
    AddDialogMagnetChanged(String),
    AddDialogDestinationChanged(String),
    AddDialogFileToggled(usize),
    AddDialogSelectAll,
    AddDialogDeselectAll,
    AddConfirmed,
    AddCancelled,
    /// Skip the current torrent in the queue (multi-add only).
    AddCancelThis,
    /// Discard all remaining torrents in the queue.
    AddCancelAll,
    AddCompleted(Result<(), String>),
    // Recent-paths dropdown in add dialog
    AddDialogToggleDropdown,
    AddDialogDismissDropdown,
    AddDialogRecentPathSelected(String),
    /// A destination path was confirmed — bubble up to AppState to persist.
    ProfilePathUsed(String),
    /// Fired when a SetFileWanted RPC completes (success or failure).
    /// Carries the file indices so the inspector can clear pending_wanted.
    FileWantedSettled(bool, Vec<usize>),
    /// Fired when a periodic session-get poll completes.
    SessionDataLoaded(Result<SessionData, String>),
    /// Fired when a torrent-set bandwidth call completes.
    BandwidthSaved(Result<(), String>),
    /// Fired by the toolbar turtle button — intercepted by main_screen.
    TurtleModeToggled,
    // Context-menu flow
    /// Sent on every mouse-move to track the cursor position for context-menu anchoring.
    CursorMoved(iced::Point),
    /// Right-click on a torrent row opens the context menu at the last cursor position.
    TorrentRightClicked(i64),
    /// Window was resized — used to keep edge-mitigation bounds up to date.
    WindowResized {
        width: f32,
        height: f32,
    },
    /// Dismiss the open context menu (click-away or any action).
    DismissContextMenu,
    /// Start action chosen from the context menu.
    ContextMenuStart,
    /// Pause action chosen from the context menu.
    ContextMenuPause,
    /// Delete action chosen from the context menu.
    ContextMenuDelete,
    /// "Set Data Location" chosen from the context menu — opens the modal dialog.
    OpenSetLocation,
    // Set-location dialog
    SetLocationPathChanged(String),
    SetLocationMoveToggled,
    SetLocationApply,
    SetLocationCancel,
    // Escalated to parent — intercepted by MainScreen before reaching update()
    Disconnect,
    // Escalated to app — opens Settings screen
    OpenSettingsClicked,
    // Column sort
    ColumnHeaderClicked(SortColumn),
    // Filter chips
    FilterAllClicked,
    FilterToggled(StatusFilter),
    // Keyboard
    /// Tab key pressed while the add-torrent dialog is open.
    DialogTabKeyPressed {
        shift: bool,
    },
    /// Enter key pressed while the add-torrent dialog is open.
    DialogEnterPressed,
    /// Keyboard modifier keys changed (Shift, Ctrl, Cmd, Alt).
    ModifiersChanged(Modifiers),
    /// Cmd+A / Ctrl+A — select all visible (filtered) torrents.
    KeyboardSelectAll,
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct TorrentListScreen {
    pub params: ConnectionParams,
    pub torrents: Vec<TorrentData>,
    /// `true` while an RPC result is pending.
    pub is_loading: bool,
    /// `true` once the first torrent-list response has been received.
    pub initial_load_done: bool,
    pub sender: Option<mpsc::Sender<RpcWork>>,
    pub error: Option<String>,
    /// Set of IDs currently selected in the list.
    pub selected_ids: HashSet<i64>,
    /// The ID of the last row targeted by a plain or Ctrl/Cmd-click.
    /// Used as the fixed end of a Shift-click range.
    pub selection_anchor: Option<i64>,
    /// Current global keyboard modifier state, kept in sync via `modifiers_subscription()`.
    pub modifiers: Modifiers,
    pub confirming_delete: Option<(Vec<i64>, bool)>,
    pub add_dialog: AddDialogState,
    pub sort_column: Option<SortColumn>,
    pub sort_dir: SortDir,
    /// Active filter chips — only torrents matching at least one bucket are shown.
    pub filters: HashSet<StatusFilter>,
    /// Most recent mouse cursor position, updated on every `CursorMoved` event.
    pub last_cursor_position: iced::Point,
    /// When `Some`, the context menu is open for the given torrent ID at the given point.
    pub context_menu: Option<(i64, iced::Point)>,
    /// When `Some`, the "Set Data Location" modal dialog is active.
    pub set_location_dialog: Option<SetLocationDialog>,
    /// Approximate window dimensions used for context-menu edge mitigation (px).
    pub window_height: f32,
    /// Approximate window width used for right-edge context-menu mitigation (px).
    pub window_width: f32,
    /// Copy of the active profile's recent download paths, kept in sync by `AppState`.
    /// Used by the add-dialog dropdown without requiring the profile to be threaded
    /// through the view hierarchy.
    pub recent_download_paths: Vec<String>,
}

impl TorrentListScreen {
    pub fn new(credentials: TransmissionCredentials, session_id: String) -> Self {
        TorrentListScreen {
            params: ConnectionParams::new(credentials, session_id),
            torrents: Vec::new(),
            is_loading: false,
            initial_load_done: false,
            error: None,
            selected_ids: HashSet::new(),
            selection_anchor: None,
            modifiers: Modifiers::default(),
            confirming_delete: None,
            add_dialog: AddDialogState::Hidden,
            sender: None,
            sort_column: None,
            sort_dir: SortDir::Asc,
            filters: StatusFilter::all().into_iter().collect(),
            last_cursor_position: iced::Point::ORIGIN,
            context_menu: None,
            set_location_dialog: None,
            window_height: 800.0,
            window_width: 900.0,
            recent_download_paths: Vec::new(),
        }
    }

    /// Return the currently selected torrent, if any.
    /// Returns `Some` only when exactly one torrent is selected.
    #[must_use]
    pub fn selected_torrent(&self) -> Option<&TorrentData> {
        if self.selected_ids.len() != 1 {
            return None;
        }
        let id = self.selected_ids.iter().next().copied()?;
        self.torrents.iter().find(|t| t.id == id)
    }

    /// Return the visible (filtered + sorted) torrents in display order.
    ///
    /// Used by click-selection handlers to resolve index-based ranges (Shift-click)
    /// without coupling update logic to the view layer.
    #[must_use]
    pub fn visible_torrents(&self) -> Vec<&TorrentData> {
        filters::display_torrents(
            &self.torrents,
            self.sort_column,
            self.sort_dir,
            &self.filters,
        )
    }

    /// Remove from `selected_ids` any torrent IDs that are not currently
    /// visible (i.e. filtered out or no longer present on the daemon).
    ///
    /// Also clears `selection_anchor` if the anchor torrent is no longer
    /// visible, so that subsequent Shift-click ranges are calculated from a
    /// valid starting point.
    pub(crate) fn prune_selection_to_visible(&mut self) {
        let visible_ids: std::collections::HashSet<i64> =
            self.visible_torrents().into_iter().map(|t| t.id).collect();
        self.selected_ids.retain(|id| visible_ids.contains(id));
        if let Some(anchor) = self.selection_anchor
            && !visible_ids.contains(&anchor)
        {
            self.selection_anchor = None;
        }
    }

    pub(crate) fn enqueue(&self, work: RpcWork) {
        if let Some(tx) = &self.sender {
            if let Err(e) = tx.try_send(work) {
                tracing::error!("RPC work queue full, dropping work item: {e}");
            }
        } else {
            tracing::warn!("RPC worker not ready yet, dropping work item");
        }
    }

    pub(crate) fn enqueue_torrent_get(&self) {
        self.enqueue(RpcWork::TorrentGet(self.params.clone()));
    }

    /// Keyboard subscription for the add-torrent dialog.
    ///
    /// Only active while the dialog is open — returns `Subscription::none()` when
    /// `add_dialog` is `Hidden` so Tab and Enter are not captured on the main list.
    pub fn dialog_subscription(&self) -> Subscription<Message> {
        if matches!(self.add_dialog, AddDialogState::Hidden) {
            return Subscription::none();
        }
        iced::keyboard::listen().filter_map(|event| {
            use iced::keyboard::{Event, Key, key::Named};
            if let Event::KeyPressed { key, modifiers, .. } = event {
                match key.as_ref() {
                    Key::Named(Named::Tab) => Some(Message::DialogTabKeyPressed {
                        shift: modifiers.shift(),
                    }),
                    Key::Named(Named::Enter) if !modifiers.control() && !modifiers.alt() => {
                        Some(Message::DialogEnterPressed)
                    }
                    _ => None,
                }
            } else {
                None
            }
        })
    }

    /// Subscription that tracks the global mouse cursor position.
    ///
    /// Always active — the `update()` handler for `CursorMoved` only stores the
    /// point and returns `Task::none()`, so the cost per event is negligible.
    pub fn cursor_subscription(&self) -> Subscription<Message> {
        iced::event::listen_with(|event, _status, _id| match event {
            iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                Some(Message::CursorMoved(position))
            }
            iced::Event::Window(iced::window::Event::Opened { size, .. }) => {
                Some(Message::WindowResized {
                    width: size.width,
                    height: size.height,
                })
            }
            iced::Event::Window(iced::window::Event::Resized(iced::Size { width, height })) => {
                Some(Message::WindowResized { width, height })
            }
            _ => None,
        })
    }

    /// Subscription that tracks global keyboard modifier state (Shift, Ctrl, Cmd, Alt)
    /// and Cmd+A / Ctrl+A select-all.
    pub fn modifiers_subscription() -> Subscription<Message> {
        iced::keyboard::listen().filter_map(|event| {
            use iced::keyboard::{Event, Key, Modifiers};
            match event {
                Event::ModifiersChanged(m) => Some(Message::ModifiersChanged(m)),
                Event::KeyPressed {
                    key: Key::Character(c),
                    modifiers,
                    ..
                } if (modifiers.command() || modifiers.control()) && c.as_str() == "a" => {
                    let _ = Modifiers::CTRL; // suppress unused-warning if any
                    Some(Message::KeyboardSelectAll)
                }
                _ => None,
            }
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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

    /// Tick is ignored when `is_loading` is already true.
    #[test]
    fn tick_ignored_when_loading() {
        let mut screen = make_screen();
        screen.is_loading = true;
        let _ = update(&mut screen, Message::Tick);
        assert!(screen.is_loading);
    }

    /// Tick starts a load when the screen is idle.
    #[test]
    fn tick_fires_when_not_loading() {
        let mut screen = make_screen();
        screen.is_loading = false;
        let _ = update(&mut screen, Message::Tick);
        assert!(screen.is_loading);
    }

    /// `TorrentsUpdated(Ok)` replaces torrents and clears `is_loading`.
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

    /// `TorrentsUpdated(Err)` clears `is_loading` and stores the error.
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

    /// `SessionIdRotated` updates the stored session ID.
    #[test]
    fn session_id_rotated_updates_id() {
        let mut screen = make_screen();
        let _ = update(
            &mut screen,
            Message::SessionIdRotated("new-id-xyz".to_owned()),
        );
        assert_eq!(screen.params.session_id, "new-id-xyz");
    }

    /// `TorrentSelected` (plain click) selects the torrent, switches between
    /// torrents, and deselects when the same singly-selected torrent is clicked.
    #[test]
    fn torrent_selected_toggles() {
        let mut screen = make_screen();
        screen.torrents = vec![make_torrent(1, "A"), make_torrent(2, "B")];

        let _ = update(&mut screen, Message::TorrentSelected(1));
        assert!(screen.selected_ids.contains(&1));

        // Switching to another torrent narrows to that one.
        let _ = update(&mut screen, Message::TorrentSelected(2));
        assert!(screen.selected_ids.contains(&2));
        assert!(!screen.selected_ids.contains(&1));

        // Plain-clicking the sole selected torrent deselects it.
        let _ = update(&mut screen, Message::TorrentSelected(2));
        assert!(screen.selected_ids.is_empty());
    }

    /// `ClearSelection` clears all selected ids and resets the anchor.
    #[test]
    fn clear_selection_clears_all() {
        let mut screen = make_screen();
        screen.torrents = vec![make_torrent(1, "A"), make_torrent(2, "B")];
        screen.selected_ids = [1, 2].into_iter().collect();
        screen.selection_anchor = Some(1);

        let _ = update(&mut screen, Message::ClearSelection);
        assert!(screen.selected_ids.is_empty());
        assert!(screen.selection_anchor.is_none());
    }

    /// `selected_torrent()` returns the matching torrent when exactly one is selected.
    #[test]
    fn selected_torrent_returns_correct_entry() {
        let mut screen = make_screen();
        screen.torrents = vec![make_torrent(1, "Alpha"), make_torrent(2, "Beta")];
        screen.selected_ids = [2].into_iter().collect();
        let t = screen.selected_torrent().expect("should have a selection");
        assert_eq!(t.id, 2);
        assert_eq!(t.name, "Beta");
    }

    /// `selected_torrent()` returns `None` when nothing is selected.
    #[test]
    fn selected_torrent_none_when_no_selection() {
        let screen = make_screen();
        assert!(screen.selected_torrent().is_none());
    }

    /// `DeleteClicked` arms the delete-confirmation state.
    #[test]
    fn delete_clicked_sets_confirming() {
        let mut screen = make_screen();
        screen.torrents = vec![make_torrent(5, "Arch")];
        screen.selected_ids = [5].into_iter().collect();
        let _ = update(&mut screen, Message::DeleteClicked);
        assert_eq!(screen.confirming_delete, Some((vec![5], false)));
    }

    /// `DeleteClicked` is a no-op when nothing is selected.
    #[test]
    fn delete_clicked_no_selection_is_noop() {
        let mut screen = make_screen();
        let _ = update(&mut screen, Message::DeleteClicked);
        assert_eq!(screen.confirming_delete, None);
    }

    /// `DeleteCancelled` clears the delete-confirmation state.
    #[test]
    fn delete_cancelled_clears_confirming() {
        let mut screen = make_screen();
        screen.confirming_delete = Some((vec![3], true));
        let _ = update(&mut screen, Message::DeleteCancelled);
        assert_eq!(screen.confirming_delete, None);
    }

    /// `DeleteConfirmed` clears the dialog state and starts a delete action.
    #[test]
    fn delete_confirmed_clears_confirming_and_loads() {
        let mut screen = make_screen();
        screen.confirming_delete = Some((vec![7], true));
        let _ = update(&mut screen, Message::DeleteConfirmed);
        assert_eq!(screen.confirming_delete, None);
        assert!(screen.is_loading);
    }

    /// `DeleteConfirmed` is a no-op when no delete confirmation is active.
    #[test]
    fn delete_confirmed_no_state_is_noop() {
        let mut screen = make_screen();
        let _ = update(&mut screen, Message::DeleteConfirmed);
        assert!(!screen.is_loading);
    }

    /// `DeleteLocalDataToggled` updates the checkbox state.
    #[test]
    fn delete_local_data_toggled_updates_state() {
        let mut screen = make_screen();
        screen.confirming_delete = Some((vec![9], false));
        let _ = update(&mut screen, Message::DeleteLocalDataToggled(true));
        assert_eq!(screen.confirming_delete, Some((vec![9], true)));
        let _ = update(&mut screen, Message::DeleteLocalDataToggled(false));
        assert_eq!(screen.confirming_delete, Some((vec![9], false)));
    }

    /// `ActionCompleted(Ok)` keeps loading active and triggers a refresh.
    #[test]
    fn action_completed_ok_fires_refresh() {
        let mut screen = make_screen();
        screen.is_loading = true;
        let _ = update(&mut screen, Message::ActionCompleted(Ok(())));
        assert!(screen.is_loading);
    }

    /// `ActionCompleted(Err)` clears loading and stores the error.
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

    /// Poll tick is ignored while an action is still in flight.
    #[test]
    fn tick_ignored_while_action_in_flight() {
        let mut screen = make_screen();
        screen.is_loading = true;
        // Any non-empty selection keeps the tick guarded.
        screen.selected_ids = [1].into_iter().collect();
        let _ = update(&mut screen, Message::Tick);
        assert!(screen.is_loading);
    }

    // ── Add-torrent dialog tests ─────────────────────────────────────────────

    /// `AddLinkClicked` opens the add-link dialog.
    #[test]
    fn add_link_clicked_opens_add_link_dialog() {
        let mut screen = make_screen();
        let _ = update(&mut screen, Message::AddLinkClicked);
        assert!(matches!(screen.add_dialog, AddDialogState::AddLink { .. }));
    }

    /// `AddConfirmed` is a no-op when the magnet input is empty.
    #[test]
    fn add_confirmed_empty_magnet_is_noop() {
        let mut screen = make_screen();
        screen.add_dialog = AddDialogState::AddLink {
            magnet: String::new(),
            destination: String::new(),
            error: None,
        };
        let _ = update(&mut screen, Message::AddConfirmed);
        assert!(matches!(screen.add_dialog, AddDialogState::AddLink { .. }));
    }

    /// `AddConfirmed` with a valid magnet starts loading.
    #[test]
    fn add_confirmed_valid_magnet_emits_task() {
        let mut screen = make_screen();
        screen.add_dialog = AddDialogState::AddLink {
            magnet: "magnet:?xt=urn:btih:abc123".to_owned(),
            destination: String::new(),
            error: None,
        };
        let _ = update(&mut screen, Message::AddConfirmed);
        assert!(screen.is_loading);
    }

    /// `AddCancelled` closes the add dialog.
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

    /// `TorrentFileRead(Ok)` opens the add-file dialog with the parsed files.
    #[test]
    fn torrent_file_read_ok_opens_add_file_dialog() {
        use std::collections::VecDeque;
        let mut screen = make_screen();
        let result = FileReadResult {
            metainfo_b64: "dGVzdA==".to_owned(),
            files: vec![add_dialog::TorrentFileInfo {
                path: "movie.mkv".to_owned(),
                size_bytes: 1_073_741_824,
            }],
        };
        let mut queue = VecDeque::new();
        queue.push_back(result);
        let _ = update(&mut screen, Message::TorrentFileRead(Ok(queue)));
        match &screen.add_dialog {
            AddDialogState::AddFile { files, .. } => {
                assert_eq!(files.len(), 1);
                assert_eq!(files[0].path, "movie.mkv");
            }
            other => panic!("expected AddFile, got {other:?}"),
        }
    }

    /// `AddCompleted(Ok)` triggers an immediate refresh (dialog already advanced by AddConfirmed).
    #[test]
    fn add_completed_ok_polls() {
        let mut screen = make_screen();
        // Dialog is already Hidden (advanced by AddConfirmed before AddCompleted arrives).
        let _ = update(&mut screen, Message::AddCompleted(Ok(())));
        assert!(screen.is_loading);
    }

    /// `AddCompleted(Err)` stores the error in the general error banner.
    #[test]
    fn add_completed_err_stores_error_in_banner() {
        let mut screen = make_screen();
        let _ = update(
            &mut screen,
            Message::AddCompleted(Err("daemon error".to_owned())),
        );
        assert_eq!(screen.error.as_deref(), Some("daemon error"));
    }

    /// `ColumnHeaderClicked` cycles unsorted, ascending, descending, then unsorted.
    #[test]
    fn column_header_clicked_cycles_sort() {
        let mut screen = make_screen();
        assert_eq!(screen.sort_column, None);

        let _ = update(&mut screen, Message::ColumnHeaderClicked(SortColumn::Name));
        assert_eq!(screen.sort_column, Some(SortColumn::Name));
        assert_eq!(screen.sort_dir, SortDir::Asc);

        let _ = update(&mut screen, Message::ColumnHeaderClicked(SortColumn::Name));
        assert_eq!(screen.sort_column, Some(SortColumn::Name));
        assert_eq!(screen.sort_dir, SortDir::Desc);

        let _ = update(&mut screen, Message::ColumnHeaderClicked(SortColumn::Name));
        assert_eq!(screen.sort_column, None);
    }

    /// Clicking a different column starts ascending order on the new column.
    #[test]
    fn column_header_clicked_different_column_resets() {
        let mut screen = make_screen();

        let _ = update(&mut screen, Message::ColumnHeaderClicked(SortColumn::Name));
        assert_eq!(screen.sort_column, Some(SortColumn::Name));

        let _ = update(&mut screen, Message::ColumnHeaderClicked(SortColumn::Size));
        assert_eq!(screen.sort_column, Some(SortColumn::Size));
        assert_eq!(screen.sort_dir, SortDir::Asc);
    }

    // ── DialogTabKeyPressed guards ────────────────────────────────────────────

    /// Tab is a no-op when the dialog is hidden.
    #[test]
    fn dialog_tab_noop_when_hidden() {
        let mut screen = make_screen();
        assert!(matches!(screen.add_dialog, AddDialogState::Hidden));
        let _ = update(&mut screen, Message::DialogTabKeyPressed { shift: false });
        // State must be unchanged.
        assert!(matches!(screen.add_dialog, AddDialogState::Hidden));
    }

    /// Tab is a no-op in AddFile mode (only one text input — destination).
    #[test]
    fn dialog_tab_noop_in_add_file_mode() {
        let mut screen = make_screen();
        screen.add_dialog = AddDialogState::AddFile {
            metainfo_b64: "dGVzdA==".to_owned(),
            files: vec![],
            selected: vec![],
            destination: "/downloads".to_owned(),
            error: None,
            pending_torrents: std::collections::VecDeque::new(),
            is_dropdown_open: false,
            total_count: 1,
        };
        let _ = update(&mut screen, Message::DialogTabKeyPressed { shift: false });
        // Dialog state must be unchanged.
        assert!(matches!(screen.add_dialog, AddDialogState::AddFile { .. }));
    }

    /// Tab in AddLink mode returns a focus task (no state mutation, no error).
    #[test]
    fn dialog_tab_active_in_add_link_mode() {
        let mut screen = make_screen();
        screen.add_dialog = AddDialogState::AddLink {
            magnet: String::new(),
            destination: String::new(),
            error: None,
        };
        // Forward Tab.
        let _ = update(&mut screen, Message::DialogTabKeyPressed { shift: false });
        // Dialog must still be open — no state change on any field.
        assert!(matches!(screen.add_dialog, AddDialogState::AddLink { .. }));
    }

    /// Shift-Tab in AddLink mode also returns a focus task.
    #[test]
    fn dialog_shift_tab_active_in_add_link_mode() {
        let mut screen = make_screen();
        screen.add_dialog = AddDialogState::AddLink {
            magnet: String::new(),
            destination: String::new(),
            error: None,
        };
        let _ = update(&mut screen, Message::DialogTabKeyPressed { shift: true });
        assert!(matches!(screen.add_dialog, AddDialogState::AddLink { .. }));
    }

    // ── DialogEnterPressed guards ─────────────────────────────────────────────

    /// Enter with an empty magnet is a no-op (does not set is_loading).
    #[test]
    fn dialog_enter_noop_with_empty_magnet() {
        let mut screen = make_screen();
        screen.add_dialog = AddDialogState::AddLink {
            magnet: String::new(),
            destination: String::new(),
            error: None,
        };
        let _ = update(&mut screen, Message::DialogEnterPressed);
        assert!(!screen.is_loading, "empty magnet must not trigger submit");
    }

    /// Enter with whitespace-only magnet is also a no-op.
    #[test]
    fn dialog_enter_noop_with_whitespace_magnet() {
        let mut screen = make_screen();
        screen.add_dialog = AddDialogState::AddLink {
            magnet: "   ".to_owned(),
            destination: String::new(),
            error: None,
        };
        let _ = update(&mut screen, Message::DialogEnterPressed);
        assert!(!screen.is_loading);
    }

    /// Enter with a non-empty magnet triggers submit (sets is_loading).
    #[test]
    fn dialog_enter_submits_with_valid_magnet() {
        let mut screen = make_screen();
        screen.add_dialog = AddDialogState::AddLink {
            magnet: "magnet:?xt=urn:btih:abc123".to_owned(),
            destination: String::new(),
            error: None,
        };
        let _ = update(&mut screen, Message::DialogEnterPressed);
        assert!(screen.is_loading, "valid magnet must trigger submit");
    }

    /// Enter with an AddFile dialog (metainfo present) triggers submit.
    #[test]
    fn dialog_enter_submits_with_add_file() {
        let mut screen = make_screen();
        screen.add_dialog = AddDialogState::AddFile {
            metainfo_b64: "dGVzdA==".to_owned(),
            files: vec![],
            selected: vec![true],
            destination: "/downloads".to_owned(),
            error: None,
            pending_torrents: std::collections::VecDeque::new(),
            is_dropdown_open: false,
            total_count: 1,
        };
        let _ = update(&mut screen, Message::DialogEnterPressed);
        assert!(
            screen.is_loading,
            "AddFile with metainfo must trigger submit"
        );
    }

    /// Enter when the dialog is Hidden is a no-op.
    #[test]
    fn dialog_enter_noop_when_hidden() {
        let mut screen = make_screen();
        let _ = update(&mut screen, Message::DialogEnterPressed);
        assert!(!screen.is_loading);
    }

    // ── Selective file selection ─────────────────────────────────────────────

    fn make_add_file_screen(n: usize) -> TorrentListScreen {
        let mut screen = make_screen();
        let files: Vec<add_dialog::TorrentFileInfo> = (0..n)
            .map(|i| add_dialog::TorrentFileInfo {
                path: format!("file{i}.mkv"),
                size_bytes: 1024,
            })
            .collect();
        screen.add_dialog = AddDialogState::AddFile {
            metainfo_b64: "dGVzdA==".to_owned(),
            selected: vec![true; n],
            files,
            destination: String::new(),
            error: None,
            pending_torrents: std::collections::VecDeque::new(),
            is_dropdown_open: false,
            total_count: 1,
        };
        screen
    }

    /// `AddDialogSelectAll` sets every entry in `selected` to `true`.
    #[test]
    fn add_dialog_select_all_sets_all_true() {
        let mut screen = make_add_file_screen(3);
        // Deselect one entry first.
        let _ = update(&mut screen, Message::AddDialogFileToggled(1));
        // Then select all.
        let _ = update(&mut screen, Message::AddDialogSelectAll);
        if let AddDialogState::AddFile { selected, .. } = &screen.add_dialog {
            assert_eq!(selected, &vec![true, true, true]);
        } else {
            panic!("expected AddFile state");
        }
    }

    /// `AddDialogDeselectAll` sets every entry in `selected` to `false`.
    #[test]
    fn add_dialog_deselect_all_sets_all_false() {
        let mut screen = make_add_file_screen(3);
        let _ = update(&mut screen, Message::AddDialogDeselectAll);
        if let AddDialogState::AddFile { selected, .. } = &screen.add_dialog {
            assert_eq!(selected, &vec![false, false, false]);
        } else {
            panic!("expected AddFile state");
        }
    }

    /// `AddDialogFileToggled` flips only the targeted index.
    #[test]
    fn add_dialog_file_toggled_flips_single_index() {
        let mut screen = make_add_file_screen(3);
        let _ = update(&mut screen, Message::AddDialogFileToggled(1));
        if let AddDialogState::AddFile { selected, .. } = &screen.add_dialog {
            assert_eq!(selected, &vec![true, false, true]);
        } else {
            panic!("expected AddFile state");
        }
    }

    // ── matching_filters tests ────────────────────────────────────────────────

    fn torrent_with(status: i32, rate_dl: i64, rate_ul: i64) -> TorrentData {
        TorrentData {
            id: 1,
            name: "test".to_owned(),
            status,
            rate_download: rate_dl,
            rate_upload: rate_ul,
            ..Default::default()
        }
    }

    /// Status 4 maps to Downloading only (no active rates).
    #[test]
    fn matching_filters_status_4_is_downloading() {
        let t = torrent_with(4, 0, 0);
        let buckets = matching_filters(&t);
        assert!(
            buckets.contains(&StatusFilter::Downloading),
            "expected Downloading"
        );
        assert!(
            !buckets.contains(&StatusFilter::Active),
            "no active rate, should not be Active"
        );
    }

    /// Status 3 (queue download) also maps to Downloading.
    #[test]
    fn matching_filters_status_3_is_downloading() {
        let t = torrent_with(3, 0, 0);
        let buckets = matching_filters(&t);
        assert!(buckets.contains(&StatusFilter::Downloading));
    }

    /// Downloading torrent with rate_download > 0 matches both Downloading and Active.
    #[test]
    fn matching_filters_active_download_is_both_downloading_and_active() {
        let t = torrent_with(4, 500_000, 0);
        let buckets = matching_filters(&t);
        assert!(
            buckets.contains(&StatusFilter::Downloading),
            "expected Downloading"
        );
        assert!(
            buckets.contains(&StatusFilter::Active),
            "expected Active from rate_download"
        );
    }

    /// Status 6 maps to Seeding only (no active rates).
    #[test]
    fn matching_filters_status_6_is_seeding() {
        let t = torrent_with(6, 0, 0);
        let buckets = matching_filters(&t);
        assert!(buckets.contains(&StatusFilter::Seeding));
        assert!(!buckets.contains(&StatusFilter::Active));
    }

    /// Status 5 (queue seed) maps to Seeding.
    #[test]
    fn matching_filters_status_5_is_seeding() {
        let t = torrent_with(5, 0, 0);
        let buckets = matching_filters(&t);
        assert!(buckets.contains(&StatusFilter::Seeding));
    }

    /// Seeding torrent with rate_upload > 0 matches both Seeding and Active.
    #[test]
    fn matching_filters_active_seed_is_both_seeding_and_active() {
        let t = torrent_with(6, 0, 100_000);
        let buckets = matching_filters(&t);
        assert!(buckets.contains(&StatusFilter::Seeding));
        assert!(buckets.contains(&StatusFilter::Active));
    }

    /// Status 0 maps to Paused.
    #[test]
    fn matching_filters_status_0_is_paused() {
        let t = torrent_with(0, 0, 0);
        let buckets = matching_filters(&t);
        assert!(buckets.contains(&StatusFilter::Paused));
        assert!(!buckets.contains(&StatusFilter::Active));
    }

    /// Status 1 maps to Error.
    #[test]
    fn matching_filters_status_1_is_error() {
        let t = torrent_with(1, 0, 0);
        let buckets = matching_filters(&t);
        assert!(buckets.contains(&StatusFilter::Error));
    }

    /// Status 2 maps to Error.
    #[test]
    fn matching_filters_status_2_is_error() {
        let t = torrent_with(2, 0, 0);
        let buckets = matching_filters(&t);
        assert!(buckets.contains(&StatusFilter::Error));
    }

    // ── FilterAllClicked handler tests ────────────────────────────────────────

    /// FilterAllClicked when all 5 filters are active clears the set.
    #[test]
    fn filter_all_clicked_when_all_selected_clears_set() {
        let mut screen = make_screen();
        assert_eq!(screen.filters.len(), StatusFilter::all().len());
        let _ = update(&mut screen, Message::FilterAllClicked);
        assert!(screen.filters.is_empty());
    }

    /// FilterAllClicked when fewer than 5 filters are active restores all 5.
    #[test]
    fn filter_all_clicked_when_partial_restores_all() {
        let mut screen = make_screen();
        screen.filters.clear();
        screen.filters.insert(StatusFilter::Downloading);
        let _ = update(&mut screen, Message::FilterAllClicked);
        assert_eq!(screen.filters.len(), StatusFilter::all().len());
    }

    /// FilterAllClicked when empty also restores all 5.
    #[test]
    fn filter_all_clicked_when_empty_restores_all() {
        let mut screen = make_screen();
        screen.filters.clear();
        let _ = update(&mut screen, Message::FilterAllClicked);
        assert_eq!(screen.filters.len(), StatusFilter::all().len());
    }

    // ── FilterToggled handler tests ───────────────────────────────────────────

    /// FilterToggled removes a present filter.
    #[test]
    fn filter_toggled_removes_present_filter() {
        let mut screen = make_screen();
        assert!(screen.filters.contains(&StatusFilter::Seeding));
        let _ = update(&mut screen, Message::FilterToggled(StatusFilter::Seeding));
        assert!(!screen.filters.contains(&StatusFilter::Seeding));
    }

    /// FilterToggled inserts an absent filter.
    #[test]
    fn filter_toggled_inserts_absent_filter() {
        let mut screen = make_screen();
        screen.filters.remove(&StatusFilter::Error);
        assert!(!screen.filters.contains(&StatusFilter::Error));
        let _ = update(&mut screen, Message::FilterToggled(StatusFilter::Error));
        assert!(screen.filters.contains(&StatusFilter::Error));
    }

    /// Toggling a filter twice leaves the set unchanged.
    #[test]
    fn filter_toggled_twice_is_idempotent() {
        let mut screen = make_screen();
        let original_len = screen.filters.len();
        let _ = update(&mut screen, Message::FilterToggled(StatusFilter::Paused));
        let _ = update(&mut screen, Message::FilterToggled(StatusFilter::Paused));
        assert_eq!(screen.filters.len(), original_len);
    }

    // ── Context menu handler tests ────────────────────────────────────────────

    /// CursorMoved stores the point without changing other state.
    #[test]
    fn cursor_moved_stores_point() {
        let mut screen = make_screen();
        let pt = iced::Point::new(42.0, 100.0);
        let _ = update(&mut screen, Message::CursorMoved(pt));
        assert_eq!(screen.last_cursor_position, pt);
        assert!(screen.context_menu.is_none());
    }

    /// TorrentRightClicked opens the context menu for the given id at last cursor pos.
    #[test]
    fn torrent_right_clicked_opens_context_menu() {
        let mut screen = make_screen();
        screen.torrents = vec![make_torrent(3, "Fedora")];
        let pt = iced::Point::new(50.0, 80.0);
        screen.last_cursor_position = pt;
        let _ = update(&mut screen, Message::TorrentRightClicked(3));
        assert_eq!(screen.context_menu, Some((3, pt)));
    }

    /// DismissContextMenu clears the menu.
    #[test]
    fn dismiss_context_menu_clears_state() {
        let mut screen = make_screen();
        screen.context_menu = Some((1, iced::Point::new(10.0, 10.0)));
        let _ = update(&mut screen, Message::DismissContextMenu);
        assert!(screen.context_menu.is_none());
    }

    /// ContextMenuDelete dismisses the menu and opens the delete confirmation dialog.
    #[test]
    fn context_menu_delete_dismisses_and_confirms() {
        let mut screen = make_screen();
        screen.torrents = vec![make_torrent(7, "Arch")];
        screen.selected_ids = [7].into_iter().collect();
        screen.context_menu = Some((7, iced::Point::ORIGIN));
        let _ = update(&mut screen, Message::ContextMenuDelete);
        assert!(screen.context_menu.is_none());
        assert!(matches!(
            screen.confirming_delete,
            Some((ref ids, false)) if ids.contains(&7)
        ));
    }

    /// OpenSetLocation dismisses the menu and opens the dialog prefilled with downloadDir.
    #[test]
    fn open_set_location_opens_dialog_with_prefill() {
        let mut screen = make_screen();
        screen.torrents = vec![TorrentData {
            id: 9,
            name: "Arch".to_owned(),
            status: 0,
            download_dir: "/data/torrents".to_owned(),
            ..Default::default()
        }];
        screen.selected_ids = [9].into_iter().collect();
        screen.context_menu = Some((9, iced::Point::ORIGIN));
        let _ = update(&mut screen, Message::OpenSetLocation);
        assert!(screen.context_menu.is_none());
        let dlg = screen.set_location_dialog.expect("dialog should be open");
        assert!(dlg.ids.contains(&9));
        assert_eq!(dlg.path, "/data/torrents");
        assert!(dlg.move_data, "move_data defaults to true");
    }

    /// ContextMenuStart dismisses the menu and sets is_loading.
    #[test]
    fn context_menu_start_dismisses_and_enqueues() {
        let mut screen = make_screen();
        screen.selected_ids = [4].into_iter().collect();
        screen.context_menu = Some((4, iced::Point::ORIGIN));
        let _ = update(&mut screen, Message::ContextMenuStart);
        assert!(screen.context_menu.is_none());
        assert!(screen.is_loading);
    }

    /// ContextMenuPause dismisses the menu and sets is_loading.
    #[test]
    fn context_menu_pause_dismisses_and_enqueues() {
        let mut screen = make_screen();
        screen.selected_ids = [5].into_iter().collect();
        screen.context_menu = Some((5, iced::Point::ORIGIN));
        let _ = update(&mut screen, Message::ContextMenuPause);
        assert!(screen.context_menu.is_none());
        assert!(screen.is_loading);
    }

    // ── Set location dialog handler tests ─────────────────────────────────────

    /// SetLocationPathChanged updates the path in the dialog.
    #[test]
    fn set_location_path_changed_updates_path() {
        let mut screen = make_screen();
        screen.set_location_dialog = Some(SetLocationDialog {
            ids: vec![1],
            path: "/old".to_owned(),
            move_data: true,
        });
        let _ = update(
            &mut screen,
            Message::SetLocationPathChanged("/new/path".to_owned()),
        );
        let dlg = screen.set_location_dialog.unwrap();
        assert_eq!(dlg.path, "/new/path");
    }

    /// SetLocationMoveToggled flips the move_data flag.
    #[test]
    fn set_location_move_toggled_flips_flag() {
        let mut screen = make_screen();
        screen.set_location_dialog = Some(SetLocationDialog {
            ids: vec![1],
            path: String::new(),
            move_data: true,
        });
        let _ = update(&mut screen, Message::SetLocationMoveToggled);
        assert!(!screen.set_location_dialog.as_ref().unwrap().move_data);
        let _ = update(&mut screen, Message::SetLocationMoveToggled);
        assert!(screen.set_location_dialog.as_ref().unwrap().move_data);
    }

    /// SetLocationCancel closes the dialog without dispatching any RPC.
    #[test]
    fn set_location_cancel_closes_dialog() {
        let mut screen = make_screen();
        screen.set_location_dialog = Some(SetLocationDialog {
            ids: vec![2],
            path: "/some/path".to_owned(),
            move_data: false,
        });
        let _ = update(&mut screen, Message::SetLocationCancel);
        assert!(screen.set_location_dialog.is_none());
        assert!(!screen.is_loading, "cancel must not trigger any RPC");
    }

    /// SetLocationApply with a non-empty path closes the dialog.
    #[test]
    fn set_location_apply_closes_dialog_with_valid_path() {
        let mut screen = make_screen();
        screen.set_location_dialog = Some(SetLocationDialog {
            ids: vec![3],
            path: "/valid/path".to_owned(),
            move_data: true,
        });
        let _ = update(&mut screen, Message::SetLocationApply);
        assert!(screen.set_location_dialog.is_none());
    }

    /// SetLocationApply with an empty/whitespace-only path is a no-op.
    #[test]
    fn set_location_apply_noop_with_empty_path() {
        let mut screen = make_screen();
        screen.set_location_dialog = Some(SetLocationDialog {
            ids: vec![3],
            path: "  ".to_owned(),
            move_data: true,
        });
        let _ = update(&mut screen, Message::SetLocationApply);
        // Dialog is cleared but no RPC is enqueued (sender is None so no panic).
        assert!(screen.set_location_dialog.is_none());
        assert!(!screen.is_loading);
    }

    // ── Multi-select behaviour ────────────────────────────────────────────────

    /// Cmd/Ctrl-click adds a second torrent to the selection without clearing
    /// the first, and `selected_torrent()` returns `None` for multi-select.
    #[test]
    fn cmd_click_creates_multi_select_and_selected_torrent_returns_none() {
        let mut screen = make_screen();
        screen.torrents = vec![make_torrent(1, "A"), make_torrent(2, "B")];

        // Plain click to select first.
        let _ = update(&mut screen, Message::TorrentSelected(1));
        assert_eq!(screen.selected_ids.len(), 1);
        assert!(screen.selected_torrent().is_some());

        // Cmd-click to add the second.
        screen.modifiers = Modifiers::COMMAND;
        let _ = update(&mut screen, Message::TorrentSelected(2));
        assert!(screen.selected_ids.contains(&1));
        assert!(screen.selected_ids.contains(&2));
        assert_eq!(screen.selected_ids.len(), 2);

        // With 2 IDs selected, `selected_torrent()` must return None.
        assert!(
            screen.selected_torrent().is_none(),
            "selected_torrent() must be None for multi-select"
        );
    }

    /// Cmd/Ctrl-click on an already-selected torrent removes it (toggle off).
    #[test]
    fn cmd_click_deselects_already_selected_torrent() {
        let mut screen = make_screen();
        screen.torrents = vec![make_torrent(1, "A"), make_torrent(2, "B")];
        screen.selected_ids = [1, 2].into_iter().collect();
        screen.selection_anchor = Some(1);

        screen.modifiers = Modifiers::COMMAND;
        let _ = update(&mut screen, Message::TorrentSelected(2));
        assert_eq!(screen.selected_ids.len(), 1);
        assert!(!screen.selected_ids.contains(&2));
    }

    // ── Filter + selection interaction ────────────────────────────────────────

    /// Toggling a filter that hides a selected torrent must remove that torrent
    /// from `selected_ids`.
    #[test]
    fn filter_toggle_prunes_hidden_torrent_from_selection() {
        let mut screen = make_screen();
        // Torrent 1 is Seeding (status=6); torrent 2 is Paused (status=0).
        screen.torrents = vec![
            TorrentData {
                id: 1,
                name: "Seeder".to_owned(),
                status: 6,
                ..Default::default()
            },
            TorrentData {
                id: 2,
                name: "Paused".to_owned(),
                status: 0,
                ..Default::default()
            },
        ];
        screen.selected_ids = [1, 2].into_iter().collect();
        screen.selection_anchor = Some(2);

        // make_screen() starts with ALL filters active.
        // Toggle the Paused filter OFF — torrent 2 (Paused) becomes hidden.
        let _ = update(&mut screen, Message::FilterToggled(StatusFilter::Paused));

        assert!(
            screen.selected_ids.contains(&1),
            "visible torrent stays selected"
        );
        assert!(
            !screen.selected_ids.contains(&2),
            "hidden torrent must be pruned from selection"
        );
        assert_eq!(
            screen.selection_anchor, None,
            "anchor must be cleared when the anchor torrent is filtered out"
        );
    }

    /// FilterAllClicked (select all filters / clear all) must also prune selection
    /// to only the newly visible set.
    #[test]
    fn filter_all_clicked_prunes_selection_to_visible() {
        let mut screen = make_screen();
        screen.torrents = vec![
            TorrentData {
                id: 1,
                name: "Seeder".to_owned(),
                status: 6,
                ..Default::default()
            },
            TorrentData {
                id: 2,
                name: "Downloader".to_owned(),
                status: 4,
                ..Default::default()
            },
        ];
        // Narrow active filters to only Seeding — torrent 2 (Downloading) is hidden.
        screen.filters = [StatusFilter::Seeding].into_iter().collect();
        // Both torrents were somehow selected (simulating the bug scenario).
        screen.selected_ids = [1, 2].into_iter().collect();
        screen.selection_anchor = Some(2);

        // FilterAllClicked: since `filters.len() != all().len()`, it populates all filters.
        // After this, both torrents are visible.
        let _ = update(&mut screen, Message::FilterAllClicked);
        assert_eq!(
            screen.filters.len(),
            StatusFilter::all().len(),
            "all filters activated means full visibility"
        );
        assert!(screen.selected_ids.contains(&1));
        assert!(screen.selected_ids.contains(&2));
    }

    /// When the daemon stops reporting a selected torrent (removed / garbage-
    /// collected), `TorrentsUpdated` must remove it from the selection.
    #[test]
    fn torrents_updated_removes_deleted_torrent_from_selection() {
        let mut screen = make_screen();
        screen.torrents = vec![make_torrent(10, "Gone"), make_torrent(11, "Here")];
        screen.selected_ids = [10, 11].into_iter().collect();
        screen.selection_anchor = Some(10);

        // Daemon response no longer includes torrent 10 (e.g. it was deleted).
        let new_torrents = vec![make_torrent(11, "Here")];
        let _ = update(&mut screen, Message::TorrentsUpdated(Ok(new_torrents)));

        assert!(
            !screen.selected_ids.contains(&10),
            "removed torrent must be pruned from selection"
        );
        assert!(
            screen.selected_ids.contains(&11),
            "surviving torrent stays selected"
        );
        assert_eq!(
            screen.selection_anchor, None,
            "anchor must be cleared when the anchor torrent disappears"
        );
    }

    /// `prune_selection_to_visible` removes filtered-out IDs from `selected_ids`.
    #[test]
    fn prune_selection_to_visible_removes_filtered_ids() {
        let mut screen = make_screen();
        screen.torrents = vec![
            TorrentData {
                id: 1,
                name: "Seeder".to_owned(),
                status: 6,
                ..Default::default()
            },
            TorrentData {
                id: 2,
                name: "Paused".to_owned(),
                status: 0,
                ..Default::default()
            },
        ];
        screen.selected_ids = [1, 2].into_iter().collect();
        screen.selection_anchor = Some(2);
        // Only Seeding filter active — torrent 2 (Paused) is now hidden.
        screen.filters = [StatusFilter::Seeding].into_iter().collect();

        screen.prune_selection_to_visible();

        assert!(screen.selected_ids.contains(&1));
        assert!(!screen.selected_ids.contains(&2));
        assert_eq!(screen.selection_anchor, None);
    }

    /// Shift-click range selection only spans visible (non-filtered) torrents.
    #[test]
    fn shift_click_range_respects_filter() {
        let mut screen = make_screen();
        // Five torrents: seeders (1, 3, 5) and paused (2, 4).
        screen.torrents = vec![
            TorrentData {
                id: 1,
                name: "S1".to_owned(),
                status: 6,
                ..Default::default()
            },
            TorrentData {
                id: 2,
                name: "P1".to_owned(),
                status: 0,
                ..Default::default()
            },
            TorrentData {
                id: 3,
                name: "S2".to_owned(),
                status: 6,
                ..Default::default()
            },
            TorrentData {
                id: 4,
                name: "P2".to_owned(),
                status: 0,
                ..Default::default()
            },
            TorrentData {
                id: 5,
                name: "S3".to_owned(),
                status: 6,
                ..Default::default()
            },
        ];
        screen.filters = [StatusFilter::Seeding].into_iter().collect();

        // Plain-click torrent 1 (first visible seeder) to set the anchor.
        let _ = update(&mut screen, Message::TorrentSelected(1));
        assert_eq!(screen.selection_anchor, Some(1));

        // Shift-click torrent 5; the range should include only visible torrents (1,3,5).
        screen.modifiers = Modifiers::SHIFT;
        let _ = update(&mut screen, Message::TorrentSelected(5));
        assert!(screen.selected_ids.contains(&1));
        assert!(screen.selected_ids.contains(&3));
        assert!(screen.selected_ids.contains(&5));
        // Paused torrents must not be in the selection.
        assert!(!screen.selected_ids.contains(&2));
        assert!(!screen.selected_ids.contains(&4));
    }

    /// `KeyboardSelectAll` (Cmd+A) selects only visible torrents, not hidden ones.
    #[test]
    fn keyboard_select_all_only_selects_visible() {
        let mut screen = make_screen();
        screen.torrents = vec![
            TorrentData {
                id: 1,
                name: "Seeder".to_owned(),
                status: 6,
                ..Default::default()
            },
            TorrentData {
                id: 2,
                name: "Paused".to_owned(),
                status: 0,
                ..Default::default()
            },
        ];
        screen.filters = [StatusFilter::Seeding].into_iter().collect();

        let _ = update(&mut screen, Message::KeyboardSelectAll);

        assert!(
            screen.selected_ids.contains(&1),
            "visible torrent must be selected"
        );
        assert!(
            !screen.selected_ids.contains(&2),
            "filtered-out torrent must not be selected by Cmd+A"
        );
    }
}
