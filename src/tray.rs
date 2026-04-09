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
//! iced event loop starts). On failure (e.g. missing libappindicator on Linux)
//! it returns `None` and the app continues without a tray.
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

/// Cloneable handles to the mutable tray menu items.
///
/// Calling `.set_text()` or `.set_enabled()` on these updates the native OS
/// menu immediately — no menu rebuild is required.
pub struct TrayMenuItems {
    pub speed_down: MenuItem,
    pub speed_up: MenuItem,
    pub resume: MenuItem,
    pub pause: MenuItem,
    /// The turtle mode item uses a plain `MenuItem` (not `CheckMenuItem`) so
    /// that it sits in the same gutter column as every other item. The active
    /// state is signalled by swapping the label prefix between ● (on) and ○
    /// (off) — both glyphs have identical advance-widths in proportional fonts.
    pub turtle_mode: MenuItem,
}

impl TrayMenuItems {
    /// Enable or disable the daemon-dependent action items.
    ///
    /// Call with `true` immediately after a successful connect and `false`
    /// after disconnect. The speed labels and Show / Exit items are unaffected.
    pub fn set_connected(&self, connected: bool) {
        self.resume.set_enabled(connected);
        self.pause.set_enabled(connected);
        self.turtle_mode.set_enabled(connected);
    }

    /// Update the turtle mode label to reflect the current active state.
    ///
    /// Uses ● (U+25CF) for on and ○ (U+25CB) for off; both are from the same
    /// Unicode block and have identical advance-widths in proportional fonts,
    /// so the "Turtle Mode" text never shifts position.
    pub fn set_turtle_active(&self, active: bool) {
        self.turtle_mode.set_text(if active {
            "● Turtle Mode"
        } else {
            "○ Turtle Mode"
        });
    }

    /// Reset the speed labels to the idle placeholder.
    pub fn reset_speeds(&self) {
        self.speed_down.set_text("↓  —");
        self.speed_up.set_text("↑  —");
    }
}

/// System tray icon state.
///
/// The `_icon` field owns the `TrayIcon` handle; dropping it destroys the OS
/// tray icon, so this struct must be kept alive for the application lifetime.
pub struct TrayState {
    _icon: TrayIcon,
    pub items: TrayMenuItems,
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
    let icon = load_icon().or_else(|| {
        tracing::warn!("Could not decode tray icon — tray will show no icon");
        None
    });

    // Speed labels: disabled, updated on every torrent poll.
    let speed_down = MenuItem::new("↓  —", false, None);
    let speed_up = MenuItem::new("↑  —", false, None);

    // Action items — all plain MenuItems so every label sits in the same
    // gutter column with no OS-level checkmark offset.
    // resume / pause / turtle start DISABLED and are enabled after connect.
    let resume = MenuItem::with_id(ID_RESUME, "Resume All", false, None);
    let pause = MenuItem::with_id(ID_PAUSE, "Pause All", false, None);
    // ○ = unchecked (turtle off). The ● / ○ pair has equal advance-widths.
    let turtle_mode = MenuItem::with_id(ID_TURTLE, "○ Turtle Mode", false, None);
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
        Ok(tray_icon) => Some(TrayState {
            _icon: tray_icon,
            items: TrayMenuItems {
                speed_down,
                speed_up,
                resume,
                pause,
                turtle_mode,
            },
        }),
        Err(e) => {
            tracing::warn!("Failed to create system tray icon: {e}");
            None
        }
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
