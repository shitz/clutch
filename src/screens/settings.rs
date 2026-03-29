//! Settings screen — a full-screen `Screen` variant for managing connection
//! profiles and application preferences.
//!
//! # Layout
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │  ← Back    Settings                                              │
//! ├──────────────────────────────────────────────────────────────────┤
//! │  [ General ]  [ Connections ]                                    │  ← Tab bar
//! ├──────────────────────────────────────────────────────────────────┤
//! │  (tab content)                                                   │
//! └──────────────────────────────────────────────────────────────────┘
//! ```
//!
//! When the user has unsaved changes in the Connections tab, any navigation
//! (tab switch, profile switch, close) triggers an unsaved-change guard dialog
//! rendered as a `stack!` + `opaque` overlay.
//!
//! # Message routing
//!
//! `SettingsScreen::update()` returns a `(Task<Message>, Option<SettingsResult>)`.
//! `SettingsResult` signals the parent (`app::update`) that settings have been
//! saved and carries the updated [`ProfileStore`], or that the screen should close.

use iced::widget::{
    Space, button, column, container, opaque, row, scrollable, stack, text, text_input,
};
use iced::{Alignment, Color, Element, Length, Task};

use uuid::Uuid;

use crate::profile::{ConnectionProfile, GeneralSettings, ProfileStore, ThemeConfig};
use crate::rpc::TransmissionCredentials;

// ── Tab ───────────────────────────────────────────────────────────────────────

/// The currently active Settings tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    #[default]
    General,
    Connections,
}

// ── Pending navigation (for unsaved-change guard) ─────────────────────────────

/// A navigation action deferred while waiting for the user to resolve unsaved changes.
#[derive(Debug, Clone)]
pub enum PendingNavigation {
    SwitchTab(SettingsTab),
    SwitchProfile(Uuid),
    Close,
}

// ── Test Connection result ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum TestResult {
    Success,
    Failure(String),
}

// ── Profile draft (editable copy) ────────────────────────────────────────────

/// An in-memory editable copy of a [`ConnectionProfile`].
///
/// Edits mutate only this draft. The canonical profile is updated only on Save.
#[derive(Debug, Clone)]
pub struct ProfileDraft {
    pub id: Uuid,
    pub name: String,
    pub host: String,
    pub port: String,
    pub username: String,
    /// Current password field value (loaded from keyring on profile selection).
    pub password: String,
    /// `true` if the password field has been touched since the draft was created.
    pub password_changed: bool,
    /// Result of the last [Test Connection] probe, if any.
    pub test_result: Option<TestResult>,
    /// `true` while a test probe is in-flight.
    pub testing: bool,
}

impl ProfileDraft {
    fn from_profile(profile: &ConnectionProfile) -> Self {
        // Password is intentionally NOT loaded here — fetched on demand
        // (Test Connection / connect) to avoid triggering the OS keychain
        // unlock dialog just for browsing profile settings.
        Self {
            id: profile.id,
            name: profile.name.clone(),
            host: profile.host.clone(),
            port: profile.port.to_string(),
            username: profile.username.clone().unwrap_or_default(),
            password: String::new(),
            password_changed: false,
            test_result: None,
            testing: false,
        }
    }

    fn from_blank(profile: &ConnectionProfile) -> Self {
        Self {
            id: profile.id,
            name: profile.name.clone(),
            host: profile.host.clone(),
            port: profile.port.to_string(),
            username: String::new(),
            password: String::new(),
            password_changed: false,
            test_result: None,
            testing: false,
        }
    }

    fn to_credentials(&self) -> Option<TransmissionCredentials> {
        let port: u16 = self.port.parse().ok()?;
        Some(TransmissionCredentials {
            host: self.host.clone(),
            port,
            username: if self.username.is_empty() {
                None
            } else {
                Some(self.username.clone())
            },
            password: if self.password.is_empty() {
                None
            } else {
                Some(self.password.clone())
            },
        })
    }
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
    TestConnectionResult(Result<crate::rpc::SessionInfo, String>),
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
}

