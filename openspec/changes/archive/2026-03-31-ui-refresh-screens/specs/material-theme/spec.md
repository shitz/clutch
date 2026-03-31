## MODIFIED Requirements

### Requirement: Material Design 3 theme

The application SHALL support Light, Dark, and System theme options. The theme selection control in Settings > General SHALL be implemented as a 3-segment `segmented_control` component (labels: "Light", "Dark", "System") rather than three discrete buttons. The active theme option SHALL be highlighted with the brand primary color in the segmented control.

#### Scenario: Default theme applied on launch

- **WHEN** the application starts
- **THEN** the Dark Clutch theme is active with Magnetic Blue as primary and `SURFACE_DARK` as background

#### Scenario: Theme segmented control reflects current selection

- **WHEN** the Settings > General tab is shown
- **THEN** the segmented control for theme selection shows the current active theme option highlighted

#### Scenario: Selecting a theme option via segmented control

- **WHEN** the user clicks a non-active segment in the theme segmented control
- **THEN** the theme changes to the selected option and the segment becomes highlighted

#### Scenario: Toggle is visible at all times

- **WHEN** the main screen is shown
- **THEN** a light/dark mode toggle control is visible in the toolbar

#### Scenario: Named constants are the sole source of brand colors

- **WHEN** a developer inspects `src/theme.rs`
- **THEN** all brand colors are defined as named `const Color` values at the top of the file and nowhere else

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

