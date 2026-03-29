//! Transmission JSON-RPC client.
//!
//! This module provides async functions for communicating with a Transmission
//! daemon over HTTP. All functions are pure `async fn`s that accept a full URL
//! and return `Result<_, RpcError>`. They are designed to be called exclusively
//! inside `iced::Command::perform()` — never from `update()` directly.
//!
//! # Session-Id lifecycle
//!
//! Transmission uses an `X-Transmission-Session-Id` header to prevent CSRF. On
//! the first request (or when the session rotates), the daemon returns 409 with
//! the new ID in the response header. Callers must store this ID and re-issue
//! the command. This module surfaces rotation via `RpcError::SessionRotated`.

use std::time::Duration;

use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};

/// The `X-Transmission-Session-Id` header name.
const SESSION_ID_HEADER: &str = "X-Transmission-Session-Id";

// ── Credentials ───────────────────────────────────────────────────────────────

/// Connection credentials for a Transmission daemon.
///
/// Use [`rpc_url`] to build the full HTTP endpoint URL from these fields.
#[derive(Debug, Clone)]
pub struct TransmissionCredentials {
    /// Hostname or IP address of the daemon (e.g. `"localhost"`).
    pub host: String,
    /// TCP port the RPC endpoint listens on (default `9091`).
    pub port: u16,
    /// Optional HTTP Basic Auth username.
    pub username: Option<String>,
    /// Optional HTTP Basic Auth password.
    pub password: Option<String>,
}

impl TransmissionCredentials {
    /// Build the full RPC endpoint URL from these credentials.
    pub fn rpc_url(&self) -> String {
        format!("http://{}:{}/transmission/rpc", self.host, self.port)
    }
}

// ── Errors ────────────────────────────────────────────────────────────────────

/// Errors that can occur when communicating with the Transmission daemon.
#[derive(Debug, Clone)]
pub enum RpcError {
    /// The daemon returned 409, indicating the session ID has rotated.
    /// The caller must store the new ID (the `String`) and re-issue the request.
    SessionRotated(String),
    /// The daemon returned 401 Unauthorized. Credentials are wrong or missing.
    AuthError,
    /// The daemon could not be reached (connection refused, timeout, DNS, etc.).
    ConnectionError(String),
    /// The daemon responded but the body could not be parsed as expected JSON.
    ParseError(String),
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpcError::SessionRotated(_) => write!(f, "Session ID rotated"),
            RpcError::AuthError => write!(f, "Authentication failed"),
            RpcError::ConnectionError(msg) => write!(f, "Connection error: {msg}"),
            RpcError::ParseError(msg) => write!(f, "Parse error: {msg}"),
        }
    }
}

// ── Data models ───────────────────────────────────────────────────────────────

/// A single file within a torrent, as returned by `torrent-get`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct TorrentFile {
    pub name: String,
    pub length: i64,
}

/// Per-file download progress, parallel to the `files` array.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct TorrentFileStats {
    #[serde(rename = "bytesCompleted")]
    pub bytes_completed: i64,
}

/// Tracker statistics for a single tracker announce URL.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct TrackerStat {
    pub host: String,
    /// Number of seeders; `-1` when unknown.
    #[serde(rename = "seederCount")]
    pub seeder_count: i32,
    /// Number of leechers; `-1` when unknown.
    #[serde(rename = "leecherCount")]
    pub leecher_count: i32,
    /// Unix timestamp of last successful announce; `0` when never announced.
    #[serde(rename = "lastAnnounceTime")]
    pub last_announce_time: i64,
}

/// A single connected peer.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct PeerInfo {
    pub address: String,
    #[serde(rename = "clientName")]
    #[allow(dead_code)] // deserialized but display-hidden pending column settings
    pub client_name: String,
    /// Bytes per second the peer is sending to us.
    #[serde(rename = "rateToClient")]
    pub rate_to_client: i64,
    /// Bytes per second we are sending to the peer.
    #[serde(rename = "rateToPeer")]
    pub rate_to_peer: i64,
}

