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

//! Split into sub-modules:
//! - [`draft`] — in-memory editable copy of a connection profile
//! - [`state`] — screen state and helpers
//! - [`update`] — message handling (`SettingsScreen::update`)
//! - [`view`] — UI rendering (`SettingsScreen::view`)

mod draft;
mod state;
mod update;
mod view;

use uuid::Uuid;

use crate::profile::{ProfileStore, ThemeConfig};
use crate::rpc::SessionInfo;

pub use self::state::SettingsScreen;

// ── Tab ───────────────────────────────────────────────────────────────────────

/// The currently active Settings tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    #[default]
    General,
    Connections,
    About,
}

// ── Message ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Message {
    // Navigation
    TabClicked(SettingsTab),
    CloseClicked,

    // General tab
    ThemeConfigChanged(ThemeConfig),
    RefreshIntervalChanged(String),
    GeneralSaveClicked,
    GeneralRevertClicked,

    // Connections tab – list
    ProfileListClicked(Uuid),
    AddProfileClicked,
    DeleteProfileClicked,
    DeleteConfirmed,
    DeleteCancelled,

    // Connections tab – draft form
    DraftNameChanged(String),
    DraftHostChanged(String),
    DraftPortChanged(String),
    DraftUsernameChanged(String),
    DraftPasswordChanged(String),
    TestConnectionClicked,
    TestConnectionResult(Result<SessionInfo, String>),
    SaveClicked,
    RevertClicked,

    // Unsaved-change guard
    GuardSave,
    GuardDiscard,
    GuardCancel,
}

// ── Result reported back to app::update ──────────────────────────────────────

/// Outcome produced by certain `SettingsScreen::update()` calls.
pub enum SettingsResult {
    /// General settings were saved; caller must update `AppState::theme`.
    GeneralSettingsSaved {
        theme_config: ThemeConfig,
        store: ProfileStore,
    },
    /// A profile was saved that is the currently active connection; caller must reconnect.
    ActiveProfileSaved {
        profile_id: Uuid,
        store: ProfileStore,
    },
    /// Profile store changed (non-active profile); caller can just update its local copy.
    StoreUpdated(ProfileStore),
    /// User closed the Settings screen; carry updated store to AppState.
    Closed(ProfileStore),
    /// Profile was saved and has a non-empty password that needs encrypting.
    ///
    /// The store already contains the updated profile (non-password fields).
    /// The caller must encrypt `password` using the master passphrase and store
    /// the result as `ConnectionProfile::encrypted_password`.
    SaveWithPassword {
        profile_id: Uuid,
        password: String,
        store: ProfileStore,
    },
    /// Test connection requested for a profile that already has a saved (encrypted) password
    /// but the draft has not been changed. The caller must decrypt the password and run
    /// the RPC probe, routing the `TestConnectionResult` back to the settings screen.
    TestConnectionWithId { profile_id: Uuid },
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::ConnectionProfile;

    fn store_with_profile() -> (ProfileStore, uuid::Uuid) {
        let p = ConnectionProfile::new_blank();
        let id = p.id;
        let mut store = ProfileStore::default();
        store.profiles.push(p);
        (store, id)
    }

    fn screen_with_selected_profile() -> (SettingsScreen, uuid::Uuid) {
        let (store, id) = store_with_profile();
        let mut s = SettingsScreen::new(&store, None, SettingsTab::Connections);
        let _ = s.update(Message::ProfileListClicked(id));
        (s, id)
    }

    #[test]
    fn draft_field_change_sets_dirty() {
        let (mut s, _) = screen_with_selected_profile();
        assert!(!s.dirty);
        let (_, result) = s.update(Message::DraftHostChanged("newhost".to_owned()));
        assert!(s.dirty);
        assert!(result.is_none());
    }

    #[test]
    fn revert_restores_draft_and_clears_dirty() {
        let (mut s, _) = screen_with_selected_profile();
        let original = s.draft.as_ref().unwrap().host.clone();
        let _ = s.update(Message::DraftHostChanged("changed".to_owned()));
        assert!(s.dirty);
        let _ = s.update(Message::RevertClicked);
        assert!(!s.dirty);
        assert_eq!(s.draft.as_ref().unwrap().host, original);
    }

    #[test]
    fn save_is_noop_when_port_invalid() {
        let (mut s, _) = screen_with_selected_profile();
        let _ = s.update(Message::DraftPortChanged("notaport".to_owned()));
        let (_, result) = s.update(Message::SaveClicked);
        assert!(result.is_none(), "save should be blocked on invalid port");
        assert!(s.dirty, "dirty flag remains set after failed save");
    }

    #[test]
    fn tab_click_when_dirty_sets_guard() {
        let (mut s, _) = screen_with_selected_profile();
        let _ = s.update(Message::DraftHostChanged("dirty".to_owned()));
        assert!(s.dirty);
        let (_, _) = s.update(Message::TabClicked(SettingsTab::General));
        assert_eq!(
            s.active_tab,
            SettingsTab::Connections,
            "tab must not switch yet"
        );
        assert!(s.confirm_discard.is_some(), "guard must be armed");
    }

    #[test]
    fn guard_discard_clears_dirty_and_navigates() {
        let (mut s, _) = screen_with_selected_profile();
        let _ = s.update(Message::DraftHostChanged("dirty".to_owned()));
        let _ = s.update(Message::TabClicked(SettingsTab::General));
        assert!(s.confirm_discard.is_some());
        let _ = s.update(Message::GuardDiscard);
        assert!(!s.dirty);
        assert!(s.confirm_discard.is_none());
        assert_eq!(
            s.active_tab,
            SettingsTab::General,
            "deferred nav must execute"
        );
    }

    #[test]
    fn general_save_blocked_when_interval_out_of_range() {
        let (store, _) = store_with_profile();
        let mut s = SettingsScreen::new(&store, None, SettingsTab::General);
        let _ = s.update(Message::RefreshIntervalChanged("31".to_owned()));
        let (_, result) = s.update(Message::GeneralSaveClicked);
        assert!(
            s.general_validation_error.is_some(),
            "validation error should be set"
        );
        assert!(result.is_none(), "save should be blocked");
    }

    /// R13: GuardDiscard + Close must return SettingsResult::Closed.
    #[test]
    fn guard_discard_close_returns_closed() {
        let (mut s, _) = screen_with_selected_profile();
        let _ = s.update(Message::DraftHostChanged("dirty".to_owned()));
        // Arm the guard with Close
        let _ = s.update(Message::CloseClicked);
        assert!(s.confirm_discard.is_some());
        let (_, result) = s.update(Message::GuardDiscard);
        assert!(
            matches!(result, Some(SettingsResult::Closed(_))),
            "discard + close must return Closed"
        );
    }
}
