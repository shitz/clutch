## Context

Clutch is a Rust/iced GUI application with no current distribution mechanism — users must clone and build from source. The first public release (v0.10.0) needs installer packages for macOS, Windows, and Linux so that end users can install the app without a Rust toolchain. Distribution is via GitHub Releases only for the initial release; platform-specific package managers (Homebrew, Winget, AUR) are out of scope.

The app is unsigned on all platforms. This is an accepted constraint: macOS Gatekeeper and Windows SmartScreen will warn users, but the warnings can be bypassed with documented steps.

## Goals / Non-Goals

**Goals:**

- Produce platform-native installer artifacts on every tagged release: `.dmg` (macOS), `.exe` NSIS installer (Windows), and `.AppImage` + `.deb` (Linux).
- Automate the build and upload via a GitHub Actions release workflow triggered on `v*` tags.
- Maintain a human-readable `CHANGELOG.md` in keepachangelog format starting from v0.10.0.

**Non-Goals:**

- Code signing or notarization on any platform.
- Homebrew tap, Winget submission, or AUR PKGBUILD — deferred until after a stable release.
- Auto-update / in-app update mechanism.
- Artifact verification (checksums, SBOM) — can be added later.

## Decisions

### Use cargo-packager

cargo-packager integrates natively with Cargo's metadata system (`[package.metadata.packager]` in `Cargo.toml`), requires no separate config file, and supports DMG, NSIS, AppImage, and Debian packages out of the box.

_Alternatives considered_: `cargo-bundle` (unmaintained), `tauri-bundler` (couples to Tauri runtime), custom shell scripts (maintenance burden).

### Platform formats: DMG / NSIS / AppImage + deb

- **macOS DMG**: drag-and-drop install is the standard expectation for unsigned apps distributed outside the App Store.
- **Windows NSIS**: lightweight wizard-style installer that registers the app in Start Menu and Add/Remove Programs. Users must click "More info → Run anyway" due to SmartScreen.
- **Linux AppImage + deb**: AppImage provides a universal self-contained executable; `.deb` offers native integration on Debian/Ubuntu systems. Both attach to the same GitHub Release.

### GitHub Actions release workflow on `v*` tags

A separate `release.yml` workflow (distinct from the existing CI workflow) triggers only on pushed version tags. This keeps CI fast and decoupled from release builds. The matrix covers `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`, and `x86_64-pc-windows-msvc`.

_Alternative_: Merge release into the CI workflow — rejected because release builds take significantly longer and should not run on every PR.

### keepachangelog format for CHANGELOG.md

The format is widely recognized, human-readable, and has a clear convention for an `[Unreleased]` section that gets promoted at release time. Semantic Versioning will be followed for version numbers.

## Risks / Trade-offs

- **Unsigned macOS binary triggers "app is damaged" Gatekeeper error** → Users must install via `brew install --cask --no-quarantine` or manually clear the quarantine attribute with `xattr -cr`. Document this prominently in README and the release notes.
- **Windows SmartScreen blue-screen warning** → Users click "More info → Run anyway". Document in release notes. Risk accepted until the app gains enough installs to build SmartScreen reputation.
- **Linux AppImage requires FUSE** → Some minimal systems lack FUSE. The `.deb` is the fallback. Both are attached to releases.
- **cargo-packager version drift** → The workflow pins `cargo install cargo-packager --locked` to ensure reproducibility, relying on the `Cargo.lock` of cargo-packager itself.
- **cargo-packager outputs to `target/release/`** → The upload step globs `target/release/*.dmg`, `target/release/*.exe`, etc. If cargo-packager changes its output layout this must be updated.

## Migration Plan

1. Add `[package.metadata.packager]` to `Cargo.toml` — no runtime impact.
2. Create `.github/workflows/release.yml`.
3. Create `CHANGELOG.md` with the v0.10.0 entry.
4. Tag `v0.10.0` to trigger the first release build and verify artifacts upload correctly.
5. Rollback: delete the tag and the GitHub Release draft if anything fails. No code changes to revert.

## Open Questions

_All open questions resolved._

> **macOS architecture**: Apple Silicon (`aarch64-apple-darwin`) only for v0.10.0. Intel (`x86_64-apple-darwin`) builds are deferred.
>
> **CHANGELOG version links**: Version numbers SHALL link to their GitHub Release URL at `https://github.com/shitz/clutch/releases/tag/vX.Y.Z`. The `[Unreleased]` section links to the diff against the latest tag.
