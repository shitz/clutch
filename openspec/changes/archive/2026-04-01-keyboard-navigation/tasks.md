## 1. Shared Infrastructure

- [x] 1.1 Add `text_input::Id` constants (or lazy statics) for every text input that
      participates in a Tab ring: Quick Connect fields (`QC_HOST`, `QC_PORT`,
      `QC_USERNAME`, `QC_PASSWORD`), Add Dialog fields (`ADD_MAGNET`, `ADD_DESTINATION`),
      Settings profile fields (`PROF_NAME`, `PROF_HOST`, `PROF_PORT`, `PROF_USERNAME`,
      `PROF_PASSWORD`).
- [x] 1.2 Bind each constant Id to its `text_input` widget via `.id(ID)` at every call
      site in the view functions.

## 2. Tab Focus Cycling — Connection Screen (Quick Connect)

- [x] 2.1 Add a `Message::TabKeyPressed { shift: bool }` variant to
      `screens/connection.rs` (or re-use a shared keyboard message if appropriate).
- [x] 2.2 Implement the focus-advance logic in `connection::update`: given the currently
      focused field (tracked via a new `focused_field: Option<QcField>` enum in
      `ConnectionScreen`), emit `text_input::focus(next_id)` task on Tab and
      `text_input::focus(prev_id)` task on Shift-Tab.
- [x] 2.3 Add an `iced::keyboard::on_key_press` subscription in `app::subscription` when
      `Screen::Connection` is the active screen; the handler emits
      `Message::Connection(connection::Message::TabKeyPressed { shift })` on Tab,
      and `Message::Connection(connection::Message::EnterPressed)` on Enter (no
      modifiers).

## 3. Enter Confirmation — Connection Screen (Quick Connect)

- [x] 3.1 Add `Message::EnterPressed` to `screens/connection.rs`.
- [x] 3.2 Handle `EnterPressed` in `connection::update`: if Quick Connect tab is active
      and `!self.is_connecting`, trigger the same code path as `ConnectClicked`.

## 4. Auto-Focus — Connection Screen

- [x] 4.1 In `ConnectionScreen::new_launchpad` (and on `TabSelected(QuickConnect)`),
      return a `text_input::focus(first_empty_qc_id)` task. "First empty" is resolved
      in order: Host → Port → Username → Password; if all are non-empty, focus Host.

## 5. Tab Focus Cycling — Add Torrent Dialog

- [x] 5.1 Add `Message::DialogTabKeyPressed { shift: bool }` and
      `Message::DialogEnterPressed` variants to `torrent_list/mod.rs` (or re-use
      existing key message infrastructure).
- [x] 5.2 Implement focus-advance in `torrent_list::update`: cycle
      ADD_MAGNET → ADD_DESTINATION in magnet mode; ADD_DESTINATION only in file mode.
- [x] 5.3 Extend the main screen's `iced::keyboard::on_key_press` subscription to also
      emit the dialog key messages when `add_dialog` is not `Hidden`.

## 6. Enter Confirmation — Add Torrent Dialog

- [x] 6.1 Handle `DialogEnterPressed` in `torrent_list::update`: if the dialog is open
      and guard conditions are met (non-empty magnet in magnet mode, or metainfo present
      in file mode), emit `Message::AddConfirmed`.

## 7. Auto-Focus — Add Torrent Dialog

- [x] 7.1 In the `update()` branch that transitions `AddDialogState` from `Hidden` to
      `AddLink { .. }`, return `Task::batch([existing_task, text_input::focus(ADD_MAGNET)])`.
- [x] 7.2 In the `update()` branch that transitions to `AddFile { .. }`, return
      `Task::batch([existing_task, text_input::focus(ADD_DESTINATION)])`.

## 8. Tab Focus Cycling — Settings Profile Form

- [x] 8.1 Add `Message::TabKeyPressed { shift: bool }` and `Message::EnterPressed` to
      `screens/settings/mod.rs`.
- [x] 8.2 Implement focus cycling in `settings::update` for the profile edit form:
      PROF_NAME → PROF_HOST → PROF_PORT → PROF_USERNAME → PROF_PASSWORD → PROF_NAME.
- [x] 8.3 Extend `app::subscription` to emit settings Tab/Enter messages when
      `Screen::Settings` is active.

## 9. Quality Gates

- [x] 9.1 Run `cargo fmt && cargo check && cargo clippy -- -D warnings` with zero
      warnings or errors.
- [x] 9.2 Manually verify Tab cycling on the Quick Connect form.
- [x] 9.3 Manually verify Enter triggers Connect on Quick Connect.
- [x] 9.4 Manually verify auto-focus on Quick Connect tab switch.
- [x] 9.5 Manually verify Tab cycling, Enter, and auto-focus in the Add Torrent (file)
      dialog.
- [x] 9.6 Manually verify Tab cycling, Enter, and auto-focus in the Add Link (magnet)
      dialog.
- [x] 9.7 Update `CHANGELOG.md` with the new keyboard-navigation behaviour under
      `[Unreleased]`.
