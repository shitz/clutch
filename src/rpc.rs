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

/// A single torrent as returned by the `torrent-get` RPC method.
///
/// Field names use `serde` rename attributes to match Transmission's camelCase
/// JSON keys.
#[derive(Debug, Clone, Deserialize)]
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
async fn post_rpc(
    url: &str,
    credentials: &TransmissionCredentials,
    session_id: &str,
    method: &str,
    arguments: Option<serde_json::Value>,
) -> Result<RpcResponse, RpcError> {
    let client = reqwest::Client::new();
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

    let resp = req.send().await.map_err(|e| RpcError::ConnectionError(e.to_string()))?;

    match resp.status().as_u16() {
        409 => {
            let new_id = extract_session_id(resp.headers())
                .unwrap_or_default()
                .to_owned();
            Err(RpcError::SessionRotated(new_id))
        }
        401 => Err(RpcError::AuthError),
        _ => {
            let body: RpcResponse = resp
                .json()
                .await
                .map_err(|e| RpcError::ParseError(e.to_string()))?;
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
    match post_rpc(url, credentials, session_id, "session-get", None).await {
        Ok(_) => Ok(SessionInfo { session_id: session_id.to_owned() }),
        Err(RpcError::SessionRotated(new_id)) => {
            // Retry with the fresh session id.
            post_rpc(url, credentials, &new_id, "session-get", None).await?;
            Ok(SessionInfo { session_id: new_id })
        }
        Err(e) => Err(e),
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
    let args = serde_json::json!({
        "fields": ["id", "name", "status", "percentDone"]
    });

    let resp = post_rpc(url, credentials, session_id, "torrent-get", Some(args)).await?;

    let torrents: Vec<TorrentData> = serde_json::from_value(resp.arguments["torrents"].clone())
        .map_err(|e| RpcError::ParseError(e.to_string()))?;

    Ok(torrents)
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
                ResponseTemplate::new(409)
                    .insert_header(SESSION_ID_HEADER, "new-session-id"),
            )
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let result = post_rpc(&url, &creds, "old-id", "session-get", None).await;

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
            .respond_with(
                ResponseTemplate::new(409)
                    .insert_header(SESSION_ID_HEADER, "rotated-id"),
            )
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
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "result": "success",
                    "arguments": {
                        "torrents": [
                            { "id": 1, "name": "Ubuntu ISO", "status": 6, "percentDone": 1.0 },
                            { "id": 2, "name": "Arch Linux", "status": 4, "percentDone": 0.43 }
                        ]
                    }
                })),
            )
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

    /// 7.6 – A malformed response body must yield ParseError.
    #[tokio::test]
    async fn torrent_get_parse_error() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/transmission/rpc"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "result": "success",
                    "arguments": {
                        "torrents": [{ "unexpected_field": true }]
                    }
                })),
            )
            .mount(&server)
            .await;

        let creds = test_credentials();
        let url = format!("{}/transmission/rpc", server.uri());
        let result = torrent_get(&url, &creds, "sid").await;

        assert!(matches!(result, Err(RpcError::ParseError(_))));
    }
}
