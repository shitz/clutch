## Why

With the brand palette and M3 component primitives in place (from `ui-refresh-theme-components`), the individual screens still use ad-hoc flat underline tabs, square buttons, and no card groupings. This change applies those primitives to every screen — giving the app a cohesive, modern feel that matches the Clutch brand without touching any business logic.

## What Changes

- **Connection screen**: Replace flat underline `Saved Profiles / Quick Connect` tabs with the `segmented_control` component; wrap each saved profile row in a rounded card container with hover elevation
- **Main screen toolbar**: Replace square icon buttons with `icon_button` helpers; use `primary_pill_button` for the Add (`+`) action
- **Torrent list**: Increase row vertical padding and list spacing; round progress bar corners (radius 100.0); add an empty-state view when the torrent list is empty
- **Settings screen**: Replace the 3-button Light/Dark/System theme switcher with a 3-segment `segmented_control`; wrap the General and Connections setting groups in `m3_card` containers; apply `m3_card` background to the bottom details/inspector pane
- **App icon**: Set the window icon to `assets/Clutch_Icon_256x256.png` in `main.rs`

## Capabilities

### New Capabilities

- `empty-state-view`: Display a desaturated logo and muted helper text when the torrent list is empty

### Modified Capabilities

- `connection-screen`: Tabs replaced with segmented control; profile rows styled as m3_card entries
- `torrent-list`: Toolbar button shapes updated; progress bars rounded; list spacing increased
- `material-theme`: Settings theme switcher replaced with segmented control; settings groups wrapped in m3_card

## Impact

- `src/screens/connection.rs` (or `connection/view.rs`): Tab rendering changed
- `src/screens/torrent_list/view.rs`: Toolbar, list rows, progress bars, empty state
- `src/screens/settings/view.rs`: Theme switcher, settings group containers
- `src/screens/inspector.rs` (bottom pane): Card background applied
- `src/main.rs`: Window icon set
- All changes are purely view/style; no state, message type, or RPC model changes
- Depends on: `ui-refresh-theme-components` (segmented_control, icon_button, primary_pill_button, m3_card, clutch_theme must already be merged)
