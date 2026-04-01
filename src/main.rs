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

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clutch::{app, theme};

fn main() -> iced::Result {
    tracing_subscriber::fmt::init();

    let icon = iced::window::icon::from_file_data(crate::theme::ICON_256_BYTES, None).ok();

    iced::application(app::AppState::new, app::update, app::view)
        .title("Clutch")
        .subscription(app::subscription)
        .theme(app::AppState::current_theme)
        // Material Icons font for toolbar and inspector glyphs
        .font(theme::MATERIAL_ICONS_BYTES)
        // iced_aw's internal font (required for Tabs widget rendering)
        .font(iced_aw::ICED_AW_FONT_BYTES)
        // Minimum window size: wide enough for all 9 torrent list columns
        .window(iced::window::Settings {
            min_size: Some(iced::Size {
                width: 900.0,
                height: 600.0,
            }),
            icon,
            ..Default::default()
        })
        .run()
}
