## MODIFIED Requirements

### Requirement: .torrent file flow

When the user clicks "Add Torrent", the native file picker SHALL accept multiple `.torrent`
file selections. The app SHALL parse each selected file locally to extract its file list
before opening the dialog. No RPC call is made at this stage.

All selected files are loaded into a queue. Files are processed in the order they were
selected. The total number of queued torrents SHALL be displayed in the dialog as an
N-of-M counter (e.g. "1 of 3") whenever M > 1.

#### Scenario: Multiple files picked and queued

- **WHEN** the user selects N `.torrent` files from the file picker (N ≥ 2)
- **THEN** all N files are parsed locally
- **THEN** the add-torrent dialog opens showing the first torrent's file list
- **THEN** a counter "1 of N" is displayed in the dialog title or header

#### Scenario: Single file picked — no counter shown

- **WHEN** the user selects exactly 1 `.torrent` file
- **THEN** the dialog opens with that file's content and no N-of-M counter is shown

#### Scenario: File picker cancelled

- **WHEN** the user dismisses the file picker without selecting a file
- **THEN** the dialog does not open and the UI state is unchanged

#### Scenario: File picked and parsed

- **WHEN** the user selects one or more `.torrent` files
- **THEN** each file is read and parsed locally before the dialog opens

## ADDED Requirements

### Requirement: Sequential dialog queue

The add-torrent dialog SHALL process a queue (`VecDeque`) of pending torrents one at a time.
After the user clicks **Add** or **Cancel This** for the current torrent, the next torrent in
the queue is dequeued and displayed immediately in the same dialog without closing it. When
the queue is exhausted the dialog is dismissed.

#### Scenario: Add advances to next torrent in queue

- **WHEN** the user clicks Add in the dialog
- **AND** at least one torrent remains in the queue
- **THEN** the `torrent-add` RPC is dispatched for the current torrent
- **THEN** the next torrent from the queue is loaded into the dialog via `pop_front()`
- **THEN** the destination `text_input` value is left unchanged (sticky path)
- **THEN** the N-of-M counter is updated

#### Scenario: Add closes dialog when queue is empty

- **WHEN** the user clicks Add for the last torrent in the queue
- **THEN** the `torrent-add` RPC is dispatched
- **THEN** the dialog is dismissed

#### Scenario: Cancel This advances to next torrent

- **WHEN** the user clicks "Cancel This" (queue has remaining items)
- **THEN** the current torrent is discarded (no RPC call)
- **THEN** the next torrent from the queue is loaded into the dialog
- **THEN** the N-of-M counter is updated

### Requirement: Bulk cancel for multi-add queue

When the add-torrent dialog is open with more than one torrent remaining in the queue, the
dialog SHALL display two cancel actions: **Cancel This** (skip the current torrent) and
**Cancel All** (discard the current torrent and all remaining queued torrents). When the
queue has exactly one torrent remaining, a single **Cancel** button SHALL be shown.

#### Scenario: Single torrent — one Cancel button shown

- **WHEN** the queue contains exactly one torrent (M = 1)
- **THEN** a single "Cancel" button is shown

#### Scenario: Multiple torrents — two cancel buttons shown

- **WHEN** the queue contains two or more torrents
- **THEN** "Cancel This" and "Cancel All" buttons are both shown in place of the single Cancel

#### Scenario: Cancel All dismisses dialog

- **WHEN** the user clicks "Cancel All"
- **THEN** the dialog is dismissed immediately
- **THEN** no RPC call is issued for any remaining queued torrent
