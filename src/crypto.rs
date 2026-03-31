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

//! # Storage format
//!
//! Encrypted passwords are stored as a single string `"salt_b64$nonce_b64$ciphertext_b64"`
//! using URL-safe no-padding base64, making them safe to embed inline in a TOML
//! `[[profiles]]` array without creating sub-tables.
//!
//! # Algorithms
//!
//! - **KDF**: Argon2id (m=19456, t=2, p=1) — OWASP-recommended parameters.
//! - **AEAD**: ChaCha20-Poly1305 — constant-time in software, authenticated.
//! - **Passphrase hashing**: Argon2id PHC string via `password-hash` crate.

use argon2::{Algorithm, Argon2, Params, Version};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chacha20poly1305::{
    ChaCha20Poly1305, Nonce,
    aead::{Aead, KeyInit},
};
use rand::RngCore;

// ── Constants ─────────────────────────────────────────────────────────────────

const ARGON2_M_COST: u32 = 19_456; // 19 MiB
const ARGON2_T_COST: u32 = 2;
const ARGON2_P_COST: u32 = 1;
const ARGON2_KEY_LEN: usize = 32;
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;

// ── Internal helpers ──────────────────────────────────────────────────────────

fn argon2_params() -> Params {
    Params::new(
        ARGON2_M_COST,
        ARGON2_T_COST,
        ARGON2_P_COST,
        Some(ARGON2_KEY_LEN),
    )
    .expect("static Argon2 params are valid")
}

fn derive_key(passphrase: &str, salt: &[u8]) -> Option<[u8; ARGON2_KEY_LEN]> {
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon2_params());
    let mut key = [0u8; ARGON2_KEY_LEN];
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .ok()?;
    Some(key)
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Encrypt `plaintext` with a key derived from `passphrase` via Argon2id.
///
/// A fresh random salt (16 B) and nonce (12 B) are generated on every call.
/// Returns a packed string `"salt_b64$nonce_b64$ciphertext_b64"`.
pub fn encrypt_password(passphrase: &str, plaintext: &str) -> String {
    let mut salt = [0u8; SALT_LEN];
    rand::thread_rng().fill_bytes(&mut salt);
    let mut nonce_arr = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_arr);

    let key = derive_key(passphrase, &salt).expect("Argon2 key derivation failed");
    let cipher = ChaCha20Poly1305::new(chacha20poly1305::Key::from_slice(&key));
    let nonce = Nonce::from_slice(&nonce_arr);
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .expect("ChaCha20-Poly1305 encryption failed");

    format!(
        "{}${}${}",
        URL_SAFE_NO_PAD.encode(salt),
        URL_SAFE_NO_PAD.encode(nonce_arr),
        URL_SAFE_NO_PAD.encode(&ciphertext)
    )
}

/// Decrypt a packed string produced by [`encrypt_password`].
///
/// Returns `None` on wrong passphrase, tampered ciphertext, or malformed input.
pub fn decrypt_password(passphrase: &str, packed: &str) -> Option<String> {
    let mut parts = packed.splitn(3, '$');
    let salt = URL_SAFE_NO_PAD.decode(parts.next()?).ok()?;
    let nonce_bytes = URL_SAFE_NO_PAD.decode(parts.next()?).ok()?;
    let ciphertext = URL_SAFE_NO_PAD.decode(parts.next()?).ok()?;

    let key = derive_key(passphrase, &salt)?;
    let cipher = ChaCha20Poly1305::new(chacha20poly1305::Key::from_slice(&key));
    let nonce = Nonce::from_slice(&nonce_bytes);
    let plaintext = cipher.decrypt(nonce, ciphertext.as_slice()).ok()?;

    String::from_utf8(plaintext).ok()
}

/// Hash `passphrase` using Argon2id and return a PHC string.
///
/// The returned string embeds its own salt and parameters and can be stored in
/// `ProfileStore::master_passphrase_hash`. Verify with [`verify_passphrase`].
pub fn hash_passphrase(passphrase: &str) -> String {
    use argon2::{PasswordHasher, password_hash::SaltString};
    use rand::rngs::OsRng;

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, argon2_params());
    argon2
        .hash_password(passphrase.as_bytes(), &salt)
        .expect("Argon2 hash failed")
        .to_string()
}

