## Why

The app currently uses the default MD3 purple palette, which is disconnected from the Clutch brand identity. Before any screen-level polish can be applied, we need the foundational layer — brand-correct colors, reusable button shapes, a segmented control, and a card surface helper — so that all downstream screen work builds on consistent primitives.

## What Changes

- Replace the purple-primary MD3 palette with a Clutch brand palette derived from the logo's Magnetic Blue (`#2A64A7`) and dark/light surface colors
- Add named palette constants (`MAGNETIC_BLUE`, `SURFACE_DARK`, `SURFACE_LIGHT`, etc.) to make the brand relationship explicit in code
- Introduce a `clutch_theme(is_dark: bool)` function replacing the separate `material_dark_theme()` / `material_light_theme()` pair
- Add a `primary_pill_button` helper: fully rounded, wider padding, for primary actions
- Add an `icon_button` helper: transparent background, circular hover highlight, for icon-only toolbar actions
- Add a `segmented_control` helper: a row of connected rounded buttons acting as a single M3 toggle, replacing flat underline tabs and the 3-button theme switcher
- Add an `m3_card` container helper: uniformly rounded (16 px), elevated surface, generalising the existing `inspector_surface` pattern

## Capabilities

### New Capabilities

- `clutch-brand-theme`: Handcrafted M3 palette using Clutch logo colors for both light and dark modes
- `m3-components`: Reusable button and container primitives (pill button, icon button, segmented control, m3_card)

### Modified Capabilities

- `material-theme`: Primary color changes from MD3 purple to Magnetic Blue; `clutch_theme()` replaces the two separate theme functions

## Impact

- `src/theme.rs`: All changes are self-contained here — new constants, new helpers, updated palette functions
- `src/app.rs`: Update calls from `material_dark_theme()` / `material_light_theme()` to `clutch_theme(is_dark)`
- No breaking changes to public message types or screen state
- No new dependencies required
