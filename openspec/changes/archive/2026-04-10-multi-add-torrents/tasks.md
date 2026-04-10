## 1. Profile — Recent Path Storage

- [x] 1.1 Add `recent_download_paths: Vec<String>` field to `ConnectionProfile` in
      `src/profile.rs`, annotated with `#[serde(default)]`
- [x] 1.2 Verify existing profile TOML files round-trip cleanly (field absent → empty Vec on
      load, Vec present → persisted on save)
- [x] 1.3 Add unit tests in `src/profile.rs` covering: load without field, save with 3 paths,
      dedup + cap at 5

## 2. App Routing — Path History Update

- [x] 2.1 Add `Message::ProfilePathUsed(String)` variant to the top-level `Message` enum
- [x] 2.2 Handle `Message::ProfilePathUsed(path)` in `app::update`: prepend path, dedup,
      truncate to 5, save profile to disk via existing profile-save path
- [x] 2.3 Add unit tests for the dedup-and-cap logic in the handler

## 3. Multi-File Picker

- [x] 3.1 Switch `rfd::FileDialog::pick_file()` to `rfd::FileDialog::pick_files()` in the
      file-picker `Task::perform()` call inside `torrent_list/`
- [x] 3.2 Update the message that receives the picked path(s) to carry `Vec<PathBuf>` instead
      of `Option<PathBuf>`
- [x] 3.3 Update the parsing step to iterate over all picked files in order and collect
      directly into a `VecDeque<TorrentFileInfo>` (preserves OS-returned order for FIFO)

## 4. Dialog Queue State

- [x] 4.1 Add `pending_torrents: VecDeque<TorrentFileInfo>` to `AddTorrentDialogState`
      (or equivalent struct)
- [x] 4.2 Add `is_dropdown_open: bool` to `AddTorrentDialogState` (default `false`)
- [x] 4.3 On receiving the `VecDeque<TorrentFileInfo>` from the picker, call
      `pop_front()` to set `active_torrent`; remaining items stay in `pending_torrents`
- [x] 4.4 Pre-fill `current_path_input` with `recent_download_paths[0]` (or empty) **only
      on initial dialog open** — do not reset the field when advancing the carousel

## 5. Carousel Update Logic

- [x] 5.1 On **Add**: dispatch `torrent-add` RPC, emit `Message::ProfilePathUsed(path)` if
      path is non-empty, then call `pending_torrents.pop_front()` to load the next torrent
      (or close dialog if empty); **leave `current_path_input` unchanged**
- [x] 5.2 On **Cancel This**: discard current torrent (no RPC), call
      `pending_torrents.pop_front()` (or close dialog if empty); **leave `current_path_input`
      unchanged**
- [x] 5.3 On **Cancel All**: clear `pending_torrents`, close dialog, no RPC

## 6. Dialog View Updates

- [x] 6.1 Render an N-of-M counter (e.g. "2 of 5") in the dialog header when
      `pending_torrents.len() + 1 > 1`
- [x] 6.2 Show single "Cancel" button when queue has only the current torrent (no pending)
- [x] 6.3 Show "Cancel This" and "Cancel All" buttons when `pending_torrents` is non-empty
- [x] 6.4 Add messages `AddDialogToggleDropdown`, `AddDialogDismissDropdown`, and
      `AddDialogRecentPathSelected(String)` to the screen's `Message` enum
- [x] 6.5 Handle the three new messages in the torrent-list update function: toggle/dismiss
      `is_dropdown_open`; on select, set `current_path_input` and close dropdown
- [x] 6.6 Build the destination combobox row: `text_input` + ▼ `icon_button` wrapped in
      `iced_aw::DropDown`; overlay lists recent paths using `theme::m3_menu_item` buttons
      inside a `scrollable` container styled with `theme::m3_menu_card` (max height 200 px)
- [x] 6.7 Disable the ▼ toggle button (no `on_press`) when `recent_download_paths` is empty

## 7. Quality Gates

- [x] 7.1 Run `cargo fmt` and fix any formatting issues
- [x] 7.2 Run `cargo clippy -- -D warnings` and resolve all warnings
- [x] 7.3 Run `cargo test` and confirm all tests pass
- [x] 7.4 Update `CHANGELOG.md` with an `Added` entry for multi-add and recent paths
