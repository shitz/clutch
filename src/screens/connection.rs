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

//! Shows two tabs:
//! - **Saved Profiles** (default when profiles exist): clickable profile cards
//!   plus a "Manage / Add Profile…" button that opens Settings > Connections.
//! - **Quick Connect** (default when no profiles exist): a one-time ephemeral
//!   connection form. Credentials are held in memory only — nothing is saved to
//!   disk or encrypted in the config file.

use iced::widget::{Space, button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Element, Length, Task};
use uuid::Uuid;

use crate::profile::ConnectionProfile;
use crate::rpc::TransmissionCredentials;

// ── Tab state ─────────────────────────────────────────────────────────────────

/// Which tab is shown on the connection screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionTab {
    SavedProfiles,
    QuickConnect,
}

// ── Connect result ────────────────────────────────────────────────────────────

/// Returned by `update()` when a session probe succeeds.
///
/// Carries everything `app::update` needs to build `Screen::Main`.
#[derive(Debug, Clone)]
pub struct ConnectSuccess {
    /// `Some` if a saved profile was used (its `last_connected` should be set).
    /// `None` for an ephemeral quick-connect (nothing is persisted).
    pub profile_id: Option<Uuid>,
    pub creds: TransmissionCredentials,
    pub session_id: String,
}

// ── Message ───────────────────────────────────────────────────────────────────

/// Messages handled by the connection screen.
#[derive(Debug, Clone)]
pub enum Message {
    /// Tab change on the launchpad.
    TabSelected(ConnectionTab),
    /// User clicked a saved profile card.
    ProfileSelected(Uuid),
    /// User clicked Connect in the saved profiles action bar.
    ConnectProfile(Uuid),
    /// Quick-connect form field changes.
    HostChanged(String),
    PortChanged(String),
    UsernameChanged(String),
    PasswordChanged(String),
    /// Quick-connect "Connect" button.
    ConnectClicked,
    /// Result of a connection probe.
    ProbeResult(Result<crate::rpc::SessionInfo, String>),
    /// User clicked "Manage / Add Profile" on the launchpad.
    ManageProfilesClicked,
    /// Initiate a probe using pre-built credentials (set by app-level intercept).
    /// Bypasses credential lookup in the connection screen.
    ConnectWithCreds { profile_id: Uuid, creds: TransmissionCredentials },
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ConnectionScreen {
    /// Which tab is active.
    pub tab: ConnectionTab,
    /// Snapshot of saved profiles shown in the Saved Profiles tab.
    /// Refreshed each time the screen is (re-)constructed.
    pub profiles: Vec<ConnectionProfile>,

    // Quick-connect form fields.
    pub qc_host: String,
    pub qc_port: String,
    pub qc_username: String,
    pub qc_password: String,

    /// `true` while any probe is in-flight.
    pub is_connecting: bool,
    /// UUID of the saved profile currently being probed (None = quick connect).
    pub connecting_profile_id: Option<Uuid>,
    /// Credentials for the in-flight probe — used to build `ConnectSuccess`.
    connecting_creds: Option<TransmissionCredentials>,
    /// UUID of the saved profile highlighted in the list (not yet connecting).
    pub selected_profile_id: Option<Uuid>,

    /// Pre-decoded logo image handle — created once to avoid per-frame re-decode.
    pub logo_handle: iced::widget::image::Handle,

    /// Error from the most recent failed probe.
    pub error: Option<String>,
}

impl ConnectionScreen {
    /// Create the launchpad pre-loaded with saved profiles.
    ///
    /// Opens **Saved Profiles** tab when profiles exist, **Quick Connect** otherwise.
    pub fn new_launchpad(profiles: &[ConnectionProfile]) -> Self {
        let tab = if profiles.is_empty() {
            ConnectionTab::QuickConnect
        } else {
            ConnectionTab::SavedProfiles
        };
        ConnectionScreen {
            tab,
            selected_profile_id: profiles.first().map(|p| p.id),
            profiles: profiles.to_vec(),
            qc_host: "localhost".to_owned(),
            qc_port: "9091".to_owned(),
            qc_username: String::new(),
            qc_password: String::new(),
            is_connecting: false,
            connecting_profile_id: None,
            connecting_creds: None,
            logo_handle: iced::widget::image::Handle::from_bytes(crate::theme::LOGO_BYTES),
            error: None,
        }
    }

