//! Transmission JSON-RPC client.
//!
//! Async functions for communicating with a Transmission daemon over HTTP.
//! All public functions are designed to be called from `Task::perform()` or
//! the serialized RPC worker subscription — never from `update()` directly.
//!
//! # Module structure
//!
//! - [`models`] — Data types shared with the UI (`TorrentData`, `TransmissionCredentials`, etc.)
//! - [`error`] — The [`RpcError`] type
//! - `api` — High-level RPC functions (`session_get`, `torrent_get`, etc.)
//! - `transport` — Low-level HTTP transport with a shared `reqwest::Client`
//! - [`worker`] — Serialized worker types (`RpcWork`, `RpcResult`, `execute_work`)

mod api;
pub mod error;
pub mod models;
mod transport;
pub mod worker;

// Re-export commonly used types at crate::rpc level.
pub use api::session_get;
pub use models::{AddPayload, ConnectionParams, SessionInfo, TorrentData, TransmissionCredentials};
pub use worker::{RpcResult, RpcWork, execute_work};
