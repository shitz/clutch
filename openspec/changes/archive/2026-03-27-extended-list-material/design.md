## Context

Clutch v0.4 ships with a working detail inspector and all extended torrent fields (`totalSize`, `rateDownload`, `rateUpload`, `eta`, `uploadRatio`, `downloadedEver`, `uploadedEver`, `files`, `fileStats`, `trackerStats`, `peers`) already arriving on every 1 s poll. The torrent list renders only Name, Status, and a monochrome progress bar. The UI is functional but visually plain — a generic iced default theme with text-label toolbar buttons.

v0.5 has two orthogonal concerns that share the same milestone to avoid a later, larger refactor:

1. **Extended columns & sort** — expose the data already in memory.
2. **Material Design 3** — a visual overhaul using `iced_aw`, Material Icons, a custom palette, and a light/dark toggle.

Constraints from the architecture document apply unchanged: `update()` must return in microseconds, all RPC calls remain serialized through the MPSC worker, and no GTK/web-view dependencies.

## Goals / Non-Goals

**Goals:**

- Add six new torrent list columns (Size, Downloaded, ↓ Speed, ↑ Speed, ETA, Ratio) using data already fetched.
- Single-column sort (asc/desc/none) by clicking any column header.
- Color-coded progress bars: green (downloading), blue (seeding), gray (paused/stopped).
- Custom `iced::Theme` with Material Design 3 palette, light and dark variants, switchable at runtime via a toolbar toggle.
- Material Icons font loaded at compile-time; toolbar icon glyphs replacing all text-only buttons.
- `iced_aw::Tabs` replacing the hand-rolled inspector tab bar.
- `iced_aw::FloatingElement` FAB replacing the "Add" toolbar text button.
- A new `src/theme.rs` module consolidating palette, icon helper, and elevated-surface container style.

**Non-Goals:**

- Multi-column sort.
- Resizable or reorderable columns (deferred to a later milestone).
- Persisting column sort preference to disk (v1.0 settings).
- Per-torrent right-click context menus.
- Any new RPC calls or protocol changes.

## Decisions

### D1 — `iced_aw` version pinned to `0.13`

`iced_aw 0.13` is the correct version targeting `iced 0.14` (0.11/0.12 target iced 0.13; 0.13 targets iced 0.14). Pinning to a minor version avoids silent breakage from the `iced_aw` release cadence, which historically lags behind `iced` minor bumps.

**Alternative considered:** copy the relevant widgets in-tree. Rejected — maintenance burden of forked widget code outweighs the risk of a pinned dependency.

### D2 — New `src/theme.rs` module for all Material styling

All Material-specific code (palette constants, `icon()` helper, `elevated_surface()` style function, `material_light_theme()` / `material_dark_theme()` constructors) lives in a single `theme.rs` module imported from `app.rs`. This keeps styling concerns out of screen modules and makes future theme changes a single-file edit.

**Alternative considered:** inline styles inside each screen. Rejected — duplicates palette constants and makes a future redesign expensive.

### D3 — Theme state owned by `AppState`, passed down through view functions

`AppState` gains a `theme: ThemeMode` field (`Light` | `Dark`). `app::update()` handles `Message::ThemeToggled`. The iced application's `.theme()` callback reads `state.theme`. Screen `view()` functions receive the active `Theme` reference for container styling.

**Alternative considered:** global/static theme. Rejected — conflicts with iced's ownership model and makes testing harder.

### D4 — Sort state in `TorrentListScreen`, applied at render time

`TorrentListScreen` gains `sort_column: Option<SortColumn>` and `sort_dir: SortDir`. Sorting is a pure in-memory operation on the already-fetched `Vec<TorrentData>` — no new RPC calls. The sort is applied inside `view()` by cloning and sorting a slice of references, leaving the underlying `Vec` unsorted for efficient in-place updates.

**Alternative considered:** sort the backing `Vec` in `update()`. Rejected — would require re-sorting on every `TorrentsUpdated` message (every second), and the sort order would need to be preserved across refreshes with the same external state.

### D5 — Color-coded progress bars via a custom `iced::widget::progress_bar` style function

Each row calls a closure that returns a `progress_bar::Style` based on the torrent's `status` field (0 = stopped/gray, 4 = downloading/green, 6 = seeding/blue, other = green as default active color). This requires no additional state.

### D6 — Material Icons font bundled as `include_bytes!`

```rust
const MATERIAL_ICONS_BYTES: &[u8] = include_bytes!("../fonts/MaterialIcons-Regular.ttf");
```

The font file is committed to the repo under `fonts/`. The `icon(codepoint: char) -> Text` helper renders a single glyph. This avoids a runtime font-load failure and keeps the binary self-contained.