    fn qc_credentials(&self) -> Option<TransmissionCredentials> {
        let port: u16 = self.qc_port.parse().ok()?;
        Some(TransmissionCredentials {
            host: self.qc_host.clone(),
            port,
            username: if self.qc_username.is_empty() {
                None
            } else {
                Some(self.qc_username.clone())
            },
            password: if self.qc_password.is_empty() {
                None
            } else {
                Some(self.qc_password.clone())
            },
        })
    }

    // ── View ─────────────────────────────────────────────────────────────────

    pub fn view(&self) -> Element<'_, Message> {
        // Tab bar — M3 segmented control
        let tab_bar = crate::theme::segmented_control(
            &[
                ("Saved Profiles", ConnectionTab::SavedProfiles),
                ("Quick Connect", ConnectionTab::QuickConnect),
            ],
            self.tab,
            Message::TabSelected,
            true,
            false,
        );

        let content: Element<'_, Message> = match self.tab {
            ConnectionTab::SavedProfiles => self.view_saved_profiles(),
            ConnectionTab::QuickConnect => self.view_quick_connect(),
        };

        let error_row: Element<'_, Message> = if let Some(err) = &self.error {
            text(format!("\u{26a0} {err}"))
                .style(|t: &iced::Theme| iced::widget::text::Style {
                    color: Some(t.extended_palette().danger.base.color),
                })
                .into()
        } else {
            Space::new().into()
        };

        let tab_bar = container(container(tab_bar).width(Length::Fixed(380.0)))
            .width(Length::Fill)
            .center_x(Length::Fill);

        let panel = column![
            iced::widget::image(self.logo_handle.clone())
                .width(Length::Fixed(220.0))
                .content_fit(iced::ContentFit::ScaleDown),
            tab_bar,
            content,
            error_row,
        ]
        .spacing(16)
        .align_x(iced::Alignment::Center)
        .max_width(440);

        container(
            column![
                Space::new().height(Length::Fixed(120.0)),
                container(panel).width(Length::Fill).center_x(Length::Fill),
            ]
            .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn view_saved_profiles(&self) -> Element<'_, Message> {
        let manage_btn = button(
            row![
                crate::theme::icon(crate::theme::ICON_SETTINGS),
                text("Manage Profiles").size(14),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .on_press(Message::ManageProfilesClicked)
        .padding([10, 20])
        .style(crate::theme::m3_tonal_button);

        if self.profiles.is_empty() {
            return column![
                text("No saved profiles yet.").style(|t: &iced::Theme| {
                    iced::widget::text::Style {
                        color: Some(t.palette().text.scale_alpha(0.55)),
                    }
                }),
                manage_btn,
            ]
            .spacing(12)
            .into();
        }

        let profile_cards: Vec<Element<'_, Message>> = self
            .profiles
            .iter()
            .map(|p| {
                let is_selected = self.selected_profile_id == Some(p.id);
                let is_connecting_this = self.connecting_profile_id == Some(p.id);
                let id = p.id;
                let name = if is_connecting_this {
                    "Connecting\u{2026}".to_owned()
                } else {
                    p.name.clone()
                };
                let subtitle = format!("{}:{}", p.host, p.port);
                let card = button(
                    column![
                        text(name).size(14),
                        text(subtitle).size(12).style(|t: &iced::Theme| {
                            iced::widget::text::Style {
                                color: Some(t.palette().text.scale_alpha(0.55)),
                            }
                        }),
                    ]
                    .spacing(3),
                )
                .width(Length::Fill)
                .padding([12, 16])
                .on_press(Message::ProfileSelected(id))
                .style(move |t: &iced::Theme, _status| {
                    let is_dark = t.extended_palette().background.base.color.r < 0.5;
                    let primary = t.palette().primary;
                    if is_selected {
                        button::Style {
                            background: Some(iced::Background::Color(iced::Color {
                                a: 0.22,
                                ..primary
                            })),
                            text_color: t.palette().text,
                            border: iced::Border {
                                color: primary,
                                width: 1.0,
                                radius: 10.0.into(),
                            },
                            shadow: iced::Shadow::default(),
                            snap: false,
                        }
                    } else {
                        let bg = if is_dark {
                            crate::theme::CARD_SURFACE_DARK
                        } else {
                            crate::theme::CARD_SURFACE_LIGHT
                        };
                        button::Style {
                            background: Some(iced::Background::Color(bg)),
                            text_color: t.palette().text,
                            border: iced::Border {
                                color: iced::Color::TRANSPARENT,
                                width: 1.0,
                                radius: 10.0.into(),
                            },
                            shadow: iced::Shadow::default(),
                            snap: false,
                        }
                    }
                });
                card.into()
            })
            .collect();

        let list = container(scrollable(column(profile_cards).spacing(8)).height(Length::Shrink))
            .max_height(300.0);

        let connect_btn: Element<'_, Message> = if self.is_connecting {
            button(
                row![
                    crate::theme::icon(crate::theme::ICON_PLAY),
                    text("Connecting\u{2026}").size(14),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([10, 20])
            .style(crate::theme::m3_primary_button)
            .into()
        } else {
            let b = button(
                row![
                    crate::theme::icon(crate::theme::ICON_PLAY),
                    text("Connect").size(14),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([10, 20])
            .style(crate::theme::m3_primary_button);
            if let Some(profile_id) = self.selected_profile_id {
                b.on_press(Message::ConnectProfile(profile_id)).into()
            } else {
                b.into()
            }
        };

        let action_bar = row![manage_btn, Space::new().width(Length::Fill), connect_btn,]
            .width(Length::Fill)
            .align_y(iced::Alignment::Center);

        column![list, Space::new().height(Length::Fixed(16.0)), action_bar,].into()
    }

    fn view_quick_connect(&self) -> Element<'_, Message> {
        let connecting_quick = self.is_connecting && self.connecting_profile_id.is_none();

        let connect_btn: Element<'_, Message> = if connecting_quick {
            button(
                row![
                    crate::theme::icon(crate::theme::ICON_PLAY),
                    text("Connecting\u{2026}").size(14),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([10, 20])
            .style(crate::theme::m3_primary_button)
            .into()
        } else {
            let b = button(
                row![
                    crate::theme::icon(crate::theme::ICON_PLAY),
                    text("Connect").size(14),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .padding([10, 20])
            .style(crate::theme::m3_primary_button);
            if self.is_connecting {
                b.into()
            } else {
                b.on_press(Message::ConnectClicked).into()
            }
        };

        let action_bar = row![Space::new().width(Length::Fill), connect_btn,]
            .width(Length::Fill)
            .align_y(Alignment::Center);

        column![
            text_input("Host", &self.qc_host)
                .on_input(Message::HostChanged)
                .padding([12, 16])
                .style(crate::theme::m3_text_input),
            text_input("Port", &self.qc_port)
                .on_input(Message::PortChanged)
                .padding([12, 16])
                .style(crate::theme::m3_text_input),
            text_input("Username (optional)", &self.qc_username)
                .on_input(Message::UsernameChanged)
                .padding([12, 16])
                .style(crate::theme::m3_text_input),
            text_input("Password (optional)", &self.qc_password)
                .on_input(Message::PasswordChanged)
                .padding([12, 16])
                .style(crate::theme::m3_text_input)
                .secure(true),
            action_bar,
        ]
        .spacing(10)
        .into()
    }

    // ── Update ───────────────────────────────────────────────────────────────

    /// Handle a message directed at the connection screen.
    ///
    /// Returns `(Task, Option<ConnectSuccess>)`. When `ConnectSuccess` is `Some`
    /// a probe succeeded and the caller should transition to `Screen::Main`.
    pub fn update(&mut self, message: Message) -> (Task<Message>, Option<ConnectSuccess>) {
        match message {
            // ── Tab switching ─────────────────────────────────────────────────
            Message::TabSelected(tab) => {
                self.tab = tab;
                self.error = None;
                (Task::none(), None)
            }

            // ── Saved profile selection ──────────────────────────────────────
            Message::ProfileSelected(id) => {
                self.selected_profile_id = Some(id);
                self.error = None;
                (Task::none(), None)
            }

            // ── Saved profile connect ─────────────────────────────────────────
            Message::ConnectProfile(_) => {
                // Intercepted by app::update before reaching this screen —
                // it either shows the passphrase dialog or sends ConnectWithCreds.
                (Task::none(), None)
            }

            Message::ConnectWithCreds { profile_id, creds } => {
                let url = creds.rpc_url();
                self.is_connecting = true;
                self.connecting_profile_id = Some(profile_id);
                self.connecting_creds = Some(creds.clone());
                self.error = None;
                if let Some(p) = self.profiles.iter().find(|p| p.id == profile_id) {
                    tracing::info!(
                        profile = %p.name,
                        host = %creds.host,
                        "Connecting to saved profile"
                    );
                }
                let task = Task::perform(
                    async move {
                        crate::rpc::session_get(&url, &creds, "")
                            .await
                            .map_err(|e| e.to_string())
                    },
                    Message::ProbeResult,
                );
                (task, None)
            }

            // ── Quick-connect form ────────────────────────────────────────────
            Message::HostChanged(v) => {
                self.qc_host = v;
                (Task::none(), None)
            }
            Message::PortChanged(v) => {
                self.qc_port = v;
                (Task::none(), None)
            }
            Message::UsernameChanged(v) => {
                self.qc_username = v;
                (Task::none(), None)
            }
            Message::PasswordChanged(v) => {
                self.qc_password = v;
                (Task::none(), None)
            }

            Message::ConnectClicked => {
                let Some(creds) = self.qc_credentials() else {
                    self.error = Some("Invalid port number.".to_owned());
                    return (Task::none(), None);
                };
                self.is_connecting = true;
                self.connecting_profile_id = None;
                self.connecting_creds = Some(creds.clone());
                self.error = None;
                let url = creds.rpc_url();
                tracing::info!(host = %creds.host, port = creds.port, "Quick-connect attempt");
                let task = Task::perform(
                    async move {
                        crate::rpc::session_get(&url, &creds, "")
                            .await
                            .map_err(|e| e.to_string())
                    },
                    Message::ProbeResult,
                );
                (task, None)
            }

            // ── Probe results ─────────────────────────────────────────────────
            Message::ProbeResult(Ok(info)) => {
                tracing::info!(session_id = %info.session_id, "Connection probe succeeded");
                let creds = self.connecting_creds.take().unwrap_or_else(|| {
                    self.qc_credentials().unwrap_or(TransmissionCredentials {
                        host: self.qc_host.clone(),
                        port: 9091,
                        username: None,
                        password: None,
                    })
                });
                let success = ConnectSuccess {
                    profile_id: self.connecting_profile_id,
                    creds,
                    session_id: info.session_id,
                };
                self.is_connecting = false;
                self.connecting_profile_id = None;
                (Task::none(), Some(success))
            }

            Message::ProbeResult(Err(err)) => {
                tracing::error!(error = %err, "Connection probe failed");
                self.is_connecting = false;
                self.connecting_profile_id = None;
                self.connecting_creds = None;
                self.error = Some(err);
                (Task::none(), None)
            }

            Message::ManageProfilesClicked => {
                // Intercepted by app::update before reaching this screen.
                (Task::none(), None)
            }
        }
    }
}

impl Default for ConnectionScreen {
    fn default() -> Self {
        Self::new_launchpad(&[])
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn blank() -> ConnectionScreen {
        ConnectionScreen::default()
    }

    /// Default screen has QuickConnect tab (no profiles).
    #[test]
    fn default_shows_quick_connect_tab() {
        let s = blank();
        assert_eq!(s.tab, ConnectionTab::QuickConnect);
    }

    /// When profiles exist, launchpad defaults to SavedProfiles tab.
    #[test]
    fn launchpad_with_profiles_shows_saved_tab() {
        let mut p = ConnectionProfile::new_blank();
        p.name = "NAS".to_owned();
        let s = ConnectionScreen::new_launchpad(&[p]);
        assert_eq!(s.tab, ConnectionTab::SavedProfiles);
    }

    /// ConnectClicked sets is_connecting and clears error.
    #[test]
    fn connect_clicked_sets_connecting_and_clears_error() {
        let mut s = blank();
        s.error = Some("old error".to_owned());
        let (_, next) = s.update(Message::ConnectClicked);
        assert!(s.is_connecting);
        assert!(s.error.is_none());
        assert!(next.is_none());
    }

    /// SessionProbeResult(Err) resets state and sets error.
    #[test]
    fn probe_failure_resets_state_and_sets_error() {
        let mut s = blank();
        s.is_connecting = true;
        let (_, next) = s.update(Message::ProbeResult(Err("connection refused".to_owned())));
        assert!(!s.is_connecting);
        assert_eq!(s.error.as_deref(), Some("connection refused"));
        assert!(next.is_none());
    }

    /// Tab switching clears error.
    #[test]
    fn tab_switch_clears_error() {
        let mut s = blank();
        s.error = Some("old error".to_owned());
        let _ = s.update(Message::TabSelected(ConnectionTab::SavedProfiles));
        assert!(s.error.is_none());
        assert_eq!(s.tab, ConnectionTab::SavedProfiles);
    }
}
