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

//! System tray icon, native context menu, and iced event bridge.
//!
//! # Lifecycle
//!
//! `build()` is called from `AppState::new()` on the main OS thread (before the
//! iced event loop starts). On macOS and Windows the tray is created right
//! there; on Linux it is created on a dedicated GTK thread (see `gtk_thread`).
//! On failure (e.g. missing libappindicator on Linux) it returns `None` and the
//! app continues without a tray.
//!
//! # Event bridging
//!
//! `tray_icon` delivers events via crossbeam channels. `subscription()` polls
//! both channels every 100 ms inside an `iced::stream::channel` stream and
//! yields `Message::TrayAction` values into the iced update loop.

use iced::futures::SinkExt as _;
use tray_icon::menu::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::app::Message;

// ── Menu item ID strings ──────────────────────────────────────────────────────

const ID_RESUME: &str = "clutch_resume_all";
const ID_PAUSE: &str = "clutch_pause_all";
const ID_TURTLE: &str = "clutch_turtle";
const ID_SHOW: &str = "clutch_show";
const ID_EXIT: &str = "clutch_exit";

// ── Public types ──────────────────────────────────────────────────────────────

/// Actions dispatched from the system tray into the iced `update()` loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    ResumeAll,
    PauseAll,
    ToggleTurtle,
    ShowWindow,
    Exit,
}

/// A change to the tray menu requested by the iced `update()` loop.
#[derive(Debug, Clone, PartialEq, Eq)]
enum TrayCommand {
    SetConnected(bool),
    SetTurtleActive(bool),
    SetSpeeds { down: String, up: String },
    ResetSpeeds,
}

/// Handles to the mutable tray menu items.
///
/// Calling `.set_text()` or `.set_enabled()` on these updates the native OS
/// menu immediately — no menu rebuild is required. They must only be used on
/// the thread that created the menu.
struct TrayMenuItems {
    speed_down: MenuItem,
    speed_up: MenuItem,
    resume: MenuItem,
    pause: MenuItem,
    /// The turtle mode item uses a plain `MenuItem` (not `CheckMenuItem`) so
    /// that it sits in the same gutter column as every other item. The active
    /// state is signalled by swapping the label prefix between ● (on) and ○
    /// (off) — both glyphs have identical advance-widths in proportional fonts.
    turtle_mode: MenuItem,
}

impl TrayMenuItems {
    fn apply(&self, command: TrayCommand) {
        match command {
            // Only the daemon-dependent action items; the speed labels and
            // Show / Exit items are unaffected.
            TrayCommand::SetConnected(connected) => {
                self.resume.set_enabled(connected);
                self.pause.set_enabled(connected);
                self.turtle_mode.set_enabled(connected);
            }
            TrayCommand::SetTurtleActive(active) => {
                self.turtle_mode.set_text(turtle_label(active));
            }
            TrayCommand::SetSpeeds { down, up } => {
                self.speed_down.set_text(down);
                self.speed_up.set_text(up);
            }
            TrayCommand::ResetSpeeds => {
                self.speed_down.set_text(SPEED_DOWN_IDLE);
                self.speed_up.set_text(SPEED_UP_IDLE);
            }
        }
    }
}

const SPEED_DOWN_IDLE: &str = "↓  —";
const SPEED_UP_IDLE: &str = "↑  —";

/// Turtle mode menu label: ● (U+25CF) for on and ○ (U+25CB) for off.
fn turtle_label(active: bool) -> &'static str {
    if active {
        "● Turtle Mode"
    } else {
        "○ Turtle Mode"
    }
}

/// System tray icon state.
///
/// On macOS and Windows this owns the `TrayIcon` handle; dropping it destroys
/// the OS tray icon, so this struct must be kept alive for the application
/// lifetime. On Linux the tray lives on its own GTK thread (see [`gtk_thread`])
/// and this struct only holds the channel used to update its menu.
pub struct TrayState {
    #[cfg(not(target_os = "linux"))]
    _icon: TrayIcon,
    #[cfg(not(target_os = "linux"))]
    items: TrayMenuItems,
    #[cfg(target_os = "linux")]
    commands: std::sync::mpsc::Sender<TrayCommand>,
}

impl TrayState {
    /// Enable or disable the daemon-dependent action items.
    ///
    /// Call with `true` immediately after a successful connect and `false`
    /// after disconnect.
    pub fn set_connected(&self, connected: bool) {
        self.send(TrayCommand::SetConnected(connected));
    }

