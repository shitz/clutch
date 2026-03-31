## Context

Clutch is a native macOS torrent client built with Rust and the iced GUI framework (0.14, free-function Elm architecture). All visual styling is centralised in `src/theme.rs`. The current palette uses the generic MD3 purple primary (`#6750A4` / `#D0BCFF`) which is visually inconsistent with the logo — a steel-blue magnetic icon over a silver gear. There are no active dependencies on the purple color in any spec-level behavior: it is purely a visual concern.

The two existing theme functions (`material_dark_theme`, `material_light_theme`) already use `Theme::custom()` — the mechanism is correct, only the values need updating. All downstream screens have been designed expecting blue-as-primary, so this is a pure substitution at the theme layer.

## Goals / Non-Goals

**Goals:**

- Replace the purple palette with brand-correct Clutch colors in both light and dark modes
- Expose named color constants so brand colors are discoverable and auditable
- Provide `primary_pill_button`, `icon_button`, `segmented_control`, and `m3_card` as reusable helpers so screens don't inline style closures for common patterns
- Keep all changes in `src/theme.rs`; update call sites in `src/app.rs` (just function rename)

**Non-Goals:**

- Screen-level layout changes (that is Change 2)
- Mathematical tonal palette generation (M3 `MaterialColorUtilities`) — hand-crafted values are sufficient
- Any new dependencies or fonts
- Changing message types, state structs, or RPC models

## Decisions

### Decision: Hand-crafted palette over algorithmic generation

**Chosen:** Define `MAGNETIC_BLUE`, `SURFACE_DARK`, `SURFACE_LIGHT`, `SLATE_GREY`, and tonal variants as `const Color` values directly in `theme.rs`.

**Alternative considered:** Use `material-color-utilities` (separate crate) to generate a full 13-step tonal palette from the seed `#2A64A7`.

**Rationale:** Iced's `Palette` has only 6 fields, so the full tonal scale would be mostly unused. The hand-crafted values are easier to audit against the logo, require no new dependencies, and the loss of algorithmic tonal coverage is irrelevant at this palette granularity. Named constants preserve the brand intent at zero cost.

### Decision: Lighter primary in dark mode (`#5B9FD4` vs `#2A64A7`)

**Chosen:** Dark mode uses `MAGNETIC_BLUE_LIGHT = Color::from_rgb(0.36, 0.62, 0.83)` (`#5B9FD4`) as primary. Light mode uses the base `MAGNETIC_BLUE` (`#2A64A7`).

**Alternative considered:** Same primary in both modes.

**Rationale:** `#2A64A7` on `#1D2024` achieves roughly 4.5:1 contrast — just at the WCAG AA threshold. A lighter tint on dark backgrounds is standard M3 practice (the "primary" role in dark mode is analogous to MD3's "primary container"). `#5B9FD4` gives ~7:1 on `#1D2024` (AAA) and visually mirrors the logo gradient's lighter inner arc.

### Decision: `clutch_theme(is_dark: bool)` replaces two separate functions

**Chosen:** Single `clutch_theme(is_dark: bool) -> Theme` entry point.

**Alternative considered:** Keep two separate named functions.

**Rationale:** The two functions are identical in structure — only values differ. A single branch is shorter, less redundant, and easier for call sites.

### Decision: `m3_card` is a general container; `inspector_surface` becomes a thin wrapper

**Chosen:** `m3_card` uses uniform 16 px radius + elevation shadow. `inspector_surface` calls `m3_card` internals but overrides to asymmetric top-only corners (to match the panel-flush-to-bottom-edge layout).

**Alternative considered:** Delete `inspector_surface` and migrate call sites to `m3_card`.

**Rationale:** The inspector panel is flush with the window bottom edge — uniform rounding would show rounding where the panel meets the edge. Keeping `inspector_surface` as a variant preserves that layout contract. Future screens can use the symmetric `m3_card` directly.

### Decision: Segmented control is a pure view helper, not a widget

**Chosen:** `segmented_control<'a, Message, T>` is a free function returning `Element<'a, Message>`, parameterised over an enum `T: PartialEq + Copy + std::fmt::Display`.

**Alternative considered:** A struct implementing `iced::Widget`.

**Rationale:** Iced's free-function composition model is simpler and aligns with how the rest of `theme.rs` works. The control doesn't need internal state — the active variant is passed in from the caller's screen state.

## Risks / Trade-offs

- **Contrast on hover states**: The generated hover color for `MAGNETIC_BLUE_LIGHT` in icon buttons may need a custom alpha blend rather than iced's default 10% opacity bump — **Mitigation**: explicitly set hover background in the `icon_button` style closure rather than relying on iced's default.
- **`warning` field in Palette**: Iced 0.14's `Palette` includes a `warning` field that the snippet in discussion omitted. **Mitigation**: Use amber `Color::from_rgb(1.0, 0.72, 0.30)` (warm amber, consistent with status = downloading indicator already in `progress_bar_style`).
- **`success` dissonance**: The current `success` green in progress bars is styled independently in `progress_bar_style`, not via `palette.success`. Keeping `palette.success` green and not touching `progress_bar_style` is safe — both happen to use similar greens.
