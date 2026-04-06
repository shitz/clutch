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
use iced::widget::row;
use iced::widget::text_input;
use iced::{Border, Color, Element, Font, Length, Shadow, Theme, Vector};

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

// ── Checkbox icon codepoints ──────────────────────────────────────────────────

pub const ICON_CB_CHECKED: char = '\u{e834}'; // check_box
pub const ICON_CB_UNCHECKED: char = '\u{e835}'; // check_box_outline_blank
pub const ICON_CB_MIXED: char = '\u{e909}'; // indeterminate_check_box

// ── Tri-state checkbox ────────────────────────────────────────────────────────

/// Three-valued selection state used by the file-list header checkbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckState {
    Checked,
    Unchecked,
    Mixed,
}

/// Shared rendering core for both `m3_checkbox` and `m3_tristate_checkbox`.
///
/// Produces a `mouse_area`-wrapped row of icon + optional label.
/// `is_active` controls the icon colour: primary when `true`, muted text when `false`.
fn checkbox_widget<'a, Message: Clone + 'a>(
    codepoint: char,
    is_active: bool,
    label: &'a str,
    on_press: Message,
) -> Element<'a, Message> {
    let icon_widget = iced::widget::text(String::from(codepoint))
        .font(MATERIAL_ICONS)
        .size(20)
        .width(20)
        .align_x(iced::alignment::Horizontal::Center)
        .style(move |theme: &iced::Theme| {
            let palette = theme.extended_palette();
            iced::widget::text::Style {
                color: Some(if is_active {
                    palette.primary.base.color
                } else {
                    palette.background.base.text
                }),
            }
        });

    let content: Element<'a, Message> = if label.is_empty() {
        icon_widget.into()
    } else {
        row![icon_widget, iced::widget::text(label).size(14)]
            .spacing(8)
            .align_y(iced::alignment::Vertical::Center)
            .into()
    };

    iced::widget::mouse_area(content).on_press(on_press).into()
}

/// Render a two-state checkbox using Material Icons, matching the visual style
/// of [`m3_tristate_checkbox`].
///
/// `on_toggle` receives the new `bool` value after the click.
pub fn m3_checkbox<'a, Message: Clone + 'a>(
    checked: bool,
    label: &'a str,
    on_toggle: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    let codepoint = if checked {
        ICON_CB_CHECKED
    } else {
        ICON_CB_UNCHECKED
    };
    checkbox_widget(codepoint, checked, label, on_toggle(!checked))
}

/// Render a tri-state "Select All" header checkbox using Material Icons.
///
/// Clicking `Mixed` or `Unchecked` produces `CheckState::Checked`;
/// clicking `Checked` produces `CheckState::Unchecked`.
pub fn m3_tristate_checkbox<'a, Message: Clone + 'a>(
    state: CheckState,
    label: &'a str,
    on_toggle: impl Fn(CheckState) -> Message + 'a,
) -> Element<'a, Message> {
    let (codepoint, next_state) = match state {
        CheckState::Mixed => (ICON_CB_MIXED, CheckState::Checked),
        CheckState::Checked => (ICON_CB_CHECKED, CheckState::Unchecked),
        CheckState::Unchecked => (ICON_CB_UNCHECKED, CheckState::Checked),
    };
    checkbox_widget(
        codepoint,
        state != CheckState::Unchecked,
        label,
        on_toggle(next_state),
    )
}

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

// ── Container styles ──────────────────────────────────────────────────────────

/// Inspector panel background: slightly elevated, rounded top corners.
pub fn inspector_surface(theme: &Theme) -> container::Style {
    let is_dark = theme.extended_palette().background.base.color.r < 0.5;
    let bg = if is_dark {
        INSPECTOR_SURFACE_DARK
    } else {
        INSPECTOR_SURFACE_LIGHT
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
            color: Color::from_rgba8(0, 0, 0, 0.0),
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
                    PROGRESS_DOWNLOAD_TRACK_DARK
                } else {
                    PROGRESS_DOWNLOAD_TRACK_LIGHT
                },
                PROGRESS_DOWNLOAD_BAR,
            ),
            6 => (
                if is_dark {
                    PROGRESS_SEED_TRACK_DARK
                } else {
                    PROGRESS_SEED_TRACK_LIGHT
                },
                PROGRESS_SEED_BAR,
            ),
            _ => (
                if is_dark {
                    PROGRESS_PAUSED_TRACK_DARK
                } else {
                    PROGRESS_PAUSED_TRACK_LIGHT
                },
                if is_dark {
                    PROGRESS_PAUSED_BAR_DARK
                } else {
                    PROGRESS_PAUSED_BAR_LIGHT
                },
            ),
        };
        progress_bar::Style {
            background: iced::Background::Color(bg),
            bar: iced::Background::Color(bar),
            border: Border {
                radius: 100.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
        }
    }
}

