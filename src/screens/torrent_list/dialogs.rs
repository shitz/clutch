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

//! Context menu and modal overlays owned by the torrent list screen.

use iced::widget::{
    Space, button, checkbox, column, container, mouse_area, row, rule, stack, text,
};
use iced::{Alignment, Element, Length};

use crate::theme::{
    ICON_DELETE, ICON_FOLDER, ICON_PAUSE, ICON_PLAY, ICON_QUEUE_BOTTOM, ICON_QUEUE_DOWN,
    ICON_QUEUE_TOP, ICON_QUEUE_UP, MATERIAL_ICONS,
};

use super::{Message, SetLocationDialog, TorrentListScreen};

/// Menu width in logical pixels — used for right-edge mitigation.
const MENU_WIDTH: f32 = 220.0;

/// Builds a single M3 context-menu row with a fixed-width icon container and
/// an edge-to-edge hover highlight. Pass `on_press = None` to render disabled.
fn menu_item<'a>(
    icon_glyph: char,
    label: &'a str,
    on_press: Option<Message>,
) -> Element<'a, Message> {
    let icon_widget = text(icon_glyph)
        .font(MATERIAL_ICONS)
        .size(18)
        .width(Length::Fixed(24.0))
        .align_x(Alignment::Center);

    let content = row![icon_widget, text(label).size(14)]
        .spacing(12)
        .align_y(Alignment::Center);

    let button = button(content).width(Length::Fill).padding([8, 16]);

    if let Some(msg) = on_press {
        button
            .on_press(msg)
            .style(crate::theme::m3_menu_item)
            .into()
    } else {
        button.style(crate::theme::m3_menu_item_disabled).into()
    }
}

/// Renders a labelled horizontal divider to visually group context menu sections.
fn menu_section<'a>(label: &'a str) -> Element<'a, Message> {
    column![
        rule::horizontal(1),
        text(label)
            .size(10)
            .style(|t: &iced::Theme| iced::widget::text::Style {
                color: Some(t.palette().text.scale_alpha(0.45)),
            })
            .width(Length::Fill),
    ]
    .spacing(0)
    .padding(iced::Padding {
        top: 4.0,
        bottom: 0.0,
        left: 16.0,
        right: 16.0,
    })
    .into()
}

/// Builds the context menu overlay element when a context menu is open.
pub fn view_context_menu_overlay(state: &TorrentListScreen) -> Option<Element<'_, Message>> {
    let (torrent_id, point) = state.context_menu?;

    let effective_y = if point.y > state.window_height - 150.0 {
        point.y - 150.0
    } else {
        point.y
    };
    let effective_x = if point.x + MENU_WIDTH > state.window_width {
        (point.x - MENU_WIDTH).max(0.0)
    } else {
        point.x
    };

    let torrent_opt = state
        .torrents
        .iter()
        .find(|torrent| torrent.id == torrent_id);
    // Enable Start/Pause based on aggregate status across all selected torrents.
    let selected_torrents: Vec<_> = state
        .torrents
        .iter()
        .filter(|t| state.selected_ids.contains(&t.id))
        .collect();
    let can_start = if selected_torrents.is_empty() {
        torrent_opt.is_some_and(|t| !matches!(t.status, 3..=6))
    } else {
        selected_torrents.iter().any(|t| !matches!(t.status, 3..=6))
    };
    let can_pause = if selected_torrents.is_empty() {
        torrent_opt.is_some_and(|t| matches!(t.status, 3..=6))
    } else {
        selected_torrents.iter().any(|t| matches!(t.status, 3..=6))
    };

    let menu_card = container(
        column![
            menu_item(
                ICON_PLAY,
                "Start",
                can_start.then_some(Message::ContextMenuStart)
            ),
            menu_item(
                ICON_PAUSE,
                "Pause",
                can_pause.then_some(Message::ContextMenuPause)
            ),
            menu_item(ICON_DELETE, "Delete", Some(Message::ContextMenuDelete)),
            menu_item(
                ICON_FOLDER,
                "Set Data Location",
                Some(Message::OpenSetLocation)
            ),
            menu_section("Queue"),
            menu_item(
                ICON_QUEUE_TOP,
                "Move to Top",
                Some(Message::ContextMenuQueueMoveTop)
            ),
            menu_item(
                ICON_QUEUE_UP,
                "Move Up",
                Some(Message::ContextMenuQueueMoveUp)
            ),
            menu_item(
                ICON_QUEUE_DOWN,
                "Move Down",
                Some(Message::ContextMenuQueueMoveDown)
            ),
            menu_item(
                ICON_QUEUE_BOTTOM,
                "Move to Bottom",
                Some(Message::ContextMenuQueueMoveBottom),
            ),
        ]
        .spacing(0),
    )
    .padding(iced::Padding {
        top: 8.0,
        bottom: 8.0,
        left: 0.0,
        right: 0.0,
    })
    .width(Length::Fixed(MENU_WIDTH))
    .style(crate::theme::m3_menu_card);

    let positioned = container(menu_card)
        .padding(iced::Padding {
            top: effective_y,
            left: effective_x,
            right: 0.0,
            bottom: 0.0,
        })
        .width(Length::Fill)
        .height(Length::Fill);

    let click_away = mouse_area(
        container(Space::new())
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(Message::DismissContextMenu);

    Some(stack![click_away, positioned].into())
}

