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

//! # Architecture
//!
//! The app follows `iced`'s Elm architecture with free-function style (iced 0.14):
//!
//! ```text
//! view(&State) → Element → user interaction → Message
//!                                                  ↓
//!                                    update(&mut State, Message)
//!                                                  ↓
//!                                          Task<Message>
//! ```
//!
//! # Non-blocking invariant
//!
//! **`update()` must return in microseconds.** All I/O (RPC calls, file I/O)
//! must be performed inside `iced::Task::perform()`. The returned `Task`
//! is executed by the tokio runtime on a background thread; the result arrives
//! back as a new `Message`. Violating this invariant will freeze the UI.

//! Split into private helpers:
//! - [`routing`] — startup flow, global intercepts, and connection bridging
//! - [`settings_bridge`] — reconciliation of settings results back into `AppState`
//! - [`keyboard`] — screen-specific keyboard subscriptions

mod keyboard;
mod routing;
mod settings_bridge;

use uuid::Uuid;

use iced::{Element, Subscription, Task, Theme};
use secrecy::SecretString;

use crate::auth::AuthDialog;
use crate::profile::{ProfileStore, resolve_theme_config};
use crate::screens::connection::{self, ConnectionScreen};
use crate::screens::main_screen::{self, MainScreen};
use crate::screens::settings::{self, SettingsScreen};

// ── Screen ────────────────────────────────────────────────────────────────────

/// Top-level screen router.
///
/// Holds exactly one screen at a time, making illegal UI states unrepresentable.
#[derive(Debug)]
pub enum Screen {
    /// The initial connection form. Shown on startup and after Disconnect.
    Connection(ConnectionScreen),
    /// The main torrent list. Shown after a successful connection.
    Main(Box<MainScreen>),
    /// The full-screen settings / profiles editor.
    Settings(SettingsScreen),
}

// ── Message ───────────────────────────────────────────────────────────────────

/// Every event that can occur in the application.
#[derive(Debug, Clone)]
pub enum Message {
    // -- Startup --
    /// Config file loaded from disk.
    ProfilesLoaded(ProfileStore),
    /// Result of the auto-connect probe fired after `ProfilesLoaded`.
    AutoConnectResult(Result<crate::rpc::SessionData, String>),

    // -- Connection screen (delegated) --
    Connection(connection::Message),

    // -- Main screen (delegated) --
    Main(main_screen::Message),

    // -- Settings screen (delegated) --
    Settings(settings::Message),

    // -- Auth dialog --
    AuthSetupPassphraseChanged(String),
    AuthSetupConfirmChanged(String),
    AuthUnlockPassphraseChanged(String),
    SubmitSetupPassphrase,
    SubmitUnlockPassphrase,
    DismissAuthDialog,
    /// Tab pressed while an auth dialog is active — cycles within the dialog.
    AuthTabKeyPressed {
        shift: bool,
    },
    /// Enter pressed while an auth dialog is active — triggers primary CTA.
    AuthEnterPressed,
    /// Returned when the async passphrase hash + encryption task completes.
    SetupPassphraseComplete {
        passphrase: String,
        hash: String,
        profile_id: Uuid,
        encrypted_password: String,
    },
    /// Returned when the async passphrase verify task completes.
    UnlockPassphraseResult {
        passphrase: String,
        valid: bool,
    },
    /// Returned when an async encrypt-only task completes (already-unlocked save).
    EncryptPasswordReady {
        profile_id: Uuid,
        encrypted_password: String,
    },

    /// Fire-and-forget: result of a background save; no state change needed.
    Noop,

    /// A download directory was used when adding a torrent.
    /// Updates the active profile's `recent_download_paths` and persists to disk.
    ProfilePathUsed(String),

    // -- Tray --
    /// An action dispatched from the native system tray context menu or icon click.
    TrayAction(crate::tray::TrayAction),
    /// The main window's close button was clicked. We intercept it to hide
    /// instead of exit so the app continues running in the system tray.
    WindowCloseRequested(iced::window::Id),
}

// ── Theme mode ────────────────────────────────────────────────────────────────

/// Resolved display theme — always `Light` or `Dark`.
///
/// `System` preference is in [`crate::profile::ThemeConfig`] and gets resolved
/// at startup and on-demand. Only the concrete resolved value is stored here.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
}

// ── App state ─────────────────────────────────────────────────────────────────

