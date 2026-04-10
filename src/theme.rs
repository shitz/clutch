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

//! All styling concerns are centralised in `crate::theme`.
//!
//! This file is the public facade for the theme layer. Private implementation
//! details live under `src/theme/`, but callers continue to import everything
//! from `crate::theme` so the styling API stays stable as the internals evolve.
//!
//! # Usage
//!
//! Register the fonts in `main.rs`:
//! ```ignore
//! .font(theme::MATERIAL_ICONS_BYTES)
//! .font(iced_aw::ICED_AW_FONT_BYTES)
//! ```
//!
//! Apply styles:
//! ```ignore
//! container(content).style(theme::m3_card)
//! progress_bar(...).style(theme::progress_bar_style(torrent.status))
//! icon('\u{E034}')  // pause glyph
//! ```

use iced::{Color, Font, Theme};

mod buttons;
mod containers;
mod inputs;
mod progress;
mod segmented;
mod selection;

pub use self::buttons::{
    active_icon_button, danger_pill_button, icon_button, m3_filter_chip, m3_menu_item,
    m3_menu_item_disabled, m3_primary_button, m3_tonal_button,
};
pub use self::containers::{
    auth_dialog_card, dialog_card, dialog_scrim, inspector_surface, m3_card, m3_menu_card,
    m3_tooltip, selected_row,
};
pub use self::inputs::m3_text_input;
pub use self::progress::progress_bar_style;
pub use self::segmented::segmented_control;
pub use self::selection::{CheckState, m3_checkbox, m3_tristate_checkbox};

// ── Material Icons font ───────────────────────────────────────────────────────

/// Raw bytes of the bundled Material Icons Regular TTF font.
pub const MATERIAL_ICONS_BYTES: &[u8] = include_bytes!("../fonts/MaterialIcons-Regular.ttf");

// ── Asset image bytes ─────────────────────────────────────────────────────────

/// Clutch wordmark PNG — used on the connection screen and empty torrent list.
pub const LOGO_BYTES: &[u8] = include_bytes!("../assets/Clutch_Logo.png");

/// Clutch icon PNG (256 × 256) — used for the OS window icon.
pub const ICON_256_BYTES: &[u8] = include_bytes!("../assets/Clutch_Icon_256x256.png");

/// Clutch icon PNG (512 × 512) — used in-app for the loading splash.
pub const ICON_512_BYTES: &[u8] = include_bytes!("../assets/Clutch_Icon_512x512.png");

/// Font handle for rendering Material icon glyphs.
pub const MATERIAL_ICONS: Font = Font::with_name("Material Icons");

// ── Material icon codepoints ──────────────────────────────────────────────────

pub const ICON_PAUSE: char = '\u{E034}';
pub const ICON_PLAY: char = '\u{E037}';
pub const ICON_DELETE: char = '\u{E872}';
pub const ICON_ADD: char = '\u{E145}';
pub const ICON_LINK: char = '\u{E157}';
pub const ICON_LOGOUT: char = '\u{E9BA}';
pub const ICON_DOWNLOAD: char = '\u{E2C4}';
pub const ICON_UPLOAD: char = '\u{E2C6}';
pub const ICON_SETTINGS: char = '\u{E8B8}';
pub const ICON_TRASH: char = '\u{E872}';
pub const ICON_CLOSE: char = '\u{E5CD}';
pub const ICON_SAVE: char = '\u{E161}';
pub const ICON_UNDO: char = '\u{E166}';
/// Material Icons "speed" glyph — used for the Turtle Mode toolbar toggle.
pub const ICON_SPEED: char = '\u{E9E4}';
/// Material Icons "folder" glyph — used for the Set Data Location context menu item.
pub const ICON_FOLDER: char = '\u{E2C7}';
/// Material Icons "arrow_drop_down" glyph — used for the recent-paths dropdown toggle.
pub const ICON_ARROW_DROP_DOWN: char = '\u{E5C5}';

// ── Checkbox icon codepoints ──────────────────────────────────────────────────

pub const ICON_CB_CHECKED: char = '\u{e834}'; // check_box
pub const ICON_CB_UNCHECKED: char = '\u{e835}'; // check_box_outline_blank
pub const ICON_CB_MIXED: char = '\u{e909}'; // indeterminate_check_box

/// Render a single Material icon glyph at 24 px.
pub fn icon(codepoint: char) -> iced::widget::Text<'static> {
    iced::widget::text(String::from(codepoint))
        .font(MATERIAL_ICONS)
        .size(24)
}

// ── Clutch brand color constants ──────────────────────────────────────────────

/// Primary brand blue — derived from the magnetic icon in the Clutch logo.
pub const MAGNETIC_BLUE: Color = Color::from_rgb(0.16, 0.39, 0.65); // #2A64A7

/// Lightened primary for dark-mode surfaces (~7:1 contrast on SURFACE_DARK).
pub const MAGNETIC_BLUE_LIGHT: Color = Color::from_rgb(0.36, 0.62, 0.83); // #5B9FD4

/// Dark mode app background.
pub const SURFACE_DARK: Color = Color::from_rgb(0.11, 0.13, 0.15); // #1D2024

/// Light mode app background.
pub const SURFACE_LIGHT: Color = Color::from_rgb(0.98, 0.99, 1.0); // #FAFCFF

/// Amber warning — dark mode (warm, consistent with the downloading indicator).
pub const AMBER_DARK: Color = Color::from_rgb(1.0, 0.72, 0.30);

