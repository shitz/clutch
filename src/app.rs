//! Application-level state, screen routing, and the top-level Elm loop.
//!
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

use iced::{Element, Subscription, Task};

use crate::screens::connection::ConnectionScreen;
#[allow(unused_imports)]
use crate::screens::main_screen::MainScreen;

// ── Screen ────────────────────────────────────────────────────────────────────

/// Top-level screen router.
///
/// Holds exactly one screen at a time, making illegal UI states
/// unrepresentable. The compiler prevents accessing torrent data while the
/// connection form is visible, and vice versa.
#[derive(Debug)]
pub enum Screen {
    /// The initial connection form. Shown on startup and after a failed connect.
    Connection(ConnectionScreen),
    /// The main torrent list. Shown after a successful connection.
    Main(MainScreen),
}

// ── Message ───────────────────────────────────────────────────────────────────

/// Every event that can occur in the application.
///
/// Messages flow from the `view` function (user interactions) or from completed
/// async `Task`s (RPC results, timer ticks) into `update`.
#[derive(Debug, Clone)]
pub enum Message {
    // -- Connection screen events --
    /// User edited the Host field.
    HostChanged(String),
    /// User edited the Port field.
    PortChanged(String),
    /// User edited the Username field.
    UsernameChanged(String),
    /// User edited the Password field.
    PasswordChanged(String),
    /// User clicked the Connect button.
    ConnectClicked,
    /// Result of the `session-get` connectivity probe initiated by `ConnectClicked`.
    SessionProbeResult(Result<crate::rpc::SessionInfo, String>),

    // -- Main screen events --
    /// Fired by the polling subscription every 5 seconds.
    Tick,
    /// Result of a `torrent-get` RPC call. `Ok` carries the fresh torrent list;
    /// `Err` carries a human-readable error string.
    TorrentsUpdated(Result<Vec<crate::rpc::TorrentData>, String>),
    /// The daemon returned 409, meaning the session ID has rotated.
    /// The `String` is the new session ID. Re-fires the pending `torrent-get`.
    SessionIdRotated(String),

    // -- Main screen action events (v0.2) --
    /// User clicked a torrent row — toggles selection.
    TorrentSelected(i64),
    /// User clicked the Pause toolbar button.
    PauseClicked,
    /// User clicked the Resume toolbar button.
    ResumeClicked,
    /// User clicked the Delete toolbar button — opens the confirmation row.
    DeleteClicked,
    /// User toggled the "Delete local data" checkbox in the confirmation row.
    DeleteLocalDataToggled(bool),
    /// User confirmed the delete action.
    DeleteConfirmed,
    /// User cancelled the delete action.
    DeleteCancelled,
    /// Result of a `torrent-start`, `torrent-stop`, or `torrent-remove` call.
    ActionCompleted(Result<(), String>),

    // -- Main screen add-torrent events (v0.3) --
    /// User clicked the "Add Torrent" button — opens the native file picker.
    AddTorrentClicked,
    /// Result of opening + reading + parsing a `.torrent` file inside `Task::perform`.
    TorrentFileRead(Result<crate::screens::main_screen::FileReadResult, String>),
    /// User clicked the "Add Link" button — opens the magnet-link dialog.
    AddLinkClicked,
    /// User typed in the magnet URI field of the add dialog.
    AddDialogMagnetChanged(String),
    /// User edited the destination folder field of the add dialog.
    AddDialogDestinationChanged(String),
    /// User confirmed the add-torrent dialog.
    AddConfirmed,
    /// User cancelled the add-torrent dialog.
    AddCancelled,
    /// Result of a `torrent-add` RPC call.
    AddCompleted(Result<(), String>),

    /// The serialized RPC worker subscription has started and is ready.
    /// Store the sender and use it for all subsequent RPC calls.
    RpcWorkerReady(tokio::sync::mpsc::Sender<crate::rpc::RpcWork>),

    /// User clicked the Disconnect button on the main screen.
    /// Returns the app to the connection form.
    Disconnect,
}

// ── App state ─────────────────────────────────────────────────────────────────

/// Root application state.
///
/// Delegates all rendering and update logic to the active [`Screen`].
#[derive(Debug)]
pub struct AppState {
    /// The currently visible screen.
    pub screen: Screen,
}

impl AppState {
    /// Create the initial app state, showing the connection form.
    pub fn new() -> Self {
        AppState {
            screen: Screen::Connection(ConnectionScreen::new()),
        }
    }
}

// ── Elm functions ─────────────────────────────────────────────────────────────

/// Update application state in response to a message.
///
/// # Non-blocking invariant
///
/// This function **must return in microseconds**. Every branch either
/// mutates in-memory state or delegates to a [`Screen`] method that itself
/// returns immediately. All async work is encapsulated in the returned `Task`.
pub fn update(state: &mut AppState, message: Message) -> Task<Message> {
    // Disconnect is screen-agnostic; handle it before dispatching.
    if let Message::Disconnect = &message {
        tracing::info!("Disconnecting from daemon, returning to connection screen");
        state.screen = Screen::Connection(ConnectionScreen::new());
        return Task::none();
    }

    match &mut state.screen {
        Screen::Connection(conn) => {
            let (task, next_screen) = conn.update(message);
            if let Some(next) = next_screen {
                state.screen = next;
            }
            task
        }
        Screen::Main(main) => main.update(message),
    }
}

/// Render the current screen.
pub fn view(state: &AppState) -> Element<'_, Message> {
    match &state.screen {
        Screen::Connection(conn) => conn.view(),
        Screen::Main(main) => main.view(),
    }
}

/// Return active subscriptions.
///
/// Only the main screen subscribes to the polling timer; the connection screen
/// has no background activity.
pub fn subscription(state: &AppState) -> Subscription<Message> {
    match &state.screen {
        Screen::Connection(_) => Subscription::none(),
        Screen::Main(main) => main.subscription(),
    }
}
