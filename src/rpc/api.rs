//! High-level RPC API functions for the Transmission daemon.
//!
//! Each function performs exactly one logical RPC operation. Session rotation
//! (409) is surfaced as `RpcError::SessionRotated` — the caller must retry.
//! The exception is `session_get`, which retries once automatically as it is
//! used as the initial connectivity probe.

use std::time::Duration;

use super::error::RpcError;
use super::models::{AddPayload, SessionInfo, TorrentData, TransmissionCredentials};
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

/// Probe the daemon with a lightweight `session-get` call.
///
/// Handles one level of session rotation automatically. Returns the current
/// session ID on success.
pub async fn session_get(
    url: &str,
    credentials: &TransmissionCredentials,
    session_id: &str,
) -> Result<SessionInfo, RpcError> {
    tracing::debug!(%url, "Probing daemon with session-get");
    match post_rpc(
        url,
        credentials,
        session_id,
        "session-get",
        None,
        Duration::from_secs(5),
    )
    .await
    {
        Ok(_) => {
            tracing::info!(%url, %session_id, "session-get probe succeeded");
            Ok(SessionInfo {
                session_id: session_id.to_owned(),
            })
        }
        Err(RpcError::SessionRotated(new_id)) => {
            tracing::debug!(%url, new_id = %new_id, "Session ID rotated during probe, retrying once");
            match post_rpc(
                url,
                credentials,
                &new_id,
                "session-get",
                None,
                Duration::from_secs(5),
            )
            .await
            {
                Ok(_) => {
                    tracing::info!(%url, session_id = %new_id, "session-get probe succeeded after rotation");
                    Ok(SessionInfo { session_id: new_id })
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
            "files", "fileStats", "trackerStats", "peers"
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

/// Start (resume) a torrent by its Transmission ID.
pub async fn torrent_start(
    url: &str,
    credentials: &TransmissionCredentials,
    session_id: &str,
    id: i64,
) -> Result<(), RpcError> {
    tracing::debug!(%url, %session_id, id, "Sending torrent-start");
    let args = serde_json::json!({ "ids": [id] });
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

/// Pause (stop) a torrent by its Transmission ID.
pub async fn torrent_stop(
    url: &str,
    credentials: &TransmissionCredentials,
    session_id: &str,
    id: i64,
) -> Result<(), RpcError> {
    tracing::debug!(%url, %session_id, id, "Sending torrent-stop");
    let args = serde_json::json!({ "ids": [id] });
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

/// Remove a torrent. When `delete_local_data` is `true` the daemon also
/// deletes all downloaded files from disk.
pub async fn torrent_remove(
    url: &str,
    credentials: &TransmissionCredentials,
    session_id: &str,
    id: i64,
    delete_local_data: bool,
) -> Result<(), RpcError> {
    tracing::debug!(%url, %session_id, id, delete_local_data, "Sending torrent-remove");
    let args = serde_json::json!({ "ids": [id], "delete-local-data": delete_local_data });
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
) -> Result<(), RpcError> {
    tracing::debug!(%url, %session_id, "Sending torrent-add");
    let mut args = match &payload {
        AddPayload::Magnet(uri) => serde_json::json!({ "filename": uri }),
        AddPayload::Metainfo(b64) => serde_json::json!({ "metainfo": b64 }),
    };
    if let Some(dir) = download_dir.as_deref().filter(|d| !d.is_empty()) {
        args["download-dir"] = serde_json::Value::String(dir.to_owned());
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

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

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
        assert!(torrent_start(&url, &creds, "sid", 42).await.is_ok());
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
        let result = torrent_start(&url, &creds, "old-sid", 1).await;
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
            torrent_start(&url, &creds, "sid", 1).await,
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
        assert!(torrent_stop(&url, &creds, "sid", 7).await.is_ok());
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
        let result = torrent_stop(&url, &creds, "old-sid", 7).await;
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
            torrent_stop(&url, &creds, "sid", 7).await,
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
        assert!(torrent_remove(&url, &creds, "sid", 3, false).await.is_ok());
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
        assert!(torrent_remove(&url, &creds, "sid", 3, true).await.is_ok());
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
        let result = torrent_remove(&url, &creds, "old-id", 3, false).await;
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
            torrent_remove(&url, &creds, "sid", 3, false).await,
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
            )
            .await,
            Err(RpcError::AuthError)
        ));
    }
}
