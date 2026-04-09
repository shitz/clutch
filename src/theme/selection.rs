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

//! Checkbox-like selection widgets built from Material Icons glyphs.

use iced::Element;
use iced::widget::row;

use super::{ICON_CB_CHECKED, ICON_CB_MIXED, ICON_CB_UNCHECKED, MATERIAL_ICONS};

/// Three-valued selection state used by the file-list header checkbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckState {
    Checked,
    Unchecked,
    Mixed,
}

/// Shared rendering core for both `m3_checkbox` and `m3_tristate_checkbox`.
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

/// Render a two-state checkbox using Material Icons.
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