// ── Button styles ─────────────────────────────────────────────────────────────

/// Selected-row highlight: flat color, no border, no shadow.
pub fn selected_row(theme: &Theme) -> container::Style {
    let is_dark = theme.extended_palette().background.base.color.r < 0.5;
    let bg = if is_dark {
        // Subtle tint of brand blue at ~18 % opacity blended over the surface
        Color {
            r: MAGNETIC_BLUE_LIGHT.r,
            g: MAGNETIC_BLUE_LIGHT.g,
            b: MAGNETIC_BLUE_LIGHT.b,
            a: 0.18,
        }
    } else {
        Color {
            r: MAGNETIC_BLUE.r,
            g: MAGNETIC_BLUE.g,
            b: MAGNETIC_BLUE.b,
            a: 0.12,
        }
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

/// Icon-only toolbar button: transparent background with a circular primary
/// tint on hover/press.
///
/// The caller must attach `.on_press(message)` to make the button active.
pub fn icon_button<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
) -> iced::widget::Button<'a, Message> {
    iced::widget::button(
        iced::widget::container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill),
    )
    .padding(0)
    .width(Length::Fixed(36.0))
    .height(Length::Fixed(36.0))
    .style(|theme: &Theme, status| {
        let primary = theme.palette().primary;
        let is_dark = theme.extended_palette().background.base.color.r < 0.5;
        let bg = match status {
            button::Status::Hovered => Some(iced::Background::Color(Color {
                r: primary.r,
                g: primary.g,
                b: primary.b,
                a: 0.12,
            })),
            button::Status::Pressed => Some(iced::Background::Color(Color {
                r: primary.r,
                g: primary.g,
                b: primary.b,
                a: 0.20,
            })),
            _ => None,
        };
        let text_color = match status {
            button::Status::Disabled => {
                if is_dark {
                    DISABLED_DARK
                } else {
                    DISABLED_LIGHT
                }
            }
            _ => theme.palette().text,
        };
        button::Style {
            background: bg,
            text_color,
            border: Border {
                radius: 100.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            shadow: Shadow::default(),
            snap: false,
        }
    })
}

/// Like [`icon_button`] but highlights with the primary colour when `active` is `true`.
///
/// Use this for toggle buttons whose pressed state should be persistently
/// visible (e.g. the Turtle Mode speed-limiter toggle in the toolbar).
pub fn active_icon_button<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    active: bool,
) -> iced::widget::Button<'a, Message> {
    iced::widget::button(
        iced::widget::container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center(Length::Fill),
    )
    .padding(0)
    .width(Length::Fixed(36.0))
    .height(Length::Fixed(36.0))
    .style(move |theme: &Theme, status| {
        let primary = theme.palette().primary;
        let is_dark = theme.extended_palette().background.base.color.r < 0.5;
        let bg = if active {
            Some(iced::Background::Color(Color {
                r: primary.r,
                g: primary.g,
                b: primary.b,
                a: match status {
                    button::Status::Pressed => 0.32,
                    _ => 0.20,
                },
            }))
        } else {
            match status {
                button::Status::Hovered => Some(iced::Background::Color(Color {
                    r: primary.r,
                    g: primary.g,
                    b: primary.b,
                    a: 0.12,
                })),
                button::Status::Pressed => Some(iced::Background::Color(Color {
                    r: primary.r,
                    g: primary.g,
                    b: primary.b,
                    a: 0.20,
                })),
                _ => None,
            }
        };
        let text_color = if active {
            primary
        } else {
            match status {
                button::Status::Disabled => {
                    if is_dark {
                        DISABLED_DARK
                    } else {
                        DISABLED_LIGHT
                    }
                }
                _ => theme.palette().text,
            }
        };
        button::Style {
            background: bg,
            text_color,
            border: Border {
                radius: 100.0.into(),
                width: 0.0,
                color: Color::TRANSPARENT,
            },
            shadow: Shadow::default(),
            snap: false,
        }
    })
}

