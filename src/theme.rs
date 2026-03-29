//! Material Design 3 theme, icon helper, and shared widget styles.
//!
//! All styling concerns are centralised here so future theme changes require
//! edits in exactly one file.
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
//! container(content).style(theme::elevated_surface)
//! progress_bar(...).style(theme::progress_bar_style(torrent.status))
//! icon('\u{E034}')  // pause glyph
//! ```

use iced::widget::button;
use iced::widget::container;
use iced::widget::progress_bar;
use iced::{Border, Color, Font, Shadow, Theme, Vector};

// ── Material Icons font ───────────────────────────────────────────────────────

/// Raw bytes of the bundled Material Icons Regular TTF font.
pub const MATERIAL_ICONS_BYTES: &[u8] = include_bytes!("../fonts/MaterialIcons-Regular.ttf");

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

/// Render a single Material icon glyph at 24 px.
pub fn icon(codepoint: char) -> iced::widget::Text<'static> {
    iced::widget::text(String::from(codepoint))
        .font(MATERIAL_ICONS)
        .size(24)
}

// ── Material Design 3 palettes ────────────────────────────────────────────────

/// Material Design 3 dark palette.
pub fn material_dark_theme() -> Theme {
    Theme::custom(
        "MaterialDark".to_owned(),
        iced::theme::Palette {
            background: Color::from_rgb8(28, 27, 31), // MD3 Surface
            text: Color::from_rgb8(230, 225, 229),    // MD3 On-Surface
            primary: Color::from_rgb8(208, 188, 255), // MD3 Primary (purple)
            success: Color::from_rgb8(134, 218, 112), // MD3 Tertiary / positive
            warning: Color::from_rgb8(255, 183, 77),  // MD3 amber
            danger: Color::from_rgb8(242, 184, 181),  // MD3 Error container
        },
    )
}

/// Material Design 3 light palette.
pub fn material_light_theme() -> Theme {
    Theme::custom(
        "MaterialLight".to_owned(),
        iced::theme::Palette {
            background: Color::from_rgb8(255, 251, 254), // MD3 Background
            text: Color::from_rgb8(28, 27, 31),          // MD3 On-Background
            primary: Color::from_rgb8(103, 80, 164),     // MD3 Primary
            success: Color::from_rgb8(56, 106, 32),
            warning: Color::from_rgb8(230, 81, 0), // MD3 amber dark
            danger: Color::from_rgb8(179, 38, 30), // MD3 Error
        },
    )
}

// ── Container styles ──────────────────────────────────────────────────────────

/// Inspector panel background: slightly elevated, rounded top corners.
pub fn inspector_surface(theme: &Theme) -> container::Style {
    let is_dark = theme.extended_palette().background.base.color.r < 0.5;
    let bg = if is_dark {
        Color::from_rgb8(36, 35, 40)
    } else {
        Color::from_rgb8(249, 245, 255)
    };
    container::Style {
        text_color: None,
        background: Some(iced::Background::Color(bg)),
        border: Border {
            radius: iced::border::Radius {
                top_left: 12.0,
                top_right: 12.0,
                bottom_left: 0.0,
                bottom_right: 0.0,
            },
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.2), // 0.2 alpha
            offset: Vector::new(0.0, -2.0),
            blur_radius: 4.0,
        },
        snap: false,
    }
}

// ── Progress bar styles ───────────────────────────────────────────────────────

/// Return a progress bar style closure based on the torrent's Transmission status code.
///
/// - Status 4 (Downloading) → green bar
/// - Status 6 (Seeding)     → blue bar
/// - All other states       → gray bar (paused, checking, queued)
pub fn progress_bar_style(status: i32) -> impl Fn(&Theme) -> progress_bar::Style {
    move |theme: &Theme| {
        let is_dark = theme.extended_palette().background.base.color.r < 0.5;
        let (bg, bar) = match status {
            4 => (
                if is_dark {
                    Color::from_rgb8(28, 55, 30)
                } else {
                    Color::from_rgb8(200, 230, 201)
                },
                Color::from_rgb8(56, 142, 60),
            ),
            6 => (
                if is_dark {
                    Color::from_rgb8(18, 42, 75)
                } else {
                    Color::from_rgb8(187, 222, 251)
                },
                Color::from_rgb8(25, 118, 210),
            ),
            _ => (
                if is_dark {
                    Color::from_rgb8(50, 49, 54)
                } else {
                    Color::from_rgb8(224, 224, 224)
                },
                if is_dark {
                    Color::from_rgb8(120, 119, 125)
                } else {
                    Color::from_rgb8(117, 117, 117)
                },
            ),
        };
        progress_bar::Style {
            background: iced::Background::Color(bg),
            bar: iced::Background::Color(bar),
            border: Border::default(),
        }
    }
}

// ── Button styles ─────────────────────────────────────────────────────────────

/// Dim secondary icon buttons so the primary action stands out in dark mode.
pub fn dim_secondary(theme: &Theme, status: button::Status) -> button::Style {
    let is_dark = theme.extended_palette().background.base.color.r < 0.5;
    let (base_bg, text_color) = if is_dark {
        (
            Color::from_rgb8(46, 45, 50),
            Color::from_rgb8(230, 230, 230), // soft white icons
        )
    } else {
        let p = theme.extended_palette();
        (
            p.secondary.base.color.scale_alpha(0.35),
            p.secondary.base.text.scale_alpha(0.8),
        )
    };
    let bg = match status {
        button::Status::Hovered | button::Status::Pressed => {
            if is_dark {
                Color::from_rgb8(62, 61, 66)
            } else {
                theme
                    .extended_palette()
                    .secondary
                    .base
                    .color
                    .scale_alpha(0.5)
            }
        }
        _ => base_bg,
    };
    button::Style {
        background: Some(iced::Background::Color(bg)),
        text_color,
        border: Border {
            radius: 6.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

/// Selected-row highlight: flat color, no border, no shadow.
pub fn selected_row(theme: &Theme) -> container::Style {
    let is_dark = theme.extended_palette().background.base.color.r < 0.5;
    let bg = if is_dark {
        // Subtle tint of primary purple at ~18 % opacity blended over the surface
        Color::from_rgba8(208, 188, 255, 0.18)
    } else {
        Color::from_rgba8(103, 80, 164, 0.12)
    };
    container::Style {
        text_color: None, // inherit from theme
        background: Some(iced::Background::Color(bg)),
        border: Border {
            radius: 6.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

// ── Material tab styles ───────────────────────────────────────────────────────

/// Active tab: primary-color text, transparent background.
pub fn tab_active(theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: None,
        text_color: theme.palette().primary,
        border: Border {
            radius: 0.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

/// Inactive tab: muted text, transparent background.
pub fn tab_inactive(theme: &Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: None,
        text_color: theme.palette().text.scale_alpha(0.5),
        border: Border {
            radius: 0.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

/// 2 px underline bar shown under the active tab label.
pub fn tab_underline(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(iced::Background::Color(theme.palette().primary)),
        border: Border::default(),
        shadow: Shadow::default(),
        text_color: None,
        snap: false,
    }
}