/// A single torrent as returned by the `torrent-get` RPC method.
///
/// Field names use `serde` rename attributes to match Transmission's camelCase
/// JSON keys. All v0.4 extended fields use `#[serde(default)]` so responses
/// from older Transmission versions or partial field sets still parse cleanly.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct TorrentData {
    /// Transmission's unique numeric torrent identifier.
    pub id: i64,
    /// Human-readable torrent name.
    pub name: String,
    /// Transmission status integer:
    /// - `0` = Stopped / Paused
    /// - `1` = Queued to check files
    /// - `2` = Checking files
    /// - `3` = Queued to download
    /// - `4` = Downloading
    /// - `5` = Queued to seed
    /// - `6` = Seeding
    pub status: i32,
    /// Download completion as a fraction in `[0.0, 1.0]`.
    #[serde(rename = "percentDone")]
    pub percent_done: f64,

    // ── v0.4 extended fields ──────────────────────────────────────────────
    /// Total size of all wanted files in bytes.
    #[serde(rename = "totalSize", default)]
    pub total_size: i64,
    /// Total bytes downloaded (including wasted).
    #[serde(rename = "downloadedEver", default)]
    pub downloaded_ever: i64,
    /// Total bytes uploaded.
    #[serde(rename = "uploadedEver", default)]
    pub uploaded_ever: i64,
    /// Upload-to-download ratio; `-1.0` when ratio is unavailable.
    #[serde(rename = "uploadRatio", default)]
    pub upload_ratio: f64,
    /// Estimated seconds until download completes; `-1` when unavailable.
    #[serde(default)]
    pub eta: i64,
    /// Current download rate in bytes/s.
    #[serde(rename = "rateDownload", default)]
    pub rate_download: i64,
    /// Current upload rate in bytes/s.
    #[serde(rename = "rateUpload", default)]
    pub rate_upload: i64,
    /// Per-file list; empty when not requested or not applicable.
    #[serde(default)]
    pub files: Vec<TorrentFile>,
    /// Per-file download statistics; parallel to `files`.
    #[serde(rename = "fileStats", default)]
    pub file_stats: Vec<TorrentFileStats>,
    /// Per-tracker announce statistics.
    #[serde(rename = "trackerStats", default)]
    pub tracker_stats: Vec<TrackerStat>,
    /// Currently connected peers.
    #[serde(default)]
    pub peers: Vec<PeerInfo>,
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// RPC request wrapper sent to the Transmission daemon.
#[derive(Serialize)]
struct RpcRequest<'a> {
    method: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    arguments: Option<serde_json::Value>,
}

/// Response envelope from the Transmission daemon.
#[derive(Debug, Deserialize)]
struct RpcResponse {
    result: String,
    #[serde(default)]
    arguments: serde_json::Value,
}

/// Send a single JSON-RPC POST to `url` with the given `session_id`.
///
/// # Errors
///
/// - Returns `Err(RpcError::SessionRotated(new_id))` when the daemon responds
///   with 409, meaning the caller must store `new_id` and retry.
/// - Returns `Err(RpcError::AuthError)` for 401 responses.
/// - Returns `Err(RpcError::ConnectionError(_))` for transport-level failures.
///
/// # Note
///
/// This function performs exactly one HTTP request. It does **not** retry on
/// session rotation — that responsibility belongs to the caller.
///
/// `timeout` overrides the default 10-second transport timeout. Use a longer
/// value for operations (e.g. `torrent-add`) where the daemon may block on
/// disk I/O before returning a response.
async fn post_rpc(
    url: &str,
    credentials: &TransmissionCredentials,
    session_id: &str,
    method: &str,
    arguments: Option<serde_json::Value>,
    timeout: Duration,
) -> Result<RpcResponse, RpcError> {
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .unwrap_or_default();
    let body = RpcRequest { method, arguments };

    let mut req = client
        .post(url)
        .header(SESSION_ID_HEADER, session_id)
        .json(&body);

    if let (Some(user), Some(pass)) = (&credentials.username, &credentials.password) {
        req = req.basic_auth(user, Some(pass));
    } else if let Some(user) = &credentials.username {
        req = req.basic_auth(user, Option::<&str>::None);
    }

    tracing::debug!(
        %url, %method, %session_id,
        has_auth = credentials.username.is_some(),
        "Sending RPC request"
    );

    let resp = req.send().await.map_err(|e| {
        tracing::error!(error = %e, %url, %method, "RPC transport error (is the daemon running?)");
        RpcError::ConnectionError(e.to_string())
    })?;

    let status = resp.status().as_u16();
    tracing::debug!(%url, %method, status, "RPC response received");

    match status {
        409 => {
            let new_id = extract_session_id(resp.headers())
                .unwrap_or_default()
                .to_owned();
            tracing::debug!(%url, old_id = %session_id, new_id = %new_id, "Session ID rotated (409)");
            Err(RpcError::SessionRotated(new_id))
        }
        401 => {
            tracing::error!(%url, "RPC authentication failed (401) — check username/password");
            Err(RpcError::AuthError)
        }
        _ => {
            let body: RpcResponse = resp.json().await.map_err(|e| {
                tracing::error!(error = %e, %url, %method, "Failed to parse RPC response body");
                RpcError::ParseError(e.to_string())
            })?;
            Ok(body)
        }
    }
}

