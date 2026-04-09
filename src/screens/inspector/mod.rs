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

//! Displays per-torrent detail in a tabbed panel.
//!
//! `view()` accepts an immutable reference to the currently selected
//! `TorrentData` — all data arrives via the polling subscription; the
//! inspector owns no RPC state.
//!
//! # Architecture
//!
//! This module is a self-contained Elm component:
//! - [`InspectorScreen`] — state (active tab only)
//! - [`Message`] — messages that can be dispatched to this component
//! - [`update`] — pure state transition
//! - [`view`] — renders the panel for the given torrent

mod state;
mod update;
mod view;

pub use state::{InspectorBulkOptionsState, InspectorOptionsState, InspectorScreen};
pub use update::update;
pub use view::view;

// ── ActiveTab ─────────────────────────────────────────────────────────────────

/// The currently visible inspector tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ActiveTab {
    #[default]
    General,
    Files,
    Trackers,
    Peers,
    Options,
}

// ── Message ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    TabSelected(ActiveTab),
    FileWantedToggled {
        torrent_id: i64,
        file_index: usize,
        wanted: bool,
    },
    AllFilesWantedToggled {
        torrent_id: i64,
        file_count: usize,
        wanted: bool,
    },
    /// Emitted when the SetFileWanted RPC completes (success or failure).
    /// Removes the given indices from `pending_wanted`.
    FileWantedSetSuccess {
        indices: Vec<usize>,
    },
    // ── Options tab messages ──────────────────────────────────────────────
    OptionsDownloadLimitToggled(bool),
    OptionsDownloadLimitChanged(String),
    OptionsDownloadLimitSubmitted,
    OptionsUploadLimitToggled(bool),
    OptionsUploadLimitChanged(String),
    OptionsUploadLimitSubmitted,
    OptionsRatioModeChanged(u8),
    OptionsRatioLimitChanged(String),
    OptionsRatioLimitSubmitted,
    OptionsHonorGlobalToggled(bool),
    // ── Bulk Options tab messages (multi-select) ──────────────────────────
    BulkDownloadLimitToggled(bool),
    BulkDownloadLimitChanged(String),
    BulkDownloadLimitSubmitted,
    BulkUploadLimitToggled(bool),
    BulkUploadLimitChanged(String),
    BulkUploadLimitSubmitted,
    BulkRatioModeChanged(u8),
    BulkRatioLimitChanged(String),
    BulkRatioLimitSubmitted,
    BulkHonorGlobalToggled(bool),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `TabSelected` updates `active_tab`.
    #[test]
    fn tab_selected_updates_active() {
        let mut screen = InspectorScreen::new();
        assert_eq!(screen.active_tab, ActiveTab::General);

        let _ = update(&mut screen, Message::TabSelected(ActiveTab::Files));
        assert_eq!(screen.active_tab, ActiveTab::Files);

        let _ = update(&mut screen, Message::TabSelected(ActiveTab::Trackers));
        assert_eq!(screen.active_tab, ActiveTab::Trackers);

        let _ = update(&mut screen, Message::TabSelected(ActiveTab::Peers));
        assert_eq!(screen.active_tab, ActiveTab::Peers);

        let _ = update(&mut screen, Message::TabSelected(ActiveTab::General));
        assert_eq!(screen.active_tab, ActiveTab::General);
    }

    // ── Selective file download (inspector) ─────────────────────────────────

    /// `FileWantedToggled` inserts the toggled index into `pending_wanted`.
    #[test]
    fn file_wanted_toggled_updates_pending() {
        let mut screen = InspectorScreen::new();
        let _ = update(
            &mut screen,
            Message::FileWantedToggled {
                torrent_id: 1,
                file_index: 2,
                wanted: false,
            },
        );
        assert_eq!(screen.pending_wanted.get(&2), Some(&false));
        assert!(!screen.pending_wanted.contains_key(&0));
    }

    /// `FileWantedSetSuccess` removes only the specified indices.
    #[test]
    fn file_wanted_set_success_clears_only_specified_indices() {
        let mut screen = InspectorScreen::new();
        // Seed several pending entries.
        screen.pending_wanted.insert(0, true);
        screen.pending_wanted.insert(1, false);
        screen.pending_wanted.insert(2, true);

        let _ = update(
            &mut screen,
            Message::FileWantedSetSuccess {
                indices: vec![0, 2],
            },
        );

        assert!(
            !screen.pending_wanted.contains_key(&0),
            "index 0 should be cleared"
        );
        assert!(
            !screen.pending_wanted.contains_key(&2),
            "index 2 should be cleared"
        );
        assert_eq!(
            screen.pending_wanted.get(&1),
            Some(&false),
            "index 1 must remain untouched"
        );
    }

    /// `AllFilesWantedToggled` inserts all indices into `pending_wanted`.
    #[test]
    fn all_files_wanted_toggled_populates_all_indices() {
        let mut screen = InspectorScreen::new();
        let _ = update(
            &mut screen,
            Message::AllFilesWantedToggled {
                torrent_id: 7,
                file_count: 4,
                wanted: false,
            },
        );
        for i in 0..4 {
            assert_eq!(
                screen.pending_wanted.get(&i),
                Some(&false),
                "index {i} should be set to false"
            );
        }
    }

    /// The inspector `Message` enum has no variant that would clear
    /// `pending_wanted` on a background poll. Calling
    /// `TabSelected` (a non-file message) leaves `pending_wanted` unchanged.
    #[test]
    fn poll_does_not_clear_pending_wanted() {
        let mut screen = InspectorScreen::new();
        screen.pending_wanted.insert(5, true);

        // Simulate any non-file-wanted message arriving (e.g. from a background poll path).
        let _ = update(&mut screen, Message::TabSelected(ActiveTab::Files));

        assert_eq!(
            screen.pending_wanted.get(&5),
            Some(&true),
            "pending_wanted must not be cleared by unrelated messages"
        );
    }
}
