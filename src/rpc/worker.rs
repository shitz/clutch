//! Serialized RPC worker types and execution logic.
//!
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
    },
}

/// The typed outcome of one [`RpcWork`] item.
#[derive(Debug)]
pub enum RpcResult {
    TorrentsLoaded(Result<Vec<TorrentData>, RpcError>),
    ActionDone(Result<(), RpcError>),
    TorrentAdded(Result<(), RpcError>),
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
        } => {
            match api::torrent_add(
                &p.url,
                &p.credentials,
                &p.session_id,
                payload.clone(),
                download_dir.clone(),
            )
            .await
            {
                Err(RpcError::SessionRotated(new_id)) => {
                    let r =
                        api::torrent_add(&p.url, &p.credentials, &new_id, payload, download_dir)
                            .await;
                    (Some(new_id), RpcResult::TorrentAdded(r))
                }
                other => (None, RpcResult::TorrentAdded(other)),
            }
        }
    }
}
