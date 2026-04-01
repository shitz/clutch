## Why

Clutch has no distribution pipeline — users must build from source to install it. Adding packaging and a release workflow enables non-developer users to install the app directly from GitHub Releases, making the project publicly consumable for the first time.

## What Changes

- Add `cargo-packager` configuration to `Cargo.toml` for macOS (.dmg), Windows (NSIS .exe), and Linux (.AppImage, .deb) targets.
- Add a GitHub Actions release workflow (`.github/workflows/release.yml`) that triggers on `v*` tags, builds cross-platform installers, and uploads them to GitHub Releases.
- Add a `CHANGELOG.md` following the [keepachangelog](https://keepachangelog.com/en/1.1.0/) format, seeding it with an initial `v0.10.0` entry.

## Capabilities

### New Capabilities

- `app-packaging`: cargo-packager metadata in `Cargo.toml` and platform-specific packaging configuration for macOS, Windows, and Linux.
- `release-workflow`: GitHub Actions workflow triggered on version tags that builds the app for all three platforms and uploads installer artifacts to GitHub Releases.
- `changelog`: `CHANGELOG.md` maintained in keepachangelog format with an `[Unreleased]` section and an initial `v0.10.0` release entry.

### Modified Capabilities

<!-- No existing capability requirements are changing. -->

## Impact

- `Cargo.toml`: New `[package.metadata.packager]` section and platform-specific packaging targets.
- `.github/workflows/release.yml`: New file — release pipeline using `cargo-packager` and `softprops/action-gh-release`.
- `CHANGELOG.md`: New file at repository root.
- No runtime code changes; this is purely build and distribution infrastructure.
