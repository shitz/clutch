## Requirements

### Requirement: Options tab in Detail Inspector

The Detail Inspector SHALL include a fifth tab labelled "Options" appended after the
existing "Peers" tab. The full tab order SHALL be: General | Files | Trackers | Peers |
Options.

The Options tab SHALL use a **two-column card layout**:

**Left card — Speed Limits**: A scrollable column containing:

1. **Limit Download (KB/s)** — toggler + numeric text input. Applies immediately on toggle
   (RPC dispatched) or on Enter/submit in the text field (only when toggler is ON).
2. **Limit Upload (KB/s)** — toggler + numeric text input. Same apply-on-toggle logic.
3. **Honor Global Speed Limits** — toggler only (no text field). Applies immediately on
   toggle. When ON, the torrent respects whichever session-level speed limit is currently
   active: standard limits when turtle mode is off, alternative limits when turtle mode is
   on. When OFF, the torrent ignores ALL global speed limits and is capped only by its own
   per-torrent download/upload limits (or runs uncapped if those are also disabled).
   The RPC includes the full bandwidth state (download/upload limits and flags) so that
   per-torrent limits take effect as soon as the torrent opts out of the global cap.

**Right card — Seeding Ratio**: A 3-way segmented control [Global | Custom | Unlimited]
bound to `seedRatioMode`:

- **Global** (mode 0) — use the daemon's global ratio limit.
- **Custom** (mode 1) — use a per-torrent ratio; a "Custom ratio" text input appears
  below the control.
- **Unlimited** (mode 2) — no ratio cap.

The segmented control fires an RPC immediately when changed. The ratio text input fires an
RPC on Enter/submit (only when Custom is selected; otherwise ignored).

There is **no Save button**. Every control change results in an immediate `torrent-set`
RPC call (toggle changes) or an on-submit RPC call (text field changes).

All text inputs use `theme::m3_text_input`. Speed inputs accept only ASCII digit
characters. The ratio input accepts digits and at most one `'.'`.

#### Scenario: Options tab appears in inspector

- **WHEN** the Detail Inspector is rendered with a torrent selected
- **THEN** a tab labelled "Options" is visible as the fifth tab

#### Scenario: Download limit toggle immediately applies via RPC

- **WHEN** the "Limit Download (KB/s)" toggler is switched ON
- **THEN** a `torrent-set` RPC call is dispatched with `downloadLimited=true` and the
  current value from the text field
- **WHEN** the toggler is switched OFF
- **THEN** a `torrent-set` RPC call is dispatched with `downloadLimited=false`

#### Scenario: Download limit text field applies on submit

- **WHEN** the "Limit Download (KB/s)" toggler is ON and the user presses Enter
- **THEN** a `torrent-set` RPC call is dispatched with `downloadLimited=true` and the
  entered numeric value
- **WHEN** the toggler is OFF and the user presses Enter
- **THEN** no RPC is dispatched

#### Scenario: Upload limit toggle and submit behave symmetrically to download

- **WHEN** the "Limit Upload (KB/s)" toggler is switched and the user submits the field
- **THEN** the same rules as download apply, using `uploadLimited` and `uploadLimit`

#### Scenario: Honor Global Speed Limits toggle sends full bandwidth state

- **WHEN** the "Honor Global Speed Limits" toggler is switched OFF
- **THEN** a `torrent-set` RPC call is dispatched with `honorsSessionLimits=false`,
  `downloadLimited`, `downloadLimit`, `uploadLimited`, and `uploadLimit` all set to their
  current values, so that per-torrent limits take effect immediately (the torrent will
  ignore both standard and alternative global limits and use only its own per-torrent caps)
- **WHEN** the "Honor Global Speed Limits" toggler is switched ON
- **THEN** a `torrent-set` RPC call is dispatched with `honorsSessionLimits=true` and the
  same bandwidth context (the torrent will again respect the active global limit: standard
  when turtle mode is off, alternative when turtle mode is on)

#### Scenario: Seeding ratio segmented control applies immediately

- **WHEN** the user selects "Custom" in the seeding ratio control
- **THEN** a `torrent-set` RPC call is dispatched with `seedRatioMode=1` and the current
  `seedRatioLimit` value
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
- **THEN** a `torrent-set` RPC call is dispatched with `seedRatioMode=1` and the entered
  value
- **WHEN** Enter is pressed and the mode is not Custom
- **THEN** no RPC is dispatched

#### Scenario: Options tab populated from selected torrent data

- **WHEN** the user selects a new torrent in the list
- **THEN** all Options controls are reset to the newly selected torrent's daemon-reported
  values

#### Scenario: Numeric-only input accepted in speed fields

- **WHEN** the user types a non-digit character into a KB/s text input
- **THEN** the character is silently discarded and the field value does not change

#### Scenario: Decimal input accepted in ratio field

- **WHEN** the user types digits and at most one `'.'` into the ratio text input
- **THEN** the field value is updated
- **WHEN** the user types a non-digit, non-`.` character into the ratio field
- **THEN** the character is silently discarded

### Requirement: Per-torrent options data in TorrentData

The `TorrentData` model SHALL include the following new fields fetched by `torrent-get`:

- `download_limited: bool`
- `download_limit: u64` (KB/s)
- `upload_limited: bool`
- `upload_limit: u64` (KB/s)
- `seed_ratio_limit: f64`
- `seed_ratio_mode: u8` (0 = global, 1 = per-torrent, 2 = unlimited)
- `honors_session_limits: bool`

#### Scenario: TorrentData carries per-torrent limit fields

- **WHEN** `torrent-get` response is received
- **THEN** each `TorrentData` entry includes all seven per-torrent limit fields with
  correct values deserialized from the RPC JSON
