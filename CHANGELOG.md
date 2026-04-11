# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.14.0] - 2026-04-11

### Added

- Queue management: The right-click context menu now includes **Move to Top**, **Move Up**, **Move
  Down**, and **Move to Bottom** actions.
- A new **Queueing** card in the Connections settings panel lets you enable/disable the
  download and seed queues and configure the maximum number of active transfers for each.
- Multi-add torrent support: the file picker now allows selecting multiple `.torrent` files at
  once. Selected files are queued in FIFO order and presented one-by-one in the add dialog.
- Cancel This / Cancel All buttons in the add dialog when multiple torrents are queued: "Cancel
  This" skips the current torrent and advances to the next; "Cancel All" dismisses the entire
  queue.
- Recent download paths: the destination field in the add dialog now includes a dropdown listing
  the last ten directories used for successful adds. The most recently used path is pre-filled
  automatically. Paths are stored per-profile and persisted across sessions.

## [0.13.0] - 2026-04-09

### Added

- System tray icon with a native context menu: Clutch minimises to the system tray instead of
  quitting when the window close button is clicked. The context menu provides Resume All, Pause
  All, Turtle Mode toggle (with live checked state), Show Clutch, and Exit actions.
- Live aggregate download/upload speed labels in the tray context menu, updated on every torrent
  poll using per-torrent rate data.
- Multi-select in the torrent list: plain click selects a single torrent, Ctrl/Cmd-click
  toggles individual rows, Shift-click extends a contiguous range from the anchor row, and
  Cmd+A / Ctrl+A selects all visible (filtered) torrents.
- Bulk actions: Pause, Resume, and Delete toolbar buttons now operate on all selected torrents
  at once. The delete confirmation dialog adapts its title for single vs. multi-torrent
  deletions.
- Bulk bandwidth editing in the inspector: when more than one torrent is selected the inspector
  changes to bulk-edit mode. The speed-limit and seeding-ratio controls apply changes to all
  selected torrents simultaneously.
- Right-click on an already-selected torrent keeps the full multi-selection active so context
  menu actions (Start, Pause, Delete, Set Data Location) apply to all selected torrents.
- Clicking in the empty space below the last torrent row selects the last torrent, matching
  standard list-view behaviour.

### Fixed

- ETA column showed a large stale value (e.g. "41776h 6m") for 100% downloaded torrents that
  are seeding. ETA now correctly shows "—" for seeding torrents.
- Pressing Enter in the Set Data Location path field now triggers Apply, consistent with other
  dialogs.

### Changed

- Eliminated per-torrent heap allocation from the render hot path.

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