/// M3 segmented control: a connected row of buttons acting as a single toggle.
///
/// `options` is a slice of `(label, value)` pairs.
/// `active` is the currently selected value.
/// `on_select` maps a selected value to a `Message`.
/// `equal_width` — when `true` each segment gets `Length::FillPortion(1)` so
/// all segments are equal width; when `false` segments shrink to their label.
/// `compact` — when `true` uses reduced vertical padding for space-constrained contexts.
pub fn segmented_control<'a, Message: Clone + 'a, T: PartialEq + Copy>(
    options: &[(&'a str, T)],
    active: T,
    on_select: impl Fn(T) -> Message + 'a,
    equal_width: bool,
    compact: bool,
) -> Element<'a, Message> {
    let count = options.len();
    let mut buttons: Vec<Element<'a, Message>> = Vec::with_capacity(count);

    for (i, (label, value)) in options.iter().enumerate() {
        let is_active = *value == active;
        let msg = on_select(*value);

        const ROUNDNESS: f32 = 16.0;
        let radius = if count <= 1 {
            iced::border::Radius::from(ROUNDNESS)
        } else if i == 0 {
            iced::border::Radius {
                top_left: ROUNDNESS,
                bottom_left: ROUNDNESS,
                top_right: 0.0,
                bottom_right: 0.0,
            }
        } else if i == count - 1 {
            iced::border::Radius {
                top_left: 0.0,
                bottom_left: 0.0,
                top_right: ROUNDNESS,
                bottom_right: ROUNDNESS,
            }
        } else {
            iced::border::Radius::from(0.0)
        };

        let btn = iced::widget::button(iced::widget::text(*label).align_x(iced::Alignment::Center))
            .on_press(msg)
            .padding(if compact { [5, 10] } else { [8, 16] })
            .width(if equal_width {
                Length::FillPortion(1)
            } else {
                Length::Shrink
            })
            .style(move |theme: &Theme, status| {
                let is_dark = theme.extended_palette().background.base.color.r < 0.5;
                let primary = theme.palette().primary;
                let is_hovered =
                    matches!(status, button::Status::Hovered | button::Status::Pressed);

                let (bg, text_color) = if is_active {
                    (
                        Some(iced::Background::Color(Color { a: 0.18, ..primary })),
                        primary,
                    )
                } else if is_hovered {
                    (
                        Some(iced::Background::Color(Color {
                            r: primary.r,
                            g: primary.g,
                            b: primary.b,
                            a: 0.10,
                        })),
                        theme.palette().text,
                    )
                } else {
                    let surface = if is_dark {
                        SEGCTL_SURFACE_DARK
                    } else {
                        SEGCTL_SURFACE_LIGHT
                    };
                    (
                        Some(iced::Background::Color(surface)),
                        theme.palette().text.scale_alpha(0.65),
                    )
                };

                button::Style {
                    background: bg,
                    text_color,
                    border: Border {
                        radius,
                        width: 1.0,
                        color: if is_dark {
                            SEGCTL_BORDER_DARK
                        } else {
                            SEGCTL_BORDER_LIGHT
                        },
                    },
                    shadow: Shadow::default(),
                    snap: false,
                }
            })
            .into();

        buttons.push(btn);
    }

    row(buttons).spacing(0).into()
}

/// M3 card surface container style: uniform 16 px radius, tonal elevation,
/// subtle drop shadow.
///
/// Use with `container(content).style(theme::m3_card)`.
pub fn m3_card(theme: &Theme) -> container::Style {
    let is_dark = theme.extended_palette().background.base.color.r < 0.5;
    let bg = if is_dark {
        CARD_SURFACE_DARK
    } else {
        CARD_SURFACE_LIGHT
    };
    container::Style {
        text_color: None,
        background: Some(iced::Background::Color(bg)),
        border: Border {
            radius: 16.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.18),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 6.0,
        },
        snap: false,
    }
}

/// M3 Plain Tooltip: dark elevated surface, rounded corners, no harsh border.
pub fn m3_tooltip(theme: &Theme) -> container::Style {
    let is_dark = theme.extended_palette().background.base.color.r < 0.5;
    // Use a slightly darker/lighter contrasting surface regardless of mode.
    let bg = if is_dark {
        Color::from_rgb8(46, 50, 58)
    } else {
        Color::from_rgb8(42, 46, 54)
    };
    container::Style {
        background: Some(iced::Background::Color(bg)),
        text_color: Some(Color::from_rgb8(220, 224, 232)),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 6.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.28),
            offset: Vector::new(0.0, 2.0),
            blur_radius: 8.0,
        },
        snap: false,
    }
}

// ── Text input styles ─────────────────────────────────────────────────────────

