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

use iced::Task;

use crate::profile::ConnectionProfile;

use super::draft::{ProfileDraft, TestResult};
use super::state::{PendingNavigation, SettingsScreen};
use super::{Message, SettingsResult};

impl SettingsScreen {
    /// Handle a settings message.
    ///
    /// Returns `(task, Option<SettingsResult>)`. When `SettingsResult` is `Some`,
    /// the caller (`app::update`) must act on the result.
    pub fn update(&mut self, msg: Message) -> (Task<Message>, Option<SettingsResult>) {
        // ── Unsaved-change guard intercept ────────────────────────────────────
        if self.dirty && self.confirm_discard.is_none() {
            match &msg {
                Message::TabClicked(tab) if *tab != self.active_tab => {
                    self.confirm_discard = Some(PendingNavigation::SwitchTab(*tab));
                    return (Task::none(), None);
                }
                Message::ProfileListClicked(id) if Some(*id) != self.selected_profile_id => {
                    self.confirm_discard = Some(PendingNavigation::SwitchProfile(*id));
                    return (Task::none(), None);
                }
                Message::CloseClicked => {
                    self.confirm_discard = Some(PendingNavigation::Close);
                    return (Task::none(), None);
                }
                _ => {}
            }
        }

        match msg {
            // ── Navigation ────────────────────────────────────────────────────
            Message::TabClicked(tab) => {
                self.active_tab = tab;
                (Task::none(), None)
            }
            Message::CloseClicked => {
                let store = self.build_store_snapshot();
                (Task::none(), Some(SettingsResult::Closed(store)))
            }

            // ── General tab ───────────────────────────────────────────────────
            Message::ThemeConfigChanged(cfg) => {
                self.theme_draft = cfg;
                self.general_dirty = true;
                let store = self.build_store_snapshot();
                (
                    Task::none(),
                    Some(SettingsResult::GeneralSettingsSaved {
                        theme_config: cfg,
                        store,
                    }),
                )
            }
            Message::RefreshIntervalChanged(v) => {
                self.refresh_interval_draft = v;
                self.validate_refresh_interval();
                self.general_saved = false;
                self.general_dirty = true;
                (Task::none(), None)
            }
            Message::GeneralSaveClicked => {
                self.validate_refresh_interval();
                if self.general_validation_error.is_some() {
                    return (Task::none(), None);
                }
                self.general_saved = true;
                self.general_dirty = false;
                self.theme_saved = self.theme_draft;
                self.refresh_interval_saved = self.refresh_interval_draft.clone();
                let store = self.build_store_snapshot();
                let store_clone = store.clone();
                let task = Task::perform(async move { store_clone.save().await }, |_| {
                    Message::GeneralRevertClicked
                });
                (
                    task,
                    Some(SettingsResult::GeneralSettingsSaved {
                        theme_config: self.theme_draft,
                        store,
                    }),
                )
            }
            Message::GeneralRevertClicked => {
                self.theme_draft = self.theme_saved;
                self.refresh_interval_draft = self.refresh_interval_saved.clone();
                self.general_dirty = false;
                self.general_saved = false;
                self.validate_refresh_interval();
                let store = self.build_store_snapshot();
                (
                    Task::none(),
                    Some(SettingsResult::GeneralSettingsSaved {
                        theme_config: self.theme_saved,
                        store,
                    }),
                )
            }

            // ── Connections – list ────────────────────────────────────────────
            Message::ProfileListClicked(id) => {
                self.selected_profile_id = Some(id);
                if let Some(p) = self.profiles.iter().find(|p| p.id == id) {
                    self.draft = Some(ProfileDraft::from_profile(p));
                }
                self.dirty = false;
                (Task::none(), None)
            }
            Message::AddProfileClicked => {
                let blank = ConnectionProfile::new_blank();
                let id = blank.id;
                self.profiles.push(blank.clone());
                self.selected_profile_id = Some(id);
                self.draft = Some(ProfileDraft::from_blank(&blank));
                self.dirty = true;
                (Task::none(), None)
            }
            Message::DeleteProfileClicked => {
                if let Some(id) = self.selected_profile_id {
                    self.confirm_delete_id = Some(id);
                }
                (Task::none(), None)
            }
            Message::DeleteCancelled => {
                self.confirm_delete_id = None;
                (Task::none(), None)
            }
            Message::DeleteConfirmed => {
                let Some(id) = self.confirm_delete_id.take() else {
                    return (Task::none(), None);
                };
                self.profiles.retain(|p| p.id != id);
                if self.selected_profile_id == Some(id) {
                    self.selected_profile_id = None;
                    self.draft = None;
                    self.dirty = false;
                }
                let store = self.build_store_snapshot();
                let s = store.clone();
                let task = Task::perform(async move { s.save().await }, |_| Message::RevertClicked);
                (task, Some(SettingsResult::StoreUpdated(store)))
            }

            // ── Connections – draft form ──────────────────────────────────────
            Message::DraftNameChanged(v) => {
                if let Some(d) = &mut self.draft {
                    d.name = v;
                    self.dirty = true;
                }
                (Task::none(), None)
            }
            Message::DraftHostChanged(v) => {
                if let Some(d) = &mut self.draft {
                    d.host = v;
                    self.dirty = true;
                }
                (Task::none(), None)
            }
            Message::DraftPortChanged(v) => {
                if let Some(d) = &mut self.draft {
                    d.port = v;
                    self.dirty = true;
                }
                (Task::none(), None)
            }
            Message::DraftUsernameChanged(v) => {
                if let Some(d) = &mut self.draft {
                    d.username = v;
                    self.dirty = true;
                }
                (Task::none(), None)
            }
            Message::DraftPasswordChanged(v) => {
                if let Some(d) = &mut self.draft {
                    d.password = v;
                    d.password_changed = true;
                    self.dirty = true;
                }
                (Task::none(), None)
            }
            Message::TestConnectionClicked => {
                let Some(d) = &mut self.draft else {
                    return (Task::none(), None);
                };
                // If the profile has a saved password and the user hasn't typed a new one,
                // delegate to app::update which has access to the decrypted passphrase.
                if d.has_saved_password && !d.password_changed {
                    let profile_id = d.id;
                    d.testing = true;
                    d.test_result = None;
                    return (
                        Task::none(),
                        Some(SettingsResult::TestConnectionWithId { profile_id }),
                    );
                }
                let Some(creds) = d.to_credentials() else {
                    return (Task::none(), None);
                };
                d.testing = true;
                d.test_result = None;
                let url = creds.rpc_url();
                let task = Task::perform(
                    async move {
                        crate::rpc::session_get(&url, &creds, "")
                            .await
                            .map_err(|e| e.to_string())
                    },
                    Message::TestConnectionResult,
                );
                (task, None)
            }
            Message::TestConnectionResult(result) => {
                if let Some(d) = &mut self.draft {
                    d.testing = false;
                    d.test_result = Some(match result {
                        Ok(_) => TestResult::Success,
                        Err(e) => TestResult::Failure(e),
                    });
                }
                (Task::none(), None)
            }
            Message::SaveClicked => {
                let Some(draft) = self.draft.clone() else {
                    return (Task::none(), None);
                };
                if !self.draft_is_saveable() {
                    return (Task::none(), None);
                }
                let id = draft.id;
                if let Some(p) = self.profiles.iter_mut().find(|p| p.id == id) {
                    let port: u16 = draft.port.parse().unwrap_or(9091);
                    p.name = draft.name.clone();
                    p.host = draft.host.clone();
                    p.port = port;
                    p.username = if draft.username.is_empty() {
                        None
                    } else {
                        Some(draft.username.clone())
                    };
                    // Clear encrypted password if user explicitly set an empty password.
                    if draft.password_changed && draft.password.is_empty() {
                        p.encrypted_password = None;
                    }
                }
                self.dirty = false;
                if let Some(p) = self.profiles.iter().find(|p| p.id == id) {
                    self.draft = Some(ProfileDraft::from_profile(p));
                }
                let store = self.build_store_snapshot();
                // If the user entered a new non-empty password, hand it off to app::update
                // for passphrase-protected encryption rather than storing it directly.
                if draft.password_changed && !draft.password.is_empty() {
                    // The password will be encrypted asynchronously. Mark the draft so
                    // the placeholder shows immediately rather than after the next reload.
                    if let Some(d) = &mut self.draft {
                        d.has_saved_password = true;
                    }
                    return (
                        Task::none(),
                        Some(SettingsResult::SaveWithPassword {
                            profile_id: id,
                            password: draft.password.clone(),
                            store,
                        }),
                    );
                }
                let store_clone = store.clone();
                let task = Task::perform(async move { store_clone.save().await }, |_| {
                    Message::RevertClicked
                });
                let result = if self.active_profile_id == Some(id) {
                    SettingsResult::ActiveProfileSaved {
                        profile_id: id,
                        store,
                    }
                } else {
                    SettingsResult::StoreUpdated(store)
                };
                (task, Some(result))
            }
            Message::RevertClicked => {
                if let Some(id) = self.selected_profile_id
                    && let Some(p) = self.profiles.iter().find(|p| p.id == id)
                {
                    self.draft = Some(ProfileDraft::from_profile(p));
                }
                self.dirty = false;
                (Task::none(), None)
            }

            // ── Unsaved-change guard ──────────────────────────────────────────
            Message::GuardSave => {
                let pending = self.confirm_discard.take();
                let (save_task, save_result) = self.update(Message::SaveClicked);
                if let Some(nav) = pending {
                    match nav {
                        PendingNavigation::Close => {
                            // Save was done. Override result with Closed unless
                            // save triggered a reconnect (ActiveProfileSaved).
                            let result = match save_result {
                                Some(SettingsResult::ActiveProfileSaved { .. }) => save_result,
                                _ => Some(SettingsResult::Closed(self.build_store_snapshot())),
                            };
                            return (save_task, result);
                        }
                        other => self.execute_pending_nav(other),
                    }
                }
                (save_task, save_result)
            }
            Message::GuardDiscard => {
                let pending = self.confirm_discard.take();
                self.dirty = false;
                if let Some(id) = self.selected_profile_id
                    && let Some(p) = self.profiles.iter().find(|p| p.id == id)
                {
                    self.draft = Some(ProfileDraft::from_profile(p));
                }
                if let Some(nav) = pending {
                    match nav {
                        PendingNavigation::Close => {
                            let store = self.build_store_snapshot();
                            return (Task::none(), Some(SettingsResult::Closed(store)));
                        }
                        other => self.execute_pending_nav(other),
                    }
                }
                (Task::none(), None)
            }
            Message::GuardCancel => {
                self.confirm_discard = None;
                (Task::none(), None)
            }

            // ── Keyboard ─────────────────────────────────────────────────
            Message::TabKeyPressed { shift } => {
                // Only active in the Connections tab when a profile is being edited.
                if self.active_tab != super::SettingsTab::Connections || self.draft.is_none() {
                    return (Task::none(), None);
                }
                // Use iced's built-in focus cycling so Tab after a mouse click
                // continues from the field the user actually clicked into.
                let task = if shift {
                    iced::widget::operation::focus_previous()
                } else {
                    iced::widget::operation::focus_next()
                };
                (task, None)
            }

            // Enter has no primary action in the settings screen (non-goal).
            Message::EnterPressed => (Task::none(), None),
        }
    }
}