// ── State ─────────────────────────────────────────────────────────────────────

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

    // ── General tab transient state ──────────────────────────────────────────
    /// Set to `true` after the user explicitly clicks Save on the General tab.
    pub general_saved: bool,
    /// Set to `true` when the general tab has unsaved changes.
    pub general_dirty: bool,
    /// Last-persisted values, used to implement Revert on the General tab.
    pub theme_saved: ThemeConfig,
    pub refresh_interval_saved: String,
}

impl SettingsScreen {
    /// Create the Settings screen pre-configured with the given context.
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
            selected_profile_id: None,
            draft: None,
            active_profile_id,
            general_saved: false,
            general_dirty: false,
            theme_saved: store.general.theme,
            refresh_interval_saved: store.general.refresh_interval.to_string(),
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn build_store_snapshot(&self) -> ProfileStore {
        ProfileStore {
            last_connected: None, // caller fills this from AppState
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

    fn validate_refresh_interval(&mut self) {
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

    /// Check if the current draft can be saved (port is valid, name non-empty).
    fn draft_is_saveable(&self) -> bool {
        let Some(d) = &self.draft else { return false };
        !d.name.is_empty() && d.port.parse::<u16>().is_ok()
    }
}

// ── Elm functions ─────────────────────────────────────────────────────────────

impl SettingsScreen {
    /// Handle a settings message.
    ///
    /// Returns `(task, Option<SettingsResult>)`. When `SettingsResult` is `Some`,
    /// the caller (`app::update`) must act on the result (update store, re-connect,
    /// or navigate away).
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
                // Immediate theme preview — emit result so app::update can act on it.
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
                let task = Task::perform(
                    async move { store_clone.save().await },
                    |_| Message::GeneralRevertClicked, // no-op after save
                );
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
                // Re-apply the saved theme as a live preview.
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
                ProfileStore::delete_password(id);
                if self.selected_profile_id == Some(id) {
                    self.selected_profile_id = None;
                    self.draft = None;
                    self.dirty = false;
                }
                let store = self.build_store_snapshot();
                // Clear last_connected if it matched the deleted profile.
                // The caller will fill in the real last_connected from AppState.
                let task = {
                    let s = store.clone();
                    Task::perform(async move { s.save().await }, |_| Message::RevertClicked)
                };
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
                // Load password from keyring on first use if the user hasn't
                // typed one — this is the only time we touch the keychain.
                if !d.password_changed && d.password.is_empty() {
                    if let Some(pw) = ProfileStore::get_password(d.id) {
                        d.password = pw;
                    }
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
                // Apply draft to local profile list.
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
                }
                if draft.password_changed && !draft.password.is_empty() {
                    ProfileStore::set_password(id, &draft.password);
                }
                self.dirty = false;
                // Refresh draft to reflect saved state.
                if let Some(p) = self.profiles.iter().find(|p| p.id == id) {
                    self.draft = Some(ProfileDraft::from_profile(p));
                }
                let store = self.build_store_snapshot();
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
                if let Some(id) = self.selected_profile_id {
                    if let Some(p) = self.profiles.iter().find(|p| p.id == id) {
                        self.draft = Some(ProfileDraft::from_profile(p));
                    }
                }
                self.dirty = false;
                (Task::none(), None)
            }

            // ── Unsaved-change guard ──────────────────────────────────────────
            Message::GuardSave => {
                let pending = self.confirm_discard.take();
                // Perform the save first.
                let (save_task, save_result) = self.update(Message::SaveClicked);
                // Then execute the deferred navigation.
                if let Some(nav) = pending {
                    self.execute_pending_nav(nav);
                }
                (save_task, save_result)
            }
            Message::GuardDiscard => {
                let pending = self.confirm_discard.take();
                self.dirty = false;
                if let Some(id) = self.selected_profile_id {
                    if let Some(p) = self.profiles.iter().find(|p| p.id == id) {
                        self.draft = Some(ProfileDraft::from_profile(p));
                    }
                }
                if let Some(nav) = pending {
                    self.execute_pending_nav(nav);
                }
                (Task::none(), None)
            }
            Message::GuardCancel => {
                self.confirm_discard = None;
                (Task::none(), None)
            }
        }
    }