/// M3 Outlined text-field style.
///
/// Inactive: subtle 1px border. Hovered: slightly brighter border.
/// Focused: primary-colour 2px border.
pub fn m3_text_input(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let is_dark = theme.extended_palette().background.base.color.r < 0.5;
    let bg = if is_dark {
        iced::Background::Color(Color::from_rgb8(26, 29, 34))
    } else {
        iced::Background::Color(Color::from_rgb8(255, 255, 255))
    };
    let text_color = theme.palette().text;
    let placeholder_color = theme.palette().text.scale_alpha(0.40);
    let selection_color = theme.extended_palette().primary.weak.color;
    let radius = 8.0.into();

    match status {
        text_input::Status::Active => text_input::Style {
            background: bg,
            border: Border {
                color: if is_dark {
                    Color::from_rgb8(55, 60, 70)
                } else {
                    Color::from_rgb8(190, 195, 205)
                },
                width: 1.0,
                radius,
            },
            icon: text_color,
            value: text_color,
            placeholder: placeholder_color,
            selection: selection_color,
        },
        text_input::Status::Hovered => text_input::Style {
            background: bg,
            border: Border {
                color: if is_dark {
                    Color::from_rgb8(85, 92, 105)
                } else {
                    Color::from_rgb8(140, 148, 165)
                },
                width: 1.0,
                radius,
            },
            icon: text_color,
            value: text_color,
            placeholder: placeholder_color,
            selection: selection_color,
        },
        text_input::Status::Focused { .. } => text_input::Style {
            background: bg,
            border: Border {
                color: theme.palette().primary,
                width: 2.0,
                radius,
            },
            icon: text_color,
            value: text_color,
            placeholder: placeholder_color,
            selection: selection_color,
        },
        text_input::Status::Disabled => text_input::Style {
            background: if is_dark {
                iced::Background::Color(Color::from_rgb8(20, 22, 26))
            } else {
                iced::Background::Color(Color::from_rgb8(240, 240, 243))
            },
            border: Border {
                color: if is_dark {
                    Color::from_rgb8(40, 43, 50)
                } else {
                    Color::from_rgb8(210, 213, 220)
                },
                width: 1.0,
                radius,
            },
            icon: placeholder_color,
            value: placeholder_color,
            placeholder: placeholder_color,
            selection: selection_color,
        },
    }
}

// ── Additional button styles ──────────────────────────────────────────────────

/// M3 "filled tonal" button — primary brand wash background, solid primary text.
///
/// Use for secondary actions that are important but not the primary CTA
/// (e.g. "Test Connection").
pub fn m3_tonal_button(theme: &Theme, status: button::Status) -> button::Style {
    let primary = theme.palette().primary;
    match status {
        button::Status::Active => button::Style {
            background: Some(iced::Background::Color(Color { a: 0.15, ..primary })),
            text_color: primary,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 100.0.into(),
            },
            shadow: Shadow::default(),
            snap: false,
        },
        button::Status::Hovered => button::Style {
            background: Some(iced::Background::Color(Color { a: 0.25, ..primary })),
            text_color: primary,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 100.0.into(),
            },
            shadow: Shadow::default(),
            snap: false,
        },
        button::Status::Pressed => button::Style {
            background: Some(iced::Background::Color(Color { a: 0.35, ..primary })),
            text_color: primary,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 100.0.into(),
            },
            shadow: Shadow::default(),
            snap: false,
        },
        button::Status::Disabled => button::Style {
            background: None,
            text_color: theme.palette().text.scale_alpha(0.45),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 100.0.into(),
            },
            shadow: Shadow::default(),
            snap: false,
        },
    }
}

/// M3 "filled" primary button — solid brand-primary background, white text.
///
/// Use for the primary CTA (e.g. "Save").
pub fn m3_primary_button(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    let radius = 100.0.into();
    match status {
        button::Status::Active => button::Style {
            background: Some(iced::Background::Color(p.primary.base.color)),
            text_color: p.primary.base.text,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius,
            },
            shadow: Shadow::default(),
            snap: false,
        },
        button::Status::Hovered => button::Style {
            background: Some(iced::Background::Color(p.primary.strong.color)),
            text_color: p.primary.base.text,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius,
            },
            shadow: Shadow::default(),
            snap: false,
        },
        button::Status::Pressed => button::Style {
            background: Some(iced::Background::Color(p.primary.strong.color)),
            text_color: p.primary.base.text,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius,
            },
            shadow: Shadow::default(),
            snap: false,
        },
        button::Status::Disabled => button::Style {
            background: Some(iced::Background::Color(Color {
                a: 0.38,
                ..p.primary.base.color
            })),
            text_color: p.primary.base.text.scale_alpha(0.38),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius,
            },
            shadow: Shadow::default(),
            snap: false,
        },
    }
}
