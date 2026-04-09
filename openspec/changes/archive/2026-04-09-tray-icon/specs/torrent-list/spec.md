## MODIFIED Requirements

### Requirement: Window close hides to tray

The close button (or Cmd+W on macOS) SHALL hide the main window to the system tray instead of
terminating the application.

#### Scenario: Window close hides instead of exits

- **WHEN** the user clicks the window close button or presses the OS close shortcut
- **THEN** the main window SHALL be hidden
- **AND** the application process SHALL continue running
- **AND** the tray icon SHALL remain present

#### Scenario: App is not terminated on window close

- **WHEN** the main window has been hidden via the close button
- **THEN** background operations (torrent seeding, RPC polling) SHALL continue uninterrupted