/// Verify `passphrase` against a PHC hash string produced by [`hash_passphrase`].
///
/// Returns `false` on any error (wrong passphrase, malformed hash, etc.).
pub fn verify_passphrase(passphrase: &str, hash: &str) -> bool {
    use argon2::{PasswordVerifier, password_hash::PasswordHash};

    let Ok(parsed) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(passphrase.as_bytes(), &parsed)
        .is_ok()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── encrypt_password / decrypt_password ───────────────────────────────────

    #[test]
    fn roundtrip_basic() {
        let encrypted = encrypt_password("hunter2", "secret");
        let decrypted = decrypt_password("hunter2", &encrypted);
        assert_eq!(decrypted.as_deref(), Some("secret"));
    }

    #[test]
    fn wrong_passphrase_returns_none() {
        let encrypted = encrypt_password("correct", "secret");
        assert!(decrypt_password("wrong", &encrypted).is_none());
    }

    #[test]
    fn empty_plaintext_roundtrips() {
        let enc = encrypt_password("pass", "");
        assert_eq!(decrypt_password("pass", &enc).as_deref(), Some(""));
    }

    #[test]
    fn unicode_passphrase_and_plaintext_roundtrip() {
        let enc = encrypt_password("pässwörð✓", "mysecret🔑");
        assert_eq!(
            decrypt_password("pässwörð✓", &enc).as_deref(),
            Some("mysecret🔑")
        );
    }

    #[test]
    fn each_call_produces_different_ciphertext() {
        let a = encrypt_password("pass", "secret");
        let b = encrypt_password("pass", "secret");
        assert_ne!(a, b, "different salts/nonces must yield different output");
    }

    #[test]
    fn packed_format_has_three_dollar_separated_parts() {
        let packed = encrypt_password("pass", "secret");
        assert_eq!(
            packed.splitn(4, '$').count(),
            3,
            "expected salt$nonce$ciphertext"
        );
    }

    #[test]
    fn tampered_ciphertext_returns_none() {
        let mut packed = encrypt_password("pass", "secret");
        // Flip the last character of the ciphertext segment.
        let last = packed.pop().unwrap();
        let replacement = if last == 'A' { 'B' } else { 'A' };
        packed.push(replacement);
        assert!(decrypt_password("pass", &packed).is_none());
    }

    #[test]
    fn malformed_packed_string_returns_none() {
        assert!(decrypt_password("pass", "notbase64$notbase64$notbase64").is_none());
        assert!(decrypt_password("pass", "only_one_part").is_none());
        assert!(decrypt_password("pass", "a$b").is_none());
    }

    #[test]
    fn long_password_roundtrips() {
        let long_password = "x".repeat(10_000);
        let enc = encrypt_password("pass", &long_password);
        assert_eq!(
            decrypt_password("pass", &enc).as_deref(),
            Some(long_password.as_str())
        );
    }

    // ── hash_passphrase / verify_passphrase ───────────────────────────────────

    #[test]
    fn hash_and_verify_correct_passphrase() {
        let hash = hash_passphrase("mysecret");
        assert!(verify_passphrase("mysecret", &hash));
    }

    #[test]
    fn verify_wrong_passphrase_returns_false() {
        let hash = hash_passphrase("mysecret");
        assert!(!verify_passphrase("wrongsecret", &hash));
    }

    #[test]
    fn verify_empty_passphrase_against_empty_hash_is_false() {
        // A garbage hash string must not panic, just return false.
        assert!(!verify_passphrase("anything", "not-a-valid-phc-string"));
    }

    #[test]
    fn each_hash_call_produces_different_phc_string() {
        let h1 = hash_passphrase("same");
        let h2 = hash_passphrase("same");
        assert_ne!(h1, h2, "different salts must yield different PHC strings");
    }

    #[test]
    fn hash_is_valid_phc_format() {
        let hash = hash_passphrase("test");
        // PHC strings start with $argon2id$
        assert!(
            hash.starts_with("$argon2id$"),
            "expected PHC format, got: {hash}"
        );
    }

    #[test]
    fn unicode_passphrase_hashes_and_verifies() {
        let hash = hash_passphrase("pässwörð✓");
        assert!(verify_passphrase("pässwörð✓", &hash));
        assert!(!verify_passphrase("passworld", &hash));
    }
}
