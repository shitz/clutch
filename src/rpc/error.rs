//! RPC error type for the Transmission JSON-RPC client.

/// Errors that can occur when communicating with the Transmission daemon.
#[derive(Debug, Clone)]
pub enum RpcError {
    /// The daemon returned 409, indicating the session ID has rotated.
    /// The caller must store the new ID and re-issue the request.
    SessionRotated(String),
    /// The daemon returned 401 Unauthorized.
    AuthError,
    /// The daemon could not be reached (connection refused, timeout, DNS, etc.).
    ConnectionError(String),
    /// The daemon responded but the body could not be parsed as expected JSON.
    ParseError(String),
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SessionRotated(_) => write!(f, "Session ID rotated"),
            Self::AuthError => write!(f, "Authentication failed"),
            Self::ConnectionError(msg) => write!(f, "Connection error: {msg}"),
            Self::ParseError(msg) => write!(f, "Parse error: {msg}"),
        }
    }
}

impl std::error::Error for RpcError {}
