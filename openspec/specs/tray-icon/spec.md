## ADDED Requirements

### Requirement: Tray icon lifecycle

A system tray icon SHALL be created during application startup and remain present for the
lifetime of the process.

#### Scenario: Tray icon appears on launch

- **WHEN** Clutch starts on a supported desktop environment
- **THEN** a Clutch icon appears in the system tray / menu bar

#### Scenario: Graceful degradation on unsupported Linux environments

- **WHEN** the tray icon fails to initialize (e.g. no libappindicator on Linux)
- **THEN** a warning is logged and the application continues as a normal windowed application
  with no tray icon

### Requirement: Native context menu

The tray icon SHALL expose a native context menu containing the following items in order:

1. ↓ Speed — disabled (read-only) text showing current global download rate
2. ↑ Speed — disabled (read-only) text showing current global upload rate
3. Separator
4. Resume All — activates `torrent-start` for all known torrent IDs
5. Pause All — activates `torrent-stop` for all known torrent IDs
6. Separator
7. Turtle Mode — checkable item reflecting the daemon's `alt-speed-enabled` session flag
8. Separator
9. Show Clutch — brings the main window to the foreground
10. Exit — terminates the application

#### Scenario: Speed labels reflect current transfer rates

- **WHEN** a torrent-get poll completes
- **THEN** the ↓ Speed and ↑ Speed menu items SHALL be updated to reflect the aggregate
  download and upload rates from the refreshed torrent list

#### Scenario: Turtle Mode checked state reflects daemon state

- **WHEN** a session-get poll completes
- **THEN** the Turtle Mode menu item SHALL be checked if and only if
  `alt-speed-enabled` is true on the daemon

### Requirement: Bulk actions from tray

#### Scenario: Resume All while connected

- **WHEN** the user clicks "Resume All" and Clutch is in the Main screen
- **THEN** a `torrent-start` RPC call SHALL be issued for all currently known torrent IDs

#### Scenario: Pause All while connected

- **WHEN** the user clicks "Pause All" and Clutch is in the Main screen
- **THEN** a `torrent-stop` RPC call SHALL be issued for all currently known torrent IDs

#### Scenario: Bulk action while not connected

- **WHEN** the user clicks "Resume All" or "Pause All" and no connection is active
- **THEN** the action SHALL be silently ignored (no RPC call, no error)

### Requirement: Turtle Mode toggle from tray

#### Scenario: Turtle Mode toggle

- **WHEN** the user clicks the Turtle Mode check item in the tray menu
- **THEN** a `session-set` RPC call SHALL toggle the daemon's `alt-speed-enabled` flag,
  identical in behaviour to clicking the Turtle Mode button in the main toolbar

### Requirement: Show window from tray

#### Scenario: Show Clutch restores the window

- **WHEN** the user clicks "Show Clutch" in the tray menu or clicks the tray icon
- **THEN** the main window SHALL be made visible and brought to the foreground

### Requirement: Exit from tray

#### Scenario: Exit terminates the process

- **WHEN** the user clicks "Exit" in the tray menu
- **THEN** the application SHALL terminate cleanly, including stopping the RPC worker
