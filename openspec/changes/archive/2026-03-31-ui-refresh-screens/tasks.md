## 1. App Icon

- [x] 1.1 In `src/main.rs`, load `assets/Clutch_Icon_256x256.png` at compile time using `include_bytes!`, decode to RGBA8 using `image::load_from_memory(...).to_rgba8()`, and pass to `iced::window::icon::from_rgba` to set the window icon

## 2. Connection Screen

- [x] 2.1 In `src/screens/connection.rs`, replace the `tab_active` / `tab_inactive` / `tab_underline` button+underline construction with a call to `crate::theme::segmented_control` passing the two `ConnectionTab` variants and `Message::TabSelected` as the on_select callback
- [x] 2.2 Remove imports of `tab_active`, `tab_inactive`, `tab_underline` from `connection.rs` once unused
- [x] 2.3 Wrap each saved profile row container with `.style(crate::theme::m3_card)` and increase the row padding to `[12, 16]`
- [x] 2.4 Update the Quick Connect "Connect" button to use `crate::theme::primary_pill_button("Connect")` instead of a plain `button`

## 3. Main Toolbar

- [x] 3.1 In `src/screens/torrent_list/view.rs`, replace each icon-only toolbar button (Pause, Resume, Delete, Settings, Disconnect, theme toggle) with `crate::theme::icon_button(crate::theme::icon(ICON_*))` and remove the `dim_secondary` style calls
- [x] 3.2 Replace the Add Torrent toolbar button with `crate::theme::primary_pill_button` (or `crate::theme::icon_button` with the add icon if a pill label is not desired — decide at implementation time based on space)

## 4. Torrent List Rows & Progress Bars

- [x] 4.1 Increase row vertical padding in `view_normal_row` from `[8, 0]` to `[10, 0]` and increase list column spacing from `2` to `4`
- [x] 4.2 Add a `border` field to the `progress_bar::Style` closure in `crate::theme::progress_bar_style` to set `border::radius` to `100.0` for both the track background and bar fill
- [x] 4.3 Add the empty-state conditional branch: if `state.torrents.is_empty()` and the screen is not in initial loading, render an `image` widget (Clutch logo, scaled down, wrapped in a semi-transparent container at ~25% opacity) centered in the list area with a `text("No torrents. Add one with +")` below, styled with the muted secondary text color

## 5. Settings Screen

- [x] 5.1 In `src/screens/settings/view.rs`, replace the `view_tab_bar` method's `tab_active` / `tab_inactive` / `tab_underline` button+column construction with `crate::theme::segmented_control` over `SettingsTab` variants
- [x] 5.2 Replace the three discrete `button("Light")` / `button("Dark")` / `button("System")` buttons in `view_general_tab` with a single `crate::theme::segmented_control` over `ThemeConfig` variants with `Message::ThemeConfigChanged` as on_select
- [x] 5.3 Wrap the appearance settings row (theme segmented control) and the behaviour settings row (refresh interval) in separate `container(...).style(crate::theme::m3_card).padding(16)` containers, with a column heading `text` label above each card
- [x] 5.4 Wrap the profile list section and the profile edit form section in `container(...).style(crate::theme::m3_card).padding(16)` in `view_connections_tab`

## 6. Inspector Pane

- [x] 6.1 In the main screen view where the inspector panel container is constructed, wrap the outermost inspector container with `.style(crate::theme::m3_card)`
- [x] 6.2 Remove or reduce the shadow from `inspector_surface` if visual double-shadow stacking is noticeable (check at runtime and adjust `inspector_surface` shadow alpha to 0.0 if needed)

## 7. UI Polish (post-initial implementation)

- [x] 7.1 Connection screen: redesign profile rows as selectable cards — clicking a row selects it (tonal wash highlight via `selected_row`) without immediately connecting; add an action bar below the list with "Manage Profiles" (`m3_tonal_button`) and "Connect" (`m3_primary_button`); first profile pre-selected on open
- [x] 7.2 Connection screen: cache the logo `Handle` in `ConnectionState` (loaded once with `Handle::from_memory`) to avoid re-decoding on every frame; apply fixed 80 px top margin before the logo; make the profile list scrollable with `max_height(300)`
- [x] 7.3 Connection screen: constrain segmented control inside a centered container (fixed width 400 px)
- [x] 7.4 Settings: constrain tab bar segmented control to maximum 400 px width, centered in the content area
- [x] 7.5 Settings: style overlay dialogs (add/edit profile, confirm import) with fixed `max_width(360)`, right-aligned M3 button row (`m3_tonal_button` cancel left, `m3_primary_button` confirm right); destructive confirm uses a danger-styled pill button instead
- [x] 7.6 Settings: profile list selection uses `selected_row` container style with a `m3_primary_button`-colored border; action row uses `icon_button` + label icon buttons
- [x] 7.7 Add torrent dialog: replace plain `text_input` with `m3_text_input` style; right-align buttons [Cancel (`m3_tonal_button`)] [Add (`m3_primary_button`)]
- [x] 7.8 Torrent list: restyle delete confirmation dialog — "Cancel" uses `m3_tonal_button`, "Delete" uses a danger-styled pill button; right-align the button row
- [x] 7.9 Torrent list: add hover tooltip to each column header button using `m3_tooltip` container style to display the full column name
- [x] 7.10 Inspector: change tab segmented control to use `compact: true` so it fits without stretching the pane; center the segmented control in the header row
- [x] 7.11 Inspector: lay out General tab info fields in a two-column grid for efficient use of space
