## MODIFIED Requirements

### Requirement: Options tab in Detail Inspector

The Detail Inspector SHALL include a fifth tab labelled "Options" appended after the existing
"Peers" tab. The full tab order SHALL be: General | Files | Trackers | Peers | Options.

The Options tab operates in two modes depending on the selection state:

**Single-torrent mode** (`selected_ids.len() == 1`): Behaviour is unchanged from the original
specification. Controls are pre-populated from the selected torrent's daemon-reported values.
Every change immediately dispatches a `torrent-set` RPC for that single torrent.

**Bulk Options mode** (`selected_ids.len() > 1`): The General, Files, Trackers, and Peers tabs
are hidden (or rendered in a visually unclickable, disabled state). The active tab is forced to
and locked at Options. All controls start in a **blank / unset state** — no values are
pre-populated. Only fields that the user explicitly interacts with are included in the outgoing
`torrent-set` RPC payload. Fields the user does not touch are sent as `None` (omitted from
JSON), ensuring existing per-torrent settings for the other fields are not overwritten.

A `InspectorBulkOptionsState` struct SHALL be introduced alongside the existing
`InspectorOptionsState`. It mirrors `InspectorOptionsState` but all boolean fields are
`Option<bool>` and all numeric string fields start as empty strings:

- `download_limited: Option<bool>` (initially `None`)
- `download_limit_val: String` (initially `""`)
- `upload_limited: Option<bool>` (initially `None`)
- `upload_limit_val: String` (initially `""`)
- `ratio_mode: Option<u8>` (initially `None`)
- `ratio_limit_val: String` (initially `""`)
- `honors_session_limits: Option<bool>` (initially `None`)

Bulk-mode RPCs are dispatched with `ids` set to all IDs in `selected_ids`. Only
`Some(...)` fields appear in the JSON payload.

The tab uses the same **two-column card layout** as single-torrent mode:

**Left card — Speed Limits**: Limit Download (KB/s) toggler + text input; Limit Upload (KB/s)
toggler + text input; Honor Global Speed Limits toggler. In bulk mode togglers dispatch only
the toggled field (e.g. toggling download limited sets `download_limited` and `download_limit`
from the text field, leaving upload fields as `None`).

**Right card — Seeding Ratio**: 3-way segmented control [Global | Custom | Unlimited]. A text
input for custom ratio appears only when Custom is selected.

There is **no Save button**. Toggle interactions dispatch immediately; text field interactions
dispatch on Enter/submit.

#### Scenario: Options tab appears in inspector (single selection)

- **WHEN** exactly one torrent is selected and the Detail Inspector is rendered
- **THEN** a tab labelled "Options" is visible as the fifth tab along with General, Files,
  Trackers, and Peers

#### Scenario: Bulk Options mode hides non-Options tabs

- **WHEN** `selected_ids.len() > 1`
- **THEN** the General, Files, Trackers, and Peers tabs are absent or visually disabled
- **THEN** the active tab is Options and cannot be changed

#### Scenario: Bulk Options mode starts blank

- **WHEN** the inspector enters Bulk Options mode
- **THEN** all togglers are in their default (unset) visual state and all text inputs are empty

#### Scenario: Bulk checkbox interaction — first touch sets the field

- **WHEN** in Bulk Options mode the download limit toggler is unchecked (visually showing
  `unwrap_or(false)` = `false` for the initial `None` state) and the user clicks it
- **THEN** `bulk_options.download_limited` becomes `Some(true)` (the field is now "touched")
- **THEN** a `torrent-set` RPC is dispatched with `downloadLimited=true` for all selected IDs

#### Scenario: Bulk checkbox interaction — double-click to explicitly set false

- **WHEN** the user clicks the download limit toggler once (sets `Some(true)`) then clicks it
  again
- **THEN** `bulk_options.download_limited` becomes `Some(false)`
- **THEN** a `torrent-set` RPC is dispatched with `downloadLimited=false` for all selected IDs

#### Scenario: Untouched bulk fields are not included in RPC

- **WHEN** in Bulk Options mode the user only touches the download limit toggler and presses
  Enter in another field without interacting with it
