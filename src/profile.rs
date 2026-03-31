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

//! [`ProfileStore`] is the root config structure, serialized as TOML to the
//! OS-appropriate config directory (e.g. `~/.config/clutch/config.toml` on Linux,
//! `~/Library/Application Support/clutch/config.toml` on macOS). Passwords are
//! stored encrypted inside the config file, protected by a master passphrase.
//! The master passphrase is held in memory for the session and never written to disk.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::crypto;

// ── Theme preference ──────────────────────────────────────────────────────────

/// User-visible theme preference stored in the config file.
///
/// `System` is resolved to a concrete [`crate::app::ThemeMode`] at startup
/// (or immediately when the user selects it at runtime) by calling
/// [`resolve_theme_config`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeConfig {
    Light,
    Dark,
    #[default]
    System,
}

/// Resolve a [`ThemeConfig`] to a concrete [`crate::app::ThemeMode`].
///
/// Calls `dark_light::detect()` exactly once if `config` is `ThemeConfig::System`.
/// Falls back to `ThemeMode::Light` if detection is unavailable or returns `Default`.
pub fn resolve_theme_config(config: ThemeConfig) -> crate::app::ThemeMode {
    match config {
        ThemeConfig::Light => crate::app::ThemeMode::Light,
        ThemeConfig::Dark => crate::app::ThemeMode::Dark,
        ThemeConfig::System => match dark_light::detect() {
            dark_light::Mode::Dark => crate::app::ThemeMode::Dark,
            _ => crate::app::ThemeMode::Light,
        },
    }
}

// ── General settings ──────────────────────────────────────────────────────────

/// Application-wide preferences stored in the `[general]` config section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralSettings {
    /// User's theme preference. Resolved to Light/Dark at runtime.
    pub theme: ThemeConfig,
    /// Daemon refresh interval in seconds (1–30).
    pub refresh_interval: u8,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            theme: ThemeConfig::System,
            refresh_interval: 1,
        }
    }
}

// ── Connection profile ────────────────────────────────────────────────────────

/// A saved Transmission daemon connection.
///
/// The optional password is stored encrypted in [`encrypted_password`].
/// It is decrypted on demand using the session-scoped master passphrase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionProfile {
    /// UUID v4 — stable identifier. Generated once at profile creation.
    pub id: Uuid,
    /// Human-readable display name (e.g. "Home NAS").
    pub name: String,
    /// Hostname or IP address of the Transmission daemon.
    pub host: String,
    /// TCP port of the Transmission RPC endpoint (default 9091).
    pub port: u16,
    /// Optional Basic Auth username.
    pub username: Option<String>,
    /// Encrypted Transmission password, or `None` when no password is set.
    ///
    /// Packed format: `"salt_b64$nonce_b64$ciphertext_b64"` — a single TOML string
    /// value, avoiding sub-table serialization issues.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encrypted_password: Option<String>,
}

impl ConnectionProfile {
    /// Create a new blank profile with a fresh UUID and sensible defaults.
    pub fn new_blank() -> Self {
        Self {
            id: Uuid::new_v4(),
            name: "New Profile".to_owned(),
            host: "localhost".to_owned(),
            port: 9091,
            username: None,
            encrypted_password: None,
        }
    }

    /// Build [`crate::rpc::TransmissionCredentials`] for this profile.
    ///
    /// If the profile has an `encrypted_password` and a `passphrase` is
    /// provided, the password is decrypted on the spot. If decryption fails
    /// (wrong passphrase or tampered data) the connection proceeds without a
    /// password and a warning is logged.
    pub fn credentials(&self, passphrase: Option<&str>) -> crate::rpc::TransmissionCredentials {
        let password = match (&self.encrypted_password, passphrase) {
            (Some(packed), Some(pw)) => {
                let decrypted = crypto::decrypt_password(pw, packed);
                if decrypted.is_none() {
                    tracing::warn!(profile = %self.id, "Password decryption failed; connecting without password");
                }
                decrypted
            }
            _ => None,
        };
        crate::rpc::TransmissionCredentials {
            host: self.host.clone(),
            port: self.port,
            username: self.username.clone(),
            password,
        }
    }
}

// ── Profile store ─────────────────────────────────────────────────────────────

const CONFIG_FILE: &str = "config.toml";
const CONFIG_DIR: &str = "clutch";

