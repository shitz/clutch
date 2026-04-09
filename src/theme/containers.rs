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

//! Container and surface styles used across Clutch screens and dialogs.

use iced::widget::container;
use iced::{Border, Color, Shadow, Theme, Vector};

use super::{
    CARD_SURFACE_DARK, CARD_SURFACE_LIGHT, INSPECTOR_SURFACE_DARK, INSPECTOR_SURFACE_LIGHT,
    MAGNETIC_BLUE, MAGNETIC_BLUE_LIGHT,
};

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

/// Selected-row highlight: flat color, no border, no shadow.
pub fn selected_row(theme: &Theme) -> container::Style {
    let is_dark = theme.extended_palette().background.base.color.r < 0.5;
    let bg = if is_dark {
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
        text_color: None,
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

/// M3 card surface container style: uniform 16 px radius, tonal elevation,
/// subtle drop shadow.
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

/// Auth overlay card: simple rounded surface without extra elevation.
pub fn auth_dialog_card(theme: &Theme) -> container::Style {
    let is_dark = theme.extended_palette().background.base.color.r < 0.5;
    let bg = if is_dark {
        CARD_SURFACE_DARK
    } else {
        CARD_SURFACE_LIGHT
    };

    container::Style {
        background: Some(iced::Background::Color(bg)),
        border: Border {
            radius: 12.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Elevated dialog card used by destructive confirmations and settings overlays.
pub fn dialog_card(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(iced::Background::Color(palette.background.base.color)),
        border: Border {
            radius: 12.0.into(),
            width: 1.0,
            color: palette.background.strong.color,
        },
        shadow: Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.35),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 16.0,
        },
        ..Default::default()
    }
}

/// Backdrop scrim used behind modal overlays.
pub fn dialog_scrim(alpha: f32) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(iced::Background::Color(Color::from_rgba(
            0.0, 0.0, 0.0, alpha,
        ))),
        ..Default::default()
    }
}

/// M3 Plain Tooltip: dark elevated surface, rounded corners, no harsh border.
pub fn m3_tooltip(theme: &Theme) -> container::Style {
    let is_dark = theme.extended_palette().background.base.color.r < 0.5;
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

/// Floating context-menu card: matches the M3 menu surface spec.
pub fn m3_menu_card(theme: &Theme) -> container::Style {
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
            radius: 4.0.into(),
            width: 0.0,
            color: Color::TRANSPARENT,
        },
        shadow: Shadow {
            color: Color::from_rgba8(0, 0, 0, 0.30),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 14.0,
        },
        snap: false,
    }
}
