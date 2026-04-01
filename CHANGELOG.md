# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/shitz/clutch/compare/v0.10.0...HEAD
[0.10.0]: https://github.com/shitz/clutch/releases/tag/v0.10.0