    fn execute_pending_nav(&mut self, nav: PendingNavigation) {
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
                // Nothing to do here — the Close message will be re-issued.
                // CloseClicked is handled specially in app::update.
            }
        }
    }
}

// ── View ──────────────────────────────────────────────────────────────────────

impl SettingsScreen {
    pub fn view(&self) -> Element<'_, Message> {
        let main_content = column![
            self.view_header(),
            self.view_tab_bar(),
            match self.active_tab {
                SettingsTab::General => self.view_general_tab(),
                SettingsTab::Connections => self.view_connections_tab(),
            },
        ]
        .spacing(0);

        // Overlay layers.
        if let Some(id) = self.confirm_delete_id {
            let name = self
                .profiles
                .iter()
                .find(|p| p.id == id)
                .map(|p| p.name.as_str())
                .unwrap_or("this profile");
            let title = format!("Delete \"{}\"?", name);
            let dialog = self.view_overlay_dialog(
                title,
                "This cannot be undone. The saved password will also be removed from the system keyring.",
                vec![
                    ("Cancel", Message::DeleteCancelled, false),
                    ("Delete", Message::DeleteConfirmed, true),
                ],
            );
            return stack![main_content, opaque(dialog)].into();
        }

        if self.confirm_discard.is_some() {
            let dialog = self.view_overlay_dialog(
                "You have unsaved changes".to_owned(),
                "Do you want to save your changes or discard them?",
                vec![
                    ("Cancel", Message::GuardCancel, false),
                    ("Discard", Message::GuardDiscard, false),
                    ("Save", Message::GuardSave, true),
                ],
            );
            return stack![main_content, opaque(dialog)].into();
        }

