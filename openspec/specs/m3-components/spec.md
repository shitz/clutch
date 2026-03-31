## ADDED Requirements

### Requirement: Primary filled button helper

The application SHALL provide an `m3_primary_button` style function in `src/theme.rs`. The style SHALL produce a solid primary-color background with white (on-primary) text and a fully rounded pill shape (radius 100.0). Padding SHALL be at least `[10, 24]`. This style is used for all primary CTAs (Connect, Save, Add).

#### Scenario: Primary button renders with solid brand color

- **WHEN** `m3_primary_button` style is applied to a button
- **THEN** the button has the brand primary color as a solid background and white label text

#### Scenario: Primary button responds to press

- **WHEN** the user clicks the primary button
- **THEN** the on_press message is dispatched

### Requirement: Tonal (secondary) button helper

The application SHALL provide an `m3_tonal_button` style function in `src/theme.rs`. The style SHALL produce a 15 % alpha primary color wash background with primary-color text and a fully rounded pill shape (radius 100.0). This style is used for all secondary/cancel actions (Cancel, Manage Profiles, Test Connection, Undo).

#### Scenario: Tonal button renders with brand wash background

- **WHEN** `m3_tonal_button` style is applied to a button
- **THEN** the button has a low-opacity primary wash background and primary-colored text

#### Scenario: Tonal button responds to press

- **WHEN** the user clicks the tonal button
- **THEN** the on_press message is dispatched

### Requirement: Icon button helper

The application SHALL provide an `icon_button<'a>(content: Element<'a, Message>) -> Button<'a, Message>` helper. The button SHALL use a fixed 36×36 px size with the content centered via an inner `container(...).center(Fill)` wrapper. The background SHALL be transparent by default. On hover and press, the button SHALL show a subtle circular background highlight using a low-opacity blend of the theme primary color (alpha ≤ 0.15). The border radius SHALL be 100.0. No border SHALL be shown.

#### Scenario: Icon button transparent at rest

- **WHEN** an icon button is rendered without hover
- **THEN** no background color is visible

#### Scenario: Icon button shows hover highlight

- **WHEN** the user hovers over an icon button
- **THEN** a circular tint appears behind the icon using the primary color at low opacity

### Requirement: Segmented control component

The application SHALL provide a `segmented_control` view helper that accepts a slice of `(label: &str, value: T)` pairs, an active value, an on_select callback, an `equal_width: bool` flag, and a `compact: bool` flag, and returns an `Element` representing a connected row of buttons styled as an M3 segmented button group. The helper SHALL:

- Render all segments as a single visual unit (no gap between segments)
- Apply outer-pill rounding on first/last segment ends (radius 16 px); inner joints are square
- Highlight the active segment with an 18 % alpha primary color wash background and primary-color text
- Render inactive segments with a surface background and muted text
- Emit the associated value as a message when a segment is pressed
- When `compact` is true, reduce vertical padding for space-constrained placements (e.g., inspector tabs)

#### Scenario: Active segment is visually distinct with tonal wash

- **WHEN** a segmented control is rendered with a selected value
- **THEN** the matching segment has an 18 % alpha primary wash background and primary-color text

#### Scenario: Pressing an inactive segment emits its value

- **WHEN** the user clicks an inactive segment
- **THEN** the on_press callback receives that segment's associated value as the message

#### Scenario: Outer ends are rounded pill, inner edges are square

- **WHEN** the segmented control is rendered with 2 or more segments
- **THEN** the leftmost segment has rounded left corners and the rightmost has rounded right corners; interior joints are square

### Requirement: M3 card surface container

The application SHALL provide an `m3_card` function with the signature `fn m3_card(theme: &Theme) -> container::Style`. The style SHALL apply a uniform border radius of 16.0 px, tonal elevation background, and a subtle drop shadow. The existing `inspector_surface` style SHALL retain asymmetric top-only rounding for panels flush with a window edge.

#### Scenario: Card surface distinguishes content regions

- **WHEN** content is wrapped in a container styled with `m3_card`
- **THEN** the container visually floats above the app background due to tonal elevation and shadow

### Requirement: M3 outlined text input style

The application SHALL provide `m3_text_input` as a style function for `text_input` widgets. The style SHALL apply: 8 px border radius, 1 px surface-variant border at rest, and 2 px primary-color border when focused.

#### Scenario: Unfocused text input has subtle outline

- **WHEN** a text field with `m3_text_input` style is not focused
- **THEN** the field shows a thin surface-variant border

#### Scenario: Focused text input has primary outline

- **WHEN** the user focuses a text field with `m3_text_input` style
- **THEN** the border thickens and uses the primary brand color

### Requirement: Tooltip container style

The application SHALL provide `m3_tooltip` as a `container::Style` function. The style SHALL apply: dark elevated background (`rgb(46, 50, 58)`), white text, 6 px border radius, and a drop shadow.

#### Scenario: Tooltip is visually elevated

- **WHEN** a tooltip container is rendered
- **THEN** it appears as a dark floating card above the content beneath it

### Requirement: Selected row highlight

The application SHALL provide `selected_row` as a `container::Style` function; used to highlight the selected row in list contexts. The style SHALL apply an 18 % alpha primary brand-blue wash background with 6 px border radius and no border.

#### Scenario: Selected row has primary wash background

- **WHEN** a list row is selected and wrapped in a container styled with `selected_row`
- **THEN** a subtle primary-blue tint distinguishes it from unselected rows
