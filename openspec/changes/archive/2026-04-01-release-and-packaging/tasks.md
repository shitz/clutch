## 1. cargo-packager Configuration

- [x] 1.1 Bump `version` in `Cargo.toml` from `0.9.0` to `0.10.0`
- [x] 1.2 Add `[package.metadata.packager]` section to `Cargo.toml` with `product-name`, `identifier`, `description`, `authors`, and `icons`
- [x] 1.3 Add `[package.metadata.packager.target.macos]` with `formats = ["dmg"]`
- [x] 1.4 Add `[package.metadata.packager.target.windows]` with `formats = ["nsis"]`
- [x] 1.5 Add `[package.metadata.packager.target.linux]` with `formats = ["appimage", "deb"]`
- [x] 1.6 Verify `assets/Clutch_Icon_512x512.png` exists at the path referenced by the `icons` field

## 2. Release Workflow

- [x] 2.1 Create `.github/workflows/release.yml` with the workflow triggered on `push: tags: v*`
- [x] 2.2 Add build matrix with three entries: `macos-latest`/`aarch64-apple-darwin`, `ubuntu-latest`/`x86_64-unknown-linux-gnu`, `windows-latest`/`x86_64-pc-windows-msvc`
- [x] 2.3 Add checkout and Rust toolchain setup steps (using `dtolnay/rust-toolchain@stable` with `targets: ${{ matrix.target }}`)
- [x] 2.4 Add conditional Linux dependency installation step (`sudo apt-get install -y pkg-config libx11-dev libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev`)
- [x] 2.5 Add `cargo install cargo-packager --locked` step
- [x] 2.6 Add `cargo packager --release` build step
- [x] 2.7 Add artifact upload step using `softprops/action-gh-release@v2` globbing `dist/*.dmg`, `dist/*.exe`, `dist/*.AppImage`, `dist/*.deb`

## 3. Changelog

- [x] 3.1 Create `CHANGELOG.md` at the repository root with the keepachangelog header and `[Unreleased]` section
- [x] 3.2 Add `[0.10.0] - 2026-04-01` release entry documenting the initial feature set under `Added`
- [x] 3.3 Add link reference definitions at the bottom of `CHANGELOG.md` so version headings link to `https://github.com/shitz/clutch/releases/tag/v0.10.0` and `[Unreleased]` links to the HEAD diff

## 4. Verification

- [x] 4.1 Run `cargo packager --release` locally on macOS and confirm a `.dmg` is produced in `dist/`
- [x] 4.2 Push a `v0.10.0` tag to GitHub and confirm the release workflow runs and attaches all four installer artifacts to the GitHub Release
