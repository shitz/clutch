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

use iced::Subscription;
use iced::keyboard::{Event, Key, key::Named};

use crate::screens::{connection, settings};

use super::{AppState, Message, Screen};

#[derive(Clone, Copy)]
enum FormScreen {
    Connection,
    Settings,
}

pub(super) fn subscription(state: &AppState) -> Subscription<Message> {
    let dialog_active = state.active_dialog.is_some();

    match &state.screen {
        Screen::Connection(_) => connection_subscription(dialog_active),
        Screen::Settings(_) => settings_subscription(dialog_active),
        Screen::Main(main) => main.subscription().map(Message::Main),
    }
}

fn connection_subscription(dialog_active: bool) -> Subscription<Message> {
    iced::keyboard::listen()
        .with(dialog_active)
        .filter_map(connection_key_event)
}

fn settings_subscription(dialog_active: bool) -> Subscription<Message> {
    iced::keyboard::listen()
        .with(dialog_active)
        .filter_map(settings_key_event)
}

fn connection_key_event((dialog_active, event): (bool, Event)) -> Option<Message> {
    map_key_event(FormScreen::Connection, dialog_active, event)
}

fn settings_key_event((dialog_active, event): (bool, Event)) -> Option<Message> {
    map_key_event(FormScreen::Settings, dialog_active, event)
}

fn map_key_event(screen: FormScreen, dialog_active: bool, event: Event) -> Option<Message> {
    if let Event::KeyPressed { key, modifiers, .. } = event {
        match key.as_ref() {
            Key::Named(Named::Tab) if dialog_active => Some(Message::AuthTabKeyPressed {
                shift: modifiers.shift(),
            }),
            Key::Named(Named::Enter)
                if dialog_active && !modifiers.control() && !modifiers.alt() =>
            {
                Some(Message::AuthEnterPressed)
            }
            Key::Named(Named::Tab) => Some(screen_tab_message(screen, modifiers.shift())),
            Key::Named(Named::Enter) if !modifiers.control() && !modifiers.alt() => {
                Some(screen_enter_message(screen))
            }
            _ => None,
        }
    } else {
        None
    }
}

fn screen_tab_message(screen: FormScreen, shift: bool) -> Message {
    match screen {
        FormScreen::Connection => Message::Connection(connection::Message::TabKeyPressed { shift }),
        FormScreen::Settings => Message::Settings(settings::Message::TabKeyPressed { shift }),
    }
}

fn screen_enter_message(screen: FormScreen) -> Message {
    match screen {
        FormScreen::Connection => Message::Connection(connection::Message::EnterPressed),
        FormScreen::Settings => Message::Settings(settings::Message::EnterPressed),
    }
}
