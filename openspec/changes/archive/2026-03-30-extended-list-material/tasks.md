## 1. Dependencies & Assets

- [x] 1.1 Add `iced_aw = "0.13"` to `Cargo.toml` and verify it compiles with the existing `iced 0.14` dependency
- [x] 1.2 Download `MaterialIcons-Regular.ttf` from Google Fonts and place it at `fonts/MaterialIcons-Regular.ttf`
- [x] 1.3 Verify the font file is committed and accessible via `include_bytes!` in a scratch test

## 2. Theme Module (`src/theme.rs`)

- [x] 2.1 Create `src/theme.rs` with `MATERIAL_ICONS_BYTES` constant (`include_bytes!("../fonts/MaterialIcons-Regular.ttf")`) and `MATERIAL_ICONS_FONT: Font`
- [x] 2.2 Implement `pub fn icon(codepoint: char) -> Text<'static>` helper rendering a 24 px icon glyph
- [x] 2.3 Define Material Design 3 color constants for light and dark palettes (background, surface, primary, error, on-surface text)
- [x] 2.4 Implement `pub fn material_light_theme() -> Theme` using `Theme::Custom` with the light palette
- [x] 2.5 Implement `pub fn material_dark_theme() -> Theme` using `Theme::Custom` with the dark palette
- [x] 2.6 Implement `pub fn elevated_surface(theme: &Theme) -> container::Style` with 12 px border radius and drop shadow
- [x] 2.7 Expose `pub fn progress_bar_style(status: i32) -> impl Fn(&Theme) -> progress_bar::Style` returning green/blue/gray per status
- [x] 2.8 Register `MATERIAL_ICONS_BYTES` in the iced font loader in `main.rs` via `.font()`

## 3. Theme State in AppState (`src/app.rs`)

- [x] 3.1 Add `ThemeMode` enum (`Light` | `Dark`) to `app.rs`
- [x] 3.2 Add `theme: ThemeMode` field to `AppState` (default `Dark`)
- [x] 3.3 Add `Message::ThemeToggled` variant to `app::Message`
- [x] 3.4 Handle `Message::ThemeToggled` in `app::update()` (toggle between `Light` and `Dark`)
- [x] 3.5 Wire `.theme(|state| state.current_theme())` into the iced application builder in `main.rs`
- [x] 3.6 Add `AppState::current_theme(&self) -> Theme` that delegates to `theme::material_light/dark_theme()`

## 4. Inspector Tabs (`src/screens/inspector.rs`)

- [x] 4.1 Add `iced_aw` tab bar: replace the hand-rolled `Row` of styled buttons with `iced_aw::Tabs`
- [x] 4.2 Define tab labels using `icon()` + text or text-only labels for General, Files, Trackers, Peers
- [x] 4.3 Map `iced_aw` tab close/select callbacks to `inspector::Message::TabSelected`; provide no-op close callback
- [x] 4.4 Apply Material primary color to active tab styling via a custom tab bar style
- [x] 4.5 Update `inspector::view()` to receive `theme: &Theme` and apply `elevated_surface` to the inspector container
- [x] 4.6 Run existing inspector unit tests; fix any compilation errors introduced by the tab bar change

## 5. Extended Torrent List Columns (`src/screens/torrent_list.rs`)

