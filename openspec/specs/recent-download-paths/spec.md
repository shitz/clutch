# recent-download-paths Specification

## Purpose

Define per-profile persistence and UI display of recently used download directories. Recent
paths are scoped to each `ConnectionProfile` so that server-specific paths are not mixed.

## Requirements

### Requirement: Per-profile recent path storage

Each `ConnectionProfile` SHALL store up to 5 distinct download directory paths most recently
used when adding a torrent. The list SHALL be persisted in the profile TOML file under the
key `recent_download_paths`.

#### Scenario: Profile file without recent_download_paths loads cleanly

- **WHEN** an existing profile TOML file is loaded that does not contain `recent_download_paths`
- **THEN** the field is initialised to an empty `Vec<String>` and the profile loads without
  error

#### Scenario: Recent paths persisted on save

- **WHEN** the active profile has one or more entries in `recent_download_paths`
- **AND** the profile is saved to disk
- **THEN** the TOML file includes `recent_download_paths` with the correct entries in order

### Requirement: Path history updated on successful add

When the user confirms the add-torrent dialog with a non-empty destination field, the entered
path SHALL be recorded in the active profile's `recent_download_paths`.

Recording SHALL:

1. Prepend the new path at index 0.
2. Remove any duplicate occurrences of the same path (case-sensitive) at higher indices.
3. Truncate the list to a maximum of 5 entries.
4. Persist the updated profile to disk.

#### Scenario: New path added to front of history

- **WHEN** the user clicks Add with a non-empty destination path `/srv/media`
- **AND** `/srv/media` is not already in `recent_download_paths`
- **THEN** `/srv/media` is inserted at index 0
- **THEN** the list is trimmed to at most 5 entries
- **THEN** the profile is saved

#### Scenario: Duplicate path moved to front

- **WHEN** the user clicks Add with destination path `/srv/media`
- **AND** `/srv/media` already exists at index 2 in `recent_download_paths`
- **THEN** the old entry at index 2 is removed
- **THEN** `/srv/media` is inserted at index 0
- **THEN** the profile is saved

#### Scenario: Empty destination does not update history

- **WHEN** the user clicks Add with an empty destination field
- **THEN** `recent_download_paths` is unchanged
- **THEN** the daemon uses its own configured default directory

#### Scenario: List capped at five entries

- **WHEN** `recent_download_paths` already holds 5 entries
- **AND** the user adds a sixth distinct path
- **THEN** only the 5 most recent entries are retained

### Requirement: Recent-paths dropdown in add-torrent dialog

The destination field row in the add-torrent dialog SHALL include a ▼ toggle button. Clicking
the toggle button SHALL open an `iced_aw::DropDown` overlay anchored below the row, listing
the entries in `recent_download_paths` for the active profile. The overlay SHALL NOT open when
`recent_download_paths` is empty (the toggle button is visually disabled in that case).

Selecting a path from the overlay SHALL set the destination `text_input` to that path and close
the overlay. The `text_input` remains freely editable at all times. Clicking outside the overlay
SHALL dismiss it without changing the input.

`AddTorrentDialogState` SHALL include an `is_dropdown_open: bool` field to track overlay
visibility.

#### Scenario: Toggle button opens dropdown

- **WHEN** `recent_download_paths` is non-empty
- **AND** the user clicks the ▼ toggle button
- **THEN** the dropdown overlay opens showing all recent paths in order

#### Scenario: Toggle button disabled when history empty

- **WHEN** `recent_download_paths` is empty
- **THEN** the ▼ toggle button has no `on_press` handler and appears disabled

#### Scenario: Selecting a path fills the input and closes the dropdown

- **WHEN** the dropdown is open
- **AND** the user clicks a path entry displaying `/mnt/downloads`
- **THEN** the destination `text_input` value becomes `/mnt/downloads`
- **THEN** the dropdown overlay is closed
- **THEN** the dialog remains open

#### Scenario: Clicking outside dismisses dropdown

- **WHEN** the dropdown is open
- **AND** the user clicks anywhere outside the overlay
- **THEN** the dropdown is dismissed
- **THEN** the destination `text_input` value is unchanged

### Requirement: Destination pre-filled from most recent path

When the add-torrent dialog first opens (initial pick, before any carousel advance), the
destination field SHALL be pre-filled with `recent_download_paths[0]` if the list is non-empty.
The field SHALL NOT be reset or re-filled when the carousel advances to the next torrent —
the path a user typed (or selected from the dropdown) is sticky for the entire queue session.

#### Scenario: Destination pre-filled on initial dialog open

- **WHEN** the active profile has at least one entry in `recent_download_paths`
- **AND** the add-torrent dialog opens for the first time (first torrent in the queue)
- **THEN** the destination `text_input` is initialised with `recent_download_paths[0]`

#### Scenario: Destination empty on initial open when no history

- **WHEN** the active profile has no entries in `recent_download_paths`
- **AND** the add-torrent dialog opens for the first time
- **THEN** the destination `text_input` is initialised to an empty string

#### Scenario: Destination value preserved across carousel advances

- **WHEN** the user has typed or selected a path `/new/path` for the current torrent
- **AND** the user clicks Add or Cancel This to advance to the next torrent
- **THEN** the destination `text_input` still contains `/new/path`
- **THEN** `recent_download_paths[0]` is NOT applied to the field
