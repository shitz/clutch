## ADDED Requirements

### Requirement: Release workflow triggers on version tags

The release workflow SHALL trigger automatically when a Git tag matching the pattern `v*` is pushed to the repository.

#### Scenario: Tag push triggers release

- **WHEN** a tag such as `v0.10.0` is pushed
- **THEN** the release workflow starts and all build jobs run

#### Scenario: Non-tag push does not trigger release

- **WHEN** a commit is pushed to a branch without a matching tag
- **THEN** the release workflow does not run

### Requirement: Cross-platform build matrix

The release workflow SHALL execute build jobs on `macos-latest` (target `aarch64-apple-darwin`), `ubuntu-latest` (target `x86_64-unknown-linux-gnu`), and `windows-latest` (target `x86_64-pc-windows-msvc`).

#### Scenario: All three platforms build in parallel

- **WHEN** the release workflow is triggered
- **THEN** three jobs run in parallel, one per platform

### Requirement: cargo-packager builds release artifacts

Each platform job SHALL install `cargo-packager` and run `cargo packager --release` to produce the platform-native installer(s).

#### Scenario: macOS job produces DMG

- **WHEN** the macOS job completes successfully
- **THEN** a `.dmg` artifact exists under `dist/`

#### Scenario: Linux job produces AppImage and deb

- **WHEN** the Linux job completes successfully
- **THEN** `.AppImage` and `.deb` artifacts exist under `dist/`

#### Scenario: Windows job produces NSIS installer

- **WHEN** the Windows job completes successfully
- **THEN** an `.exe` artifact exists under `dist/`

### Requirement: Linux system dependencies installed before build

The Linux build job SHALL install the required system packages (`pkg-config`, `libx11-dev`, `libasound2-dev`, `libudev-dev`, `libwayland-dev`, `libxkbcommon-dev`) before building.

#### Scenario: Linux build succeeds with dependencies

- **WHEN** the system dependencies are installed and `cargo packager --release` runs on Ubuntu
- **THEN** the build completes without linker or pkg-config errors

### Requirement: Artifacts uploaded to GitHub Release

All installer artifacts SHALL be uploaded to the GitHub Release associated with the triggering tag using `softprops/action-gh-release`.

#### Scenario: Release contains all platform artifacts

- **WHEN** all three platform jobs complete successfully
- **THEN** the GitHub Release for the tag contains `.dmg`, `.exe`, `.AppImage`, and `.deb` files

#### Scenario: Upload uses GITHUB_TOKEN

- **WHEN** the upload step runs
- **THEN** it authenticates using the built-in `GITHUB_TOKEN` secret — no additional secrets required
