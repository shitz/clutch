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
pub use api::{session_get, session_set};
pub use models::{
    AddPayload, ConnectionParams, SessionData, SessionSetArgs, TorrentBandwidthArgs, TorrentData,
    TransmissionCredentials,
};
pub use worker::{RpcResult, RpcWork, execute_work};
