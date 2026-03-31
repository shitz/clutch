## MODIFIED Requirements

### Requirement: Material Design 3 theme

The application SHALL support two built-in themes — Light and Dark — both derived from a Material Design 3 palette seeded from the Clutch brand color Magnetic Blue (`#2A64A7`). Each theme SHALL be implemented as a custom `iced::Theme` via a single `clutch_theme(is_dark: bool) -> Theme` function. The dark palette SHALL use a lightened primary (`#5B9FD4`) for sufficient contrast on dark surfaces. The following named color constants SHALL be defined in `src/theme.rs`:

- `MAGNETIC_BLUE`: `Color::from_rgb(0.16, 0.39, 0.65)` — base brand blue
- `MAGNETIC_BLUE_LIGHT`: `Color::from_rgb(0.36, 0.62, 0.83)` — lightened primary for dark mode
- `SURFACE_DARK`: `Color::from_rgb(0.11, 0.13, 0.15)` — dark mode background
- `SURFACE_LIGHT`: `Color::from_rgb(0.98, 0.99, 1.0)` — light mode background
- `SLATE_GREY`: `Color::from_rgb(0.37, 0.42, 0.48)` — secondary/muted color
- `TEXT_DARK` / `TEXT_LIGHT` — primary text per mode
- `SUCCESS_DARK` / `SUCCESS_LIGHT` — success state color per mode
- `DANGER_DARK` / `DANGER_LIGHT` — destructive/error state color per mode
- `DISABLED_DARK` / `DISABLED_LIGHT` — disabled state color per mode
- `INSPECTOR_SURFACE_DARK` / `INSPECTOR_SURFACE_LIGHT` — inspector pane background per mode
- `CARD_SURFACE_DARK` / `CARD_SURFACE_LIGHT` — card container background per mode
- `SEGCTL_SURFACE_DARK` / `SEGCTL_SURFACE_LIGHT` — segmented control background per mode
- `SEGCTL_BORDER_DARK` / `SEGCTL_BORDER_LIGHT` — segmented control border per mode
- `PROGRESS_TRACK_DARK` / `PROGRESS_TRACK_LIGHT` — progress bar track per mode
- `PROGRESS_GREEN` / `PROGRESS_BLUE` / `PROGRESS_GREY` — progress bar fill by torrent status

The active theme SHALL apply to all standard iced widgets automatically via the theme system.

#### Scenario: Default theme applied on launch

- **WHEN** the application starts
- **THEN** the Dark Clutch theme is active with Magnetic Blue as primary and `SURFACE_DARK` as background

#### Scenario: Light theme active when toggled

- **WHEN** the user activates the light mode toggle
- **THEN** all widgets are rendered using the Light Clutch palette with `SURFACE_LIGHT` background and `MAGNETIC_BLUE` primary

#### Scenario: Dark theme active when toggled back

- **WHEN** the user activates the dark mode toggle while light mode is active
- **THEN** all widgets are rendered using the Dark Clutch palette with `MAGNETIC_BLUE_LIGHT` primary

#### Scenario: Toggle is visible at all times

- **WHEN** the main screen is shown
- **THEN** a light/dark mode toggle control is visible in the toolbar

#### Scenario: Named constants are the sole source of brand colors

- **WHEN** a developer inspects `src/theme.rs`
- **THEN** all brand colors are defined as named `const Color` values at the top of the file and nowhere else
