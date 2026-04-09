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

use iced::widget::{button, row};
use iced::{Border, Color, Element, Length, Shadow, Theme};

use super::{SEGCTL_BORDER_DARK, SEGCTL_BORDER_LIGHT, SEGCTL_SURFACE_DARK, SEGCTL_SURFACE_LIGHT};

/// M3 segmented control: a connected row of buttons acting as a single toggle.
pub fn segmented_control<'a, Message: Clone + 'a, T: PartialEq + Copy>(
    options: &[(&'a str, T)],
    active: T,
    on_select: impl Fn(T) -> Message + 'a,
    equal_width: bool,
    compact: bool,
) -> Element<'a, Message> {
    let count = options.len();
    let mut buttons: Vec<Element<'a, Message>> = Vec::with_capacity(count);

    for (index, (label, value)) in options.iter().enumerate() {
        let is_active = *value == active;
        let msg = on_select(*value);

        const ROUNDNESS: f32 = 16.0;
        let radius = if count <= 1 {
            iced::border::Radius::from(ROUNDNESS)
        } else if index == 0 {
            iced::border::Radius {
                top_left: ROUNDNESS,
                bottom_left: ROUNDNESS,
                top_right: 0.0,
                bottom_right: 0.0,
            }
        } else if index == count - 1 {
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
