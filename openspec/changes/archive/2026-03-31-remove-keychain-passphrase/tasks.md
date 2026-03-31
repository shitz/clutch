## 1. Dependencies and Config Model

- [x] 1.1 Remove `keyring` from `Cargo.toml`; add `argon2`, `chacha20poly1305`, `rand`, and `base64` dependencies
- [x] 1.2 Add `EncryptedCredentials` struct (salt, nonce, ciphertext as `String`) with `Serialize`/`Deserialize` to `src/profile.rs`
- [x] 1.3 Add `encrypted_password: Option<EncryptedCredentials>` field to `ConnectionProfile`
- [x] 1.4 Add `master_passphrase_hash: Option<String>` field to `ProfileStore`

## 2. Crypto Helpers

- [x] 2.1 Implement `encrypt_password(passphrase: &str, plaintext: &str) -> EncryptedCredentials` in `src/profile.rs` using Argon2id KDF + ChaCha20-Poly1305 with a fresh random salt and nonce
- [x] 2.2 Implement `decrypt_password(passphrase: &str, creds: &EncryptedCredentials) -> Option<String>` returning `None` on authentication failure
- [x] 2.3 Implement `hash_passphrase(passphrase: &str) -> String` using `argon2::hash_encoded` (Argon2id, m≥19456, t≥2)
- [x] 2.4 Implement `verify_passphrase(passphrase: &str, hash: &str) -> bool` using `argon2::verify_encoded`
- [x] 2.5 Remove `get_password`, `set_password`, `delete_password` keyring helpers and the `KEYRING_SERVICE` constant from `ProfileStore`

## 3. Profile Credentials Flow

- [x] 3.1 Update `ConnectionProfile::credentials()` to accept `passphrase: Option<&str>` and decrypt `encrypted_password` when present; fall back to `None` password if no passphrase is provided or decryption fails
- [x] 3.2 Ensure profile deletion no longer calls `delete_password` (remove all keyring cleanup code from the settings update logic)

## 4. App State

- [x] 4.1 Add `unlocked_passphrase: Option<String>` field to `AppState` (starts as `None`)
- [x] 4.2 Add `AuthDialog` enum with variants `SetupPassphrase { pending_profile: ConnectionProfile, pending_password: String }` and `Unlock { pending_action: PendingAction }`
- [x] 4.3 Add `PendingAction` enum with `ConnectToProfile(Uuid)` variant
- [x] 4.4 Add `active_dialog: Option<AuthDialog>` field to `AppState`
- [x] 4.5 Add messages: `OpenSetupPassphraseDialog`, `OpenUnlockDialog(PendingAction)`, `SubmitSetupPassphrase { passphrase: String, confirm: String }`, `SubmitUnlockPassphrase(String)`, `DismissAuthDialog`

## 5. Passphrase Dialog UI

- [x] 5.1 Implement `view_setup_passphrase_dialog` in `src/screens/` rendering a modal overlay with two masked text inputs and "Create" / "Cancel" buttons
- [x] 5.2 Implement `view_unlock_dialog` rendering a modal overlay with one masked text input and "Unlock" / "Cancel" buttons
- [x] 5.3 Wire both dialogs into the top-level `App::view` so they render as an overlay when `active_dialog` is `Some`

## 6. Save Profile Flow (Settings Screen)

- [x] 6.1 Update the save-profile handler in `src/screens/settings/update.rs`: if password is non-empty and `master_passphrase_hash` is `None`, dispatch `OpenSetupPassphraseDialog` instead of saving immediately
- [x] 6.2 Update the save-profile handler: if password is non-empty and `master_passphrase_hash` is `Some` but `unlocked_passphrase` is `None`, dispatch `OpenUnlockDialog` instead of saving
- [x] 6.3 Handle `SubmitSetupPassphrase` in `App::update`: validate confirmation match, call `hash_passphrase`, set `master_passphrase_hash`, set `unlocked_passphrase`, encrypt pending password, save profile, dismiss dialog
- [x] 6.4 Handle `SubmitUnlockPassphrase` in `App::update`: call `verify_passphrase`; on success set `unlocked_passphrase` and resume pending action; on failure show inline error in dialog

## 7. Connect Flow (Connection Screen)

- [x] 7.1 Update connect handler in `src/screens/connection.rs`: if selected profile has `encrypted_password` and `unlocked_passphrase` is `None`, dispatch `OpenUnlockDialog(ConnectToProfile(id))` instead of connecting
- [x] 7.2 After successful `SubmitUnlockPassphrase`, execute pending `ConnectToProfile` action automatically

## 8. Cleanup and Verification

- [x] 8.1 Search codebase for any remaining `keyring::` references and remove them
- [x] 8.2 Run `cargo build` and resolve all compile errors
- [x] 8.3 Manually test: save a profile with a password (setup flow), close and reopen app (unlock flow), connect successfully with decrypted password
- [x] 8.4 Manually test: cancel setup dialog — profile is saved without password; cancel unlock dialog — no connection is made
