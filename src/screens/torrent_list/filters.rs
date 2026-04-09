// Copyright 2026 The clutch authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Filtering and count helpers for torrent-list status chips.

use std::collections::HashSet;

use crate::rpc::TorrentData;

use super::StatusFilter;
use super::sort::{SortColumn, SortDir, sort_torrents};

/// Aggregate counts for the torrent-list status filter chips.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FilterCounts {
    pub downloading: u32,
    pub seeding: u32,
    pub paused: u32,
    pub active: u32,
    pub error: u32,
}

/// Returns all [`StatusFilter`] buckets that apply to `t`.
///
/// A torrent may match more than one bucket simultaneously; for example, a
/// torrent actively downloading at 500 KB/s will be in both `Downloading` and
/// `Active`. The caller should use `.iter().any(|f| set.contains(f))` to
/// decide visibility.
pub fn matching_filters(t: &TorrentData) -> Vec<StatusFilter> {
    let mut out = Vec::with_capacity(2);
    match t.status {
        3 | 4 => out.push(StatusFilter::Downloading),
        5 | 6 => out.push(StatusFilter::Seeding),
        0 => out.push(StatusFilter::Paused),
        1 | 2 => out.push(StatusFilter::Error),
        _ => {}
    }
    if t.rate_download > 0 || t.rate_upload > 0 {
        out.push(StatusFilter::Active);
    }
    out
}

/// Count how many torrents belong to each status-filter bucket.
pub(crate) fn count_filters(torrents: &[TorrentData]) -> FilterCounts {
    let mut counts = FilterCounts::default();

    for t in torrents {
        // Inline the matching logic to avoid per-torrent Vec allocation.
        match t.status {
            3 | 4 => counts.downloading += 1,
            5 | 6 => counts.seeding += 1,
            0 => counts.paused += 1,
            1 | 2 => counts.error += 1,
            _ => {}
        }
        if t.rate_download > 0 || t.rate_upload > 0 {
            counts.active += 1;
        }
    }

    counts
}

/// Return whether a torrent should remain visible under the current filter set.
pub(crate) fn torrent_matches_active_filters(
    torrent: &TorrentData,
    active_filters: &HashSet<StatusFilter>,
) -> bool {
    // Inline the matching logic to avoid per-torrent Vec allocation.
    let status_match = match torrent.status {
        3 | 4 => active_filters.contains(&StatusFilter::Downloading),
        5 | 6 => active_filters.contains(&StatusFilter::Seeding),
        0 => active_filters.contains(&StatusFilter::Paused),
        1 | 2 => active_filters.contains(&StatusFilter::Error),
        _ => false,
    };
    let active_match = (torrent.rate_download > 0 || torrent.rate_upload > 0)
        && active_filters.contains(&StatusFilter::Active);
    status_match || active_match
}

/// Sort the list, then retain only the torrents allowed by the active filters.
pub(crate) fn display_torrents<'a>(
    torrents: &'a [TorrentData],
    sort_column: Option<SortColumn>,
    sort_dir: SortDir,
    active_filters: &HashSet<StatusFilter>,
) -> Vec<&'a TorrentData> {
    let sorted: Vec<&TorrentData> = match sort_column {
        Some(column) => sort_torrents(torrents, column, sort_dir),
        None => torrents.iter().collect(),
    };

    sorted
        .into_iter()
        .filter(|torrent| torrent_matches_active_filters(torrent, active_filters))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_torrent(
        id: i64,
        name: &str,
        status: i32,
        rate_download: i64,
        rate_upload: i64,
    ) -> TorrentData {
        TorrentData {
            id,
            name: name.to_owned(),
            status,
            rate_download,
            rate_upload,
            ..Default::default()
        }
    }

    #[test]
    fn count_filters_tallies_multi_bucket_membership() {
        let torrents = vec![
            make_torrent(1, "alpha", 4, 1024, 0),
            make_torrent(2, "beta", 0, 0, 0),
            make_torrent(3, "gamma", 1, 0, 0),
        ];

        let counts = count_filters(&torrents);

        assert_eq!(counts.downloading, 1);
        assert_eq!(counts.active, 1);
        assert_eq!(counts.paused, 1);
        assert_eq!(counts.error, 1);
        assert_eq!(counts.seeding, 0);
    }

    #[test]
    fn display_torrents_applies_sort_before_filtering() {
        let torrents = vec![
            make_torrent(1, "zeta", 4, 1, 0),
            make_torrent(2, "alpha", 0, 0, 0),
            make_torrent(3, "beta", 4, 1, 0),
        ];
        let filters = HashSet::from([StatusFilter::Downloading]);

        let display = display_torrents(&torrents, Some(SortColumn::Name), SortDir::Asc, &filters);

        assert_eq!(display.len(), 2);
        assert_eq!(display[0].name, "beta");
        assert_eq!(display[1].name, "zeta");
    }
}
