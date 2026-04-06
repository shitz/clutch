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

use base64::Engine as _;
use iced::Task;

use crate::rpc::{AddPayload, RpcWork};

use super::add_dialog::{self, AddDialogState, FileReadResult, TorrentFileInfo};
use super::sort::SortDir;
use super::{Message, TorrentListScreen};

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
                    params: state.params.clone(),
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
                    params: state.params.clone(),
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
                    params: state.params.clone(),
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
            let n = result.files.len();
            state.add_dialog = AddDialogState::AddFile {
                metainfo_b64: result.metainfo_b64,
                files: result.files,
                selected: vec![true; n],
                destination: String::new(),
                error: None,
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
            state.add_dialog = AddDialogState::AddLink {
                magnet: String::new(),
                destination: String::new(),
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
            state.is_loading = true;
            tracing::info!("Submitting torrent-add");
            state.enqueue(RpcWork::TorrentAdd {
                params: state.params.clone(),
                payload,
                download_dir,
                files_unwanted,
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
    }
}