    /// Update the turtle mode label to reflect the current active state.
    pub fn set_turtle_active(&self, active: bool) {
        self.send(TrayCommand::SetTurtleActive(active));
    }

    /// Show the aggregate download / upload rates (bytes per second).
    pub fn set_speeds(&self, download: i64, upload: i64) {
        self.send(TrayCommand::SetSpeeds {
            down: format!("↓  {}", crate::format::format_speed(download)),
            up: format!("↑  {}", crate::format::format_speed(upload)),
        });
    }

    /// Reset the speed labels to the idle placeholder.
    pub fn reset_speeds(&self) {
        self.send(TrayCommand::ResetSpeeds);
    }

    fn send(&self, command: TrayCommand) {
        #[cfg(not(target_os = "linux"))]
        self.items.apply(command);
        // A send only fails if the tray thread is gone, in which case there
        // is no menu left to update.
        #[cfg(target_os = "linux")]
        let _ = self.commands.send(command);
    }
}

impl std::fmt::Debug for TrayState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrayState").finish_non_exhaustive()
    }
}

// ── Construction ──────────────────────────────────────────────────────────────

/// Build the system tray icon and its native context menu.
///
/// Returns `None` on failure (e.g. no libappindicator on Linux) after logging
/// a warning. The app continues as a regular windowed app in that case.
pub fn build() -> Option<TrayState> {
    #[cfg(target_os = "linux")]
    return gtk_thread::spawn();

    #[cfg(not(target_os = "linux"))]
    build_on_current_thread().map(|(icon, items)| TrayState { _icon: icon, items })
}

/// Build the tray on the calling thread, which then owns all of its objects.
fn build_on_current_thread() -> Option<(TrayIcon, TrayMenuItems)> {
    // On Linux, the tray backend panics instead of returning an error, e.g.
    // when no (ayatana-)appindicator3 library is installed.
    std::panic::catch_unwind(try_build).unwrap_or_else(|_| {
        tracing::warn!("Failed to create system tray icon: tray backend panicked");
        None
    })
}

fn try_build() -> Option<(TrayIcon, TrayMenuItems)> {
    let icon = load_icon().or_else(|| {
        tracing::warn!("Could not decode tray icon — tray will show no icon");
        None
    });

    // Speed labels: disabled, updated on every torrent poll.
    let speed_down = MenuItem::new(SPEED_DOWN_IDLE, false, None);
    let speed_up = MenuItem::new(SPEED_UP_IDLE, false, None);

    // Action items — all plain MenuItems so every label sits in the same
    // gutter column with no OS-level checkmark offset.
    // resume / pause / turtle start DISABLED and are enabled after connect.
    let resume = MenuItem::with_id(ID_RESUME, "Resume All", false, None);
    let pause = MenuItem::with_id(ID_PAUSE, "Pause All", false, None);
    let turtle_mode = MenuItem::with_id(ID_TURTLE, turtle_label(false), false, None);
    let show = MenuItem::with_id(ID_SHOW, "Show Clutch", true, None);
    let exit = MenuItem::with_id(ID_EXIT, "Exit", true, None);

    let menu = Menu::new();
    menu.append_items(&[
        &speed_down,
        &speed_up,
        &PredefinedMenuItem::separator(),
        &resume,
        &pause,
        &PredefinedMenuItem::separator(),
        &turtle_mode,
        &PredefinedMenuItem::separator(),
        &show,
        &exit,
    ])
    .map_err(|e| tracing::warn!("Failed to build tray menu: {e}"))
    .ok()?;

    let mut builder = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Clutch");

    if let Some(ic) = icon {
        builder = builder.with_icon(ic);
    }

    match builder.build() {
        Ok(tray_icon) => Some((
            tray_icon,
            TrayMenuItems {
                speed_down,
                speed_up,
                resume,
                pause,
                turtle_mode,
            },
        )),
        Err(e) => {
            tracing::warn!("Failed to create system tray icon: {e}");
            None
        }
    }
}

/// The Linux tray backend (libappindicator + a GTK menu) needs an initialized
/// GTK with a running GTK main loop, which iced (winit) does not provide. As
/// recommended by `tray-icon`, the tray therefore lives on a dedicated thread
/// that initializes GTK, owns every tray object and runs `gtk::main()`. Menu
/// updates reach it as [`TrayCommand`]s over a channel; menu clicks still
/// arrive through the global `MenuEvent` channel polled by [`subscription`].
#[cfg(target_os = "linux")]
mod gtk_thread {
    use std::sync::mpsc;
    use std::time::Duration;