/// Extract the `X-Transmission-Session-Id` value from a header map.
fn extract_session_id(headers: &HeaderMap) -> Option<&str> {
    headers.get(SESSION_ID_HEADER)?.to_str().ok()
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Result of a successful `session-get` probe.
#[derive(Debug, Clone)]
pub struct SessionInfo {
    /// The current `X-Transmission-Session-Id` to use for subsequent calls.
    pub session_id: String,
}

/// Probe the daemon with a lightweight `session-get` call.
///
/// This is the recommended first call after the user clicks "Connect". It
/// verifies connectivity and authentication and returns the initial session ID.
///
/// Handles one level of session rotation automatically: if the first attempt
/// returns 409 the new session ID is captured and the call is retried once.
///
/// # Errors
///
/// - `RpcError::AuthError` — bad credentials (HTTP 401).
/// - `RpcError::ConnectionError(_)` — daemon unreachable.
/// - `RpcError::ParseError(_)` — unexpected response body.
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
                    tracing::info!(%url, session_id = %new_id, "session-get probe succeeded after session rotation");
                    Ok(SessionInfo { session_id: new_id })
                }
                Err(e) => {
                    tracing::error!(error = %e, %url, "session-get retry failed after session rotation");
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

/// Fetch the list of all torrents from the daemon.
///
/// Requests `id`, `name`, `status`, and `percentDone` fields for every torrent.
///
/// # Deserialization
///
/// JSON parsing is performed inside this function (on the tokio thread) so that
/// `update()` receives an already-parsed `Vec<TorrentData>` and never blocks
/// the UI thread on CPU-bound work.
///
/// # Errors
///
/// - `RpcError::SessionRotated(new_id)` — caller must store `new_id` and retry.
/// - `RpcError::AuthError` — credentials rejected.
/// - `RpcError::ConnectionError(_)` — daemon unreachable.
/// - `RpcError::ParseError(_)` — response body could not be parsed.
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
            tracing::error!(error = %e, "Failed to deserialize torrent list from response");
            RpcError::ParseError(e.to_string())
        })?;

    tracing::debug!(
        count = torrents.len(),
        "torrent-get deserialized successfully"
    );
    Ok(torrents)
}

/// Start (resume) a torrent by its Transmission ID.
///
/// # Errors
///
/// - `RpcError::SessionRotated(new_id)` — caller must handle rotation and retry.
/// - `RpcError::AuthError` — credentials rejected.
/// - `RpcError::ConnectionError(_)` — daemon unreachable.
/// - `RpcError::ParseError(_)` — daemon returned a non-success result string.
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
    if resp.result == "success" {
        tracing::info!(id, "torrent-start succeeded");
        Ok(())
    } else {
        tracing::error!(id, result = %resp.result, "torrent-start returned non-success");
        Err(RpcError::ParseError(format!(
            "torrent-start failed: {}",
            resp.result
        )))
    }
}

/// Pause (stop) a torrent by its Transmission ID.
///
/// # Errors
///
/// - `RpcError::SessionRotated(new_id)` — caller must handle rotation and retry.
/// - `RpcError::AuthError` — credentials rejected.
/// - `RpcError::ConnectionError(_)` — daemon unreachable.
/// - `RpcError::ParseError(_)` — daemon returned a non-success result string.
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
    if resp.result == "success" {
        tracing::info!(id, "torrent-stop succeeded");
        Ok(())
    } else {
        tracing::error!(id, result = %resp.result, "torrent-stop returned non-success");
        Err(RpcError::ParseError(format!(
            "torrent-stop failed: {}",
            resp.result
        )))
    }
}

