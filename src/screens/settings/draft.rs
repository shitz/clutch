//! Profile draft — an in-memory editable copy of a connection profile.

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