**Alternative considered:** system font lookup. Rejected — cross-platform unreliable; macOS, Windows, and Linux all have different font search paths.

### D7 — Add Torrent button as toolbar icon (FloatingElement fallback applied)

`iced_aw 0.13` does not ship a `FloatingElement` widget (the feature is not present in the published crate). The design fallback documented above is therefore applied: the "Add Torrent" action is kept in the toolbar, rendered as a Material icon button using `icon('\u{E145}')` instead of a text label. All other toolbar buttons (Pause, Resume, Delete, theme toggle) are likewise icon-only. This is functionally equivalent to the original FAB intent — a prominent, always-visible primary action.

### D8 — `iced_aw::Tabs` for the inspector tab bar

Replace the current `Row` of styled buttons with `iced_aw::Tabs`. The `TabId` enum maps to `inspector::ActiveTab`. `iced_aw::Tabs` handles active/inactive styling automatically and accepts a custom tab bar style for Material theming. `iced_aw::Tabs` does not require an `on_close` callback; the close button is simply not wired (omit the `.on_close()` builder call).

### D9 — Hard-coded pixel minimums for narrow numeric columns and minimum window size

The five narrow numeric columns (Size, ↓ Speed, ↑ Speed, ETA, Ratio) are assigned a fixed minimum width in pixels so their values never wrap or clip mid-digit:

| Column  | Minimum width |
| ------- | ------------- |
| Size    | 80 px         |
| ↓ Speed | 90 px         |
| ↑ Speed | 90 px         |
| ETA     | 80 px         |
| Ratio   | 60 px         |

These are expressed via `iced::widget::container::width(Length::Fixed(n))` on the header and row cells. The Name column uses `Length::Fill` to consume the remaining space.

The iced application builder is configured with `.min_size(Size { width: 900.0, height: 500.0 })` in `main.rs` to prevent the window from being resized narrower than the sum of all fixed columns plus a reasonable Name column width.

**Alternative considered:** rely entirely on `FillPortion`. Rejected — a 9-column layout with `FillPortion` alone produces illegible speed/ETA values at typical window widths.

## Risks / Trade-offs

- **`iced_aw` 0.13 API instability** → Pin the exact version in `Cargo.toml`; document the pin reason in a comment. If an API breaks, the module boundary in `theme.rs` isolates the blast radius.
- **FAB layout interaction with split pane** → `FloatingElement` is an overlay; if it obscures inspector content when the inspector is open, fall back to a toolbar icon button. The design decision (D7) documents this escape hatch.
- **Binary size increase from bundled font** → MaterialIcons-Regular.ttf is ~350 KB. Acceptable for a desktop GUI app; the binary today is already multi-MB due to `iced`'s wgpu renderer.
- **Column width constraints with 9 columns** → Narrow numeric columns (↓ Speed, ↑ Speed, ETA, Ratio) have hard-coded pixel minimums (see D9). A minimum window size is enforced at startup. If the window is resized below that minimum, text may still truncate but layout will not break.
- **`iced_aw` Tabs style API** → The `StyleSheet` trait for `Tabs` may differ between `iced_aw` minor versions. Any required style customisation is isolated to `theme.rs`.

## Migration Plan

This is a purely additive UI change with no data-model migration, no storage changes, and no protocol version bump. The app remains shippable at every commit. Suggested landing order:

1. Add `iced_aw` to `Cargo.toml`; verify it compiles with `iced 0.14`.
2. Add `fonts/MaterialIcons-Regular.ttf` and `src/theme.rs` with palette + icon helper.
3. Wire theme state and toggle into `AppState` and `app::update()`.
4. Replace inspector tab bar with `iced_aw::Tabs`.
5. Add new torrent list columns and sort.
6. Add color-coded progress bars.
7. Replace toolbar "Add" button with FAB; replace text-label toolbar buttons with icon glyphs.
8. Run all 69 existing tests; update or add tests for sort logic and theme toggle.
9. Manual smoke-test against a live Transmission daemon (light + dark mode, sort, all columns).

**Rollback:** revert commits up to the last green CI run. No data is persisted by this change.

## Resolved Questions

- **Icon codepoints:** Confirmed standard Material Icons codepoints: pause (U+E034), play_arrow (U+E037), delete (U+E872), add (U+E145), settings (U+E8B8), dark_mode (U+E51C), light_mode (U+E518). These are stable across all MaterialIcons releases and match the regular glyph map.
- **`iced_aw` Tabs `on_close` requirement:** `iced_aw::Tabs` does not mandate an `on_close` callback. Omit the `.on_close()` builder call entirely. Resolved in D8.
- **Column minimum widths:** Hard-coded pixel minimums adopted for narrow numeric columns; minimum window size enforced via `.min_size()`. Resolved in D9.