    use gtk::glib;

    use super::{TrayCommand, TrayState, build_on_current_thread};

    /// How often the GTK thread applies pending menu updates.
    const COMMAND_POLL_INTERVAL: Duration = Duration::from_millis(100);
    /// How long app startup waits for the tray thread to report back.
    const STARTUP_TIMEOUT: Duration = Duration::from_secs(5);

    pub(super) fn spawn() -> Option<TrayState> {
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let (commands_tx, commands_rx) = mpsc::channel();
        if let Err(e) = std::thread::Builder::new()
            .name("clutch-tray".into())
            .spawn(move || run(ready_tx, commands_rx))
        {
            tracing::warn!("Failed to spawn the system tray thread: {e}");
            return None;
        }
        match ready_rx.recv_timeout(STARTUP_TIMEOUT) {
            Ok(true) => Some(TrayState {
                commands: commands_tx,
            }),
            Ok(false) => None,
            Err(_) => {
                tracing::warn!("Timed out waiting for the system tray to start");
                None
            }
        }
    }

    fn run(ready: mpsc::SyncSender<bool>, commands: mpsc::Receiver<TrayCommand>) {
        if let Err(e) = gtk::init() {
            tracing::warn!("Failed to initialize GTK for the system tray: {e}");
            let _ = ready.send(false);
            return;
        }
        let Some((icon, items)) = build_on_current_thread() else {
            let _ = ready.send(false);
            return;
        };
        let _ = ready.send(true);

        glib::timeout_add_local(COMMAND_POLL_INTERVAL, move || {
            // Keep the tray icon alive for as long as the main loop runs.
            let _ = &icon;
            loop {
                match commands.try_recv() {
                    Ok(command) => items.apply(command),
                    Err(mpsc::TryRecvError::Empty) => return glib::ControlFlow::Continue,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        gtk::main_quit();
                        return glib::ControlFlow::Break;
                    }
                }
            }
        });
        gtk::main();
    }
}

/// Decode the application icon PNG bytes into a flat RGBA buffer for `tray_icon`.
///
/// `tray_icon::Icon::from_rgba` requires raw decoded `[R, G, B, A, ...]` bytes,
/// NOT the PNG-encoded `include_bytes!` data. We use the `image` crate to decode.
fn load_icon() -> Option<tray_icon::Icon> {
    let img = image::load_from_memory(crate::theme::ICON_256_BYTES)
        .map_err(|e| tracing::warn!("Failed to decode icon image: {e}"))
        .ok()?;
    let rgba = img.into_rgba8();
    let (width, height) = rgba.dimensions();
    tray_icon::Icon::from_rgba(rgba.into_raw(), width, height)
        .map_err(|e| tracing::warn!("Failed to create tray icon from RGBA: {e}"))
        .ok()
}

// ── Event bridging ────────────────────────────────────────────────────────────

/// Map a raw `MenuId` string back to a `TrayAction`.
fn menu_id_to_action(id: &MenuId) -> Option<TrayAction> {
    match id.as_ref() {
        ID_RESUME => Some(TrayAction::ResumeAll),
        ID_PAUSE => Some(TrayAction::PauseAll),
        ID_TURTLE => Some(TrayAction::ToggleTurtle),
        ID_SHOW => Some(TrayAction::ShowWindow),
        ID_EXIT => Some(TrayAction::Exit),
        _ => None,
    }
}

/// Subscription that polls tray events every 100 ms and forwards them into the
/// iced event loop as `Message::TrayAction` values.
pub fn subscription() -> iced::Subscription<Message> {
    iced::Subscription::run(tray_event_stream)
}

fn tray_event_stream() -> impl iced::futures::Stream<Item = Message> {
    iced::stream::channel(8, async |mut output| {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            // Drain the native menu event channel.
            while let Ok(event) = MenuEvent::receiver().try_recv() {
                if let Some(action) = menu_id_to_action(&event.id) {
                    let _ = output.send(Message::TrayAction(action)).await;
                }
            }

            // Drain the tray icon click channel. We don't act on clicks —
            // the OS already opens the context menu on click, which is the
            // only intended interaction with the icon itself.
            while TrayIconEvent::receiver().try_recv().is_ok() {}
        }
    })
}