/// Render the delete-confirmation dialog for one or more torrents.
pub(crate) fn view_delete_dialog(
    ids: &[i64],
    torrents: &[crate::rpc::TorrentData],
    del_local: bool,
) -> Element<'static, Message> {
    let title = if ids.len() == 1 {
        let name = torrents
            .iter()
            .find(|t| Some(t.id) == ids.first().copied())
            .map(|t| t.name.as_str())
            .unwrap_or("this torrent");
        format!("Delete \"{}\"?", name)
    } else {
        format!("Delete {} torrents?", ids.len())
    };

    let card = container(
        column![
            text(title).size(18),
            text("This cannot be undone.").size(13),
            checkbox(del_local)
                .label("Also delete local data")
                .on_toggle(Message::DeleteLocalDataToggled),
            row![
                Space::new().width(Length::Fill),
                button("Cancel")
                    .on_press(Message::DeleteCancelled)
                    .padding([10, 24])
                    .style(crate::theme::m3_tonal_button),
                button("Confirm Delete")
                    .on_press(Message::DeleteConfirmed)
                    .padding([10, 24])
                    .style(crate::theme::danger_pill_button),
            ]
            .spacing(8)
            .width(Length::Fill),
        ]
        .spacing(16),
    )
    .padding(28)
    .max_width(400.0)
    .style(crate::theme::dialog_card);

    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(crate::theme::dialog_scrim(0.70))
        .into()
}

/// Render the set-location dialog for the selected torrent.
pub(crate) fn view_set_location_dialog(dlg: &SetLocationDialog) -> Element<'_, Message> {
    use iced::widget::text_input;

    let card = container(
        column![
            text("Set Data Location").size(18),
            text("Destination path on the daemon's filesystem"),
            text_input("/path/to/data", &dlg.path)
                .on_input(Message::SetLocationPathChanged)
                .on_submit(Message::SetLocationApply)
                .padding([12, 16])
                .style(crate::theme::m3_text_input),
            checkbox(dlg.move_data)
                .label("Move data to new location")
                .on_toggle(|_| Message::SetLocationMoveToggled),
            row![
                Space::new().width(Length::Fill),
                button("Cancel")
                    .on_press(Message::SetLocationCancel)
                    .padding([10, 24])
                    .style(crate::theme::m3_tonal_button),
                button("Apply")
                    .on_press(Message::SetLocationApply)
                    .padding([10, 24])
                    .style(crate::theme::m3_primary_button),
            ]
            .spacing(8)
            .width(Length::Fill),
        ]
        .spacing(16),
    )
    .padding(28)
    .max_width(440.0)
    .style(crate::theme::m3_card);

    container(card)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(crate::theme::dialog_scrim(0.70))
        .into()
}
