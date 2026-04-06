# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [v0.11.0] - 2026-04-06

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
