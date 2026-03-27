## Why

The torrent list currently shows only Name, Status, and a plain progress bar — useful for a glance but insufficient for monitoring active transfers. Users need speed, ETA, size, and ratio columns at a glance, and the plain UI doesn't match modern desktop expectations. v0.5 addresses both gaps in one milestone: richer data columns and a Material Design 3 visual language that makes the app feel polished and native.

## What Changes

- **New columns in the torrent list:** Size, Downloaded, ↓ Speed, ↑ Speed, ETA, Ratio (all data already fetched by the v0.4 polling requests).
- **Column sort:** clicking any column header cycles through ascending → descending → unsorted; the active sort column and direction are indicated in the header.
- **Color-coded progress bars:** green while downloading, blue while seeding, gray when paused/stopped.
- **Material Design 3 theme:** custom `iced::Theme` with a Material color palette, rounded containers with elevation shadows, and a light/dark toggle in the toolbar.
- **Material Icons font:** `MaterialIcons-Regular.ttf` loaded at compile time; toolbar buttons (Add, Pause, Resume, Delete, Settings) and the theme toggle rendered with icon glyphs instead of text labels.
- **`iced_aw` Tabs integration:** replace the hand-rolled inspector tab bar with `iced_aw::Tabs` for consistent Material-style tab styling.
- **Floating Action Button (FAB):** replace the toolbar "Add" text button with an `iced_aw::FloatingElement`-based FAB in the bottom-right corner.

## Capabilities

### New Capabilities

- `material-theme`: Custom Material Design 3 light/dark theme, Material Icons font helper, elevated container style, and theme-toggle state wired through the app.

### Modified Capabilities

- `torrent-list`: New columns (Size, Downloaded, ↓ Speed, ↑ Speed, ETA, Ratio), column sort, and color-coded progress bars change the visible requirements of the torrent list spec.

## Impact

- **`Cargo.toml`:** add `iced_aw = "0.10"` and `base64` already present; no removals.
- **`src/screens/torrent_list.rs`:** new column headers, sort state, row rendering for six new fields, progress bar color logic.
- **`src/screens/inspector.rs`:** swap hand-rolled tab bar for `iced_aw::Tabs`.
- **`src/screens/main_screen.rs`:** thread theme state, FAB overlay, theme-toggle message.
- **`src/app.rs`:** add `ThemeToggled` message and `current_theme` field; pass theme to iced.
- **`src/theme.rs`** (new): Material palette definitions, icon helper, elevated-surface container style.
- **`fonts/MaterialIcons-Regular.ttf`** (new asset): bundled at compile time via `include_bytes!`.
- No RPC changes — all required fields are already fetched.
