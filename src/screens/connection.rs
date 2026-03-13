//! Connection screen — the first screen shown on startup.
//!
//! Presents a form for entering the Transmission daemon address and optional
//! credentials. On "Connect", a lightweight `session-get` probe is fired. On
//! success the app transitions to [`crate::screens::main_screen::MainScreen`].
//! On failure the form is re-displayed with an inline error message.
//!
//! # State machine
//!
//! ```text
//! Idle ──[ConnectClicked]──▶ Connecting
//!                                 │
//!             ┌───────────────────┴──────────────────┐
//!             ▼                                       ▼
//!        SessionProbeResult(Ok)            SessionProbeResult(Err)
//!             │                                       │
//!             ▼                                       ▼
//!      → Screen::Main                       Idle (error shown)
//! ```

use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Length, Task};

use crate::app::{Message, Screen};
use crate::rpc::TransmissionCredentials;

/// State for the connection input form.
#[derive(Debug, Clone)]
pub struct ConnectionScreen {
    /// Daemon hostname or IP address. Defaults to `"localhost"`.
    pub host: String,
    /// Daemon RPC port as a string (validated on connect). Defaults to `"9091"`.
    pub port: String,
    /// Optional Basic Auth username.
    pub username: String,
    /// Optional Basic Auth password (masked in the UI).
    pub password: String,
    /// `true` while the `session-get` probe is in-flight. Disables the button.
    pub is_connecting: bool,
    /// Human-readable error from the last failed connection attempt, if any.
    pub error: Option<String>,
}

impl ConnectionScreen {
    /// Create a new connection form pre-filled with default values.
    pub fn new() -> Self {
        ConnectionScreen {
            host: "localhost".to_owned(),
            port: "9091".to_owned(),
            username: String::new(),
            password: String::new(),
            is_connecting: false,
            error: None,
        }
    }

    /// Build the credentials struct from the current form values.
    ///
    /// Returns `None` if the port field cannot be parsed as a `u16`.
    pub fn credentials(&self) -> Option<TransmissionCredentials> {
        let port: u16 = self.port.parse().ok()?;
        Some(TransmissionCredentials {
            host: self.host.clone(),
            port,
            username: if self.username.is_empty() { None } else { Some(self.username.clone()) },
            password: if self.password.is_empty() { None } else { Some(self.password.clone()) },
        })
    }

    /// Render the connection form.
    ///
    /// Produces text inputs for host, port, username, and password; a Connect
    /// button (disabled while `is_connecting`); and an inline error label.
    pub fn view(&self) -> Element<'_, Message> {
        let connect_btn: Element<Message> = if self.is_connecting {
            button("Connecting…").into()
        } else {
            button("Connect").on_press(Message::ConnectClicked).into()
        };

        let error_label: Element<Message> = if let Some(err) = &self.error {
            text(format!("⚠ {err}")).into()
        } else {
            text("").into()
        };

        let form = column![
            text("Connect to Transmission").size(20),
            text_input("Host", &self.host)
                .on_input(Message::HostChanged),
            text_input("Port", &self.port)
                .on_input(Message::PortChanged),
            text_input("Username (optional)", &self.username)
                .on_input(Message::UsernameChanged),
            text_input("Password (optional)", &self.password)
                .on_input(Message::PasswordChanged)
                .secure(true),
            row![connect_btn],
            error_label,
        ]
        .spacing(10)
        .max_width(400);

        container(form)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }

    /// Handle a message directed at the connection screen.
    ///
    /// Returns a `(Task, Option<Screen>)` pair. If `Option<Screen>` is `Some`,
    /// the caller must replace the active screen. Returns in microseconds —
    /// async work is in the returned `Task`.
    pub fn update(&mut self, message: Message) -> (Task<Message>, Option<Screen>) {
        match message {
            Message::HostChanged(v) => { self.host = v; (Task::none(), None) }
            Message::PortChanged(v) => { self.port = v; (Task::none(), None) }
            Message::UsernameChanged(v) => { self.username = v; (Task::none(), None) }
            Message::PasswordChanged(v) => { self.password = v; (Task::none(), None) }

            Message::ConnectClicked => {
                let Some(creds) = self.credentials() else {
                    self.error = Some("Invalid port number.".to_owned());
                    return (Task::none(), None);
                };
                self.is_connecting = true;
                self.error = None;
                let url = creds.rpc_url();
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

            Message::SessionProbeResult(Ok(info)) => {
                let creds = self.credentials().expect("credentials valid after probe");
                let next_screen = Screen::Main(
                    crate::screens::main_screen::MainScreen::new(creds, info.session_id),
                );
                (Task::none(), Some(next_screen))
            }

            Message::SessionProbeResult(Err(err)) => {
                eprintln!("Connection failed: {err}");
                self.is_connecting = false;
                self.error = Some(err);
                (Task::none(), None)
            }

            // Main-screen messages are not handled here.
            _ => (Task::none(), None),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// 6.6 – ConnectClicked sets is_connecting and clears any previous error.
    #[test]
    fn connect_clicked_sets_connecting_and_clears_error() {
        let mut conn = ConnectionScreen::new();
        conn.error = Some("previous error".to_owned());

        let (_, next) = conn.update(Message::ConnectClicked);

        assert!(conn.is_connecting);
        assert!(conn.error.is_none());
        assert!(next.is_none()); // still on connection screen
    }

    /// 6.7 – SessionProbeResult(Err) clears is_connecting and populates error.
    #[test]
    fn probe_failure_resets_connecting_and_sets_error() {
        let mut conn = ConnectionScreen::new();
        conn.is_connecting = true;

        let (_, next) = conn.update(
            Message::SessionProbeResult(Err("connection refused".to_owned())),
        );

        assert!(!conn.is_connecting);
        assert_eq!(conn.error.as_deref(), Some("connection refused"));
        assert!(next.is_none());
    }
}
