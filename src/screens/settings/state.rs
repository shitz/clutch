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

use uuid::Uuid;

use crate::profile::{ConnectionProfile, GeneralSettings, ProfileStore, ThemeConfig};

use super::SettingsTab;
use super::draft::ProfileDraft;

/// A navigation action deferred while the unsaved-change guard is shown.
#[derive(Debug, Clone)]
pub enum PendingNavigation {
    SwitchTab(SettingsTab),
    SwitchProfile(Uuid),
    Close,
}

#[derive(Debug)]
pub struct SettingsScreen {
    // ── Tab ──────────────────────────────────────────────────────────────────
    pub active_tab: SettingsTab,

    // ── Unsaved-change guard ─────────────────────────────────────────────────
    pub dirty: bool,
    pub confirm_discard: Option<PendingNavigation>,
    pub confirm_delete_id: Option<Uuid>,

    // ── General tab state ────────────────────────────────────────────────────
    pub theme_draft: ThemeConfig,
    pub refresh_interval_draft: String,
    pub general_validation_error: Option<String>,

    // ── Connections tab state ────────────────────────────────────────────────
    pub profiles: Vec<ConnectionProfile>,
    pub selected_profile_id: Option<Uuid>,
    pub draft: Option<ProfileDraft>,

    // ── Context from AppState ────────────────────────────────────────────────
    pub active_profile_id: Option<Uuid>,
    /// Preserved from the app-level store so disk saves always include it.
    pub master_passphrase_hash: Option<String>,
    // ── General tab transient state ──────────────────────────────────────────
    pub general_saved: bool,
    pub general_dirty: bool,
    pub theme_saved: ThemeConfig,
    pub refresh_interval_saved: String,
}

impl SettingsScreen {
    pub fn new(
        store: &ProfileStore,
        active_profile_id: Option<Uuid>,
        initial_tab: SettingsTab,
    ) -> Self {
        Self {
            active_tab: initial_tab,
            dirty: false,
            confirm_discard: None,
            confirm_delete_id: None,
            theme_draft: store.general.theme,
            refresh_interval_draft: store.general.refresh_interval.to_string(),
            general_validation_error: None,
            profiles: store.profiles.clone(),
            selected_profile_id: store.profiles.first().map(|p| p.id),
            draft: store.profiles.first().map(ProfileDraft::from_profile),
            active_profile_id,
            master_passphrase_hash: store.master_passphrase_hash.clone(),
            general_saved: false,
            general_dirty: false,
            theme_saved: store.general.theme,
            refresh_interval_saved: store.general.refresh_interval.to_string(),
        }
    }

    pub fn build_store_snapshot(&self) -> ProfileStore {
        ProfileStore {
            last_connected: None,
            master_passphrase_hash: self.master_passphrase_hash.clone(),
            general: GeneralSettings {
                theme: self.theme_draft,
                refresh_interval: self
                    .refresh_interval_draft
                    .parse()
                    .unwrap_or(5)
                    .clamp(1, 30),
            },
            profiles: self.profiles.clone(),
        }
    }

    pub fn validate_refresh_interval(&mut self) {
        match self.refresh_interval_draft.parse::<u8>() {
            Ok(v) if (1..=30).contains(&v) => self.general_validation_error = None,
            Ok(_) => {
                self.general_validation_error =
                    Some("Refresh interval must be between 1 and 30 seconds.".to_owned())
            }
            Err(_) => {
                self.general_validation_error =
                    Some("Enter a whole number between 1 and 30.".to_owned())
            }
        }
    }

    pub fn draft_is_saveable(&self) -> bool {
        let Some(d) = &self.draft else {
            return false;
        };
        !d.name.is_empty() && d.port.parse::<u16>().is_ok()
    }

    pub fn execute_pending_nav(&mut self, nav: PendingNavigation) {
        match nav {
            PendingNavigation::SwitchTab(tab) => {
                self.active_tab = tab;
            }
            PendingNavigation::SwitchProfile(id) => {
                self.selected_profile_id = Some(id);
                if let Some(p) = self.profiles.iter().find(|p| p.id == id) {
                    self.draft = Some(ProfileDraft::from_profile(p));
                }
            }
            PendingNavigation::Close => {
                // Handled by the caller — GuardSave/GuardDiscard return
                // SettingsResult::Closed directly for Close navigation.
            }
        }
    }
}
