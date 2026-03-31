## ADDED Requirements

### Requirement: Encrypted password storage in config

Each `ConnectionProfile` SHALL store an optional `encrypted_password: Option<String>` field in `config.toml`. The field, when present, SHALL contain a packed string `"salt_b64$nonce_b64$ciphertext_b64"` encoded with URL-safe no-padding base64 (`URL_SAFE_NO_PAD`). All three components (16-byte random salt, 12-byte nonce, ciphertext including 16-byte Poly1305 tag) are stored in this single scalar value. The password in plaintext SHALL NOT appear anywhere in the config file.

#### Scenario: Profile saved with password is persisted encrypted

- **WHEN** the user saves a profile with a non-empty password
- **THEN** `config.toml` contains `encrypted_password` as a packed `"salt$nonce$ciphertext"` string (URL-safe base64, no padding, `$`-delimited) instead of the plaintext password

#### Scenario: Profile saved without password has no encrypted field

- **WHEN** the user saves a profile with an empty password
- **THEN** `config.toml` contains no `encrypted_password` field for that profile

### Requirement: Master passphrase verification hash

`ProfileStore` SHALL store an optional `master_passphrase_hash` field in `config.toml` containing the Argon2id PHC hash string of the master passphrase. The hash SHALL be produced using `argon2::hash_encoded` with Argon2id variant and OWASP-recommended parameters (m≥19456, t≥2). The cleartext passphrase SHALL NOT appear in the config file.

#### Scenario: Hash written on first passphrase setup

- **WHEN** the user creates a master passphrase for the first time
- **THEN** `config.toml` contains a non-empty `master_passphrase_hash` field valid for `argon2::verify_encoded`

#### Scenario: Correct passphrase is accepted

- **WHEN** the user enters the correct master passphrase at the unlock dialog
- **THEN** `argon2::verify_encoded` returns `Ok(true)` and the session is unlocked

#### Scenario: Incorrect passphrase is rejected

- **WHEN** the user enters an incorrect master passphrase at the unlock dialog
- **THEN** `argon2::verify_encoded` returns `Ok(false)` and the dialog shows an error message

### Requirement: Argon2id key derivation for encryption and decryption

To encrypt or decrypt a profile password, the system SHALL derive a 32-byte key using Argon2id from the master passphrase and the profile-specific 16-byte random salt. Derivation SHALL execute on a blocking thread (`tokio::task::spawn_blocking`) to avoid blocking the iced frame loop.

#### Scenario: Derived key is used for AEAD encryption

- **WHEN** the user saves a profile with a password and the master passphrase is unlocked
- **THEN** the system derives a 32-byte key via Argon2id and encrypts the password with ChaCha20-Poly1305

#### Scenario: Derived key is used for AEAD decryption

- **WHEN** the app needs to connect using a profile that has an `encrypted_password`
- **THEN** the system derives the same 32-byte key (same passphrase + stored salt) and decrypts the ciphertext with ChaCha20-Poly1305

### Requirement: Tamper detection via authenticated encryption

ChaCha20-Poly1305 SHALL be used for all encryption/decryption. Any modification to the ciphertext, nonce, or salt stored in `config.toml` SHALL cause decryption to return an error, which the app SHALL surface as a credential-load failure (profile connects without a password, and an error is logged).

#### Scenario: Modified ciphertext causes decryption error

- **WHEN** the ciphertext in `config.toml` has been altered
- **THEN** ChaCha20-Poly1305 decryption returns an authentication error
- **THEN** the app logs the error and proceeds without a password for that profile

### Requirement: Session-scoped passphrase zeroized in memory

The master passphrase SHALL be held in `AppState.unlocked_passphrase: Option<secrecy::SecretString>` for the duration of the process lifetime. `SecretString` zeroizes its backing memory on drop, ensuring the passphrase is scrubbed when the app exits or the field is cleared. It SHALL start as `None` on every fresh launch. There SHALL be no explicit lock/logout action.

#### Scenario: Passphrase is None at startup

- **WHEN** the app launches
- **THEN** `unlocked_passphrase` is `None`

#### Scenario: Passphrase persists within a session

- **WHEN** the user has unlocked the passphrase during a session
- **THEN** subsequent connections or profile saves within the same session do not prompt for the passphrase again

#### Scenario: Passphrase memory is zeroized on drop

- **WHEN** the app process exits
- **THEN** the `SecretString` destructor overwrites the passphrase bytes before returning memory to the allocator

### Requirement: Double-submission guard on passphrase dialogs

While a passphrase dialog has a background Argon2 task in flight, the dialog SHALL enter a processing state (`is_processing: true`) that disables all inputs and action buttons. The submit button label SHALL change to "Creating…" or "Verifying…" to communicate progress. A second submit SHALL be a no-op.

