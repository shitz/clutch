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

use uuid::Uuid;

use crate::profile::ConnectionProfile;
use crate::rpc::TransmissionCredentials;

/// An in-memory editable copy of a [`ConnectionProfile`].
///
/// Edits mutate only this draft. The canonical profile is updated only on Save.
#[derive(Debug, Clone)]
pub struct ProfileDraft {
    pub id: Uuid,
    pub name: String,
    pub host: String,
    pub port: String,
    pub username: String,
    pub password: String,
    pub password_changed: bool,
    /// `true` when the profile already has an encrypted password stored on disk.
    /// Used to show a placeholder indicator and to route test-connection through
    /// the existing encrypted credentials when the user has not entered a new password.
    pub has_saved_password: bool,
    pub test_result: Option<TestResult>,
    pub testing: bool,
    /// Standard global download limit (KB/s) — empty means not set.
    pub speed_limit_down: String,
    /// Whether the standard global download limit is active.
    pub speed_limit_down_enabled: bool,
    /// Standard global upload limit (KB/s) — empty means not set.
    pub speed_limit_up: String,
    /// Whether the standard global upload limit is active.
    pub speed_limit_up_enabled: bool,
    /// Alternative download ceiling (KB/s) — empty string means "not set" (0).
    pub alt_speed_down: String,
    /// Alternative upload ceiling (KB/s) — empty string means "not set" (0).
    pub alt_speed_up: String,
    /// Seed-ratio limit value — empty string means "not set" (0.0).
    pub ratio_limit: String,
    /// Whether to enable the global seed-ratio limit.
    pub ratio_limit_enabled: bool,
}

/// Result of the last Test Connection probe.
#[derive(Debug, Clone)]
pub enum TestResult {
    Success,
    Failure(String),
}

impl ProfileDraft {
    pub fn from_profile(profile: &ConnectionProfile) -> Self {
        Self {
            id: profile.id,
            name: profile.name.clone(),
            host: profile.host.clone(),
            port: profile.port.to_string(),
            username: profile.username.clone().unwrap_or_default(),
            password: String::new(),
            password_changed: false,
            has_saved_password: profile.encrypted_password.is_some(),
            test_result: None,
            testing: false,
            speed_limit_down: if profile.speed_limit_down == 0 {
                String::new()
            } else {
                profile.speed_limit_down.to_string()
            },
            speed_limit_down_enabled: profile.speed_limit_down_enabled,
            speed_limit_up: if profile.speed_limit_up == 0 {
                String::new()
            } else {
                profile.speed_limit_up.to_string()
            },
            speed_limit_up_enabled: profile.speed_limit_up_enabled,
            alt_speed_down: if profile.alt_speed_down == 0 {
                String::new()
            } else {
                profile.alt_speed_down.to_string()
            },
            alt_speed_up: if profile.alt_speed_up == 0 {
                String::new()
            } else {
                profile.alt_speed_up.to_string()
            },
            ratio_limit: if profile.ratio_limit == 0.0 {
                String::new()
            } else {
                format!("{:.2}", profile.ratio_limit)
            },
            ratio_limit_enabled: profile.ratio_limit_enabled,
        }
    }

    pub fn from_blank(profile: &ConnectionProfile) -> Self {
        Self {
            id: profile.id,
            name: profile.name.clone(),
            host: profile.host.clone(),
            port: profile.port.to_string(),
            username: String::new(),
            password: String::new(),
            password_changed: false,
            has_saved_password: false,
            test_result: None,
            testing: false,
            speed_limit_down: String::new(),
            speed_limit_down_enabled: false,
            speed_limit_up: String::new(),
            speed_limit_up_enabled: false,
            alt_speed_down: String::new(),
            alt_speed_up: String::new(),
            ratio_limit: String::new(),
            ratio_limit_enabled: false,
        }
    }

    pub fn to_credentials(&self) -> Option<TransmissionCredentials> {
        let port: u16 = self.port.parse().ok()?;
        Some(TransmissionCredentials {
            host: self.host.clone(),
            port,
            username: if self.username.is_empty() {
                None
            } else {
                Some(self.username.clone())
            },
            password: if self.password.is_empty() {
                None
            } else {
                Some(self.password.clone())
            },
        })
    }
}
