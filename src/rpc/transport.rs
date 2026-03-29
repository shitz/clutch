//! Low-level HTTP transport for the Transmission JSON-RPC protocol.
//!
//! Provides `post_rpc` and a shared `reqwest::Client`. Internal to the `rpc`
//! module — only [`super::api`] calls into this layer.

use std::sync::LazyLock;
use std::time::Duration;

use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};

use super::error::RpcError;
use super::models::TransmissionCredentials;

pub(super) const SESSION_ID_HEADER: &str = "X-Transmission-Session-Id";

/// Shared HTTP client — constructed once, reused across all RPC calls.
///
/// Per-request timeouts are set via `RequestBuilder::timeout()`.
static HTTP_CLIENT: LazyLock<reqwest::Client> =
    LazyLock::new(|| reqwest::Client::builder().build().unwrap_or_default());

#[derive(Serialize)]
struct RpcRequest<'a> {
    method: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    arguments: Option<serde_json::Value>,
}

/// Response envelope from the Transmission daemon.
#[derive(Debug, Deserialize)]
pub(super) struct RpcResponse {
    pub result: String,
    #[serde(default)]
    pub arguments: serde_json::Value,
}

/// Send a single JSON-RPC POST to `url` with the given `session_id`.
///
/// Performs exactly one HTTP request. Does **not** retry on session rotation.
pub(super) async fn post_rpc(
    url: &str,
    credentials: &TransmissionCredentials,
    session_id: &str,
    method: &str,
    arguments: Option<serde_json::Value>,
    timeout: Duration,
) -> Result<RpcResponse, RpcError> {
    let body = RpcRequest { method, arguments };

    let mut req = HTTP_CLIENT
        .post(url)
        .timeout(timeout)
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
        tracing::error!(error = %e, %url, %method, "RPC transport error");
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
            tracing::error!(%url, "RPC authentication failed (401)");
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

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_credentials() -> TransmissionCredentials {
        TransmissionCredentials {
            host: "127.0.0.1".to_owned(),
            port: 0,
            username: None,
            password: None,
        }
    }

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
}
