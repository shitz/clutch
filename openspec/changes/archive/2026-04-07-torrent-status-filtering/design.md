## Context

Clutch polls the Transmission daemon on every tick and stores the full `Vec<TorrentData>` in
`TorrentListScreen`. Currently `view.rs` renders every torrent unconditionally. Transmission
exposes 7 integer status codes (0–6); the UI consolidates these into 5 semantic buckets for a
cleaner filter surface.

The existing codebase already has:

- `theme::segmented_control` for mutually-exclusive toggles — but filtering here is multi-select,
  so a bespoke filter-chip style is required.
- The `MATERIAL_ICONS` font constant and icon helpers in `theme.rs`.
- `TorrentListScreen` state struct in `src/screens/torrent_list/mod.rs`.

## Goals / Non-Goals

**Goals:**

- Add a `HashSet<StatusFilter>` to `TorrentListScreen` that drives client-side filtering.
- Render a horizontal filter chip row between the toolbar and the column headers.
- Each chip shows a real-time count of torrents matching its bucket.
- Multi-select: multiple status chips can be active simultaneously.
- "All" master chip selects/deselects all buckets at once.
- Material 3 chip appearance: outlined when unselected, soft primary-wash when selected, with a
  checkmark glyph prepended to selected chip labels.
- Show a centered placeholder string when the filtered list is empty.
- Add a new `m3_filter_chip` style closure to `theme.rs`.

**Non-Goals:**

- Server-side filtering (full `TorrentData` array continues to be fetched on every poll).
- Custom user-defined labels or tags.
- Horizontal scrolling of the chip row (5 chips fit comfortably within standard window widths).
- Persisting the active filter selection across app restarts.

## Decisions

### Decision 1: `StatusFilter` enum and consolidation mapping

A new `StatusFilter` enum is added in `src/screens/torrent_list/mod.rs` with 5 variants:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusFilter {
    Downloading,  // Transmission status 3 (queued) or 4 (downloading)
    Seeding,      // Transmission status 5 (queued) or 6 (seeding)
    Paused,       // Transmission status 0 (stopped)
    Active,       // derived: rate_download > 0 || rate_upload > 0
    Error,        // Transmission status 1, 2 (checking), or error_string non-empty
}
```

The mapping is a pure function `matching_filters(t: &TorrentData) -> Vec<StatusFilter>` placed
in `mod.rs`. Returning a `Vec` (rather than a single variant) is essential because "Active" is a
derived cross-cutting state: a torrent actively downloading at 500 KB/s must appear in both the
`Downloading` and `Active` buckets simultaneously. A single-return function cannot express this
overlap — it would silently drop the torrent from the `Active` count and hide it from the list
whenever the `Active` chip is the only one selected.

Count pass: iterate the returned `Vec` and increment each corresponding counter.
Filter pass: keep the torrent if `matching_filters(t).iter().any(|f| state.filters.contains(f))`.

**Alternative considered**: a single status integer → bucket lookup table. Rejected because
"Active" spans multiple raw statuses and requires inspecting rate fields; a function is cleaner.

**Alternative considered**: `status_to_filter` returning a single `StatusFilter`. Rejected
because it forces mutually exclusive buckets and breaks the `Active` overlap requirement.

### Decision 2: Default filter state — all variants active on launch

`TorrentListScreen::new()` initialises the `HashSet` with all 5 `StatusFilter` variants, so the
list is unfiltered on first load. This is consistent with the proposal: "all torrents visible by
default".

**Alternative considered**: empty set = show all. Rejected because it requires special-casing the
empty-set render path and makes "All" chip logic more confusing.

### Decision 3: `m3_filter_chip` style added to `theme.rs`

A new public function `m3_filter_chip(theme, status, is_selected) -> button::Style` is added to
`src/theme.rs`:

- **Selected**: Semi-transparent primary background (`alpha: 0.15`), primary text, no border.
- **Unselected/Hovered**: Subtle `alpha: 0.05` background fill on hover, `alpha: 0.3` outline
  border, standard text color.
- **Unselected/Default**: No background, `alpha: 0.3` outline, standard text color.
- Border radius: `8.0` (matches Material 3 chip shape, not full pill).

**Alternative considered**: reusing `segmented_control` style. Rejected — segmented controls are
mutually exclusive and have joined edges; filter chips are independent and spatially separated.

### Decision 4: `filter_chip` view helper as a module-private function in `view.rs`

A `filter_chip<'a, Message>` free function inside `torrent_list/view.rs` builds a single chip
element. It accepts `label`, `count`, `is_selected`, and `on_press` message. The count is
rendered at `11px` size with `alpha: 0.6` when unselected to establish typographic hierarchy. A
`\u{e876}` (Material Icons "done" checkmark) is prepended when selected.

**Checkmark layout stability**: dynamically inserting or removing the checkmark widget on
selection/deselection causes the label text to shift horizontally, producing a visible jitter.
To prevent this, give the checkmark `text` a fixed width (e.g. `Length::Fixed(18.0)`) so it
always occupies horizontal space regardless of whether it is visible (use an empty string when
unselected). This keeps the label position stable across state transitions. If jitter is still
noticeable during testing, an M3-compliant fallback is to omit the checkmark widget entirely and
rely solely on the background-color wash and primary text color to indicate selection.

### Decision 5: Two-pass render in `view()`

Inside `view()`:

1. **Count pass** — iterate `state.torrents` once, call `matching_filters(t)` for each torrent,
   and increment the counter for every bucket returned.
2. **Filter pass** — `.filter()` the sorted display slice using
   `matching_filters(t).iter().any(|f| state.filters.contains(f))`, then build rows.

Both passes happen entirely in the `view()` render function with no additional allocations beyond
the existing sort collect. The count pass must run over the **full** torrent list so chip counts
reflect reality even when the chip itself is unselected.

### Decision 6: New `Message` variants

Two variants are added to the `Message` enum:

```rust
FilterAllClicked,
FilterToggled(StatusFilter),
```

`FilterToggled` just receives the variant; the update handler decides whether to insert or remove
it based on current state (no boolean flag parameter needed — simpler message shape).

**Alternative considered**: `FilterToggled(StatusFilter, bool)` with an explicit enable flag as
shown in the proposal's code snippet. Rejected — the `bool` is redundant because the update
handler always knows the current state; it adds noise at the call site.

### Decision 7: Empty-state placeholder

When the filtered list is empty (but torrent data has been loaded), render a centered
`text("No torrents match the selected filters.")` in place of the rows. This is distinct from the
initial-load spinner / "connecting…" state.

## Risks / Trade-offs

**"Active" bucket overlap** → `matching_filters` returns both `Downloading` and `Active` for an
actively-downloading torrent. The `any()` membership check means it appears in the list whenever
either bucket is selected, and it is counted under both chips — which is correct expected behavior.

**All-chips deselected leaves blank list** → The empty-state placeholder handles this gracefully.
No enforcement of "at least one chip must remain active" is needed; the placeholder is clear.

**Window resize clipping** → With 5 chips the row fits comfortably at standard widths (≥800 px).
If future chips are added, wrap the chip row in a `scrollable` with a hidden scrollbar.

**Count pass on every frame** → The count tally runs on every `view()` call. Given typical
torrent library sizes (≤ a few hundred) this is a trivial linear scan with no performance impact.

## Open Questions

None — all decisions are resolved based on the proposal and the provided implementation guidance.
