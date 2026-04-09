## ADDED Requirements

### Requirement: Material Design 3 theme

The application SHALL support two built-in themes — Light and Dark — both derived from a Material Design 3 palette seeded from the Clutch brand color Magnetic Blue (`#2A64A7`). Each theme SHALL be implemented as a custom `iced::Theme` via a single `clutch_theme(is_dark: bool) -> Theme` function. The dark palette SHALL use a lightened primary (`#5B9FD4`) for sufficient contrast on dark surfaces. The following named color constants SHALL be available from the public `crate::theme` module:

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

The active theme SHALL apply to all standard iced widgets automatically via the theme system. The Settings > General tab SHALL provide a 3-segment `segmented_control` (labels: "Light", "Dark", "System") for theme selection.

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

- **WHEN** a developer inspects the public `crate::theme` module
- **THEN** all brand colors are defined as named `const Color` values at the top of the file and nowhere else

### Requirement: Material Icons font

The application SHALL bundle the MaterialIcons-Regular.ttf font at compile time. A public `icon(codepoint: char) -> Text` helper SHALL render a single Material icon glyph at a standard size (24 px). All toolbar action buttons (Pause, Resume, Delete, Settings, theme toggle) SHALL use Material icon glyphs rather than text labels.

#### Scenario: Icon glyphs rendered in toolbar

- **WHEN** the main screen toolbar is rendered
- **THEN** Pause, Resume, Delete, and theme-toggle controls display Material icon glyphs

#### Scenario: Font available cross-platform

- **WHEN** the application is built on macOS, Windows, or Linux
- **THEN** the Material Icons font is embedded in the binary and no system font installation is required

### Requirement: Elevated surface containers

Containers used as card-like surfaces (torrent list rows when selected, inspector panel background) SHALL be styled with rounded corners (12 px radius) and a drop shadow indicating elevation, consistent with Material Design 3 elevation tokens.

#### Scenario: Selected torrent row has elevated appearance

- **WHEN** a torrent row is selected
- **THEN** the row is rendered with a rounded border and drop shadow distinguishing it from unselected rows

#### Scenario: Inspector panel has elevated background

- **WHEN** the inspector panel is visible
- **THEN** its container has a slightly lighter background than the main surface and rounded top corners

### Requirement: iced_aw Tabs for inspector

The inspector tab bar SHALL be implemented using `iced_aw::Tabs`. The active tab SHALL be visually distinguished from inactive tabs using the active Material primary color. Tabs SHALL respond to click without delay.

#### Scenario: All four tabs rendered

- **WHEN** the inspector panel is visible
- **THEN** General, Files, Trackers, and Peers tabs are rendered using `iced_aw::Tabs`

#### Scenario: Active tab highlighted

- **WHEN** a tab is selected
- **THEN** that tab's label is styled with the Material primary accent color or underline indicator

### Requirement: Floating Action Button for Add Torrent

The primary "Add Torrent" action SHALL be presented as a Floating Action Button (FAB) anchored to the bottom-right corner of the main content area, implemented with `iced_aw::FloatingElement`. The FAB SHALL display the Material "add" icon (U+E145). The FAB SHALL remain visible and accessible regardless of scroll position in the torrent list.

#### Scenario: FAB visible at all times on main screen

- **WHEN** the main screen is rendered
- **THEN** the FAB is visible in the bottom-right corner

#### Scenario: FAB opens the add-torrent dialog

- **WHEN** the user clicks the FAB
- **THEN** the add-torrent dialog opens, identical to the behavior previously triggered by the toolbar Add button

#### Scenario: FAB does not obscure inspector content

- **WHEN** the inspector panel is open and the user scrolls the torrent list
- **THEN** the FAB does not overlap critical inspector content (positioned with sufficient margin)

## ADDED Requirements

### Requirement: Settings tab bar constrained width

The Settings screen tab bar (segmented control) SHALL be constrained to a maximum width of 400 px and centered in the content area, regardless of the window width.

#### Scenario: Settings tab bar does not span full width

- **WHEN** the Settings screen is shown on a wide window
- **THEN** the tab segmented control is centered and no wider than 400 px

### Requirement: Overlay dialogs use fixed width and M3 button layout

Add/edit profile dialogs, confirmation dialogs, and other modal overlays on the Settings screen SHALL be rendered with a fixed maximum width of 360 px. Button rows within these dialogs SHALL be right-aligned. Cancel actions SHALL use `m3_tonal_button`. Confirm/save actions SHALL use `m3_primary_button`. Destructive confirm actions SHALL use a danger-colored pill button.

#### Scenario: Overlay dialog is constrained in width

- **WHEN** an add, edit, or confirm dialog is shown
- **THEN** the dialog content is no wider than 360 px

#### Scenario: Overlay dialog button row is right-aligned

- **WHEN** an overlay dialog is shown
- **THEN** Cancel appears to the left of the primary action, and the row is right-aligned within the dialog

### Requirement: Inspector pane has M3 card background

The bottom details/inspector pane SHALL have its outermost container styled with the `m3_card` surface color (elevated, slightly lighter in dark mode), creating a visible separation from the torrent list above without relying on a hard horizontal divider line.

#### Scenario: Inspector pane visually separated from list

- **WHEN** the inspector pane is visible below the torrent list
- **THEN** the pane has a card-elevated background distinguishable from the main surface
