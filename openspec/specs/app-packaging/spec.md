## ADDED Requirements

### Requirement: cargo-packager metadata in Cargo.toml

`Cargo.toml` SHALL contain a `[package.metadata.packager]` section with product name, identifier, description, author, icon, and per-platform format lists.

#### Scenario: macOS platform produces DMG

- **WHEN** `cargo packager --release` is run on a macOS host
- **THEN** a `.dmg` disk image containing the `Clutch.app` bundle is produced in the `dist/` directory

#### Scenario: Windows platform produces NSIS installer

- **WHEN** `cargo packager --release` is run on a Windows host
- **THEN** an NSIS `.exe` installer is produced in the `dist/` directory

#### Scenario: Linux platform produces AppImage and deb

- **WHEN** `cargo packager --release` is run on a Linux host
- **THEN** both an `.AppImage` and a `.deb` package are produced in the `dist/` directory

### Requirement: Application icon included in packages

All produced packages SHALL embed the application icon located at `assets/Clutch_Icon_512x512.png`.

#### Scenario: Icon is present in macOS app bundle

- **WHEN** the `.dmg` is opened and the app inspected
- **THEN** the app bundle contains the Clutch icon

### Requirement: Package identifier and metadata

The package SHALL use the reverse-DNS identifier `com.github.shitz.clutch` and version matching the `[package]` version field in `Cargo.toml`.

#### Scenario: Identifier matches configuration

- **WHEN** `[package.metadata.packager]` is inspected
- **THEN** `identifier` is `com.github.shitz.clutch` and `version` matches the `[package].version` field
