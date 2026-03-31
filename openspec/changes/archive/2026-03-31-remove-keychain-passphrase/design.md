## Context

Clutch currently stores Transmission passwords exclusively in the OS keychain via the `keyring` crate. This creates two hard blockers for distribution:

1. **Code-signing entitlements**: On macOS, Keychain access via the Security framework requires a signed application with proper entitlements. Distributing an unsigned binary (e.g. direct download or Homebrew) silently loses all saved passwords.
2. **Platform availability**: On headless Linux systems or in containerised environments the secret-service daemon (used by `keyring` on non-macOS platforms) may not exist at all, causing the crate to return errors on every operation.

The replacement stores passwords encrypted directly inside `config.toml`. Encryption uses Argon2id as the KDF (to derive a 32-byte key from the master passphrase + a per-profile random salt) and ChaCha20-Poly1305 as the AEAD cipher. The master passphrase is entered once per session and kept in memory; it is never written to disk.

## Goals / Non-Goals

**Goals:**

- Remove the `keyring` crate and all OS keychain calls.
- Store encrypted passwords inside `config.toml` alongside their salt and nonce (all base64-encoded).
- Add a master passphrase that the user creates on first save and enters once per session to unlock credentials.
- Preserve the existing UX for password-free profiles — no passphrase dialog is shown when a profile has no password.
- Provide a clear first-time setup flow and a per-session unlock flow as modal overlays.

**Non-Goals:**

- Automatic migration of existing keychain entries (users re-enter passwords once).
- Multi-user or multi-device secret sharing.
- Hardware key or biometric authentication.
- Changing the TOML config format for non-password fields.

## Decisions

### Decision: Argon2id + ChaCha20-Poly1305

**Chosen**: `argon2` crate (Argon2id variant) for KDF; `chacha20poly1305` crate for AEAD.

**Rationale**: Both are from the RustCrypto organisation with audited implementations. Argon2id is the current OWASP-recommended password hashing algorithm; it is memory-hard and resists GPU/ASIC brute-force. ChaCha20-Poly1305 provides authenticated encryption (any tampering of the ciphertext or metadata in the config file is detected and rejected). The combination is the same stack used in WireGuard and modern TLS.

**Alternatives considered**:

- AES-256-GCM: requires AES hardware acceleration for performance parity; ChaCha20 is constant-time in software.
- bcrypt/scrypt: less actively maintained in the Rust ecosystem; Argon2id is the IETF standard (RFC 9106).
- OS keychain (status quo): eliminated due to distribution and availability issues documented above.

### Decision: Per-profile random salt

**Chosen**: Each `ConnectionProfile` stores its own 16-byte random salt alongside the 12-byte nonce and ciphertext.

**Rationale**: A unique salt per profile ensures that identical passwords on different profiles produce different derived keys and ciphertexts. This prevents offline correlation attacks if the config file is exposed.

**Alternatives considered**:

- Single global salt: simpler storage, but allows dictionary attacks to target all profiles simultaneously.

### Decision: Master passphrase verified via Argon2 hash, not stored plaintext

**Chosen**: `ProfileStore.master_passphrase_hash` stores the Argon2id PHC hash string (produced by `argon2::hash_encoded`) of the master passphrase. Verification uses `argon2::verify_encoded` at runtime.

**Rationale**: The PHC string embeds the salt and parameters, so only one field is needed. Argon2 is cryptographically appropriate for passphrase verification. Empty/missing `master_passphrase_hash` means no passphrase has been set yet.

**Alternatives considered**:

- Encrypt a known sentinel value with the derived key: works but requires an extra dummy ciphertext in the config; PHC string approach is cleaner.

### Decision: Packed string storage format for encrypted passwords

**Chosen**: `ConnectionProfile.encrypted_password: Option<String>` stores `"salt_b64$nonce_b64$ciphertext_b64"` as a single URL-safe no-padding base64 string.

**Rationale**: An early implementation used a nested `EncryptedCredentials` struct, which TOML serialized as a `[profiles.encrypted_password]` sub-table. This broke deserialization within a `[[profiles]]` array. The packed string stores all three fields atomically as a single scalar TOML value, fully compatible with array-of-tables. The `$` separator is safe because URL-safe no-padding base64 never contains it.

**Alternatives considered**:

- Named sub-table `[encrypted_password]`: broken in `[[profiles]]` arrays.
- Three separate fields (`encrypted_salt`, `encrypted_nonce`, `encrypted_ciphertext`): verbose, no correctness benefit.