/// Amber warning — light mode.
pub const AMBER_LIGHT: Color = Color::from_rgb(0.9, 0.45, 0.0);

// ── Theme palette colors ──────────────────────────────────────────────────────

/// Primary text color — dark mode.
pub const TEXT_DARK: Color = Color::from_rgb(0.90, 0.92, 0.95);
/// Primary text color — light mode.
pub const TEXT_LIGHT: Color = Color::from_rgb(0.08, 0.10, 0.12);
/// Success green — dark mode.
pub const SUCCESS_DARK: Color = Color::from_rgb(0.20, 0.80, 0.40);
/// Success green — light mode.
pub const SUCCESS_LIGHT: Color = Color::from_rgb(0.10, 0.60, 0.30);
/// Danger red — dark mode.
pub const DANGER_DARK: Color = Color::from_rgb(0.90, 0.40, 0.36);
/// Danger red — light mode.
pub const DANGER_LIGHT: Color = Color::from_rgb(0.80, 0.20, 0.20);

// ── Surface & container colors ────────────────────────────────────────────────

/// Inspector panel background — dark mode.
pub const INSPECTOR_SURFACE_DARK: Color = Color::from_rgb8(36, 40, 46);
/// Inspector panel background — light mode.
pub const INSPECTOR_SURFACE_LIGHT: Color = Color::from_rgb8(236, 241, 250);
/// M3 card surface — dark mode.
pub const CARD_SURFACE_DARK: Color = Color::from_rgb8(38, 42, 48);
/// M3 card surface — light mode.
pub const CARD_SURFACE_LIGHT: Color = Color::from_rgb8(240, 244, 252);
/// Segmented control inactive segment background — dark mode.
pub const SEGCTL_SURFACE_DARK: Color = Color::from_rgb8(46, 45, 50);
/// Segmented control inactive segment background — light mode.
pub const SEGCTL_SURFACE_LIGHT: Color = Color::from_rgb8(238, 236, 244);
/// Segmented control segment border — dark mode (white at 12 % opacity).
pub const SEGCTL_BORDER_DARK: Color = Color::from_rgba8(255, 255, 255, 0.12);
/// Segmented control segment border — light mode (black at 15 % opacity).
pub const SEGCTL_BORDER_LIGHT: Color = Color::from_rgba8(0, 0, 0, 0.15);

// ── State colors ──────────────────────────────────────────────────────────────

/// Disabled icon / text color — dark mode.
pub const DISABLED_DARK: Color = Color::from_rgb8(100, 104, 112);
/// Disabled icon / text color — light mode.
pub const DISABLED_LIGHT: Color = Color::from_rgb8(160, 163, 171);

// ── Progress bar colors ───────────────────────────────────────────────────────

/// Downloading state: track (background) — dark mode.
pub const PROGRESS_DOWNLOAD_TRACK_DARK: Color = Color::from_rgb8(28, 55, 30);
/// Downloading state: track — light mode.
pub const PROGRESS_DOWNLOAD_TRACK_LIGHT: Color = Color::from_rgb8(200, 230, 201);
/// Downloading state: bar fill (shared across modes).
pub const PROGRESS_DOWNLOAD_BAR: Color = Color::from_rgb8(56, 142, 60);
/// Seeding state: track — dark mode.
pub const PROGRESS_SEED_TRACK_DARK: Color = Color::from_rgb8(18, 42, 75);
/// Seeding state: track — light mode.
pub const PROGRESS_SEED_TRACK_LIGHT: Color = Color::from_rgb8(187, 222, 251);
/// Seeding state: bar fill (shared across modes).
pub const PROGRESS_SEED_BAR: Color = Color::from_rgb8(25, 118, 210);
/// Paused / other state: track — dark mode.
pub const PROGRESS_PAUSED_TRACK_DARK: Color = Color::from_rgb8(50, 49, 54);
/// Paused / other state: track — light mode.
pub const PROGRESS_PAUSED_TRACK_LIGHT: Color = Color::from_rgb8(224, 224, 224);
/// Paused / other state: bar fill — dark mode.
pub const PROGRESS_PAUSED_BAR_DARK: Color = Color::from_rgb8(120, 119, 125);
/// Paused / other state: bar fill — light mode.
pub const PROGRESS_PAUSED_BAR_LIGHT: Color = Color::from_rgb8(117, 117, 117);

// ── Clutch theme ──────────────────────────────────────────────────────────────

/// Returns the Clutch brand theme for dark or light mode.
///
/// Uses a hand-crafted M3 palette seeded from `MAGNETIC_BLUE` (`#2A64A7`).
/// Dark mode uses `MAGNETIC_BLUE_LIGHT` as primary for ~7:1 contrast on dark surfaces.
pub fn clutch_theme(is_dark: bool) -> Theme {
    if is_dark {
        Theme::custom(
            "Clutch Dark".to_owned(),
            iced::theme::Palette {
                background: SURFACE_DARK,
                text: TEXT_DARK,
                primary: MAGNETIC_BLUE_LIGHT,
                success: SUCCESS_DARK,
                warning: AMBER_DARK,
                danger: DANGER_DARK,
            },
        )
    } else {
        Theme::custom(
            "Clutch Light".to_owned(),
            iced::theme::Palette {
                background: SURFACE_LIGHT,
                text: TEXT_LIGHT,
                primary: MAGNETIC_BLUE,
                success: SUCCESS_LIGHT,
                warning: AMBER_LIGHT,
                danger: DANGER_LIGHT,
            },
        )
    }
}
