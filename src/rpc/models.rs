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

//! Shared across the RPC layer and the UI modules. Carries no transport logic.

use serde::Deserialize;

// ── Connection types ──────────────────────────────────────────────────────────

/// Connection credentials for a Transmission daemon.
#[derive(Debug, Clone)]
pub struct TransmissionCredentials {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl TransmissionCredentials {
    /// Build the full RPC endpoint URL from these credentials.
    #[must_use]
    pub fn rpc_url(&self) -> String {
        format!("http://{}:{}/transmission/rpc", self.host, self.port)
    }
}

/// Bundled connection parameters passed to the RPC worker.
///
/// Groups URL, credentials, and session ID to avoid passing three separate
/// values through every RPC call and work-queue item.
#[derive(Debug, Clone)]
pub struct ConnectionParams {
    pub url: String,
    pub credentials: TransmissionCredentials,
    pub session_id: String,
}

impl ConnectionParams {
    pub fn new(credentials: TransmissionCredentials, session_id: String) -> Self {
        let url = credentials.rpc_url();
        Self {
            url,
            credentials,
            session_id,
        }
    }
}

/// Result of a successful `session-get` probe.
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub session_id: String,
}

// ── Torrent data ──────────────────────────────────────────────────────────────

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
    #[serde(rename = "seederCount")]
    pub seeder_count: i32,
    #[serde(rename = "leecherCount")]
    pub leecher_count: i32,
    #[serde(rename = "lastAnnounceTime")]
    pub last_announce_time: i64,
}

/// A single connected peer.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct PeerInfo {
    pub address: String,
    #[serde(rename = "rateToClient")]
    pub rate_to_client: i64,
    #[serde(rename = "rateToPeer")]
    pub rate_to_peer: i64,
}

/// A single torrent as returned by the `torrent-get` RPC method.
///
/// Field names use `serde` rename attributes to match Transmission's camelCase
/// JSON keys. Extended fields use `#[serde(default)]` so partial responses
/// still parse cleanly.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct TorrentData {
    pub id: i64,
    pub name: String,
    /// Transmission status: 0=Stopped 1=QueueCheck 2=Checking 3=QueueDL 4=DL 5=QueueSeed 6=Seeding
    pub status: i32,
    #[serde(rename = "percentDone")]
    pub percent_done: f64,
    #[serde(rename = "totalSize", default)]
    pub total_size: i64,
    #[serde(rename = "downloadedEver", default)]
    pub downloaded_ever: i64,
    #[serde(rename = "uploadedEver", default)]
    pub uploaded_ever: i64,
    #[serde(rename = "uploadRatio", default)]
    pub upload_ratio: f64,
    #[serde(default)]
    pub eta: i64,
    #[serde(rename = "rateDownload", default)]
    pub rate_download: i64,
    #[serde(rename = "rateUpload", default)]
    pub rate_upload: i64,
    #[serde(default)]
    pub files: Vec<TorrentFile>,
    #[serde(rename = "fileStats", default)]
    pub file_stats: Vec<TorrentFileStats>,
    #[serde(rename = "trackerStats", default)]
    pub tracker_stats: Vec<TrackerStat>,
    #[serde(default)]
    pub peers: Vec<PeerInfo>,
}

// ── Add-torrent payload ───────────────────────────────────────────────────────

/// Payload for a `torrent-add` RPC call.
///
/// - `Magnet(uri)` — sent as the `filename` JSON field.
/// - `Metainfo(base64)` — sent as the `metainfo` JSON field.
#[derive(Debug, Clone)]
pub enum AddPayload {
    Magnet(String),
    Metainfo(String),
}