/// The root in-memory config structure, persisted to TOML.
///
/// Load with [`ProfileStore::load`] at startup via `Task::perform`.
/// Persist with [`ProfileStore::save`] after any mutation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProfileStore {
    /// UUID of the most recently successfully connected profile.
    pub last_connected: Option<Uuid>,
    /// Argon2id PHC hash string of the master passphrase, or `None` if not yet configured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub master_passphrase_hash: Option<String>,
    /// Application-wide preferences.
    #[serde(default)]
    pub general: GeneralSettings,
    /// All saved connection profiles (including encrypted passwords).
    #[serde(default)]
    pub profiles: Vec<ConnectionProfile>,
}

impl ProfileStore {
    // ── Config-file helpers ───────────────────────────────────────────────────

    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join(CONFIG_DIR).join(CONFIG_FILE))
    }

    // ── I/O ───────────────────────────────────────────────────────────────────

    /// Load the store from disk synchronously.
    ///
    /// Used during app initialisation (before the async runtime is processing
    /// UI frames) to ensure the correct theme is applied from the very first
    /// draw. The async [`Self::load`] is still called afterward to ensure
    /// `tracing` messages are emitted and any parse errors are handled.
    pub fn load_sync() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };
        match std::fs::read_to_string(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(_) => Self::default(),
            Ok(content) => toml::from_str::<Self>(&content).unwrap_or_default(),
        }
    }

    /// Persist the store to disk atomically (write to `.tmp`, then rename).
    pub async fn save(&self) -> std::io::Result<()> {
        let Some(path) = Self::config_path() else {
            tracing::warn!("Cannot determine config directory; skipping save");
            return Ok(());
        };
        let dir = path.parent().expect("config path has parent");
        tokio::fs::create_dir_all(dir).await?;
        let content = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let tmp = path.with_extension("toml.tmp");
        tokio::fs::write(&tmp, &content).await?;
        tokio::fs::rename(&tmp, &path).await?;
        tracing::debug!(path = %path.display(), "Config saved");
        Ok(())
    }

    /// Find a profile by UUID.
    #[must_use]
    pub fn get(&self, id: Uuid) -> Option<&ConnectionProfile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    /// Merge fields not managed by the settings UI from `from` into `self`.
    ///
    /// Called whenever a settings-screen snapshot replaces the app-level store,
    /// so that `last_connected`, `master_passphrase_hash`, and per-profile
    /// `encrypted_password` values are never silently cleared.
    pub fn adopt_from(&mut self, from: &ProfileStore) {
        self.last_connected = from.last_connected;
        if let Some(id) = self.last_connected
            && self.get(id).is_none()
        {
            self.last_connected = None;
        }
        self.master_passphrase_hash = from.master_passphrase_hash.clone();
        // Preserve encrypted passwords for profiles whose password was not
        // changed in the settings form (draft initialises password as empty
        // and only sets `encrypted_password = None` when explicitly cleared).
        for profile in &mut self.profiles {
            if profile.encrypted_password.is_none()
                && let Some(src) = from.get(profile.id) {
                    profile.encrypted_password = src.encrypted_password.clone();
                }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_toml() -> &'static str {
        r#"
last_connected = "550e8400-e29b-41d4-a716-446655440000"

[general]
theme = "Dark"
refresh_interval = 10

[[profiles]]
id = "550e8400-e29b-41d4-a716-446655440000"
name = "Home NAS"
host = "192.168.1.10"
port = 9091
"#
    }

    /// 2.6a – Valid TOML parses correctly.
    #[test]
    fn load_parses_valid_toml() {
        let store: ProfileStore = toml::from_str(sample_toml()).unwrap();
        assert_eq!(store.profiles.len(), 1);
        assert_eq!(store.profiles[0].name, "Home NAS");
        assert_eq!(store.general.refresh_interval, 10);
        assert_eq!(store.general.theme, ThemeConfig::Dark);
        assert!(store.last_connected.is_some());
    }

    /// 2.6b – Missing file yields empty defaults (simulated by parsing empty string).
    #[test]
    fn load_empty_toml_gives_defaults() {
        let store: ProfileStore = toml::from_str("").unwrap();
        assert!(store.profiles.is_empty());
        assert_eq!(store.general.refresh_interval, 1);
        assert_eq!(store.general.theme, ThemeConfig::System);
        assert!(store.last_connected.is_none());
    }

    /// 2.6c – Corrupt TOML fails to parse.
    #[test]
    fn load_corrupt_toml_fails() {
        let result: Result<ProfileStore, _> = toml::from_str("[[not valid toml{{");
        assert!(result.is_err());
    }
}
