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

use iced::widget::{button, container};
use iced::{Border, Color, Element, Length, Shadow, Theme};

use super::{DISABLED_DARK, DISABLED_LIGHT};

/// Icon-only toolbar button: transparent background with a circular primary
/// tint on hover/press.
///
/// The caller must attach `.on_press(message)` to make the button active.
pub fn icon_button<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
) -> iced::widget::Button<'a, Message> {
    iced::widget::button(
        container(content)
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
        container(content)
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

/// M3 "filled tonal" button — primary brand wash background, solid primary text.
///
/// Use for secondary actions that are important but not the primary CTA.
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
pub fn m3_primary_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let radius = 100.0.into();
    match status {
        button::Status::Active => button::Style {
            background: Some(iced::Background::Color(palette.primary.base.color)),
            text_color: palette.primary.base.text,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius,
            },
            shadow: Shadow::default(),
            snap: false,
        },
        button::Status::Hovered | button::Status::Pressed => button::Style {
            background: Some(iced::Background::Color(palette.primary.strong.color)),
            text_color: palette.primary.base.text,
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
                ..palette.primary.base.color
            })),
            text_color: palette.primary.base.text.scale_alpha(0.38),
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

/// Destructive pill button used by confirmation dialogs.
pub fn danger_pill_button(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let radius = 100.0.into();
    match status {
        button::Status::Active => button::Style {
            background: Some(iced::Background::Color(palette.danger.base.color)),
            text_color: palette.danger.base.text,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius,
            },
            shadow: Shadow::default(),
            snap: false,
        },
        button::Status::Hovered | button::Status::Pressed => button::Style {
            background: Some(iced::Background::Color(palette.danger.strong.color)),
            text_color: palette.danger.base.text,
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
                ..palette.danger.base.color
            })),
            text_color: palette.danger.base.text.scale_alpha(0.38),
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

/// M3 Filter Chip button style.
pub fn m3_filter_chip(theme: &Theme, status: button::Status, is_selected: bool) -> button::Style {
    let palette = theme.extended_palette();
    let radius = 8.0.into();

    if is_selected {
        button::Style {
            background: Some(iced::Background::Color(Color {
                a: 0.15,
                ..palette.primary.base.color
            })),
            text_color: palette.primary.base.color,
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius,
            },
            shadow: Shadow::default(),
            snap: false,
        }
    } else {
        let outline = Color {
            a: 0.30,
            ..palette.background.base.text
        };
        match status {
            button::Status::Hovered | button::Status::Pressed => button::Style {
                background: Some(iced::Background::Color(Color {
                    a: 0.05,
                    ..palette.background.base.text
                })),
                text_color: palette.background.base.text,
                border: Border {
                    color: outline,
                    width: 1.0,
                    radius,
                },
                shadow: Shadow::default(),
                snap: false,
            },
            _ => button::Style {
                background: None,
                text_color: palette.background.base.text,
                border: Border {
                    color: outline,
                    width: 1.0,
                    radius,
                },
                shadow: Shadow::default(),
                snap: false,
            },
        }
    }
}

/// M3 context-menu item button: transparent background with an 8 % state layer
/// on hover. Zero border radius so the highlight spans edge to edge.
pub fn m3_menu_item(theme: &Theme, status: button::Status) -> button::Style {
    let text_color = theme.palette().text;
    match status {
        button::Status::Hovered | button::Status::Pressed => button::Style {
            background: Some(iced::Background::Color(Color {
                a: 0.08,
                ..text_color
            })),
            text_color,
            border: Border::default(),
            shadow: Shadow::default(),
            snap: false,
        },
        _ => button::Style {
            background: None,
            text_color,
            border: Border::default(),
            shadow: Shadow::default(),
            snap: false,
        },
    }
}

/// M3 context-menu item button — disabled state: dimmed text, no hover effect.
pub fn m3_menu_item_disabled(theme: &Theme, _status: button::Status) -> button::Style {
    let is_dark = theme.extended_palette().background.base.color.r < 0.5;
    let text_color = if is_dark {
        DISABLED_DARK
    } else {
        DISABLED_LIGHT
    };

    button::Style {
        background: None,
        text_color,
        border: Border::default(),
        shadow: Shadow::default(),
        snap: false,
    }
}
