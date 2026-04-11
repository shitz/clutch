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

//! Each function performs exactly one logical RPC operation. Session rotation
//! (409) is surfaced as `RpcError::SessionRotated` — the caller must retry.
//! The exception is `session_get`, which retries once automatically as it is
//! used as the initial connectivity probe.

use std::time::Duration;

use super::error::RpcError;
use super::models::{
    AddPayload, SessionData, SessionSetArgs, TorrentBandwidthArgs, TorrentData,
    TransmissionCredentials,
};
use super::transport::{RpcResponse, post_rpc};

/// Check that an RPC response has `"success"` as its result string.
fn check_success(resp: RpcResponse, method: &str) -> Result<(), RpcError> {
    if resp.result == "success" {
        tracing::info!(method, "RPC call succeeded");
        Ok(())
    } else {
        tracing::error!(method, result = %resp.result, "RPC call returned non-success");
        Err(RpcError::ParseError(format!(
            "{method} failed: {}",
            resp.result
        )))
    }
}

/// Probe the daemon with a `session-get` call.
///
/// Handles one level of session rotation automatically. Returns a
/// [`SessionData`] containing the session ID and current alternative speed
/// limit settings.
pub async fn session_get(
    url: &str,
    credentials: &TransmissionCredentials,
    session_id: &str,
) -> Result<SessionData, RpcError> {
    tracing::debug!(%url, "Probing daemon with session-get");
    let args = serde_json::json!({
        "fields": ["alt-speed-enabled", "alt-speed-down", "alt-speed-up",
                   "download-queue-enabled", "download-queue-size",
                   "seed-queue-enabled", "seed-queue-size"]
    });
    match post_rpc(
        url,
        credentials,
        session_id,
        "session-get",
        Some(args.clone()),
        Duration::from_secs(5),
    )
    .await
    {
        Ok(resp) => {
            tracing::info!(%url, %session_id, "session-get probe succeeded");
            Ok(parse_session_data(session_id.to_owned(), &resp.arguments))
        }
        Err(RpcError::SessionRotated(new_id)) => {
            tracing::debug!(%url, new_id = %new_id, "Session ID rotated during probe, retrying once");
            match post_rpc(
                url,
                credentials,
                &new_id,
                "session-get",
                Some(args),
                Duration::from_secs(5),
            )
            .await
            {
                Ok(resp) => {
                    tracing::info!(%url, session_id = %new_id, "session-get probe succeeded after rotation");
                    Ok(parse_session_data(new_id, &resp.arguments))
                }
                Err(e) => {
                    tracing::error!(error = %e, %url, "session-get retry failed after rotation");
                    Err(e)
                }
            }
        }
        Err(e) => {
            tracing::error!(error = %e, %url, "session-get probe failed");
            Err(e)
        }
    }
}

