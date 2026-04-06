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
//! - [`view`] — Widget-tree rendering (toolbar, header, rows)
//! - [`update`] — Elm update function
//! - [`worker`] — Serialized RPC worker subscription stream

pub mod add_dialog;
pub mod sort;
mod update;
pub mod view;
pub mod worker;

use tokio::sync::mpsc;

use crate::rpc::{ConnectionParams, RpcWork, SessionData, TorrentData, TransmissionCredentials};

use add_dialog::AddDialogState;
use sort::{SortColumn, SortDir};

// Re-export for parent modules.
pub use add_dialog::FileReadResult;
pub use update::update;
pub use view::view;
pub use worker::rpc_worker_stream;

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
    AddDialogFileToggled(usize),
    AddDialogSelectAll,
    AddDialogDeselectAll,
    AddConfirmed,
    AddCancelled,
    AddCompleted(Result<(), String>),
    /// Fired when a SetFileWanted RPC completes (success or failure).
    /// Carries the file indices so the inspector can clear pending_wanted.
    FileWantedSettled(bool, Vec<usize>),
    /// Fired when a periodic session-get poll completes.
    SessionDataLoaded(Result<SessionData, String>),
    /// Fired when a torrent-set bandwidth call completes.
    BandwidthSaved(Result<(), String>),
    /// Fired by the toolbar turtle button — intercepted by main_screen.
    TurtleModeToggled,
    // Escalated to parent — intercepted by MainScreen before reaching update()
    Disconnect,
    // Escalated to app — opens Settings screen
    OpenSettingsClicked,
    // Column sort
    ColumnHeaderClicked(SortColumn),
    // Keyboard
    /// Tab key pressed while the add-torrent dialog is open.
    DialogTabKeyPressed {
        shift: bool,
    },
    /// Enter key pressed while the add-torrent dialog is open.
    DialogEnterPressed,
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
    pub selected_id: Option<i64>,
    pub confirming_delete: Option<(i64, bool)>,
    pub add_dialog: AddDialogState,
    pub sort_column: Option<SortColumn>,
    pub sort_dir: SortDir,
}

impl TorrentListScreen {
    pub fn new(credentials: TransmissionCredentials, session_id: String) -> Self {
        TorrentListScreen {
            params: ConnectionParams::new(credentials, session_id),
            torrents: Vec::new(),
            is_loading: false,
            initial_load_done: false,
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
    #[must_use]
    pub fn selected_torrent(&self) -> Option<&TorrentData> {
        let id = self.selected_id?;
        self.torrents.iter().find(|t| t.id == id)
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

    /// 6.1 – Tick when is_loading=true: no command, state unchanged.
    #[test]
    fn tick_ignored_when_loading() {
        let mut screen = make_screen();
        screen.is_loading = true;
        let _ = update(&mut screen, Message::Tick);
        assert!(screen.is_loading);
    }

    /// 6.2 – Tick when is_loading=false: is_loading becomes true.
    #[test]
    fn tick_fires_when_not_loading() {
        let mut screen = make_screen();
        screen.is_loading = false;
        let _ = update(&mut screen, Message::Tick);
        assert!(screen.is_loading);
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
        assert_eq!(screen.params.session_id, "new-id-xyz");
    }

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

    /// 8.3 – DeleteClicked sets confirming_delete.
    #[test]
    fn delete_clicked_sets_confirming() {
        let mut screen = make_screen();
        screen.torrents = vec![make_torrent(5, "Arch")];
        screen.selected_id = Some(5);
        let _ = update(&mut screen, Message::DeleteClicked);
        assert_eq!(screen.confirming_delete, Some((5, false)));
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
        let _ = update(&mut screen, Message::DeleteConfirmed);
        assert_eq!(screen.confirming_delete, None);
        assert!(screen.is_loading);
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
        let _ = update(&mut screen, Message::ActionCompleted(Ok(())));
        assert!(screen.is_loading);
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
        let _ = update(&mut screen, Message::Tick);
        assert!(screen.is_loading);
    }

    // ── 9.x  add-torrent dialog tests ─────────────────────────────────────────

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
        let _ = update(&mut screen, Message::AddConfirmed);
        assert!(matches!(screen.add_dialog, AddDialogState::AddLink { .. }));
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
        let _ = update(&mut screen, Message::AddConfirmed);
        assert!(screen.is_loading);
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
            files: vec![add_dialog::TorrentFileInfo {
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
        let _ = update(&mut screen, Message::AddCompleted(Ok(())));
        assert!(matches!(screen.add_dialog, AddDialogState::Hidden));
        assert!(screen.is_loading);
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

    /// 11.1 – ColumnHeaderClicked cycles Unsorted → Asc → Desc → Unsorted.
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

    /// 11.2 – Clicking a different column starts Asc on the new column.
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

    // ── 5.4 – Selective file selection ───────────────────────────────────────

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
        };
        screen
    }

    /// 5.4a – AddDialogSelectAll sets every entry in `selected` to `true`.
    #[test]
    fn add_dialog_select_all_sets_all_true() {
        let mut screen = make_add_file_screen(3);
        // Deselect one first.
        let _ = update(&mut screen, Message::AddDialogFileToggled(1));
        // Select all.
        let _ = update(&mut screen, Message::AddDialogSelectAll);
        if let AddDialogState::AddFile { selected, .. } = &screen.add_dialog {
            assert_eq!(selected, &vec![true, true, true]);
        } else {
            panic!("expected AddFile state");
        }
    }

    /// 5.4b – AddDialogDeselectAll sets every entry in `selected` to `false`.
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

    /// 5.4c – AddDialogFileToggled flips only the targeted index.
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
}