/// Remove a torrent by its Transmission ID.
///
/// When `delete_local_data` is `true` the daemon also removes all downloaded
/// files from disk. When `false` only the torrent metadata is removed.
///
/// # Errors
///
/// - `RpcError::SessionRotated(new_id)` — caller must handle rotation and retry.
/// - `RpcError::AuthError` — credentials rejected.
/// - `RpcError::ConnectionError(_)` — daemon unreachable.
/// - `RpcError::ParseError(_)` — daemon returned a non-success result string.
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
    if resp.result == "success" {
        tracing::info!(id, delete_local_data, "torrent-remove succeeded");
        Ok(())
    } else {
        tracing::error!(id, result = %resp.result, "torrent-remove returned non-success");
        Err(RpcError::ParseError(format!(
            "torrent-remove failed: {}",
            resp.result
        )))
    }
}

/// Payload for a `torrent-add` RPC call.
///
/// - `Magnet(uri)` — a magnet URI sent as the `filename` field.
/// - `Metainfo(base64)` — a Base64-encoded `.torrent` file sent as the `metainfo` field.
#[derive(Debug, Clone)]
pub enum AddPayload {
    Magnet(String),
    Metainfo(String),
}

/// Add a new torrent to the daemon.
///
/// `payload` determines whether the torrent is identified by a magnet URI or
/// by Base64-encoded `.torrent` file contents. `download_dir`, when `Some` and
/// non-empty, sets the destination directory; otherwise the daemon uses its
/// configured default.
///
/// Both `"success"` and `"torrent-duplicate"` result strings are treated as
/// `Ok(())`.
///
/// # Errors
///
/// - `RpcError::SessionRotated(new_id)` — caller must handle rotation and retry.
/// - `RpcError::AuthError` — credentials rejected.
/// - `RpcError::ConnectionError(_)` — daemon unreachable.
/// - `RpcError::ParseError(_)` — daemon returned an unexpected result string.
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
    // Use a 60-second timeout: Transmission 3.x performs synchronous disk work
    // (preallocation, hash-check) before returning the torrent-add response.
    // Dropping the connection mid-response leaves Transmission's single-threaded
    // RPC server wedged; a longer timeout lets it finish cleanly.
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

// ── Serialized-worker types ───────────────────────────────────────────────────

/// A unit of work for the serialized RPC worker subscription.
///
/// Each variant carries all parameters needed to execute one RPC call.
/// Items are submitted via a `tokio::sync::mpsc::Sender<RpcWork>` and processed
/// sequentially by the background subscription, ensuring at most one HTTP
/// connection to the daemon is in-flight at any time.
#[derive(Debug, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum RpcWork {
    TorrentGet {
        url: String,
        credentials: TransmissionCredentials,
        session_id: String,
    },
    TorrentStart {
        url: String,
        credentials: TransmissionCredentials,
        session_id: String,
        id: i64,
    },
    TorrentStop {
        url: String,
        credentials: TransmissionCredentials,
        session_id: String,
        id: i64,
    },
    TorrentRemove {
        url: String,
        credentials: TransmissionCredentials,
        session_id: String,
        id: i64,
        delete_local_data: bool,
    },
    TorrentAdd {
        url: String,
        credentials: TransmissionCredentials,
        session_id: String,
        payload: AddPayload,
        download_dir: Option<String>,
    },
}

/// The typed outcome of one [`RpcWork`] item.
#[derive(Debug)]
pub enum RpcResult {
    /// Result of a `torrent-get` call.
    TorrentsLoaded(Result<Vec<TorrentData>, RpcError>),
    /// Result of a `torrent-start`, `torrent-stop`, or `torrent-remove` call.
    ActionDone(Result<(), RpcError>),
    /// Result of a `torrent-add` call.
    TorrentAdded(Result<(), RpcError>),
}

