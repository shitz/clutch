# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.12.0] - 2026-04-08

### Added

- Filter chips row in the torrent list: six Filter Chips (All, Downloading, Seeding, Paused,
  Active, Error) between the toolbar and column headers. Chips are multi-select; clicking a chip
  toggles its status bucket. The "All" chip selects or deselects all buckets at once. Each chip
  displays a real-time count of matching torrents derived from the full un-filtered list.
- Right-click context menu on torrent rows with Start, Pause, Delete, and Set Data Location
  actions.
- Set Data Location: a new modal dialog to relocate a torrent's data on the daemon's
  filesystem. The path input is prefilled with the torrent's current download directory. A
  "Move data to new location" checkbox (default: on) controls whether the daemon physically
  moves the files or only updates its internal path record.
- The torrent inspector now displays the torrent's data path and any daemon-reported error message
  in the General tab.

### Fixed

- Passphrase hash (`master_passphrase_hash`) was silently erased from `config.toml` on every
  settings save. `SettingsScreen::build_store_snapshot` now carries the hash forward from the
  store that was loaded when the screen opened, so the hash survives Save, Delete, and General
  tab saves.
- Test Connection in the Connections settings tab sent empty credentials when the profile had an
  encrypted password and the session passphrase was not yet unlocked, causing an authentication
  failure. The test now prompts for the master passphrase first (same unlock dialog as the
  Connect flow), then fires the probe with the decrypted password.

## [0.11.0] - 2026-04-06

### Added

- Turtle Mode: toolbar speed-icon toggle button highlights blue when Transmission's
  alternative speed limits are active.
- Alternative Speed Limits per connection profile: each saved profile stores its own
  alternative download and upload ceilings (KB/s), configured in the Connections settings tab.
- Inspector Options tab (5th tab): per-torrent switches and fields for Limit Download,
  Limit Upload, Stop Seeding at Ratio, and Honor Global Speed Limits. When "Honor Global
  Speed Limits" is ON, the torrent respects whichever session-level limit is active
  (standard when turtle mode is off; alternative when turtle mode is on). When OFF, the
  torrent bypasses all global limits and is capped only by its own per-torrent limits.
- Inspector Files tab now has per-file download checkboxes.
- Keyboard Tab / Shift-Tab cycles focus through all text inputs on the active screen or
  dialog, wrapping around at the ends.
- Pressing Enter in the Quick Connect form, Add Torrent / Add Link dialog, or the Saved
  Profiles tab triggers the primary CTA (Connect / Add) without requiring a mouse click.

### Changed

- Deleting a torrent now opens a confirmation modal instead of having an inline confirmation step in
  the toolbar.

## [0.10.0] - 2026-04-01

### Added

- Connection screen with host, port, username, and password fields (defaults: `localhost:9091`).
- Transmission JSON-RPC client with `X-Transmission-Session-Id` lifecycle handling and a single
  async worker to serialise all daemon calls.
- Torrent list with sortable columns: Name, Status, Size, Downloaded, ↓ Speed, ↑ Speed, ETA, Ratio,
  Progress.
- Torrent controls: start, stop, remove.
- Add torrent via local `.torrent` file or magnet link.
- Detail inspector panel for per-torrent metadata.
- Connection profiles with encrypted password storage (`Argon2 + ChaCha20-Poly1305`).
- Settings screen for managing and switching between connection profiles.
- Material Design 3 theme with light/dark mode following the system preference.
- macOS `.dmg`, Windows NSIS `.exe`, Linux `.AppImage`, and Linux `.deb` release packages
  distributed via GitHub Releases.