fn parse_session_data(session_id: String, arguments: &serde_json::Value) -> SessionData {
    SessionData {
        session_id,
        alt_speed_enabled: arguments
            .get("alt-speed-enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        alt_speed_down: arguments
            .get("alt-speed-down")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        alt_speed_up: arguments
            .get("alt-speed-up")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        download_queue_enabled: arguments
            .get("download-queue-enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        download_queue_size: arguments
            .get("download-queue-size")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
        seed_queue_enabled: arguments
            .get("seed-queue-enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        seed_queue_size: arguments
            .get("seed-queue-size")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32,
    }
}

/// Update session-level settings on the daemon.
pub async fn session_set(
    url: &str,
    credentials: &TransmissionCredentials,
    session_id: &str,
    args: &SessionSetArgs,
) -> Result<(), RpcError> {
    tracing::debug!(%url, %session_id, "Sending session-set");
    let payload = serde_json::to_value(args).map_err(|e| RpcError::ParseError(e.to_string()))?;
    let resp = post_rpc(
        url,
        credentials,
        session_id,
        "session-set",
        Some(payload),
        Duration::from_secs(10),
    )
    .await?;
    check_success(resp, "session-set")
}

/// Set per-torrent bandwidth limits on the daemon for one or more torrents.
pub async fn torrent_set_bandwidth(
    url: &str,
    credentials: &TransmissionCredentials,
    session_id: &str,
    ids: &[i64],
    args: TorrentBandwidthArgs,
) -> Result<(), RpcError> {
    tracing::debug!(%url, %session_id, ?ids, "Sending torrent-set (bandwidth)");
    let mut payload =
        serde_json::to_value(&args).map_err(|e| RpcError::ParseError(e.to_string()))?;
    payload["ids"] = serde_json::json!(ids);
    let resp = post_rpc(
        url,
        credentials,
        session_id,
        "torrent-set",
        Some(payload),
        Duration::from_secs(10),
    )
    .await?;
    check_success(resp, "torrent-set (bandwidth)")
}

/// Fetch the complete torrent list from the daemon.
pub async fn torrent_get(
    url: &str,
    credentials: &TransmissionCredentials,
    session_id: &str,
) -> Result<Vec<TorrentData>, RpcError> {
    tracing::debug!(%url, %session_id, "Fetching torrent list");
    let args = serde_json::json!({
        "fields": [
            "id", "name", "status", "percentDone",
            "totalSize", "downloadedEver", "uploadedEver", "uploadRatio",
            "eta", "rateDownload", "rateUpload",
            "files", "fileStats", "trackerStats", "peers",
            "downloadLimited", "downloadLimit",
            "uploadLimited", "uploadLimit",
            "seedRatioLimit", "seedRatioMode",
            "honorsSessionLimits",
            "downloadDir",
            "error", "errorString"
        ]
    });

    let resp = post_rpc(
        url,
        credentials,
        session_id,
        "torrent-get",
        Some(args),
        Duration::from_secs(10),
    )
    .await?;

    let torrents: Vec<TorrentData> = serde_json::from_value(resp.arguments["torrents"].clone())
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to deserialize torrent list");
            RpcError::ParseError(e.to_string())
        })?;

    tracing::debug!(
        count = torrents.len(),
        "torrent-get deserialized successfully"
    );
    Ok(torrents)
}

/// Start (resume) one or more torrents by their Transmission IDs.
pub async fn torrent_start(
    url: &str,
    credentials: &TransmissionCredentials,
    session_id: &str,
    ids: &[i64],
) -> Result<(), RpcError> {
    tracing::debug!(%url, %session_id, ?ids, "Sending torrent-start");
    let args = serde_json::json!({ "ids": ids });
    let resp = post_rpc(
        url,
        credentials,
        session_id,
        "torrent-start",
        Some(args),
        Duration::from_secs(10),
    )
    .await?;
    check_success(resp, "torrent-start")
}

/// Pause (stop) one or more torrents by their Transmission IDs.
pub async fn torrent_stop(
    url: &str,
    credentials: &TransmissionCredentials,
    session_id: &str,
    ids: &[i64],
) -> Result<(), RpcError> {
    tracing::debug!(%url, %session_id, ?ids, "Sending torrent-stop");
    let args = serde_json::json!({ "ids": ids });
    let resp = post_rpc(
        url,
        credentials,
        session_id,
        "torrent-stop",
        Some(args),
        Duration::from_secs(10),
    )
    .await?;
    check_success(resp, "torrent-stop")
}

/// Remove one or more torrents. When `delete_local_data` is `true` the daemon also
/// deletes all downloaded files from disk.
pub async fn torrent_remove(
    url: &str,
    credentials: &TransmissionCredentials,
    session_id: &str,
    ids: &[i64],
    delete_local_data: bool,
) -> Result<(), RpcError> {
    tracing::debug!(%url, %session_id, ?ids, delete_local_data, "Sending torrent-remove");
    let args = serde_json::json!({ "ids": ids, "delete-local-data": delete_local_data });
    let resp = post_rpc(
        url,
        credentials,
        session_id,
        "torrent-remove",
        Some(args),
        Duration::from_secs(10),
    )
    .await?;
    check_success(resp, "torrent-remove")
}

/// Add a new torrent to the daemon.
///
/// Both `"success"` and `"torrent-duplicate"` are treated as `Ok(())`.
/// Uses a 60 s timeout — Transmission does synchronous disk I/O before
/// responding to `torrent-add`.
pub async fn torrent_add(
    url: &str,
    credentials: &TransmissionCredentials,
    session_id: &str,
    payload: AddPayload,
    download_dir: Option<String>,
    files_unwanted: Vec<i64>,
) -> Result<(), RpcError> {
    tracing::debug!(%url, %session_id, "Sending torrent-add");
    let mut args = match &payload {
        AddPayload::Magnet(uri) => serde_json::json!({ "filename": uri }),
        AddPayload::Metainfo(b64) => serde_json::json!({ "metainfo": b64 }),
    };
    if let Some(dir) = download_dir.as_deref().filter(|d| !d.is_empty()) {
        args["download-dir"] = serde_json::Value::String(dir.to_owned());
    }
    if !files_unwanted.is_empty() {
        args["files-unwanted"] = serde_json::json!(files_unwanted);
    }
    let resp = post_rpc(
        url,
        credentials,
        session_id,
        "torrent-add",
        Some(args),
        Duration::from_secs(60),
    )
    .await?;
    if resp.result == "success" || resp.result == "torrent-duplicate" {
        tracing::info!(result = %resp.result, "torrent-add succeeded");
        Ok(())
    } else {
        tracing::error!(result = %resp.result, "torrent-add returned non-success");
        Err(RpcError::ParseError(format!(
            "torrent-add failed: {}",
            resp.result
        )))
    }
}

/// Move one or more torrents to the top of the queue.
pub async fn queue_move_top(
    url: &str,
    credentials: &TransmissionCredentials,
    session_id: &str,
    ids: &[i64],
) -> Result<(), RpcError> {
    tracing::debug!(%url, %session_id, ?ids, "Sending queue-move-top");
    let args = serde_json::json!({ "ids": ids });
    let resp = post_rpc(
        url,
        credentials,
        session_id,
        "queue-move-top",
        Some(args),
        Duration::from_secs(10),
    )
    .await?;
    check_success(resp, "queue-move-top")
}

/// Move one or more torrents up one position in the queue.
pub async fn queue_move_up(
    url: &str,
    credentials: &TransmissionCredentials,
    session_id: &str,
    ids: &[i64],
) -> Result<(), RpcError> {
    tracing::debug!(%url, %session_id, ?ids, "Sending queue-move-up");
    let args = serde_json::json!({ "ids": ids });
    let resp = post_rpc(
        url,
        credentials,
        session_id,
        "queue-move-up",
        Some(args),
        Duration::from_secs(10),
    )
    .await?;
    check_success(resp, "queue-move-up")
}

/// Move one or more torrents down one position in the queue.
pub async fn queue_move_down(
    url: &str,
    credentials: &TransmissionCredentials,
    session_id: &str,
    ids: &[i64],
) -> Result<(), RpcError> {
    tracing::debug!(%url, %session_id, ?ids, "Sending queue-move-down");
    let args = serde_json::json!({ "ids": ids });
    let resp = post_rpc(
        url,
        credentials,
        session_id,
        "queue-move-down",
        Some(args),
        Duration::from_secs(10),
    )
    .await?;
    check_success(resp, "queue-move-down")
}

/// Move one or more torrents to the bottom of the queue.
pub async fn queue_move_bottom(
    url: &str,
    credentials: &TransmissionCredentials,
    session_id: &str,
    ids: &[i64],
) -> Result<(), RpcError> {
    tracing::debug!(%url, %session_id, ?ids, "Sending queue-move-bottom");
    let args = serde_json::json!({ "ids": ids });
    let resp = post_rpc(
        url,
        credentials,
        session_id,
        "queue-move-bottom",
        Some(args),
        Duration::from_secs(10),
    )
    .await?;
    check_success(resp, "queue-move-bottom")
}

/// Relocate one or more torrents' data on the daemon's filesystem.
///
/// When `move_data` is `true`, the daemon physically moves the existing
/// files to `location`. When `false`, it only updates its internal record
/// without touching the data (useful when you've already moved the files
/// manually).
pub async fn torrent_set_location(
    url: &str,
    credentials: &TransmissionCredentials,
    session_id: &str,
    ids: &[i64],
    location: &str,
    move_data: bool,
) -> Result<(), RpcError> {
    tracing::debug!(%url, %session_id, ?ids, %location, move_data, "Sending torrent-set-location");
    let args = serde_json::json!({ "ids": ids, "location": location, "move": move_data });
    let resp = post_rpc(
        url,
        credentials,
        session_id,
        "torrent-set-location",
        Some(args),
        Duration::from_secs(30),
    )
    .await?;
    check_success(resp, "torrent-set-location")
}

/// Relocate one or more torrents' data on the daemon's filesystem.
///
/// Pass `wanted = true` to schedule files for download; `false` to skip them.
/// The `file_indices` slice contains zero-based indices matching the `files`
/// array position in `torrent-get` responses.
pub async fn torrent_set_file_wanted(
    url: &str,
    credentials: &TransmissionCredentials,
    session_id: &str,
    torrent_id: i64,
    file_indices: &[i64],
    wanted: bool,
) -> Result<(), RpcError> {
    tracing::debug!(%url, %session_id, torrent_id, wanted, "Sending torrent-set (file wanted)");
    let field = if wanted {
        "files-wanted"
    } else {
        "files-unwanted"
    };
    let args = serde_json::json!({
        "ids": [torrent_id],
        field: file_indices,
    });
    let resp = post_rpc(
        url,
        credentials,
        session_id,
        "torrent-set",
        Some(args),
        Duration::from_secs(10),
    )
    .await?;
    check_success(resp, "torrent-set")
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::super::models::{SessionSetArgs, TorrentBandwidthArgs};
    use super::super::transport::SESSION_ID_HEADER;

    fn test_credentials() -> TransmissionCredentials {
        TransmissionCredentials {
            host: "127.0.0.1".to_owned(),
            port: 0,
            username: None,
            password: None,
        }
    }

    // ── session_get ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn session_get_populates_alt_speed_fields() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": "success",
                "arguments": {
                    "alt-speed-enabled": true,
                    "alt-speed-down": 500,
                    "alt-speed-up": 50
                }
            })))
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let data = session_get(&url, &creds, "sid").await.unwrap();
        assert!(data.alt_speed_enabled);
        assert_eq!(data.alt_speed_down, 500);
        assert_eq!(data.alt_speed_up, 50);
    }

    // ── session_get ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn session_get_success() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "result": "success", "arguments": {} })),
            )
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let result = session_get(&url, &creds, "my-session-id").await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().session_id, "my-session-id");
    }

    #[tokio::test]
    async fn session_get_retries_on_session_rotation() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(ResponseTemplate::new(409).insert_header(SESSION_ID_HEADER, "rotated-id"))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "result": "success", "arguments": {} })),
            )
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let result = session_get(&url, &creds, "stale-id").await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap().session_id, "rotated-id");
    }

    #[tokio::test]
    async fn session_get_auth_failure() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let result = session_get(&url, &creds, "sid").await;
        assert!(matches!(result, Err(RpcError::AuthError)));
    }

    #[tokio::test]
    async fn session_get_connection_error() {
        let creds = test_credentials();
        let result = session_get("http://127.0.0.1:1/transmission/rpc", &creds, "sid").await;
        assert!(matches!(result, Err(RpcError::ConnectionError(_))));
    }

    // ── torrent_get ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn torrent_get_success() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": "success",
                "arguments": {
                    "torrents": [
                        { "id": 1, "name": "Ubuntu ISO", "status": 6, "percentDone": 1.0 },
                        { "id": 2, "name": "Arch Linux", "status": 4, "percentDone": 0.43 }
                    ]
                }
            })))
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let result = torrent_get(&url, &creds, "sid").await;

        assert!(result.is_ok());
        let torrents = result.unwrap();
        assert_eq!(torrents.len(), 2);
        assert_eq!(torrents[0].name, "Ubuntu ISO");
        assert_eq!(torrents[0].status, 6);
        assert!((torrents[1].percent_done - 0.43).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn torrent_get_extended_fields() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": "success",
                "arguments": {
                    "torrents": [{
                        "id": 1, "name": "Test", "status": 4, "percentDone": 0.5,
                        "totalSize": 1073741824,
                        "downloadedEver": 536870912,
                        "uploadedEver": 104857600,
                        "uploadRatio": 0.19,
                        "eta": 300,
                        "rateDownload": 1048576,
                        "rateUpload": 204800,
                        "files": [{ "name": "movie.mkv", "length": 1073741824 }],
                        "fileStats": [{ "bytesCompleted": 536870912 }],
                        "trackerStats": [{
                            "host": "tracker.example.com",
                            "seederCount": 10,
                            "leecherCount": 3,
                            "lastAnnounceTime": 1700000000
                        }],
                        "peers": [{
                            "address": "1.2.3.4",
                            "rateToClient": 512000,
                            "rateToPeer": 0
                        }]
                    }]
                }
            })))
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let torrents = torrent_get(&url, &creds, "sid").await.unwrap();
        let t = &torrents[0];
        assert_eq!(t.total_size, 1073741824);
        assert_eq!(t.downloaded_ever, 536870912);
        assert_eq!(t.uploaded_ever, 104857600);
        assert!((t.upload_ratio - 0.19).abs() < 1e-6);
        assert_eq!(t.eta, 300);
        assert_eq!(t.rate_download, 1048576);
        assert_eq!(t.rate_upload, 204800);
        assert_eq!(t.files.len(), 1);
        assert_eq!(t.files[0].name, "movie.mkv");
        assert_eq!(t.file_stats[0].bytes_completed, 536870912);
        assert_eq!(t.tracker_stats[0].host, "tracker.example.com");
        assert_eq!(t.tracker_stats[0].seeder_count, 10);
        assert_eq!(t.peers[0].address, "1.2.3.4");
        assert_eq!(t.peers[0].rate_to_client, 512000);
    }

    #[tokio::test]
    async fn torrent_get_parse_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "result": "success",
                "arguments": {
                    "torrents": [{ "unexpected_field": true }]
                }
            })))
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let result = torrent_get(&url, &creds, "sid").await;
        assert!(matches!(result, Err(RpcError::ParseError(_))));
    }

    // ── torrent_start ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn torrent_start_success() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "result": "success", "arguments": {} })),
            )
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        assert!(torrent_start(&url, &creds, "sid", &[42]).await.is_ok());
    }

    #[tokio::test]
    async fn torrent_start_session_rotation() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(ResponseTemplate::new(409).insert_header(SESSION_ID_HEADER, "new-sid"))
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let result = torrent_start(&url, &creds, "old-sid", &[1]).await;
        assert!(matches!(result, Err(RpcError::SessionRotated(ref id)) if id == "new-sid"));
    }

    #[tokio::test]
    async fn torrent_start_auth_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        assert!(matches!(
            torrent_start(&url, &creds, "sid", &[1]).await,
            Err(RpcError::AuthError)
        ));
    }

    // ── torrent_stop ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn torrent_stop_success() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "result": "success", "arguments": {} })),
            )
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        assert!(torrent_stop(&url, &creds, "sid", &[7]).await.is_ok());
    }

    #[tokio::test]
    async fn torrent_stop_session_rotation() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(
                ResponseTemplate::new(409).insert_header(SESSION_ID_HEADER, "rotated-sid"),
            )
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let result = torrent_stop(&url, &creds, "old-sid", &[7]).await;
        assert!(matches!(result, Err(RpcError::SessionRotated(ref id)) if id == "rotated-sid"));
    }

    #[tokio::test]
    async fn torrent_stop_auth_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        assert!(matches!(
            torrent_stop(&url, &creds, "sid", &[7]).await,
            Err(RpcError::AuthError)
        ));
    }

    // ── torrent_remove ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn torrent_remove_success_keep_data() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "result": "success", "arguments": {} })),
            )
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        assert!(
            torrent_remove(&url, &creds, "sid", &[3], false)
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn torrent_remove_success_delete_data() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "result": "success", "arguments": {} })),
            )
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        assert!(
            torrent_remove(&url, &creds, "sid", &[3], true)
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn torrent_remove_session_rotation() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(ResponseTemplate::new(409).insert_header(SESSION_ID_HEADER, "new-id"))
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let result = torrent_remove(&url, &creds, "old-id", &[3], false).await;
        assert!(matches!(result, Err(RpcError::SessionRotated(ref id)) if id == "new-id"));
    }

    #[tokio::test]
    async fn torrent_remove_auth_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        assert!(matches!(
            torrent_remove(&url, &creds, "sid", &[3], false).await,
            Err(RpcError::AuthError)
        ));
    }

    // ── torrent_add ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn torrent_add_magnet_success() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "result": "success", "arguments": {} })),
            )
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        assert!(
            torrent_add(
                &url,
                &creds,
                "sid",
                AddPayload::Magnet("magnet:?xt=urn:btih:abc".to_owned()),
                None,
                vec![],
            )
            .await
            .is_ok()
        );
    }

    #[tokio::test]
    async fn torrent_add_metainfo_success() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "result": "success", "arguments": {} })),
            )
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        assert!(
            torrent_add(
                &url,
                &creds,
                "sid",
                AddPayload::Metainfo("dGVzdA==".to_owned()),
                Some("/downloads".to_owned()),
                vec![],
            )
            .await
            .is_ok()
        );
    }

    #[tokio::test]
    async fn torrent_add_duplicate_is_ok() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({ "result": "torrent-duplicate", "arguments": {} }),
            ))
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        assert!(
            torrent_add(
                &url,
                &creds,
                "sid",
                AddPayload::Magnet("magnet:?xt=urn:btih:abc".to_owned()),
                None,
                vec![],
            )
            .await
            .is_ok()
        );
    }

    #[tokio::test]
    async fn torrent_add_empty_download_dir_omitted() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "result": "success", "arguments": {} })),
            )
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        assert!(
            torrent_add(
                &url,
                &creds,
                "sid",
                AddPayload::Magnet("magnet:?xt=urn:btih:abc".to_owned()),
                Some(String::new()),
                vec![],
            )
            .await
            .is_ok()
        );
    }

    #[tokio::test]
    async fn torrent_add_session_rotation() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(ResponseTemplate::new(409).insert_header(SESSION_ID_HEADER, "new-sid"))
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let result = torrent_add(
            &url,
            &creds,
            "old-sid",
            AddPayload::Magnet("magnet:?xt=urn:btih:abc".to_owned()),
            None,
            vec![],
        )
        .await;
        assert!(matches!(result, Err(RpcError::SessionRotated(ref id)) if id == "new-sid"));
    }

    #[tokio::test]
    async fn torrent_add_auth_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        assert!(matches!(
            torrent_add(
                &url,
                &creds,
                "sid",
                AddPayload::Magnet("magnet:?xt=urn:btih:abc".to_owned()),
                None,
                vec![],
            )
            .await,
            Err(RpcError::AuthError)
        ));
    }

    // ── torrent_set_file_wanted ───────────────────────────────────────────────

    /// `torrent_set_file_wanted` with `wanted = true` sends `"files-wanted"`.
    #[tokio::test]
    async fn torrent_set_file_wanted_sends_files_wanted_field() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "result": "success", "arguments": {} })),
            )
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let result = torrent_set_file_wanted(&url, &creds, "sid", 42, &[0, 1, 2], true).await;
        assert!(result.is_ok());

        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body["method"], "torrent-set");
        assert!(
            body["arguments"]["files-wanted"].is_array(),
            "files-wanted must appear in the request body when wanted=true"
        );
        assert!(
            body["arguments"]["files-unwanted"].is_null(),
            "files-unwanted must NOT appear when wanted=true"
        );
    }

    /// `torrent_set_file_wanted` with `wanted = false` sends `"files-unwanted"`.
    #[tokio::test]
    async fn torrent_set_file_wanted_sends_files_unwanted_field() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "result": "success", "arguments": {} })),
            )
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let result = torrent_set_file_wanted(&url, &creds, "sid", 42, &[3, 4], false).await;
        assert!(result.is_ok());

        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body["method"], "torrent-set");
        assert!(
            body["arguments"]["files-unwanted"].is_array(),
            "files-unwanted must appear in the request body when wanted=false"
        );
        assert!(
            body["arguments"]["files-wanted"].is_null(),
            "files-wanted must NOT appear when wanted=false"
        );
    }

    /// `torrent_add` includes `files_unwanted` in the request body when provided.
    #[tokio::test]
    async fn torrent_add_includes_files_unwanted_in_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "result": "success", "arguments": {} })),
            )
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let result = torrent_add(
            &url,
            &creds,
            "sid",
            AddPayload::Magnet("magnet:?xt=urn:btih:abc".to_owned()),
            None,
            vec![1, 2],
        )
        .await;
        assert!(result.is_ok());

        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body["method"], "torrent-add");
        let unwanted = &body["arguments"]["files-unwanted"];
        assert!(
            unwanted.is_array(),
            "files-unwanted must be present when non-empty"
        );
        assert_eq!(
            unwanted.as_array().unwrap().len(),
            2,
            "files-unwanted must contain exactly the supplied indices"
        );
    }

    // ── session_set ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn session_set_sends_only_some_fields() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "result": "success", "arguments": {} })),
            )
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let args = SessionSetArgs {
            alt_speed_enabled: Some(true),
            alt_speed_down: Some(500),
            ..Default::default()
        };
        assert!(session_set(&url, &creds, "sid", &args).await.is_ok());

        let requests = server.received_requests().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body["method"], "session-set");
        assert_eq!(body["arguments"]["alt-speed-enabled"], true);
        assert_eq!(body["arguments"]["alt-speed-down"], 500);
        assert!(
            body["arguments"]["alt-speed-up"].is_null(),
            "alt-speed-up must be absent when None"
        );
    }

    #[tokio::test]
    async fn session_set_session_rotation() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(ResponseTemplate::new(409).insert_header(SESSION_ID_HEADER, "new-id"))
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let args = SessionSetArgs {
            alt_speed_enabled: Some(false),
            ..Default::default()
        };
        let result = session_set(&url, &creds, "old-id", &args).await;
        assert!(matches!(result, Err(RpcError::SessionRotated(ref id)) if id == "new-id"));
    }

    // ── torrent_set_bandwidth ─────────────────────────────────────────────────

    #[tokio::test]
    async fn torrent_set_bandwidth_sends_ids_and_fields() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "result": "success", "arguments": {} })),
            )
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let args = TorrentBandwidthArgs {
            download_limited: Some(true),
            download_limit: Some(1000),
            upload_limited: None,
            ..Default::default()
        };
        assert!(
            torrent_set_bandwidth(&url, &creds, "sid", &[42], args)
                .await
                .is_ok()
        );

        let requests = server.received_requests().await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body["method"], "torrent-set");
        assert_eq!(body["arguments"]["ids"][0], 42);
        assert_eq!(body["arguments"]["downloadLimited"], true);
        assert_eq!(body["arguments"]["downloadLimit"], 1000);
        assert!(
            body["arguments"]["uploadLimited"].is_null(),
            "uploadLimited must be absent when None"
        );
    }

    #[tokio::test]
    async fn torrent_set_bandwidth_session_rotation() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(ResponseTemplate::new(409).insert_header(SESSION_ID_HEADER, "new-id"))
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let result = torrent_set_bandwidth(
            &url,
            &creds,
            "old-id",
            &[1],
            TorrentBandwidthArgs::default(),
        )
        .await;
        assert!(matches!(result, Err(RpcError::SessionRotated(ref id)) if id == "new-id"));
    }

    // ── torrent_set_location ──────────────────────────────────────────────────

    #[tokio::test]
    async fn torrent_set_location_with_move_true_succeeds() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "result": "success", "arguments": {} })),
            )
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let result = torrent_set_location(&url, &creds, "sid", &[42], "/new/path", true).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn torrent_set_location_with_move_false_succeeds() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "result": "success", "arguments": {} })),
            )
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let result = torrent_set_location(&url, &creds, "sid", &[1], "/other/path", false).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn torrent_set_location_session_rotation() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(ResponseTemplate::new(409).insert_header(SESSION_ID_HEADER, "rotated"))
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let result = torrent_set_location(&url, &creds, "old", &[1], "/some/path", true).await;
        assert!(matches!(result, Err(RpcError::SessionRotated(ref id)) if id == "rotated"));
    }

    #[tokio::test]
    async fn torrent_set_location_auth_failure() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let result = torrent_set_location(&url, &creds, "sid", &[1], "/path", true).await;
        assert!(matches!(result, Err(RpcError::AuthError)));
    }

    // ── queue_move_* ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn queue_move_top_success() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "result": "success", "arguments": {} })),
            )
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        assert!(queue_move_top(&url, &creds, "sid", &[2, 5]).await.is_ok());
    }

    #[tokio::test]
    async fn queue_move_top_session_rotation() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(
                ResponseTemplate::new(409).insert_header(SESSION_ID_HEADER, "rotated-sid"),
            )
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let result = queue_move_top(&url, &creds, "old-sid", &[1]).await;
        assert!(matches!(result, Err(RpcError::SessionRotated(ref id)) if id == "rotated-sid"));
    }

    #[tokio::test]
    async fn queue_move_up_success() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "result": "success", "arguments": {} })),
            )
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        assert!(queue_move_up(&url, &creds, "sid", &[3]).await.is_ok());
    }

    #[tokio::test]
    async fn queue_move_up_session_rotation() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(ResponseTemplate::new(409).insert_header(SESSION_ID_HEADER, "new-sid"))
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let result = queue_move_up(&url, &creds, "old-sid", &[3]).await;
        assert!(matches!(result, Err(RpcError::SessionRotated(ref id)) if id == "new-sid"));
    }

    #[tokio::test]
    async fn queue_move_down_success() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "result": "success", "arguments": {} })),
            )
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        assert!(queue_move_down(&url, &creds, "sid", &[3]).await.is_ok());
    }

    #[tokio::test]
    async fn queue_move_down_session_rotation() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(ResponseTemplate::new(409).insert_header(SESSION_ID_HEADER, "new-sid"))
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let result = queue_move_down(&url, &creds, "old-sid", &[3]).await;
        assert!(matches!(result, Err(RpcError::SessionRotated(ref id)) if id == "new-sid"));
    }

    #[tokio::test]
    async fn queue_move_bottom_success() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "result": "success", "arguments": {} })),
            )
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        assert!(
            queue_move_bottom(&url, &creds, "sid", &[1, 4])
                .await
                .is_ok()
        );
    }

    #[tokio::test]
    async fn queue_move_bottom_session_rotation() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(
                ResponseTemplate::new(409).insert_header(SESSION_ID_HEADER, "rotated-sid"),
            )
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let result = queue_move_bottom(&url, &creds, "old-sid", &[1, 4]).await;
        assert!(matches!(result, Err(RpcError::SessionRotated(ref id)) if id == "rotated-sid"));
    }
}
