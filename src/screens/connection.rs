//! Connection screen — the startup launchpad.
//!
//! Shows two tabs:
//! - **Saved Profiles** (default when profiles exist): clickable profile cards
//!   plus a "Manage / Add Profile…" button that opens Settings > Connections.
//! - **Quick Connect** (default when no profiles exist): a one-time ephemeral
//!   connection form. Credentials are held in memory only — nothing is saved to
//!   disk or the OS keyring.

use iced::widget::rule;
use iced::widget::{Space, button, column, container, row, text, text_input};
use iced::{Element, Length, Task};
use uuid::Uuid;

use crate::app::Message;
use crate::profile::ConnectionProfile;
use crate::rpc::TransmissionCredentials;
use crate::theme::{tab_active, tab_inactive, tab_underline};

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
            profiles: profiles.to_vec(),
            qc_host: "localhost".to_owned(),
            qc_port: "9091".to_owned(),
            qc_username: String::new(),
            qc_password: String::new(),
            is_connecting: false,
            connecting_profile_id: None,
            connecting_creds: None,
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
        // Tab bar
        let saved_btn = button(text("Saved Profiles").size(14))
            .style(if self.tab == ConnectionTab::SavedProfiles {
                tab_active
            } else {
                tab_inactive
            })
            .padding([6, 16])
            .on_press(Message::ConnectionTabSelected(ConnectionTab::SavedProfiles));

        let quick_btn = button(text("Quick Connect").size(14))
            .style(if self.tab == ConnectionTab::QuickConnect {
                tab_active
            } else {
                tab_inactive
            })
            .padding([6, 16])
            .on_press(Message::ConnectionTabSelected(ConnectionTab::QuickConnect));

        let underline_saved = if self.tab == ConnectionTab::SavedProfiles {
            container(Space::new())
                .width(Length::Fill)
                .height(2)
                .style(tab_underline)
        } else {
            container(Space::new()).width(Length::Fill).height(2)
        };

        let underline_quick = if self.tab == ConnectionTab::QuickConnect {
            container(Space::new())
                .width(Length::Fill)
                .height(2)
                .style(tab_underline)
        } else {
            container(Space::new()).width(Length::Fill).height(2)
        };

        let tab_bar = column![
            row![
                column![saved_btn, underline_saved].width(Length::Shrink),
                column![quick_btn, underline_quick].width(Length::Shrink),
                Space::new().width(Length::Fill),
            ],
            rule::horizontal(1),
        ]
        .spacing(0);

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

        let panel = column![
            text("Connect to Transmission").size(22),
            tab_bar,
            content,
            error_row,
        ]
        .spacing(16)
        .max_width(440);

        container(panel)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }

    fn view_saved_profiles(&self) -> Element<'_, Message> {
        if self.profiles.is_empty() {
            return column![
                text("No saved profiles yet.").style(|t: &iced::Theme| {
                    iced::widget::text::Style {
                        color: Some(t.palette().text.scale_alpha(0.55)),
                    }
                }),
                button(text("\u{2699} Manage / Add Profile\u{2026}"))
                    .on_press(Message::ManageProfilesClicked)
                    .style(iced::widget::button::secondary),
            ]
            .spacing(12)
            .into();
        }

        let profile_cards: Vec<Element<'_, Message>> = self
            .profiles
            .iter()
            .map(|p| {
                let is_this = self.connecting_profile_id == Some(p.id);
                let label = if is_this {
                    "Connecting\u{2026}".to_owned()
                } else {
                    format!("{}  \u{2014}  {}:{}", p.name, p.host, p.port)
                };
                let btn = button(text(label)).width(Length::Fill).padding([10, 16]);
                if self.is_connecting {
                    btn.into()
                } else {
                    btn.on_press(Message::ConnectProfile(p.id)).into()
                }
            })
            .collect();

        let mut col = column(profile_cards).spacing(6);
        col = col.push(Space::new().height(4));
        col = col.push(rule::horizontal(1));
        col = col.push(Space::new().height(4));
        col = col.push(
            button(text("\u{2699} Manage / Add Profile\u{2026}"))
                .on_press(Message::ManageProfilesClicked)
                .style(iced::widget::button::text),
        );
        col.into()
    }

    fn view_quick_connect(&self) -> Element<'_, Message> {
        let connecting_quick = self.is_connecting && self.connecting_profile_id.is_none();

        let connect_btn: Element<'_, Message> = if connecting_quick {
            button("Connecting\u{2026}").padding([8, 24]).into()
        } else {
            let b = button("Connect").padding([8, 24]);
            if self.is_connecting {
                b.into()
            } else {
                b.on_press(Message::ConnectClicked).into()
            }
        };

        column![
            text_input("Host", &self.qc_host).on_input(Message::HostChanged),
            text_input("Port", &self.qc_port).on_input(Message::PortChanged),
            text_input("Username (optional)", &self.qc_username).on_input(Message::UsernameChanged),
            text_input("Password (optional)", &self.qc_password)
                .on_input(Message::PasswordChanged)
                .secure(true),
            connect_btn,
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
            Message::ConnectionTabSelected(tab) => {
                self.tab = tab;
                self.error = None;
                (Task::none(), None)
            }

            // ── Saved profile connect ─────────────────────────────────────────
            Message::ConnectProfile(id) => {
                let Some(profile) = self.profiles.iter().find(|p| p.id == id) else {
                    return (Task::none(), None);
                };
                let creds = profile.credentials();
                let url = creds.rpc_url();
                self.is_connecting = true;
                self.connecting_profile_id = Some(id);
                self.connecting_creds = Some(creds.clone());
                self.error = None;
                tracing::info!(
                    profile = %profile.name,
                    host = %creds.host,
                    "Connecting to saved profile"
                );
                let task = Task::perform(
                    async move {
                        crate::rpc::session_get(&url, &creds, "")
                            .await
                            .map_err(|e| e.to_string())
                    },
                    Message::SessionProbeResult,
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
                    Message::SessionProbeResult,
                );
                (task, None)
            }

            // ── Probe results ─────────────────────────────────────────────────
            Message::SessionProbeResult(Ok(info)) => {
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

            Message::SessionProbeResult(Err(err)) => {
                tracing::error!(error = %err, "Connection probe failed");
                self.is_connecting = false;
                self.connecting_profile_id = None;
                self.connecting_creds = None;
                self.error = Some(err);
                (Task::none(), None)
            }

            _ => (Task::none(), None),
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
        let (_, next) = s.update(Message::SessionProbeResult(Err(
            "connection refused".to_owned()
        )));
        assert!(!s.is_connecting);
        assert_eq!(s.error.as_deref(), Some("connection refused"));
        assert!(next.is_none());
    }

    /// Tab switching clears error.
    #[test]
    fn tab_switch_clears_error() {
        let mut s = blank();
        s.error = Some("old error".to_owned());
        let _ = s.update(Message::ConnectionTabSelected(ConnectionTab::SavedProfiles));
        assert!(s.error.is_none());
        assert_eq!(s.tab, ConnectionTab::SavedProfiles);
    }
}