/// Root application state.
#[derive(Debug)]
pub struct AppState {
    pub screen: Screen,
    /// Resolved display theme (Light or Dark).
    pub theme: ThemeMode,
    /// Persistent profiles and general settings.
    pub profiles: ProfileStore,
    /// UUID of the currently connected profile.
    pub active_profile: Option<Uuid>,
    /// Stashed main screen while Settings is open, so we can restore it
    /// without a reconnect/refetch when the user closes Settings.
    pub stashed_main: Option<Box<MainScreen>>,
    /// Master passphrase held in memory for the session (never written to disk).
    /// Wrapped in `SecretString` so the memory is zeroized on drop.
    pub unlocked_passphrase: Option<SecretString>,
    /// Active auth dialog, rendered as a modal overlay over the current screen.
    pub active_dialog: Option<AuthDialog>,
    /// Whether Turtle Mode (alternative speed limits) is currently active on the daemon.
    pub alt_speed_enabled: bool,
    /// Owned system tray icon and mutable menu-item handles.
    /// `None` on platforms where tray creation fails (e.g. bare Linux).
    pub tray: Option<crate::tray::TrayState>,
    /// ID of the main application window, captured on the first window event.
    /// Used to hide and restore the window when the user interacts with the tray.
    pub main_window_id: Option<iced::window::Id>,
}

impl AppState {
    /// Create the initial app state.
    ///
    /// Returns `(state, task)` — the task loads the profile store from disk.
    pub fn new() -> (Self, Task<Message>) {
        // Load synchronously so the correct theme is applied from the very first
        // draw — before the async ProfilesLoaded message can arrive.
        let initial = crate::profile::ProfileStore::load_sync();
        let theme = resolve_theme_config(initial.general.theme);
        // muda (the tray-icon menu backend) panics if its `Menu` is constructed
        // on a non-main thread. Tests run on worker threads, so skip tray init.
        #[cfg(not(test))]
        let tray = crate::tray::build();
        #[cfg(test)]
        let tray = None;
        let state = AppState {
            screen: Screen::Connection(ConnectionScreen::default()),
            theme,
            profiles: initial.clone(),
            active_profile: None,
            stashed_main: None,
            unlocked_passphrase: None,
            active_dialog: None,
            alt_speed_enabled: false,
            tray,
            main_window_id: None,
        };
        // Re-emit the already-loaded store via Task::done so ProfilesLoaded
        // runs on the first event-loop tick (auto-connect, launchpad rebuild)
        // without hitting disk a second time.
        let task = Task::done(Message::ProfilesLoaded(initial));
        (state, task)
    }

    /// Return the active `iced::Theme` for the `.theme()` application callback.
    pub fn current_theme(&self) -> Theme {
        crate::theme::clutch_theme(self.theme == ThemeMode::Dark)
    }
}

// ── Elm functions ─────────────────────────────────────────────────────────────

pub fn update(state: &mut AppState, message: Message) -> Task<Message> {
    if let Message::Noop = message {
        return Task::none();
    }

    if let Some(task) = routing::handle_startup_message(state, &message) {
        return task;
    }

    if let Some(task) = routing::handle_global_message(state, &message) {
        return task;
    }

    if let Some(task) = crate::auth::handle_message(state, &message) {
        // Keep the settings screen's cached hash in sync (e.g. after first-time
        // passphrase setup while the settings screen is open).
        if let Screen::Settings(settings) = &mut state.screen {
            settings.master_passphrase_hash = state.profiles.master_passphrase_hash.clone();
        }
        return task;
    }

    if matches!(&state.screen, Screen::Connection(_)) {
        return routing::handle_connection_message(state, message);
    }

    if matches!(&state.screen, Screen::Settings(_)) {
        return settings_bridge::handle_message(state, message);
    }

    match (&mut state.screen, message) {
        (Screen::Main(main), Message::Main(msg)) => main.update(msg).map(Message::Main),
        _ => Task::none(),
    }
}

/// Render the current screen, wrapping it in the auth dialog overlay if needed.
pub fn view(state: &AppState) -> Element<'_, Message> {
    let base: Element<'_, Message> = match &state.screen {
        Screen::Connection(conn) => conn.view().map(Message::Connection),
        Screen::Main(main) => main
            .view(state.theme, state.alt_speed_enabled)
            .map(Message::Main),
        Screen::Settings(settings) => settings.view().map(Message::Settings),
    };
    crate::auth::view_overlay(state.active_dialog.as_ref(), base)
}

/// Return active subscriptions.
pub fn subscription(state: &AppState) -> Subscription<Message> {
    let keyboard = keyboard::subscription(state);

    // Listen for the window close button so we can hide to tray instead of exit.
    let window_events = iced::window::close_requests().map(Message::WindowCloseRequested);

    if state.tray.is_some() {
        Subscription::batch([keyboard, window_events, crate::tray::subscription()])
    } else {
        Subscription::batch([keyboard, window_events])
    }
}
