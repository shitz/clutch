## Why

Clutch currently stores Transmission passwords in the OS keychain, which requires code-signing entitlements for distribution and fails silently on systems where a keychain daemon is unavailable (e.g. headless Linux or sandboxed environments). Replacing the keychain with passphrase-based encryption in the config file removes the code-signing dependency and makes the app work reliably on all supported platforms.

## What Changes

- **BREAKING**: Passwords are no longer stored in the OS keychain. Existing keychain entries will not be migrated automatically — users must re-enter passwords after the first launch of the updated app.
- Remove the `keyring` crate dependency from `Cargo.toml`.
- Add `argon2`, `chacha20poly1305`, `rand`, and `base64` crates for KDF + AEAD encryption.
- `ConnectionProfile` gains an optional `encrypted_password` field (salt + nonce + ciphertext, base64-encoded, stored in `config.toml`).
- `ProfileStore` gains an optional `master_passphrase_hash` field used to verify the master passphrase without storing it in plaintext.
- A new in-memory `unlocked_passphrase: Option<String>` field in `AppState` holds the master passphrase for the duration of the session.
- New UI modal for setting up a master passphrase (first use) and unlocking it (subsequent launches).
- Saving a profile with a password always encrypts it using the master passphrase before writing to disk.
- Connecting to a password-protected profile decrypts the password on-demand using the in-memory passphrase.
- Profile deletion removes the encrypted password from the config file (no keychain cleanup needed).

## Capabilities

### New Capabilities

- `credential-encryption`: Passphrase-based credential encryption system — Argon2 KDF + ChaCha20Poly1305 AEAD, config-file storage, session-scoped unlock flow, and master passphrase setup/verify UI modal.

### Modified Capabilities

- `connection-screen`: Extends saved-profiles flow with a passphrase unlock overlay (prompted before connecting to a password-protected profile) and a master-passphrase setup flow (prompted when saving the first password-protected profile).

## Impact

- `src/profile.rs`: Remove keyring helpers; add `encrypted_password` and `master_passphrase_hash` fields with encrypt/decrypt/verify functions.
- `src/app.rs`: Add `unlocked_passphrase: Option<String>` and `active_dialog: Option<AuthDialog>` to `AppState`; wire up unlock/setup messages.
- `src/screens/settings/`: Update save-profile flow to detect passphrase state and trigger setup or unlock dialog before encrypting.
- `src/screens/connection.rs`: Update connect flow to trigger unlock dialog when a profile has an encrypted password and no passphrase is in session.
- `Cargo.toml`: Remove `keyring`; add `argon2`, `chacha20poly1305`, `rand`, `base64`.
- No changes to RPC layer or torrent-list functionality.