#### Scenario: Submit button is disabled while processing

- **WHEN** the user clicks "Create" or "Unlock"
- **AND** the background KDF task is running
- **THEN** the submit and cancel buttons have no press handler (visually inert)
- **THEN** the text inputs have no input handler

#### Scenario: Failed unlock re-enables the dialog

- **WHEN** the passphrase verify task returns `valid: false`
- **THEN** `is_processing` is reset to `false`
- **THEN** the error message is shown and inputs are re-enabled

### Requirement: Write-only password indicator in Settings

The password field in the profile editor SHALL display a `"••••••••"` placeholder when the profile has a saved encrypted password and the user has not typed a new one (i.e. `has_saved_password && !password_changed`). Clicking into the field and typing replaces the stored password. Clearing the field and saving removes the password.

#### Scenario: Saved password shows placeholder

- **WHEN** the user opens a profile that has an encrypted password
- **THEN** the password field shows `"••••••••"` placeholder text (not the decrypted password)

#### Scenario: New profile password shows placeholder immediately after save

- **WHEN** the user saves a new profile with a password
- **THEN** the password field shows `"••••••••"` placeholder immediately, without waiting for the async encryption to complete

### Requirement: Test Connection uses decrypted credentials when password is unchanged

When the user clicks "Test Connection" on a profile with a saved password and has not entered a new draft password, settings SHALL emit `SettingsResult::TestConnectionWithId { profile_id }`. `app::update` SHALL decrypt the password using `unlocked_passphrase` and fire the RPC probe, routing `TestConnectionResult` back to the settings screen.

#### Scenario: Test Connection uses existing encrypted password

- **WHEN** the user clicks "Test Connection" on a profile with a saved password
- **AND** the password draft field has not been changed
- **THEN** the app decrypts the stored password using the session passphrase
- **THEN** the RPC probe uses the decrypted password

#### Scenario: Test Connection uses draft password when changed

- **WHEN** the user types a new password in the draft field and clicks "Test Connection"
- **THEN** the RPC probe uses the typed draft password (unencrypted, for immediate testing)

When the user attempts to save a profile with a non-empty password and no `master_passphrase_hash` exists, the app SHALL show a modal overlay prompting the user to create a master passphrase. The dialog SHALL include two masked text inputs (passphrase and confirmation) and "Create" and "Cancel" buttons.

#### Scenario: Setup dialog appears on first password-protected save

- **WHEN** the user clicks Save on a profile with a password and no master passphrase has been configured
- **THEN** a modal overlay is shown with "Create a master passphrase" heading, two masked inputs, and "Create" / "Cancel" buttons

#### Scenario: Mismatched confirmation is rejected

- **WHEN** the user enters different values in the passphrase and confirmation inputs
- **THEN** the dialog shows an inline error "Passphrases do not match" and the save is not performed

#### Scenario: Successful setup saves hash and encrypts password

- **WHEN** the user enters matching passphrases and clicks "Create"
- **THEN** `master_passphrase_hash` is written to `config.toml`
- **THEN** `unlocked_passphrase` is set in session memory
- **THEN** the profile password is encrypted and saved
- **THEN** the dialog is dismissed

#### Scenario: Cancelling setup aborts the save

- **WHEN** the user clicks "Cancel" on the setup dialog
- **THEN** the dialog is dismissed and the profile is saved without an encrypted password

### Requirement: Passphrase unlock dialog

When the app needs to decrypt a profile password (for connecting) and `unlocked_passphrase` is `None`, the app SHALL show a modal overlay prompting the user to enter their master passphrase. The dialog SHALL include one masked text input and "Unlock" and "Cancel" buttons.

#### Scenario: Unlock dialog appears when passphrase is needed

- **WHEN** the user initiates a connection to a profile with an `encrypted_password` and `unlocked_passphrase` is `None`
- **THEN** a modal overlay is shown with "Enter master passphrase" heading, one masked input, and "Unlock" / "Cancel" buttons

#### Scenario: Successful unlock resumes connection

- **WHEN** the user enters the correct passphrase and clicks "Unlock"
- **THEN** `unlocked_passphrase` is set in session memory
- **THEN** the connection attempt proceeds with the decrypted password

#### Scenario: Incorrect passphrase shows error without dismissing

- **WHEN** the user enters an incorrect passphrase and clicks "Unlock"
- **THEN** the dialog shows an inline error "Incorrect passphrase"
- **THEN** the dialog remains open

#### Scenario: Cancelling unlock dialog aborts the connection

- **WHEN** the user clicks "Cancel" on the unlock dialog
- **THEN** the dialog is dismissed and no connection attempt is made
