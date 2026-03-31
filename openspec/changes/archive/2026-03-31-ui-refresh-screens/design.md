## Context

This change is purely view-layer. All business logic, state structures, message types, and RPC models remain unchanged. The new component helpers from `ui-refresh-theme-components` (`segmented_control`, `icon_button`, `primary_pill_button`, `m3_card`, `clutch_theme`) must already be merged before this change lands.

The three main view files to modify are:

- `src/screens/connection.rs` — mixes state + view currently; the tab bar is inline
- `src/screens/torrent_list/view.rs` — the primary view with toolbar, header, row list
- `src/screens/settings/view.rs` — settings with discrete theme buttons and no card grouping

The inspector bottom pane (`src/screens/inspector.rs`) already uses `inspector_surface` for elevation — it receives the `m3_card`-style wrap of the entire pane, not the internal tab content.

One new capability (`empty-state-view`) is being introduced: a conditional branch in the torrent list view that renders a placeholder image + text when the torrent slice is empty.

## Goals / Non-Goals

**Goals:**

- Apply `segmented_control` to connection tabs and settings theme switcher
- Apply `icon_button` and `primary_pill_button` to all toolbar actions on the main screen
- Round progress bar corners to radius 100.0 in torrent list rows
- Increase list item vertical padding and row spacing for breathing room
- Wrap Settings groups in `m3_card` containers
- Apply `m3_card` background to the bottom inspector pane
- Add empty-state view (desaturated logo + muted text) to the torrent list
- Set window icon from `assets/Clutch_Icon_256x256.png` in `main.rs`

**Non-Goals:**

- Any animated transitions or splash screens
- Changes to routing, screen state, or message types
- Changing the torrent list columns or data displayed
- Responsive/adaptive layout for different window sizes
- Any new external dependencies

## Decisions

### Decision: Empty state uses existing `image` widget with opacity, not a separate screen

**Chosen:** In `torrent_list/view.rs`, wrap the empty state in an `iced::widget::image` widget with `content_fit: ContentFit::ScaleDown` centered in the list area, with a `text` widget below. Opacity is emulated via a semi-transparent container overlay.

**Alternative considered:** A separate `Screen::EmptyList` variant.

**Rationale:** The empty state is a visual concern, not a routing concern. The torrent list is already shown — it's just empty. A conditional branch in the view is the minimal, correct approach.

### Decision: Profile cards in connection screen use `m3_card` style inline, not a dedicated component

**Chosen:** Each saved profile row in the connection screen's Saved Profiles tab is wrapped in `container(...).style(crate::theme::m3_card)`.

**Alternative considered:** A new `profile_card` component function.

**Rationale:** The profile list is a single-use layout. An inline style application is sufficient — no abstraction needed for one call site.

### Decision: App window icon loaded at compile time via `include_bytes!`

**Chosen:** Load `assets/Clutch_Icon_256x256.png` with `include_bytes!` in `main.rs`, decode at startup with iced's `window::icon::from_rgba` helper.

**Alternative considered:** Load from filesystem at runtime.

**Rationale:** Embedding avoids file-not-found issues in packaged builds. The icon is small (256×256 PNG, ~30 KB) and justified for compile-time inclusion.

### Decision: Inspector pane gets `m3_card` background on its outer container only

**Chosen:** The outermost container wrapping the entire inspector panel gets `m3_card` style. The internal tab content and `inspector_surface` are unchanged.

**Rationale:** The inspector panel already has `inspector_surface` for its inner top area. Adding `m3_card` at the outermost level provides the card-vs-background separation without conflicting with existing internal elevation.

## Risks / Trade-offs

- **`icon::from_rgba` PNG decoding**: iced's built-in icon loading requires raw RGBA bytes, not a PNG. We need to use the `image` crate (already a transitive dependency via iced) to decode the PNG. **Mitigation**: Use `image::load_from_memory` + `.to_rgba8()` before passing to iced.
- **Segmented control generic constraint**: The `segmented_control` helper uses a generic type `T: PartialEq + Copy`. Rust's type inference should handle this at call sites, but if the inferred type is ambiguous we may need explicit turbofish annotations at one or two call sites. **Mitigation**: Use concrete enum types at all call sites (e.g., `ConnectionTab`, `ThemeConfig`).
- **Inspector pane double-elevation**: Applying both `inspector_surface` (inner) and `m3_card` (outer) could stack two drop shadows visually. **Mitigation**: Remove the shadow from `inspector_surface` if `m3_card` is the outer container, or offset them so they stack cleanly.
