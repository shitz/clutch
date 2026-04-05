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

//! All RPC calls from the main screen flow through [`RpcWork`] items processed
//! by the worker subscription. This guarantees at most one in-flight HTTP
//! connection to the daemon at any time.

use super::api;
use super::error::RpcError;
use super::models::{AddPayload, ConnectionParams, TorrentData};

/// A unit of work for the serialized RPC worker subscription.
///
/// Each variant carries all parameters needed to execute one RPC call.
#[derive(Debug, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum RpcWork {
    TorrentGet(ConnectionParams),
    TorrentStart {
        params: ConnectionParams,
        id: i64,
    },
    TorrentStop {
        params: ConnectionParams,
        id: i64,
    },
    TorrentRemove {
        params: ConnectionParams,
        id: i64,
        delete_local_data: bool,
    },
    TorrentAdd {
        params: ConnectionParams,
        payload: AddPayload,
        download_dir: Option<String>,
        files_unwanted: Vec<i64>,
    },
    SetFileWanted {
        params: ConnectionParams,
        torrent_id: i64,
        file_indices: Vec<i64>,
        wanted: bool,
    },
}

/// The typed outcome of one [`RpcWork`] item.
#[derive(Debug)]
pub enum RpcResult {
    TorrentsLoaded(Result<Vec<TorrentData>, RpcError>),
    ActionDone(Result<(), RpcError>),
    TorrentAdded(Result<(), RpcError>),
    /// Outcome of a `torrent-set` file-wanted call.
    /// Carries the original `Vec<usize>` indices so the caller can remove
    /// them from `pending_wanted` regardless of success or failure.
    FileWantedSet(Result<(), RpcError>, Vec<usize>),
}

/// Retry a single RPC call once on session rotation (HTTP 409).
///
/// Returns `(Some(new_id), result)` if rotation occurred, `(None, result)` otherwise.
/// Inlined per-arm because Rust async closures cannot express the required
/// higher-ranked lifetime bound on `&str`.
///
/// Execute one [`RpcWork`] item, retrying once on session rotation.
///
/// Returns `(new_session_id, result)` where `new_session_id` is `Some` when
/// the daemon returned 409.
pub async fn execute_work(work: RpcWork) -> (Option<String>, RpcResult) {
    match work {
        RpcWork::TorrentGet(p) => {
            match api::torrent_get(&p.url, &p.credentials, &p.session_id).await {
                Err(RpcError::SessionRotated(new_id)) => {
                    let r = api::torrent_get(&p.url, &p.credentials, &new_id).await;
                    (Some(new_id), RpcResult::TorrentsLoaded(r))
                }
                other => (None, RpcResult::TorrentsLoaded(other)),
            }
        }
        RpcWork::TorrentStart { params: p, id } => {
            match api::torrent_start(&p.url, &p.credentials, &p.session_id, id).await {
                Err(RpcError::SessionRotated(new_id)) => {
                    let r = api::torrent_start(&p.url, &p.credentials, &new_id, id).await;
                    (Some(new_id), RpcResult::ActionDone(r))
                }
                other => (None, RpcResult::ActionDone(other)),
            }
        }
        RpcWork::TorrentStop { params: p, id } => {
            match api::torrent_stop(&p.url, &p.credentials, &p.session_id, id).await {
                Err(RpcError::SessionRotated(new_id)) => {
                    let r = api::torrent_stop(&p.url, &p.credentials, &new_id, id).await;
                    (Some(new_id), RpcResult::ActionDone(r))
                }
                other => (None, RpcResult::ActionDone(other)),
            }
        }
        RpcWork::TorrentRemove {
            params: p,
            id,
            delete_local_data,
        } => {
            match api::torrent_remove(&p.url, &p.credentials, &p.session_id, id, delete_local_data)
                .await
            {
                Err(RpcError::SessionRotated(new_id)) => {
                    let r =
                        api::torrent_remove(&p.url, &p.credentials, &new_id, id, delete_local_data)
                            .await;
                    (Some(new_id), RpcResult::ActionDone(r))
                }
                other => (None, RpcResult::ActionDone(other)),
            }
        }
        RpcWork::TorrentAdd {
            params: p,
            payload,
            download_dir,
            files_unwanted,
        } => {
            match api::torrent_add(
                &p.url,
                &p.credentials,
                &p.session_id,
                payload.clone(),
                download_dir.clone(),
                files_unwanted.clone(),
            )
            .await
            {
                Err(RpcError::SessionRotated(new_id)) => {
                    let r = api::torrent_add(
                        &p.url,
                        &p.credentials,
                        &new_id,
                        payload,
                        download_dir,
                        files_unwanted,
                    )
                    .await;
                    (Some(new_id), RpcResult::TorrentAdded(r))
                }
                other => (None, RpcResult::TorrentAdded(other)),
            }
        }
        RpcWork::SetFileWanted {
            params: p,
            torrent_id,
            file_indices,
            wanted,
        } => {
            // Convert Vec<i64> indices back to Vec<usize> for the result payload.
            let usize_indices: Vec<usize> = file_indices.iter().map(|&i| i as usize).collect();
            match api::torrent_set_file_wanted(
                &p.url,
                &p.credentials,
                &p.session_id,
                torrent_id,
                &file_indices,
                wanted,
            )
            .await
            {
                Err(RpcError::SessionRotated(new_id)) => {
                    let r = api::torrent_set_file_wanted(
                        &p.url,
                        &p.credentials,
                        &new_id,
                        torrent_id,
                        &file_indices,
                        wanted,
                    )
                    .await;
                    (Some(new_id), RpcResult::FileWantedSet(r, usize_indices))
                }
                other => (None, RpcResult::FileWantedSet(other, usize_indices)),
            }
        }
    }
}
