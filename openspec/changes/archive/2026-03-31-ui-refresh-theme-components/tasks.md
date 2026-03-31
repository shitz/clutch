## 1. Brand Color Constants

- [x] 1.1 Add `MAGNETIC_BLUE`, `MAGNETIC_BLUE_LIGHT`, `SURFACE_DARK`, `SURFACE_LIGHT`, and `SLATE_GREY` as `pub const Color` values at the top of `src/theme.rs`
- [x] 1.2 Add `AMBER_DARK` and `AMBER_LIGHT` constants for the `warning` palette field (dark: `Color::from_rgb(1.0, 0.72, 0.30)`, light: `Color::from_rgb(0.9, 0.45, 0.0)`)

## 2. Clutch Theme Function

- [x] 2.1 Add `pub fn clutch_theme(is_dark: bool) -> Theme` in `src/theme.rs`, branching on `is_dark` to return a `Theme::custom(...)` with the brand palette
- [x] 2.2 Remove or deprecate `material_dark_theme()` and `material_light_theme()`, replacing them with calls to `clutch_theme`
- [x] 2.3 Update `src/app.rs` call sites to use `clutch_theme(is_dark)` instead of the two old functions

## 3. Pill Button Helper

- [x] 3.1 Add `pub fn primary_pill_button<'a>(label: &str) -> iced::widget::Button<'a, crate::app::Message>` to `src/theme.rs` with padding `[10, 24]` and a style closure setting `border::radius` to `100.0`

## 4. Icon Button Helper

- [x] 4.1 Add `pub fn icon_button<'a>(content: iced::Element<'a, crate::app::Message>) -> iced::widget::Button<'a, crate::app::Message>` with transparent background at rest and a primary-tinted circular highlight on hover/press (alpha ≤ 0.15, radius 100.0)

## 5. Segmented Control

- [x] 5.1 Add `pub fn segmented_control<'a, Message, T>` free function to `src/theme.rs` accepting `options: &[(&str, T)]`, `active: T`, and `on_select: impl Fn(T) -> Message + 'a` — returns `Element<'a, Message>`
- [x] 5.2 Apply radius `[100, 0, 0, 100]` to first segment, `[0, 100, 100, 0]` to last, `[0, 0, 0, 0]` to middle segments
- [x] 5.3 Style active segment with primary background + on-primary text; inactive segments with surface background + muted text

## 6. M3 Card Container

- [x] 6.1 Add `pub fn m3_card(theme: &Theme) -> container::Style` to `src/theme.rs` with uniform 16 px radius, tonal-elevation background, and subtle drop shadow
- [x] 6.2 Verify `inspector_surface` still uses asymmetric top-only corners and is not affected by the new helper

## 7. Additional M3 Helpers (post-initial implementation)

- [x] 7.1 Replace `primary_pill_button` with two distinct helpers: `m3_primary_button` (solid primary fill, white text, radius 100.0) and `m3_tonal_button` (15 % alpha primary wash background, primary text, radius 100.0)
- [x] 7.2 Remove `m3_text_button` (interim helper); merge its usage into `m3_tonal_button`
- [x] 7.3 Add `pub fn m3_text_input(theme: &Theme, is_focused: bool) -> text_input::Style` — M3 outlined style: 8 px radius, 1 px surface-variant border at rest, 2 px primary border when focused
- [x] 7.4 Add `pub fn m3_tooltip(theme: &Theme) -> container::Style` — dark elevated tooltip surface (`rgb(46, 50, 58)`), white text, 6 px radius, drop shadow
- [x] 7.5 Add `pub fn selected_row(theme: &Theme) -> container::Style` — 18 % alpha primary brand-blue wash, 6 px radius; used for list-row selection highlight
- [x] 7.6 Update `icon_button` to use a fixed 36×36 px size with the icon centered inside a `container(content).center(Fill)` wrapper
- [x] 7.7 Extend `segmented_control` signature with `equal_width: bool` and `compact: bool` parameters; `compact` reduces vertical padding for space-constrained contexts; active segment updated to use 18 % alpha primary wash + primary text (not solid fill)
- [x] 7.8 Centralize all asset bytes as module-level constants: `pub const LOGO_BYTES`, `pub const ICON_256_BYTES`, `pub const ICON_512_BYTES` (compile-time `include_bytes!`)
- [x] 7.9 Add full extended color constant set: `TEXT_DARK/LIGHT`, `SUCCESS_DARK/LIGHT`, `DANGER_DARK/LIGHT`, `DISABLED_DARK/LIGHT`, `INSPECTOR_SURFACE_DARK/LIGHT`, `CARD_SURFACE_DARK/LIGHT`, `SEGCTL_SURFACE_DARK/LIGHT`, `SEGCTL_BORDER_DARK/LIGHT`, `PROGRESS_TRACK_DARK/LIGHT`, `PROGRESS_GREEN/BLUE/GREY`