- **THEN** the `torrent-set` RPC payload contains only the fields the user explicitly changed
- **THEN** upload limit and ratio fields are absent from the JSON payload

#### Scenario: Bulk download limit toggle dispatches only download fields

- **WHEN** in Bulk Options mode the user switches the "Limit Download" toggler ON
- **THEN** a `torrent-set` RPC is dispatched with `ids` = all selected IDs,
  `downloadLimited=true`, the current `download_limit_val` (or 0 if empty),
  and no upload or ratio fields

#### Scenario: Bulk text field submit dispatches only the submitted field

- **WHEN** in Bulk Options mode the user types `500` in the download limit field and presses Enter
- **THEN** a `torrent-set` RPC is dispatched with `ids` = all selected IDs,
  `downloadLimited=true`, `downloadLimit=500`, and no other fields

#### Scenario: Download limit toggle immediately applies via RPC (single-torrent)

- **WHEN** in single-torrent mode the "Limit Download (KB/s)" toggler is switched ON
- **THEN** a `torrent-set` RPC call is dispatched with `downloadLimited=true` and the
  current value from the text field
- **WHEN** the toggler is switched OFF
- **THEN** a `torrent-set` RPC call is dispatched with `downloadLimited=false`

#### Scenario: Download limit text field applies on submit (single-torrent)

- **WHEN** in single-torrent mode the toggler is ON and the user presses Enter
- **THEN** a `torrent-set` RPC call is dispatched with `downloadLimited=true` and the
  entered numeric value
- **WHEN** the toggler is OFF and the user presses Enter
- **THEN** no RPC is dispatched

#### Scenario: Upload limit toggle and submit behave symmetrically to download

- **WHEN** the "Limit Upload (KB/s)" toggler is switched and the user submits the field
- **THEN** the same rules as download apply, using `uploadLimited` and `uploadLimit`

#### Scenario: Honor Global Speed Limits toggle sends full bandwidth state (single-torrent)

- **WHEN** in single-torrent mode the "Honor Global Speed Limits" toggler is switched OFF
- **THEN** a `torrent-set` RPC call is dispatched with `honorsSessionLimits=false`,
  `downloadLimited`, `downloadLimit`, `uploadLimited`, and `uploadLimit` all set to their
  current values

#### Scenario: Seeding ratio segmented control applies immediately

- **WHEN** the user selects "Custom" in the seeding ratio control
- **THEN** a `torrent-set` RPC call is dispatched with `seedRatioMode=1`
- **WHEN** the user selects "Global"
- **THEN** a `torrent-set` RPC call is dispatched with `seedRatioMode=0`
- **WHEN** the user selects "Unlimited"
- **THEN** a `torrent-set` RPC call is dispatched with `seedRatioMode=2`

#### Scenario: Custom ratio text field appears only in Custom mode

- **WHEN** the segmented control is set to Custom (mode 1)
- **THEN** a "Custom ratio" text input is visible below the control
- **WHEN** the control is set to Global or Unlimited
- **THEN** no custom ratio text input is shown

#### Scenario: Ratio text field applies on submit only in Custom mode

- **WHEN** the ratio text field has focus and the user presses Enter in Custom mode
- **THEN** a `torrent-set` RPC call is dispatched with `seedRatioMode=1` and the entered value
- **WHEN** Enter is pressed and the mode is not Custom
- **THEN** no RPC is dispatched

#### Scenario: Options tab populated from selected torrent data (single-torrent)

- **WHEN** the user selects a new single torrent in the list
- **THEN** all Options controls are reset to the newly selected torrent's daemon-reported values

#### Scenario: Numeric-only input accepted in speed fields

- **WHEN** the user types a non-digit character into a KB/s text input
- **THEN** the character is silently discarded and the field value does not change

#### Scenario: Decimal input accepted in ratio field

- **WHEN** the user types digits and at most one `'.'` into the ratio text input
- **THEN** the field value is updated
- **WHEN** the user types a non-digit, non-`.` character into the ratio field
- **THEN** the character is silently discarded