/// Execute one [`RpcWork`] item, retrying once transparently on session rotation.
///
/// Returns `(new_session_id, result)` where `new_session_id` is `Some` when the
/// daemon returned 409. The caller must persist the new ID so future work items
/// use it.
pub async fn execute_work(work: RpcWork) -> (Option<String>, RpcResult) {
    match work {
        RpcWork::TorrentGet {
            url,
            credentials,
            session_id,
        } => match torrent_get(&url, &credentials, &session_id).await {
            Err(RpcError::SessionRotated(new_id)) => {
                let r = torrent_get(&url, &credentials, &new_id).await;
                (Some(new_id), RpcResult::TorrentsLoaded(r))
            }
            r => (None, RpcResult::TorrentsLoaded(r)),
        },
        RpcWork::TorrentStart {
            url,
            credentials,
            session_id,
            id,
        } => match torrent_start(&url, &credentials, &session_id, id).await {
            Err(RpcError::SessionRotated(new_id)) => {
                let r = torrent_start(&url, &credentials, &new_id, id).await;
                (Some(new_id), RpcResult::ActionDone(r))
            }
            r => (None, RpcResult::ActionDone(r)),
        },
        RpcWork::TorrentStop {
            url,
            credentials,
            session_id,
            id,
        } => match torrent_stop(&url, &credentials, &session_id, id).await {
            Err(RpcError::SessionRotated(new_id)) => {
                let r = torrent_stop(&url, &credentials, &new_id, id).await;
                (Some(new_id), RpcResult::ActionDone(r))
            }
            r => (None, RpcResult::ActionDone(r)),
        },
        RpcWork::TorrentRemove {
            url,
            credentials,
            session_id,
            id,
            delete_local_data,
        } => match torrent_remove(&url, &credentials, &session_id, id, delete_local_data).await {
            Err(RpcError::SessionRotated(new_id)) => {
                let r = torrent_remove(&url, &credentials, &new_id, id, delete_local_data).await;
                (Some(new_id), RpcResult::ActionDone(r))
            }
            r => (None, RpcResult::ActionDone(r)),
        },
        RpcWork::TorrentAdd {
            url,
            credentials,
            session_id,
            payload,
            download_dir,
        } => {
            match torrent_add(
                &url,
                &credentials,
                &session_id,
                payload.clone(),
                download_dir.clone(),
            )
            .await
            {
                Err(RpcError::SessionRotated(new_id)) => {
                    let r = torrent_add(&url, &credentials, &new_id, payload, download_dir).await;
                    (Some(new_id), RpcResult::TorrentAdded(r))
                }
                r => (None, RpcResult::TorrentAdded(r)),
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_credentials() -> TransmissionCredentials {
        TransmissionCredentials {
            host: "127.0.0.1".to_string(),
            port: 0, // port is unused; we pass the URL directly
            username: None,
            password: None,
        }
    }

    // ── post_rpc tests ────────────────────────────────────────────────────────

    /// 7.1 – A 409 response must yield SessionRotated with the new session id.
    #[tokio::test]
    async fn post_rpc_409_returns_session_rotated() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(
                ResponseTemplate::new(409).insert_header(SESSION_ID_HEADER, "new-session-id"),
            )
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let result = post_rpc(
            &url,
            &creds,
            "old-id",
            "session-get",
            None,
            Duration::from_secs(10),
        )
        .await;

        assert!(
            matches!(result, Err(RpcError::SessionRotated(ref id)) if id == "new-session-id"),
            "expected SessionRotated(new-session-id), got {result:?}"
        );
    }

    // ── session_get tests ─────────────────────────────────────────────────────

    /// 7.2 – A successful 200 response returns Ok with the session id.
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

    /// 7.2 – session_get auto-retries on 409 and returns the new session id.
    #[tokio::test]
    async fn session_get_retries_on_session_rotation() {
        let server = MockServer::start().await;

        // First request → 409
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(ResponseTemplate::new(409).insert_header(SESSION_ID_HEADER, "rotated-id"))
            .up_to_n_times(1)
            .mount(&server)
            .await;

        // Retry with new id → 200
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

    /// 7.3 – A 401 response must yield AuthError.
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

    /// 7.4 – Connecting to a closed port must yield ConnectionError.
    #[tokio::test]
    async fn session_get_connection_error() {
        let creds = test_credentials();
        // Port 1 is unlikely to have anything listening.
        let result = session_get("http://127.0.0.1:1/transmission/rpc", &creds, "sid").await;
        assert!(matches!(result, Err(RpcError::ConnectionError(_))));
    }

    // ── torrent_get tests ─────────────────────────────────────────────────────

    /// 7.5 – A well-formed 200 torrent-get response is parsed into TorrentData.
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

    /// 2.2 – torrent_get deserializes v0.4 extended fields correctly.
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
                            "clientName": "qBittorrent",
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
        let result = torrent_get(&url, &creds, "sid").await;

        assert!(result.is_ok());
        let torrents = result.unwrap();
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
        assert_eq!(t.peers[0].client_name, "qBittorrent");
        assert_eq!(t.peers[0].rate_to_client, 512000);
    }

    /// 7.6 – A malformed response body must yield ParseError.
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

    // ── torrent_start tests ───────────────────────────────────────────────────

    /// torrent_start success — 200 with result "success" returns Ok(()).
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
        let result = torrent_start(&url, &creds, "sid", 42).await;
        assert!(result.is_ok());
    }

    /// torrent_start — 409 yields SessionRotated.
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

    /// torrent_start — 401 yields AuthError.
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
        let result = torrent_start(&url, &creds, "sid", 1).await;
        assert!(matches!(result, Err(RpcError::AuthError)));
    }

    // ── torrent_stop tests ────────────────────────────────────────────────────

    /// torrent_stop success — 200 with result "success" returns Ok(()).
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
        let result = torrent_stop(&url, &creds, "sid", 7).await;
        assert!(result.is_ok());
    }

    /// torrent_stop — 409 yields SessionRotated.
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

    /// torrent_stop — 401 yields AuthError.
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
        let result = torrent_stop(&url, &creds, "sid", 7).await;
        assert!(matches!(result, Err(RpcError::AuthError)));
    }

    // ── torrent_remove tests ──────────────────────────────────────────────────

    /// torrent_remove success without deleting local data.
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
        let result = torrent_remove(&url, &creds, "sid", 3, false).await;
        assert!(result.is_ok());
    }

    /// torrent_remove success with delete_local_data = true.
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
        let result = torrent_remove(&url, &creds, "sid", 3, true).await;
        assert!(result.is_ok());
    }

    /// torrent_remove — 409 yields SessionRotated.
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

    /// torrent_remove — 401 yields AuthError.
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
        let result = torrent_remove(&url, &creds, "sid", 3, false).await;
        assert!(matches!(result, Err(RpcError::AuthError)));
    }

    // ── torrent_add tests ─────────────────────────────────────────────────────

    /// torrent_add with a magnet URI returns Ok(()) on "success".
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
        let result = torrent_add(
            &url,
            &creds,
            "sid",
            AddPayload::Magnet("magnet:?xt=urn:btih:abc".to_owned()),
            None,
        )
        .await;
        assert!(result.is_ok());
    }

    /// torrent_add with metainfo returns Ok(()) on "success".
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
        let result = torrent_add(
            &url,
            &creds,
            "sid",
            AddPayload::Metainfo("dGVzdA==".to_owned()),
            Some("/downloads".to_owned()),
        )
        .await;
        assert!(result.is_ok());
    }

    /// torrent_add treats "torrent-duplicate" as Ok(()).
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
        let result = torrent_add(
            &url,
            &creds,
            "sid",
            AddPayload::Magnet("magnet:?xt=urn:btih:abc".to_owned()),
            None,
        )
        .await;
        assert!(result.is_ok());
    }

    /// torrent_add with empty download_dir omits the field (no crash).
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
        // Empty string → download-dir should be omitted
        let result = torrent_add(
            &url,
            &creds,
            "sid",
            AddPayload::Magnet("magnet:?xt=urn:btih:abc".to_owned()),
            Some(String::new()),
        )
        .await;
        assert!(result.is_ok());
    }

    /// torrent_add — 409 yields SessionRotated.
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

    /// torrent_add — 401 yields AuthError.
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
        let result = torrent_add(
            &url,
            &creds,
            "sid",
            AddPayload::Magnet("magnet:?xt=urn:btih:abc".to_owned()),
            None,
        )
        .await;
        assert!(matches!(result, Err(RpcError::AuthError)));
    }
}
