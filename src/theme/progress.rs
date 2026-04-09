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

//! Progress-bar styles keyed by torrent activity state.

use iced::widget::progress_bar;
use iced::{Border, Color, Theme};

use super::{
    PROGRESS_DOWNLOAD_BAR, PROGRESS_DOWNLOAD_TRACK_DARK, PROGRESS_DOWNLOAD_TRACK_LIGHT,
    PROGRESS_PAUSED_BAR_DARK, PROGRESS_PAUSED_BAR_LIGHT, PROGRESS_PAUSED_TRACK_DARK,
    PROGRESS_PAUSED_TRACK_LIGHT, PROGRESS_SEED_BAR, PROGRESS_SEED_TRACK_DARK,
    PROGRESS_SEED_TRACK_LIGHT,
};

/// Return a progress bar style closure based on the torrent's Transmission status code.
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
