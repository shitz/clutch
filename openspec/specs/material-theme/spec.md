## ADDED Requirements

### Requirement: Material Design 3 theme

The application SHALL support two built-in themes — Light and Dark — both derived from a Material Design 3 palette. Each theme SHALL be implemented as a custom `iced::Theme` with a `Palette` matching Material Design 3 color roles (background, surface, primary, error, on-surface text). The active theme SHALL apply to all standard iced widgets (buttons, progress bars, text inputs) automatically via the theme system.

#### Scenario: Default theme applied on launch

- **WHEN** the application starts
- **THEN** the Dark Material theme is active

#### Scenario: Light theme active when toggled

- **WHEN** the user activates the light mode toggle
- **THEN** all widgets are rendered using the Light Material palette

#### Scenario: Dark theme active when toggled back

- **WHEN** the user activates the dark mode toggle while light mode is active
- **THEN** all widgets are rendered using the Dark Material palette

#### Scenario: Toggle is visible at all times

- **WHEN** the main screen is shown
- **THEN** a light/dark mode toggle control is visible in the toolbar

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