### Decision: Session-scoped passphrase zeroized via `secrecy::SecretString`

**Chosen**: `AppState.unlocked_passphrase: Option<secrecy::SecretString>` holds the master passphrase for the duration of the process lifetime. It starts as `None` every launch.

**Rationale**: `SecretString` wraps a `String` with a `Zeroize` impl, overwriting the memory before deallocation. This means the master passphrase is scrubbed the moment the field is dropped (app exit or future logout). It also prevents accidental `Debug` / `Display` exposure. All read sites use `.expose_secret().as_str()` to make the access explicit.

**Alternatives considered**:

- Plain `Option<String>`: passphrase bytes remain in deallocated memory until overwritten by the allocator — unacceptable for a master key.
- Re-prompt for every connection: more secure but poor UX; out of scope.
- Persist derived key to a temp file: new disk-based secret with its own lifecycle management.

### Decision: `is_processing` guard on auth dialogs

**Chosen**: Both `AuthDialog::SetupPassphrase` and `AuthDialog::Unlock` carry an `is_processing: bool` field. It is set to `true` when the background Argon2 task is dispatched and reset to `false` on failure. While `true`, submit and cancel buttons have no `on_press` handler and text inputs have no `on_input` handler.

**Rationale**: Argon2id (m=19456, t=2) takes ~100–300 ms on a modern CPU. Without the guard a user can double-click the submit button and spawn multiple concurrent `spawn_blocking` tasks, each producing a `UnlockPassphraseResult` or `SetupPassphraseComplete` message and applying it to state independently, leading to duplicate writes and a stale `active_dialog`.

**Alternatives considered**:

- Drop duplicate messages in `update()`: fragile; requires tracking in-flight task count.
- Disable only the submit button: cancel could still fire and dismiss the dialog mid-KDF, leaving an orphaned `spawn_blocking` thread.

### Decision: `has_saved_password` + `TestConnectionWithId` for settings test flow

**Chosen**: `ProfileDraft.has_saved_password: bool` is set from `profile.encrypted_password.is_some()`. When the test-connection button is clicked and `has_saved_password && !password_changed`, settings emits `SettingsResult::TestConnectionWithId { profile_id }` instead of building credentials itself. `app::update` handles this by decrypting via `unlocked_passphrase` and firing the RPC probe, routing `TestConnectionResult` back to settings.

**Rationale**: The settings screen never has access to the cleartext passphrase — it is held exclusively in `AppState`. A settings-level test using `to_credentials()` would silently omit the password when the draft password field is empty (write-only UX). Delegating the probe to `app::update` keeps the credential boundary clean. The placeholder "••••••••" in the password field (shown when `has_saved_password && !password_changed`) makes the write-only semantics visible to the user.

**Alternatives considered**:

- Pass the passphrase down to settings: violates the principle of least privilege.
- Always clear encrypted_password on settings open: would wipe stored passwords if user opens and closes settings without re-entering the password.

## Risks / Trade-offs

- **Config file as single point of failure** → The encrypted passwords live in `config.toml`. If the file is deleted the passwords are lost. Mitigation: this is no worse than the keychain situation, and users can always re-enter passwords.
- **Forgotten master passphrase means permanent credential loss** → Unlike a keychain, there is no OS-level recovery path. Mitigation: document this clearly in the UI copy; the app degrades gracefully (profile still exists, user just re-enters password and passphrase).
- **Argon2 memory cost causes perceptible pause** → Default parameters (`m=19456`, `t=2`, `p=1`) target ~80 ms on modern hardware, which is acceptable at session unlock. Mitigation: run KDF on a blocking task (`tokio::task::spawn_blocking`) to avoid freezing the UI frame loop.
- **First-time users have no migration path** → Existing keychain entries are silently abandoned. Mitigation: users re-enter passwords on next use; the app degrades gracefully without a password until they do.

## Migration Plan

1. Ship the updated binary. On first launch the app reads the existing `config.toml` (profiles load normally; no passwords are present since they were in the keychain).
2. When the user opens Manage Profiles and edits a profile with a password, the new save flow triggers the master passphrase setup (or unlock) dialog.
3. Old keychain entries are orphaned and can be cleaned up manually via the OS keychain utility; the app no longer reads or writes them.

**Rollback**: Revert to the previous binary. The keychain entries are still present (the new version never deleted them). The config fields `encrypted_password` and `master_passphrase_hash` are ignored by the old version (unknown TOML keys are skipped by `serde`).

## Open Questions

None.
