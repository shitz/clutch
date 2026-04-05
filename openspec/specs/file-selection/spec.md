# file-selection Specification

## Purpose
TBD - created by archiving change selective-file-download. Update Purpose after archive.
## Requirements
### Requirement: Inspector Files tab shows per-file wanted checkboxes

The inspector Files tab SHALL display a checkbox at the start of each file row. The displayed
state for file index _i_ SHALL be determined as follows:

1. If a pending optimistic override exists for index _i_ in `InspectorScreen::pending_wanted`,
   show that value.
2. Otherwise, show the `wanted` field from `TorrentFileStats[i]` (last received from the daemon).
3. If `TorrentFileStats` is absent for index _i_, default to checked (wanted = true).

#### Scenario: All files wanted

- **WHEN** all `fileStats[].wanted` values are `true`
- **THEN** every checkbox in the Files tab is rendered as checked

#### Scenario: Some files unwanted

- **WHEN** one or more `fileStats[].wanted` values are `false`
- **THEN** the corresponding checkbox rows are rendered as unchecked

#### Scenario: fileStats shorter than files list

- **WHEN** `fileStats` has fewer entries than `files`
- **THEN** file rows without a corresponding `fileStats` entry are rendered as checked

### Requirement: Toggling a file checkbox updates the UI immediately

When the user clicks a file checkbox in the inspector Files tab, the checkbox SHALL flip
immediately in the UI (optimistic update) by recording the new value in
`InspectorScreen::pending_wanted`. Simultaneously the app SHALL enqueue a `torrent-set`
RPC call via the existing RPC worker queue, tagging it with the affected file indices.

When the RPC call completes (success or failure), a `FileWantedSetSuccess` message
carrying the indices is dispatched to the inspector. Only those specific indices are
removed from `pending_wanted`. On success the daemon will report the new state on the
next poll; on failure the daemon reports the unchanged state — both cases resolve cleanly
without any special error path.

This design avoids the race condition where a background `torrent-get` poll completes
_after_ the user clicks but _before_ the `torrent-set` is processed: the optimistic
override stays in place until the mutation itself is acknowledged, not until an arbitrary
read-poll arrives.

#### Scenario: User unchecks a wanted file

- **WHEN** the user unchecks a checkbox for file index _i_ on torrent _T_
- **THEN** the checkbox for file _i_ flips to unchecked immediately in the UI
- **THEN** a `torrent-set` RPC call is issued with `{ "ids": [T], "files-unwanted": [i] }`
- **THEN** when the RPC completes, `FileWantedSetSuccess { indices: [i] }` is dispatched
- **THEN** index _i_ is removed from `pending_wanted`

#### Scenario: User checks an unwanted file

- **WHEN** the user checks a checkbox for file index _i_ on torrent _T_
- **THEN** the checkbox for file _i_ flips to checked immediately in the UI
- **THEN** a `torrent-set` RPC call is issued with `{ "ids": [T], "files-wanted": [i] }`
- **THEN** when the RPC completes, `FileWantedSetSuccess { indices: [i] }` is dispatched
- **THEN** index _i_ is removed from `pending_wanted`

#### Scenario: torrent-set dispatched through worker queue

- **WHEN** the user toggles a file checkbox
- **THEN** the `torrent-set` RPC call is enqueued via the RPC worker `mpsc` channel
- **THEN** `update()` returns immediately without blocking

#### Scenario: Pending override isolated from overlapping read-polls

- **WHEN** a background `torrent-get` poll completes while a `torrent-set` is still in-flight
- **THEN** `pending_wanted` is NOT cleared by the poll result
- **THEN** the checkbox continues to show the optimistic value until `FileWantedSetSuccess` arrives

#### Scenario: RPC failure reverts to daemon state

- **WHEN** the `torrent-set` RPC call fails
- **THEN** `FileWantedSetSuccess { indices: [i] }` is still dispatched
- **THEN** index _i_ is removed from `pending_wanted`
- **THEN** the checkbox reverts to the daemon-reported `TorrentFileStats::wanted` value

### Requirement: Inspector Files tab provides a tri-state Select All header

The inspector Files tab SHALL display a **tri-state checkbox** header row above the file
list. Its state (Checked / Unchecked / Mixed) SHALL be derived at render time from the
effective wanted values (incorporating `pending_wanted` overrides):

- All files effectively wanted → **Checked**
- No files effectively wanted → **Unchecked**
- Some but not all files effectively wanted → **Mixed** (indeterminate)

The tri-state checkbox SHALL use the `m3_tristate_checkbox` helper from `src/theme.rs`
(see Design D5). Clicking the header when Mixed or Unchecked emits
`AllFilesWantedToggled { wanted: true }`; clicking when Checked emits
`AllFilesWantedToggled { wanted: false }`.

The `AllFilesWantedToggled` handler SHALL populate `pending_wanted` for all file indices
and enqueue **exactly one** `RpcWork::SetFileWanted` with the full index range
`[0..n-1]`, not one work item per file.

