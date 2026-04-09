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

//! Inspector screen state types.

use std::collections::HashMap;

use crate::rpc::TorrentData;

// ── InspectorOptionsState ────────────────────────────────────────────────────

/// Local draft for the per-torrent Options tab.
/// Reset whenever a new torrent is selected.
#[derive(Debug, Default, Clone)]
pub struct InspectorOptionsState {
    pub download_limited: bool,
    pub download_limit_val: String,
    pub upload_limited: bool,
    pub upload_limit_val: String,
    /// 0 = Global, 1 = Custom, 2 = Unlimited
    pub ratio_mode: u8,
    pub ratio_limit_val: String,
    pub honors_session_limits: bool,
}

impl InspectorOptionsState {
    /// Populate from fresh torrent data.
    pub fn from_torrent(t: &TorrentData) -> Self {
        Self {
            download_limited: t.download_limited,
            download_limit_val: t.download_limit.to_string(),
            upload_limited: t.upload_limited,
            upload_limit_val: t.upload_limit.to_string(),
            ratio_mode: t.seed_ratio_mode,
            ratio_limit_val: format!("{:.2}", t.seed_ratio_limit),
            honors_session_limits: t.honors_session_limits,
        }
    }
}

// ── InspectorScreen ───────────────────────────────────────────────────────────

/// State for the inspector detail panel.
#[derive(Debug, Default)]
pub struct InspectorScreen {
    pub active_tab: super::ActiveTab,
    /// Optimistic file-wanted overrides keyed by file index.
    /// Entries are inserted when the user toggles a checkbox and removed
    /// when the corresponding `torrent-set` RPC completes (or fails).
    pub pending_wanted: HashMap<usize, bool>,
    /// Draft state for the Options tab.
    pub options: InspectorOptionsState,
}

impl InspectorScreen {
    pub fn new() -> Self {
        Self::default()
    }
}
