## Why

Clutch's UI is currently mouse-only: text inputs cannot be cycled with Tab, dialogs
have no Enter-key shortcut for their primary action, and modal dialogs do not
auto-focus the first empty field. This forces users to break away from the keyboard
for every interaction, hurting productivity and accessibility.

## What Changes

- All text inputs across the application participate in the standard Tab focus order.
- Pressing Enter in any modal dialog activates its primary CTA (e.g. "Connect", "Save",
  "Add Torrent").
- When a modal dialog opens, focus is automatically moved to the first empty text input;
  if all fields are pre-filled, the primary CTA receives focus.

## Capabilities

### New Capabilities

- `keyboard-navigation`: Keyboard accessibility for text inputs (Tab cycling), dialog Enter
  confirmation, and auto-focus of the first empty text input when a dialog opens.

### Modified Capabilities

- `connection-screen`: Auto-focus behavior and Enter-to-confirm for the connection/login dialog.
- `add-torrent`: Auto-focus and Enter-to-confirm for the Add Torrent dialog.

## Impact

- `src/screens/connection.rs` — add `id()` / `tab_stops` / key event handling.
- `src/screens/torrent_list/add_dialog.rs` — same keyboard treatment for Add Torrent dialog.
- `src/screens/settings/` — Tab order and Enter confirmation for settings panels.
- `src/theme.rs` — no changes expected; styling helpers are unaffected.
- No new dependencies required (iced provides the focus-management primitives).
