## 1. Data Model — `StatusFilter` enum and mapping

- [x] 1.1 Add `StatusFilter` enum (`Downloading`, `Seeding`, `Paused`, `Active`, `Error`) with
      `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]` to `src/screens/torrent_list/mod.rs`
- [x] 1.2 Add `matching_filters(t: &TorrentData) -> Vec<StatusFilter>` pure function in `mod.rs`
      that returns all buckets a torrent belongs to (a torrent may match multiple buckets;
      `Active` is derived from `rate_download > 0 || rate_upload > 0`)
- [x] 1.3 Add `filters: HashSet<StatusFilter>` field to `TorrentListScreen` struct, initialised
      with all 5 variants in `TorrentListScreen::new()`

## 2. Message variants

- [x] 2.1 Add `FilterAllClicked` variant to the `Message` enum in `mod.rs`
- [x] 2.2 Add `FilterToggled(StatusFilter)` variant to the `Message` enum in `mod.rs`

## 3. Theme — `m3_filter_chip` style

- [x] 3.1 Add `pub fn m3_filter_chip(theme: &Theme, status: button::Status, is_selected: bool) -> button::Style`
      to `src/theme.rs` implementing the three visual states described in the design (selected:
      15% primary fill; hovered unselected: 5% foreground fill with outline; default unselected:
      no fill with outline; border radius 8 px)

## 4. Update logic

- [x] 4.1 Handle `Message::FilterToggled(filter)` in `src/screens/torrent_list/update.rs`:
      if the filter is present in `state.filters`, remove it; otherwise insert it
- [x] 4.2 Handle `Message::FilterAllClicked` in `update.rs`:
      if `state.filters.len() == 5`, clear the `HashSet`; otherwise re-populate it with all
      5 `StatusFilter` variants

## 5. View — filter chip row

- [x] 5.1 Add module-private `filter_chip<'a>(label, count, is_selected, on_press) -> Element`
      helper in `src/screens/torrent_list/view.rs` that prepends a `\u{e876}` checkmark glyph
      (MATERIAL_ICONS font, 16 px) when selected, renders the count at 11 px with reduced opacity,
      and applies `m3_filter_chip` style
- [x] 5.2 Add a count-pass at the top of `view()` that tallies per-bucket counts by iterating
      `state.torrents` using `matching_filters`; store results as local `u32` variables
- [x] 5.3 Build a `chips_row` using the six `filter_chip` calls ("All", "Downloading", "Seeding",
      "Paused", "Active", "Error") with the correct `is_selected` logic for each; the "All" chip is
      selected when `state.filters.len() == 5`
- [x] 5.4 Insert `chips_row` into the main layout column between the toolbar/error banner and the
      sticky column header

## 6. View — filtered row rendering

- [x] 6.1 Replace the existing `display` vector construction in `view()` with a two-step pipeline:
      first sort (existing logic), then `.filter()` using `matching_filters` against `state.filters`
- [x] 6.2 After the filter pass, if `state.initial_load_done` is true and the filtered list is
      empty but `state.torrents` is non-empty, render a centered
      `text("No torrents match the selected filters.")` placeholder instead of the rows

## 7. Tests

- [x] 7.1 Add `#[cfg(test)]` module to `src/screens/torrent_list/mod.rs` with unit tests for
      `matching_filters`: assert each Transmission status integer maps to the correct bucket(s),
      assert a torrent with `rate_download > 0` returns both its integer-based bucket and `Active`,
      and assert a stopped torrent with a non-empty `error_string` returns both `Paused` and `Error`
- [x] 7.2 Add unit tests for the `FilterAllClicked` update handler: assert that clicking All when
      all 5 filters are active clears the set, and clicking All when fewer than 5 are active
      restores all 5
- [x] 7.3 Add unit tests for the `FilterToggled` update handler: assert that toggling a present
      filter removes it and toggling an absent filter inserts it

## 8. Quality gates

- [x] 8.1 Run `cargo fmt` — no formatting warnings
- [x] 8.2 Run `cargo check` — no compile errors
- [x] 8.3 Run `cargo clippy -- -D warnings` — no clippy warnings
- [x] 8.4 Run `cargo test` — all tests pass