- [x] 5.1 Extend `TorrentData` (in `rpc.rs`) with any fields not yet present: confirm `total_size`, `downloaded_ever`, `rate_download`, `rate_upload`, `eta`, `upload_ratio` are already deserialized
- [x] 5.2 Add `SortColumn` enum (`Name`, `Status`, `Size`, `Downloaded`, `SpeedDown`, `SpeedUp`, `Eta`, `Ratio`, `Progress`) and `SortDir` enum (`Asc`, `Desc`) to `torrent_list.rs`
- [x] 5.3 Add `sort_column: Option<SortColumn>` and `sort_dir: SortDir` fields to `TorrentListScreen`
- [x] 5.4 Add `Message::ColumnHeaderClicked(SortColumn)` variant to `torrent_list::Message`
- [x] 5.5 Handle `ColumnHeaderClicked` in `torrent_list::update()`: cycle Unsorted→Asc→Desc→Unsorted; clear other column sort
- [x] 5.6 Implement `sort_torrents(torrents: &[TorrentData], col: &SortColumn, dir: &SortDir) -> Vec<&TorrentData>` pure helper function
- [x] 5.7 Update `torrent_list::view()` to sort the torrent slice before rendering rows when a sort is active
- [x] 5.8 Update column header row to render nine columns (Name, Status, Size, Downloaded, ↓ Speed, ↑ Speed, ETA, Ratio, Progress) with sort indicator arrows (↑/↓) on the active column
- [x] 5.9 Update torrent row rendering to include all nine columns with correct formatted values (reuse `format_size`, `format_speed`, `format_eta` helpers from inspector or move them to a shared location)
- [x] 5.10 Apply color-coded progress bar using `theme::progress_bar_style(torrent.status)` in each row
- [x] 5.11 Apply `elevated_surface` container style to the selected torrent row
- [x] 5.12 Route `ColumnHeaderClicked` through `main_screen::Message::List` and `app::Message::Main` — update message routing in `main_screen.rs` and `app.rs`
- [x] 5.13 Set fixed pixel widths on narrow numeric column cells (header and row): Size 80 px, ↓ Speed 90 px, ↑ Speed 90 px, ETA 80 px, Ratio 60 px; keep Name as `Length::Fill`
- [x] 5.14 Add `.min_size(Size { width: 900.0, height: 500.0 })` to the iced application builder in `main.rs` to enforce a minimum window size

## 6. Floating Action Button (`src/screens/main_screen.rs`)

- [x] 6.1 Remove the "Add Torrent" text button from the toolbar
- [x] 6.2 Wrap the main content area in `iced_aw::FloatingElement` with the FAB positioned at the bottom-right
- [x] 6.3 Render the FAB using `icon('\u{E145}')` (Material "add" glyph) inside a styled circular container
- [x] 6.4 Wire FAB click to `TorrentListMessage::AddTorrentClicked` (same message as before)
- [x] 6.5 Add the theme-toggle button to the toolbar using `icon()` glyphs for dark_mode (U+E51C) / light_mode (U+E518); wire to `Message::ThemeToggled`
- [x] 6.6 Replace toolbar Pause, Resume, Delete text buttons with their respective Material icon glyphs: pause (U+E034), play_arrow (U+E037), delete (U+E872)

## 7. Testing

- [x] 7.1 Add unit tests for `sort_torrents()` covering: ascending by each column, descending, clear sort, empty list, single-element list
- [x] 7.2 Add unit test for `TorrentListScreen` update: `ColumnHeaderClicked` cycles Unsorted→Asc→Desc→Unsorted
- [x] 7.3 Add unit test: clicking a different column clears the previous sort and starts ascending on the new column
- [x] 7.4 Add unit test for `AppState`: `ThemeToggled` toggles between Light and Dark
- [x] 7.5 Run all existing tests (`cargo test`) and fix any failures introduced by this change
- [x] 7.6 Run `cargo clippy -- -D warnings` and resolve all warnings

## 8. Manual Smoke Test

- [x] 8.1 Launch against a live Transmission daemon; verify all nine columns display correct values
- [x] 8.2 Click each column header and verify ascending sort, then descending sort, then cleared sort
- [x] 8.3 Toggle light/dark theme; verify all screens switch correctly
- [x] 8.4 Click FAB; verify add-torrent dialog opens
- [x] 8.5 Open inspector; verify `iced_aw::Tabs` renders all four tabs and tab switching works
- [x] 8.6 Verify progress bars are green (downloading), blue (seeding), gray (stopped)
- [x] 8.7 Resize the window to the minimum size (900×500); confirm all nine columns remain legible with no wrapping or clipping
- [x] 8.8 Attempt to resize below 900 px wide; confirm the window cannot shrink further
