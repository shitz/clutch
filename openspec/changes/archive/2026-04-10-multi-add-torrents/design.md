## Context

Clutch currently opens a single-file native picker (`rfd::FileDialog::pick_file()`), parses one
`.torrent`, and presents the add-torrent dialog. The dialog state in
`src/screens/torrent_list/` holds exactly one `Option<TorrentFileInfo>` and one
`current_path_input: String`. There is no memory of previously used download directories —
users retype paths from scratch every session.

The change touches three subsystems:

1. **File-picker → queue** (`torrent_list/` screen state + update).
2. **Profile persistence** (`src/profile.rs` + save/load round-trip).
3. **Dialog view** (`iced_aw::DropDown` combobox for the destination field).

## Goals / Non-Goals

**Goals:**

- Switch picker to multi-file (`pick_files()`) and process results as a FIFO queue.
- Present each torrent one at a time in the existing modal dialog with a visible N-of-M counter.
- Show "Cancel This" / "Cancel All" buttons when the queue has more than one item remaining.
- Persist the last 5 distinct download paths per `ConnectionProfile`.
- Pre-fill the destination field with the most recent path when a new dialog opens from the queue.
- Show recent paths via an `iced_aw::DropDown` overlay anchored to a ▼ toggle button beside
  the destination text input.
- Update and save the path history when the user clicks Add.

**Non-Goals:**

- Native directory-browser / folder-picker mode.
- Synchronising recent paths across profiles.
- Auto-opening the dropdown on text-input focus (`TextInput` does not emit focus events).

## Decisions

### D1 — Queue lives in `AddTorrentDialogState`, not `AppState`

**Decision:** Embed `pending_torrents: Vec<TorrentFileInfo>` directly in the existing
`AddTorrentDialogState` struct inside `torrent_list/`.

**Rationale:** The queue is local UI state that exists only while the dialog is open. Keeping it
in `AppState` would force the dialog close logic to route through the top-level update function
for every carousel advance, adding complexity without benefit.

**Alternative considered:** A separate top-level `Vec` in `AppState`. Rejected: the queue
is meaningless outside the dialog, and it would require additional routing plumbing.

### D2 — `recent_download_paths` stored inside `ConnectionProfile`

**Decision:** Add `#[serde(default)] recent_download_paths: Vec<String>` to
`ConnectionProfile` in `src/profile.rs`.

**Rationale:** Paths are daemon-specific (a local NAS has different paths than a remote seedbox),
so they must be scoped to the profile. Serde `default` ensures backward compatibility with
existing TOML files that lack the field.

**Alternative considered:** A separate per-profile side-file. Rejected: adds I/O complexity for
no gain; the TOML file is already the single source of truth for profile data.

### D3 — Path history updated via a top-level `Message::ProfilePathUsed(String)`

**Decision:** When the user clicks Add and the destination field is non-empty, the dialog emits
`Message::ProfilePathUsed(path)` alongside the existing `Message::TorrentAdd(…)`. App-level
routing handles deduplication, prepend, truncate to 5, and profile save.

**Rationale:** `AppState` owns the active `ConnectionProfile` and the profile persistence logic.
Routing path updates through `AppState` keeps the dialog free of I/O concerns and follows the
existing pattern for all RPC-triggering actions.

**Alternative considered:** Updating the profile directly inside the torrent_list update
function. Rejected: it would require passing a mutable reference to the full profile into a
sub-screen update function, coupling the subsystems.

### D4 — `iced_aw::DropDown` combobox, not suggestion chips

**Decision:** Pair the destination `text_input` with a small ▼ toggle button in a `row![]`.
Wrapping that row in `iced_aw::DropDown` gives an overlay menu of recent paths anchored to
the underlay. `AddTorrentDialogState` gains an `is_dropdown_open: bool` field. Four messages
drive the lifecycle: `AddDialogToggleDropdown`, `AddDialogDismissDropdown`, and
`AddDialogRecentPathSelected(String)` (plus the existing `AddDialogPathChanged`).

The overlay reuses the existing `theme::m3_menu_card` container style and `theme::m3_menu_item`
button style from the context-menu implementation — no new theme helpers required.

**Rationale:** `iced::TextInput` does not emit focus/blur events so auto-opening on focus is
unreliable. A dedicated toggle button is the standard desktop pattern for an editable combobox:
the user can type freely and optionally open the history with one click. The `iced_aw::DropDown`
widget handles overlay anchoring and dismiss-on-click-outside automatically.

**Alternative considered:** Static suggestion chips rendered below the input. Rejected in favour
of the dropdown now that `iced_aw::DropDown` is confirmed available — the dropdown takes no
extra vertical space in the dialog and is more conventional.

## Risks / Trade-offs

- **[Risk] profile TOML grows unboundedly across many profiles** → Mitigation: the path list is
  capped at 5 entries and deduped on every write; maximum added size per profile is negligible.
- **[Risk] Existing TOML files break on load** → Mitigation: `#[serde(default)]` returns an
  empty `Vec` for any profile file that pre-dates this change; no migration step required.
- **[Risk] Long paths overflow overlay width** → Mitigation: the overlay container is set to
  `width(Length::Fill)` so it matches the underlay row; long path strings are truncated by the
  `scrollable` wrapper inside the overlay (max height 200 px).
- **[Risk] Picking 50 files creates a jarring UX** → Mitigation: the N-of-M progress counter
  ("2 of 15") keeps users oriented; "Cancel All" avoids having to click Cancel repeatedly.
