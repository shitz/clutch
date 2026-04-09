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

use iced::widget::text_input;
use iced::{Border, Color, Theme};

/// M3 Outlined text-field style.
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