        main_content.into()
    }

    fn view_header(&self) -> Element<'_, Message> {
        row![
            button(crate::theme::icon(crate::theme::ICON_CLOSE))
                .on_press(Message::CloseClicked)
                .style(iced::widget::button::text),
            text("Settings").size(20),
            Space::new().width(Length::Fill),
        ]
        .spacing(12)
        .align_y(Alignment::Center)
        .padding([8, 16])
        .into()
    }

    fn view_tab_bar(&self) -> Element<'_, Message> {
        let tabs: &[(SettingsTab, &str)] = &[
            (SettingsTab::General, "General"),
            (SettingsTab::Connections, "Connections"),
        ];
        let buttons: Vec<Element<'_, Message>> = tabs
            .iter()
            .map(|(tab, label)| {
                let is_active = self.active_tab == *tab;
                let btn = button(text(*label).size(15).width(Length::Fill))
                    .on_press(Message::TabClicked(*tab))
                    .style(if is_active {
                        crate::theme::tab_active
                    } else {
                        crate::theme::tab_inactive
                    })
                    .width(Length::Fill)
                    .padding([6, 16]);
                let col: Element<'_, Message> = if is_active {
                    column![
                        btn,
                        container(Space::new().width(Length::Fill).height(2.0))
                            .style(crate::theme::tab_underline)
                            .width(Length::Fill),
                    ]
                    .width(Length::Fill)
                    .into()
                } else {
                    column![btn, Space::new().width(Length::Fill).height(2.0)]
                        .width(Length::Fill)
                        .into()
                };
                container(col).width(Length::FillPortion(1)).into()
            })
            .collect();
        row(buttons).spacing(0).into()
    }

    // ── General tab ──────────────────────────────────────────────────────────

    fn view_general_tab(&self) -> Element<'_, Message> {
        let theme_row = row![
            text("Theme").width(160),
            button("Light")
                .on_press(Message::ThemeConfigChanged(ThemeConfig::Light))
                .style(if self.theme_draft == ThemeConfig::Light {
                    iced::widget::button::primary
                } else {
                    iced::widget::button::secondary
                }),
            button("Dark")
                .on_press(Message::ThemeConfigChanged(ThemeConfig::Dark))
                .style(if self.theme_draft == ThemeConfig::Dark {
                    iced::widget::button::primary
                } else {
                    iced::widget::button::secondary
                }),
            button("System")
                .on_press(Message::ThemeConfigChanged(ThemeConfig::System))
                .style(if self.theme_draft == ThemeConfig::System {
                    iced::widget::button::primary
                } else {
                    iced::widget::button::secondary
                }),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let interval_row = row![
            text("Refresh interval (s)").width(160),
            text_input("1", &self.refresh_interval_draft)
                .on_input(Message::RefreshIntervalChanged)
                .width(80),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let form = column![theme_row, interval_row].spacing(16).padding(24);

        let save_enabled = self.general_validation_error.is_none();
        let save_btn = {
            let b = button(crate::theme::icon(crate::theme::ICON_SAVE))
                .style(iced::widget::button::primary)
                .padding([4, 8]);
            if save_enabled {
                b.on_press(Message::GeneralSaveClicked)
            } else {
                b
            }
        };
        let revert_btn = {
            let b = button(crate::theme::icon(crate::theme::ICON_UNDO))
                .style(iced::widget::button::secondary)
                .padding([4, 8]);
            if self.general_dirty {
                b.on_press(Message::GeneralRevertClicked)
            } else {
                b
            }
        };

        let mut status_col = column![].spacing(8);
        if let Some(err) = &self.general_validation_error {
            status_col = status_col.push(text(err.as_str()).style(|t: &iced::Theme| {
                iced::widget::text::Style {
                    color: Some(t.extended_palette().danger.base.color),
                }
            }));
        }
        if self.general_saved {
            status_col =
                status_col.push(text("\u{2713} Settings saved").style(|t: &iced::Theme| {
                    iced::widget::text::Style {
                        color: Some(t.extended_palette().success.base.color),
                    }
                }));
        }

        let action_row = row![revert_btn, save_btn]
            .spacing(4)
            .padding(iced::Padding {
                top: 8.0,
                right: 8.0,
                bottom: 8.0,
                left: 24.0,
            });

        column![
            form,
            Space::new().height(Length::Fill),
            status_col.padding(iced::Padding {
                top: 0.0,
                right: 24.0,
                bottom: 8.0,
                left: 24.0,
            }),
            action_row,
        ]
        .height(Length::Fill)
        .into()
    }

    // ── Connections tab ───────────────────────────────────────────────────────

    fn view_connections_tab(&self) -> Element<'_, Message> {
        let left = self.view_profile_list();
        let right = self.view_profile_detail();

        row![
            container(left).width(220).height(Length::Fill),
            container(right).width(Length::Fill).height(Length::Fill),
        ]
        .spacing(0)
        .height(Length::Fill)
        .into()
    }

    fn view_profile_list(&self) -> Element<'_, Message> {
        let items: Vec<Element<'_, Message>> = self
            .profiles
            .iter()
            .map(|p| {
                let is_selected = self.selected_profile_id == Some(p.id);
                let is_active = self.active_profile_id == Some(p.id);
                let label = if is_active {
                    format!("● {}", p.name)
                } else {
                    p.name.clone()
                };
                button(text(label).width(Length::Fill))
                    .on_press(Message::ProfileListClicked(p.id))
                    .style(if is_selected {
                        iced::widget::button::primary
                    } else {
                        iced::widget::button::text
                    })
                    .width(Length::Fill)
                    .padding([6, 12])
                    .into()
            })
            .collect();

        let list = scrollable(column(items).spacing(2)).height(Length::Fill);

        let delete_enabled = self.selected_profile_id.is_some()
            && self.selected_profile_id != self.active_profile_id;

        let del_btn = {
            let b = button(crate::theme::icon(crate::theme::ICON_TRASH))
                .style(iced::widget::button::secondary)
                .padding([4, 8]);
            if delete_enabled {
                b.on_press(Message::DeleteProfileClicked)
            } else {
                b
            }
        };

        column![
            list,
            row![
                button(crate::theme::icon(crate::theme::ICON_ADD))
                    .on_press(Message::AddProfileClicked)
                    .style(iced::widget::button::secondary)
                    .padding([4, 8]),
                del_btn,
            ]
            .spacing(4)
            .padding([8, 8]),
        ]
        .into()
    }

    fn view_profile_detail(&self) -> Element<'_, Message> {
        let Some(draft) = &self.draft else {
            return container(
                text(if self.profiles.is_empty() {
                    "No connections. Click '+' to add a new Transmission daemon."
                } else {
                    "Select a connection profile or create a new one."
                })
                .size(14),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .padding(24)
            .into();
        };

        let can_save = self.dirty && self.draft_is_saveable();
        let save_btn = {
            let b = button(crate::theme::icon(crate::theme::ICON_SAVE))
                .style(iced::widget::button::primary)
                .padding([4, 8]);
            if can_save {
                b.on_press(Message::SaveClicked)
            } else {
                b
            }
        };
        let revert_btn = {
            let b = button(crate::theme::icon(crate::theme::ICON_UNDO))
                .style(iced::widget::button::secondary)
                .padding([4, 8]);
            if self.dirty {
                b.on_press(Message::RevertClicked)
            } else {
                b
            }
        };

        let test_btn = {
            let b = button("Test Connection").style(iced::widget::button::secondary);
            if draft.testing {
                b
            } else {
                b.on_press(Message::TestConnectionClicked)
            }
        };

        let test_status: Element<'_, Message> = if draft.testing {
            text("Testing connection\u{2026}").into()
        } else {
            match &draft.test_result {
                Some(TestResult::Success) => text("\u{2713} Connection test successful!")
                    .style(|t: &iced::Theme| iced::widget::text::Style {
                        color: Some(t.extended_palette().success.base.color),
                    })
                    .into(),
                Some(TestResult::Failure(e)) => {
                    text(format!("\u{2717} Connection test failed: {e}"))
                        .style(|t: &iced::Theme| iced::widget::text::Style {
                            color: Some(t.extended_palette().danger.base.color),
                        })
                        .into()
                }
                None => Space::new().into(),
            }
        };

        let form = column![
            row![
                text("Profile Name").width(120),
                text_input("Name", &draft.name).on_input(Message::DraftNameChanged)
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![
                text("Host").width(120),
                text_input("localhost", &draft.host).on_input(Message::DraftHostChanged)
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![
                text("Port").width(120),
                text_input("9091", &draft.port)
                    .on_input(Message::DraftPortChanged)
                    .width(100)
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![
                text("Username").width(120),
                text_input("optional", &draft.username).on_input(Message::DraftUsernameChanged)
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![
                text("Password").width(120),
                text_input("optional", &draft.password)
                    .on_input(Message::DraftPasswordChanged)
                    .secure(true)
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![test_btn, test_status]
                .spacing(8)
                .align_y(Alignment::Center),
        ]
        .spacing(12)
        .padding(24);

        // Action row pinned to the bottom, left edge aligned with the form labels.
        let action_row = row![revert_btn, save_btn]
            .spacing(4)
            .padding(iced::Padding {
                top: 8.0,
                right: 8.0,
                bottom: 8.0,
                left: 24.0,
            });

        column![form, Space::new().height(Length::Fill), action_row]
            .height(Length::Fill)
            .into()
    }

    // ── Overlay dialog (for delete confirmation + unsaved guard) ──────────────

    fn view_overlay_dialog<'a>(
        &'a self,
        title: String,
        body: &'a str,
        actions: Vec<(&'a str, Message, bool)>,
    ) -> Element<'a, Message> {
        let buttons: Vec<Element<'_, Message>> = actions
            .into_iter()
            .map(|(label, msg, primary)| {
                button(label)
                    .on_press(msg)
                    .style(if primary {
                        iced::widget::button::danger
                    } else {
                        iced::widget::button::secondary
                    })
                    .into()
            })
            .collect();

        let card = container(
            column![
                text(title).size(18),
                text(body).size(13),
                row(buttons).spacing(8),
            ]
            .spacing(16),
        )
        .padding(28)
        .style(|t: &iced::Theme| {
            let p = t.extended_palette();
            container::Style {
                background: Some(iced::Background::Color(p.background.base.color)),
                border: iced::Border {
                    radius: 12.0.into(),
                    width: 1.0,
                    color: p.background.strong.color,
                },
                shadow: iced::Shadow {
                    color: Color::from_rgba8(0, 0, 0, 0.35),
                    offset: iced::Vector::new(0.0, 4.0),
                    blur_radius: 16.0,
                },
                ..Default::default()
            }
        });

        container(card)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .style(|_: &iced::Theme| container::Style {
                background: Some(iced::Background::Color(Color::from_rgba8(0, 0, 0, 0.70))),
                ..Default::default()
            })
            .into()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::{ConnectionProfile, ProfileStore};

    fn store_with_profile() -> (ProfileStore, uuid::Uuid) {
        let p = ConnectionProfile::new_blank();
        let id = p.id;
        let mut store = ProfileStore::default();
        store.profiles.push(p);
        (store, id)
    }

    /// Build a Settings screen on the Connections tab with a profile already selected.
    fn screen_with_selected_profile() -> (SettingsScreen, uuid::Uuid) {
        let (store, id) = store_with_profile();
        let mut s = SettingsScreen::new(&store, None, SettingsTab::Connections);
        let _ = s.update(Message::ProfileListClicked(id));
        (s, id)
    }

    /// Changing a draft field marks the screen as dirty.
    #[test]
    fn draft_field_change_sets_dirty() {
        let (mut s, _) = screen_with_selected_profile();
        assert!(!s.dirty);
        let (_, result) = s.update(Message::DraftHostChanged("newhost".to_owned()));
        assert!(s.dirty);
        assert!(result.is_none());
    }

    /// Revert restores the draft from the saved profile and clears dirty.
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

    /// SaveClicked is a no-op when the port field contains non-numeric text.
    #[test]
    fn save_is_noop_when_port_invalid() {
        let (mut s, _) = screen_with_selected_profile();
        let _ = s.update(Message::DraftPortChanged("notaport".to_owned()));
        let (_, result) = s.update(Message::SaveClicked);
        assert!(result.is_none(), "save should be blocked on invalid port");
        assert!(s.dirty, "dirty flag remains set after failed save");
    }

    /// Clicking a different tab while dirty arms the unsaved-change guard
    /// without switching the tab.
    #[test]
    fn tab_click_when_dirty_sets_guard() {
        let (mut s, _) = screen_with_selected_profile();
        // active_tab is already Connections; dirty the form
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

    /// GuardDiscard clears dirty state and then executes the deferred navigation.
    #[test]
    fn guard_discard_clears_dirty_and_navigates() {
        let (mut s, _) = screen_with_selected_profile();
        let _ = s.update(Message::DraftHostChanged("dirty".to_owned()));
        // Arm the guard by attempting to switch tab
        let _ = s.update(Message::TabClicked(SettingsTab::General));
        assert!(s.confirm_discard.is_some());
        // Discard changes
        let _ = s.update(Message::GuardDiscard);
        assert!(!s.dirty);
        assert!(s.confirm_discard.is_none());
        assert_eq!(
            s.active_tab,
            SettingsTab::General,
            "deferred nav must execute"
        );
    }

    /// GeneralSaveClicked is blocked when the refresh interval is out of the
    /// valid range (1–30 seconds).
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
}
