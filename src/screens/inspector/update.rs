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

//! Message handling for the inspector panel.

use iced::Task;

use super::{InspectorScreen, Message};

/// Apply an inspector message to the screen state and return any follow-up work.
pub fn update(state: &mut InspectorScreen, msg: Message) -> Task<Message> {
    match msg {
        Message::TabSelected(tab) => {
            state.active_tab = tab;
            Task::none()
        }
        Message::FileWantedToggled {
            file_index, wanted, ..
        } => {
            state.pending_wanted.insert(file_index, wanted);
            Task::none()
        }
        Message::AllFilesWantedToggled {
            file_count, wanted, ..
        } => {
            for i in 0..file_count {
                state.pending_wanted.insert(i, wanted);
            }
            Task::none()
        }
        Message::FileWantedSetSuccess { indices } => {
            for i in &indices {
                state.pending_wanted.remove(i);
            }
            Task::none()
        }
        // ── Options tab ───────────────────────────────────────────────────────
        Message::OptionsDownloadLimitToggled(v) => {
            state.options.download_limited = v;
            Task::none()
        }
        Message::OptionsDownloadLimitChanged(v) => {
            if v.is_empty() || v.chars().all(|c| c.is_ascii_digit()) {
                state.options.download_limit_val = v;
            }
            Task::none()
        }
        Message::OptionsUploadLimitToggled(v) => {
            state.options.upload_limited = v;
            Task::none()
        }
        Message::OptionsUploadLimitChanged(v) => {
            if v.is_empty() || v.chars().all(|c| c.is_ascii_digit()) {
                state.options.upload_limit_val = v;
            }
            Task::none()
        }
        Message::OptionsRatioModeChanged(v) => {
            state.options.ratio_mode = v;
            Task::none()
        }
        Message::OptionsRatioLimitChanged(v) => {
            // Ratio allows digits and at most one decimal point.
            let dot_count = v.chars().filter(|c| *c == '.').count();
            if v.is_empty() || (v.chars().all(|c| c.is_ascii_digit() || c == '.') && dot_count <= 1)
            {
                state.options.ratio_limit_val = v;
            }
            Task::none()
        }
        Message::OptionsHonorGlobalToggled(v) => {
            state.options.honors_session_limits = v;
            Task::none()
        }
        // Submit messages are intercepted by main_screen; nothing to update here.
        Message::OptionsDownloadLimitSubmitted
        | Message::OptionsUploadLimitSubmitted
        | Message::OptionsRatioLimitSubmitted => Task::none(),

        // ── Bulk Options tab (multi-select) ───────────────────────────────────
        Message::BulkDownloadLimitToggled(v) => {
            state.bulk_options.download_limited = Some(v);
            Task::none()
        }
        Message::BulkDownloadLimitChanged(v) => {
            if v.is_empty() || v.chars().all(|c| c.is_ascii_digit()) {
                state.bulk_options.download_limit_val = v;
            }
            Task::none()
        }
        Message::BulkUploadLimitToggled(v) => {
            state.bulk_options.upload_limited = Some(v);
            Task::none()
        }
        Message::BulkUploadLimitChanged(v) => {
            if v.is_empty() || v.chars().all(|c| c.is_ascii_digit()) {
                state.bulk_options.upload_limit_val = v;
            }
            Task::none()
        }
        Message::BulkRatioModeChanged(v) => {
            state.bulk_options.ratio_mode = Some(v);
            Task::none()
        }
        Message::BulkRatioLimitChanged(v) => {
            let dot_count = v.chars().filter(|c| *c == '.').count();
            if v.is_empty() || (v.chars().all(|c| c.is_ascii_digit() || c == '.') && dot_count <= 1)
            {
                state.bulk_options.ratio_limit_val = v;
            }
            Task::none()
        }
        Message::BulkHonorGlobalToggled(v) => {
            state.bulk_options.honors_session_limits = Some(v);
            Task::none()
        }
        // Bulk submits are intercepted by main_screen.
        Message::BulkDownloadLimitSubmitted
        | Message::BulkUploadLimitSubmitted
        | Message::BulkRatioLimitSubmitted => Task::none(),
    }
}