#### Scenario: All files wanted — header shows Checked

- **WHEN** all effective file states are wanted (after pending overrides)
- **THEN** the header tri-state checkbox renders as Checked

#### Scenario: No files wanted — header shows Unchecked

- **WHEN** all effective file states are unwanted
- **THEN** the header tri-state checkbox renders as Unchecked

#### Scenario: Some files wanted — header shows Mixed

- **WHEN** at least one file is wanted and at least one is unwanted
- **THEN** the header tri-state checkbox renders as Mixed (indeterminate)

#### Scenario: Clicking Mixed or Unchecked header selects all

- **WHEN** the header is Mixed or Unchecked and the user clicks it
- **THEN** all file checkboxes flip to checked immediately
- **THEN** a single `torrent-set` call is issued with `files-wanted: [0, 1, … n-1]`

#### Scenario: Clicking Checked header deselects all

- **WHEN** the header is Checked and the user clicks it
- **THEN** all file checkboxes flip to unchecked immediately
- **THEN** a single `torrent-set` call is issued with `files-unwanted: [0, 1, … n-1]`

### Requirement: CheckState enum and m3_tristate_checkbox theme helper

A `CheckState` enum with variants `Checked`, `Unchecked`, and `Mixed` SHALL be defined in
`src/theme.rs`. A `m3_tristate_checkbox(state, label, on_toggle)` helper function SHALL
also live in `src/theme.rs`. It SHALL render a borderless button composed of a Material
Icons glyph and a text label:

| `CheckState` | Icon constant                    | Next state on click |
| ------------ | -------------------------------- | ------------------- |
| `Checked`    | `ICON_CB_CHECKED` (`\u{e834}`)   | `Unchecked`         |
| `Unchecked`  | `ICON_CB_UNCHECKED` (`\u{e835}`) | `Checked`           |
| `Mixed`      | `ICON_CB_MIXED` (`\u{e909}`)     | `Checked`           |

The icon SHALL be coloured with `palette.primary.base.color` for `Checked` and `Mixed`
states, and `palette.background.base.text` for `Unchecked`, using the same dark/light
detection pattern already used throughout `theme.rs`.

#### Scenario: Checked icon renders with primary colour

- **WHEN** `m3_tristate_checkbox` is called with `CheckState::Checked`
- **THEN** the rendered icon uses `palette.primary.base.color`

#### Scenario: Mixed icon renders with primary colour

- **WHEN** `m3_tristate_checkbox` is called with `CheckState::Mixed`
- **THEN** the rendered icon uses `palette.primary.base.color`

#### Scenario: Unchecked icon renders with text colour

- **WHEN** `m3_tristate_checkbox` is called with `CheckState::Unchecked`
- **THEN** the rendered icon uses `palette.background.base.text`

#### Scenario: Clicking Mixed produces Checked

- **WHEN** the user clicks the tri-state checkbox in the Mixed state
- **THEN** `on_toggle(CheckState::Checked)` is emitted

### Requirement: torrent-set RPC for file wanted state

The RPC layer SHALL expose a `torrent_set_file_wanted` function that accepts a torrent ID,
a slice of file indices, and a `wanted: bool` flag. It SHALL issue a `torrent-set` call with
either `files-wanted` or `files-unwanted` according to the flag. Session rotation SHALL be
retried once automatically, following the same pattern as existing action RPCs.

#### Scenario: Setting files wanted

- **WHEN** `torrent_set_file_wanted` is called with `wanted = true` and indices `[i]`
- **THEN** the HTTP body contains `{ "method": "torrent-set", "arguments": { "ids": [id], "files-wanted": [i] } }`

#### Scenario: Setting files unwanted

- **WHEN** `torrent_set_file_wanted` is called with `wanted = false` and indices `[i]`
- **THEN** the HTTP body contains `{ "method": "torrent-set", "arguments": { "ids": [id], "files-unwanted": [i] } }`

#### Scenario: Session rotation retried

- **WHEN** the daemon returns HTTP 409 during a `torrent-set` call
- **THEN** the call is retried once with the new session ID
- **THEN** the RPC worker updates the stored session ID

### Requirement: TorrentFileStats carries wanted field

`TorrentFileStats` SHALL include a `wanted` boolean field that is populated from the
`fileStats[].wanted` value returned by `torrent-get`. The field SHALL default to `true`
when absent (e.g. older Transmission versions or incomplete responses).

#### Scenario: wanted field present in response

- **WHEN** `torrent-get` returns `fileStats` with `"wanted": false` for index _i_
- **THEN** `TorrentFileStats::wanted` for index _i_ is `false`

#### Scenario: wanted field absent from response

- **WHEN** `torrent-get` returns `fileStats` without a `wanted` key
- **THEN** `TorrentFileStats::wanted` defaults to `true`

