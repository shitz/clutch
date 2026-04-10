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

//! Elm update logic for the torrent list screen.

use base64::Engine as _;
use iced::Task;

use crate::rpc::{AddPayload, RpcWork};

use super::add_dialog::{self, AddDialogState, FileReadResult, TorrentFileInfo};
use super::sort::SortDir;
use super::{Message, SetLocationDialog, StatusFilter, TorrentListScreen};

use std::collections::VecDeque;

/// Apply a single torrent-list message to the screen state and return follow-up work.
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
            state.initial_load_done = true;
            state.error = None;
            // Prune any selected IDs for torrents that are no longer reported
            // by the daemon (removed, finished and cleaned up, etc.).
            state.prune_selection_to_visible();
            Task::none()
        }

        Message::TorrentsUpdated(Err(err)) => {
            tracing::error!(error = %err, "torrent-get failed");
            state.is_loading = false;
            state.initial_load_done = true;
            state.error = Some(err);
            Task::none()
        }

        Message::SessionIdRotated(new_id) => {
            tracing::debug!(%new_id, "Persistent session ID updated after rotation");
            state.params.session_id = new_id;
            Task::none()
        }

        Message::RpcWorkerReady(tx) => {
            tracing::debug!("RPC worker ready, accepting work");
            state.sender = Some(tx);
            Task::none()
        }

        // ── Row selection ─────────────────────────────────────────────────────
        Message::TorrentSelected(id) => {
            if state.modifiers.shift() && state.selection_anchor.is_some() {
                // Shift-click: extend the selection from the anchor to `id`.
                // Resolve indices against the current visible order at click time.
                let anchor_id = state.selection_anchor.unwrap();
                let range_ids: Option<Vec<i64>> = {
                    let visible = state.visible_torrents();
                    let anchor_pos = visible.iter().position(|t| t.id == anchor_id);
                    let target_pos = visible.iter().position(|t| t.id == id);
                    if let (Some(a), Some(b)) = (anchor_pos, target_pos) {
                        let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
                        Some(visible[lo..=hi].iter().map(|t| t.id).collect())
                    } else {
                        None
                    }
                };
                if let Some(ids) = range_ids {
                    for rid in ids {
                        state.selected_ids.insert(rid);
                    }
                } else {
                    // Anchor not in current visible set — fall back to plain-click.
                    state.selected_ids.clear();
                    state.selected_ids.insert(id);
                    state.selection_anchor = Some(id);
                }
            } else if state.modifiers.command() || state.modifiers.control() {
                // Ctrl/Cmd-click: toggle membership, update anchor.
                if state.selected_ids.contains(&id) {
                    state.selected_ids.remove(&id);
                } else {
                    state.selected_ids.insert(id);
                }
                state.selection_anchor = Some(id);
            } else {
                // Plain click: if this torrent is already the only selection,
                // deselect it; otherwise narrow the selection to just this one.
                if state.selected_ids.len() == 1 && state.selected_ids.contains(&id) {
                    state.selected_ids.clear();
                    state.selection_anchor = None;
                } else {
                    state.selected_ids.clear();
                    state.selected_ids.insert(id);
                    state.selection_anchor = Some(id);
                }
            }
            Task::none()
        }

        Message::ClearSelection => {
            state.selected_ids.clear();
            state.selection_anchor = None;
            Task::none()
        }

        Message::ModifiersChanged(m) => {
            state.modifiers = m;
            Task::none()
        }

        Message::KeyboardSelectAll => {
            // Select every currently visible (filtered + sorted) torrent.
            let visible_ids: Vec<i64> = state.visible_torrents().iter().map(|t| t.id).collect();
            for id in &visible_ids {
                state.selected_ids.insert(*id);
            }
            state.selection_anchor = visible_ids.first().copied();
            Task::none()
        }

        // ── Toolbar actions ───────────────────────────────────────────────────
        Message::PauseClicked => {
            let ids: Vec<i64> = state.selected_ids.iter().copied().collect();
            if !ids.is_empty() {
                tracing::info!(?ids, "Pausing torrents");
                state.is_loading = true;
                state.enqueue(RpcWork::TorrentStop {
                    params: state.params.clone(),
                    ids,
                });
            }
            Task::none()
        }

        Message::ResumeClicked => {
            let ids: Vec<i64> = state.selected_ids.iter().copied().collect();
            if !ids.is_empty() {
                tracing::info!(?ids, "Resuming torrents");
                state.is_loading = true;
                state.enqueue(RpcWork::TorrentStart {
                    params: state.params.clone(),
                    ids,
                });
            }
            Task::none()
        }

        Message::DeleteClicked => {
            if !state.selected_ids.is_empty() {
                let ids: Vec<i64> = state.selected_ids.iter().copied().collect();
                state.confirming_delete = Some((ids, false));
            }
            Task::none()
        }

        Message::DeleteLocalDataToggled(val) => {
            if let Some((_, del)) = &mut state.confirming_delete {
                *del = val;
            }
            Task::none()
        }

        Message::DeleteCancelled => {
            state.confirming_delete = None;
            Task::none()
        }

        Message::DeleteConfirmed => {
            if let Some((ids, delete_local_data)) = state.confirming_delete.take() {
                tracing::info!(?ids, delete_local_data, "Deleting torrents");
                state.is_loading = true;
                state.enqueue(RpcWork::TorrentRemove {
                    params: state.params.clone(),
                    ids,
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
                let handles = rfd::AsyncFileDialog::new()
                    .add_filter("Torrent", &["torrent"])
                    .pick_files()
                    .await;
                let Some(handles) = handles else {
                    return Err("cancelled".to_owned());
                };
                let mut queue: VecDeque<FileReadResult> = VecDeque::new();
                for handle in handles {
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
                    queue.push_back(FileReadResult {
                        metainfo_b64: b64,
                        files,
                    });
                }
                if queue.is_empty() {
                    return Err("cancelled".to_owned());
                }
                Ok(queue)
            },
            Message::TorrentFileRead,
        ),

        Message::TorrentFileRead(Ok(mut queue)) => {
            let total_count = queue.len();
            let Some(first) = queue.pop_front() else {
                return Task::none();
            };
            let n = first.files.len();
            let initial_dest = state
                .recent_download_paths
                .first()
                .cloned()
                .unwrap_or_default();
            state.add_dialog = AddDialogState::AddFile {
                metainfo_b64: first.metainfo_b64,
                files: first.files,
                selected: vec![true; n],
                destination: initial_dest,
                error: None,
                pending_torrents: queue,
                is_dropdown_open: false,
                total_count,
            };
            iced::widget::operation::focus(add_dialog::add_destination_id())
        }

        Message::TorrentFileRead(Err(err)) => {
            if err != "cancelled" {
                tracing::error!(error = %err, "Failed to read torrent file");
                state.error = Some(format!("Could not open torrent file: {err}"));
            }
            Task::none()
        }

        Message::AddLinkClicked => {
            let initial_dest = state
                .recent_download_paths
                .first()
                .cloned()
                .unwrap_or_default();
            state.add_dialog = AddDialogState::AddLink {
                magnet: String::new(),
                destination: initial_dest,
                error: None,
            };
            iced::widget::operation::focus(add_dialog::add_magnet_id())
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

        Message::AddDialogFileToggled(index) => {
            if let AddDialogState::AddFile { selected, .. } = &mut state.add_dialog
                && let Some(v) = selected.get_mut(index)
            {
                *v = !*v;
            }
            Task::none()
        }

        Message::AddDialogSelectAll => {
            if let AddDialogState::AddFile { selected, .. } = &mut state.add_dialog {
                selected.iter_mut().for_each(|v| *v = true);
            }
            Task::none()
        }

        Message::AddDialogDeselectAll => {
            if let AddDialogState::AddFile { selected, .. } = &mut state.add_dialog {
                selected.iter_mut().for_each(|v| *v = false);
            }
            Task::none()
        }

        Message::AddCancelled => {
            state.add_dialog = AddDialogState::Hidden;
            Task::none()
        }

        Message::AddCancelThis => {
            advance_queue(state);
            Task::none()
        }

        Message::AddCancelAll => {
            state.add_dialog = AddDialogState::Hidden;
            Task::none()
        }

        Message::AddDialogToggleDropdown => {
            if let AddDialogState::AddFile {
                is_dropdown_open, ..
            } = &mut state.add_dialog
            {
                *is_dropdown_open = !*is_dropdown_open;
            }
            Task::none()
        }

        Message::AddDialogDismissDropdown => {
            if let AddDialogState::AddFile {
                is_dropdown_open, ..
            } = &mut state.add_dialog
            {
                *is_dropdown_open = false;
            }
            Task::none()
        }

        Message::AddDialogRecentPathSelected(path) => {
            match &mut state.add_dialog {
                AddDialogState::AddFile {
                    destination,
                    is_dropdown_open,
                    ..
                } => {
                    *destination = path;
                    *is_dropdown_open = false;
                }
                AddDialogState::AddLink { destination, .. } => {
                    *destination = path;
                }
                AddDialogState::Hidden => {}
            }
            Task::none()
        }

        // ProfilePathUsed is escalated to AppState via main_screen.
        Message::ProfilePathUsed(_) => Task::none(),

        Message::AddConfirmed => {
            let (payload, download_dir, files_unwanted) = match &state.add_dialog {
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
                        vec![],
                    )
                }
                AddDialogState::AddFile {
                    metainfo_b64,
                    destination,
                    selected,
                    ..
                } => {
                    let unwanted: Vec<i64> = selected
                        .iter()
                        .enumerate()
                        .filter_map(|(i, &want)| if want { None } else { Some(i as i64) })
                        .collect();
                    (
                        AddPayload::Metainfo(metainfo_b64.clone()),
                        Some(destination.clone()),
                        unwanted,
                    )
                }
                AddDialogState::Hidden => return Task::none(),
            };

            let used_path = download_dir.as_deref().unwrap_or("").to_owned();

            state.is_loading = true;
            tracing::info!("Submitting torrent-add");
            state.enqueue(RpcWork::TorrentAdd {
                params: state.params.clone(),
                payload,
                download_dir,
                files_unwanted,
            });

            // Advance the queue immediately (sticky path — destination is not reset).
            advance_queue(state);

            // Notify AppState to persist the path history.
            if !used_path.trim().is_empty() {
                Task::done(Message::ProfilePathUsed(used_path))
            } else {
                Task::none()
            }
        }

        Message::AddCompleted(Ok(())) => {
            tracing::info!("torrent-add succeeded, refreshing list");
            // Dialog was already advanced in AddConfirmed; only refresh the list.
            state.is_loading = true;
            state.enqueue_torrent_get();
            Task::none()
        }

        Message::AddCompleted(Err(err)) => {
            tracing::error!(error = %err, "torrent-add failed");
            state.is_loading = false;
            // The dialog has already advanced; surface the error in the general banner.
            state.error = Some(err);
            Task::none()
        }

        // Disconnect / OpenSettingsClicked are intercepted by the parent.
        Message::Disconnect | Message::OpenSettingsClicked => Task::none(),

        // These are intercepted by MainScreen before reaching here.
        Message::FileWantedSettled(..)
        | Message::SessionDataLoaded(..)
        | Message::BandwidthSaved(..)
        | Message::TurtleModeToggled => Task::none(),

        // ── Add-dialog keyboard ───────────────────────────────────────────────
        Message::DialogTabKeyPressed { shift } => match &state.add_dialog {
            AddDialogState::AddLink { .. } => {
                // Use iced's built-in focus cycling so clicking a field and
                // then pressing Tab continues from the right place.
                if shift {
                    iced::widget::operation::focus_previous()
                } else {
                    iced::widget::operation::focus_next()
                }
            }
            // File mode has only one text input — Tab is a no-op.
            _ => Task::none(),
        },

        Message::DialogEnterPressed => {
            let should_confirm = match &state.add_dialog {
                AddDialogState::AddLink { magnet, .. } => !magnet.trim().is_empty(),
                AddDialogState::AddFile { metainfo_b64, .. } => !metainfo_b64.is_empty(),
                AddDialogState::Hidden => false,
            };
            if should_confirm {
                update(state, Message::AddConfirmed)
            } else {
                Task::none()
            }
        }

        // ── Column sort ───────────────────────────────────────────────────────
        Message::ColumnHeaderClicked(col) => {
            match &state.sort_column {
                Some(current) if *current == col => match state.sort_dir {
                    SortDir::Asc => state.sort_dir = SortDir::Desc,
                    SortDir::Desc => state.sort_column = None,
                },
                _ => {
                    state.sort_column = Some(col);
                    state.sort_dir = SortDir::Asc;
                }
            }
            Task::none()
        }

        // ── Filter chips ──────────────────────────────────────────────────────
        Message::FilterToggled(filter) => {
            if state.filters.contains(&filter) {
                state.filters.remove(&filter);
            } else {
                state.filters.insert(filter);
            }
            // Selection always reflects only what is visible; prune any IDs
            // that the new filter hides.
            state.prune_selection_to_visible();
            Task::none()
        }

        Message::FilterAllClicked => {
            if state.filters.len() == StatusFilter::all().len() {
                state.filters.clear();
            } else {
                state.filters = StatusFilter::all().into_iter().collect();
            }
            state.prune_selection_to_visible();
            Task::none()
        }

        // ── Cursor tracking ───────────────────────────────────────────────────
        Message::CursorMoved(point) => {
            state.last_cursor_position = point;
            Task::none()
        }

        Message::WindowResized { width, height } => {
            state.window_width = width;
            state.window_height = height;
            Task::none()
        }

        // ── Context menu ──────────────────────────────────────────────────────
        Message::TorrentRightClicked(id) => {
            // If the right-clicked torrent is not already selected, replace the
            // selection with only that torrent. If it is already selected, keep
            // the existing multi-selection so the context menu operates on all.
            if !state.selected_ids.contains(&id) {
                state.selected_ids.clear();
                state.selected_ids.insert(id);
                state.selection_anchor = Some(id);
            }
            state.context_menu = Some((id, state.last_cursor_position));
            Task::none()
        }

        Message::DismissContextMenu => {
            state.context_menu = None;
            Task::none()
        }

        Message::ContextMenuStart => {
            state.context_menu = None;
            let ids: Vec<i64> = state.selected_ids.iter().copied().collect();
            if !ids.is_empty() {
                tracing::info!(?ids, "Resuming torrents via context menu");
                state.is_loading = true;
                state.enqueue(RpcWork::TorrentStart {
                    params: state.params.clone(),
                    ids,
                });
            }
            Task::none()
        }

        Message::ContextMenuPause => {
            state.context_menu = None;
            let ids: Vec<i64> = state.selected_ids.iter().copied().collect();
            if !ids.is_empty() {
                tracing::info!(?ids, "Pausing torrents via context menu");
                state.is_loading = true;
                state.enqueue(RpcWork::TorrentStop {
                    params: state.params.clone(),
                    ids,
                });
            }
            Task::none()
        }

        Message::ContextMenuDelete => {
            state.context_menu = None;
            if !state.selected_ids.is_empty() {
                let ids: Vec<i64> = state.selected_ids.iter().copied().collect();
                state.confirming_delete = Some((ids, false));
            }
            Task::none()
        }

        Message::OpenSetLocation => {
            state.context_menu = None;
            let ids: Vec<i64> = state.selected_ids.iter().copied().collect();
            // Pre-fill path from the first selected torrent's downloadDir.
            let dir = ids
                .first()
                .and_then(|&id| state.torrents.iter().find(|t| t.id == id))
                .map(|t| t.download_dir.clone())
                .unwrap_or_default();
            if !ids.is_empty() {
                state.set_location_dialog = Some(SetLocationDialog {
                    ids,
                    path: dir,
                    move_data: true,
                });
            }
            Task::none()
        }

        // ── Set data location dialog ──────────────────────────────────────────
        Message::SetLocationPathChanged(s) => {
            if let Some(dlg) = &mut state.set_location_dialog {
                dlg.path = s;
            }
            Task::none()
        }

        Message::SetLocationMoveToggled => {
            if let Some(dlg) = &mut state.set_location_dialog {
                dlg.move_data = !dlg.move_data;
            }
            Task::none()
        }

        Message::SetLocationCancel => {
            state.set_location_dialog = None;
            Task::none()
        }

        Message::SetLocationApply => {
            if let Some(dlg) = state.set_location_dialog.take()
                && !dlg.path.trim().is_empty()
            {
                tracing::info!(
                    ids = ?dlg.ids,
                    path = %dlg.path,
                    move_data = dlg.move_data,
                    "Setting torrent data location"
                );
                state.enqueue(RpcWork::SetLocation {
                    params: state.params.clone(),
                    ids: dlg.ids,
                    location: dlg.path,
                    move_data: dlg.move_data,
                });
            }
            Task::none()
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Pop the next torrent from the queue into the active dialog slot, or close
/// the dialog if the queue is exhausted. The destination field is left unchanged
/// (sticky path behavior — the user's input carries across the whole batch).
fn advance_queue(state: &mut TorrentListScreen) {
    // Take the current dialog out of the state so we can destructure it fully.
    let current = std::mem::replace(&mut state.add_dialog, AddDialogState::Hidden);

    let AddDialogState::AddFile {
        mut pending_torrents,
        total_count,
        destination,
        ..
    } = current
    else {
        // Already Hidden or AddLink — leave as Hidden.
        return;
    };

    match pending_torrents.pop_front() {
        Some(next) => {
            let n = next.files.len();
            state.add_dialog = AddDialogState::AddFile {
                metainfo_b64: next.metainfo_b64,
                files: next.files,
                selected: vec![true; n],
                destination, // sticky: unchanged from previous torrent
                error: None,
                pending_torrents,
                is_dropdown_open: false,
                total_count,
            };
        }
        None => {
            // Queue exhausted — dialog is already Hidden from the mem::replace above.
        }
    }
}
