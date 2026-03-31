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

//! This module contains only sorting enums and the [`sort_torrents`] function.
//! It has no UI dependencies and can be tested in isolation.

use crate::rpc::TorrentData;

/// Column that the torrent list can be sorted by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    Name,
    Status,
    Size,
    SpeedDown,
    SpeedUp,
    Eta,
    Ratio,
    Progress,
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortDir {
    #[default]
    Asc,
    Desc,
}

/// Apply a single-column sort over a slice of torrents.
///
/// Returns references in the requested order; the backing slice is unchanged.
/// For ETA, unknown values (-1) are sorted to the end.
/// For Ratio and Progress, `partial_cmp` is used with `Equal` as fallback.
#[must_use]
pub fn sort_torrents(torrents: &[TorrentData], col: SortColumn, dir: SortDir) -> Vec<&TorrentData> {
    let mut sorted: Vec<&TorrentData> = torrents.iter().collect();
    sorted.sort_by(|a, b| {
        let ord = match col {
            SortColumn::Name => a.name.cmp(&b.name),
            SortColumn::Status => a.status.cmp(&b.status),
            SortColumn::Size => a.total_size.cmp(&b.total_size),
            SortColumn::SpeedDown => a.rate_download.cmp(&b.rate_download),
            SortColumn::SpeedUp => a.rate_upload.cmp(&b.rate_upload),
            SortColumn::Eta => {
                // -1 = unknown; sort unknown ETAs to the end.
                let ea = if a.eta < 0 { i64::MAX } else { a.eta };
                let eb = if b.eta < 0 { i64::MAX } else { b.eta };
                ea.cmp(&eb)
            }
            SortColumn::Ratio => {
                let ra = a.upload_ratio.max(0.0);
                let rb = b.upload_ratio.max(0.0);
                ra.partial_cmp(&rb).unwrap_or(std::cmp::Ordering::Equal)
            }
            SortColumn::Progress => a
                .percent_done
                .partial_cmp(&b.percent_done)
                .unwrap_or(std::cmp::Ordering::Equal),
        };
        if dir == SortDir::Desc {
            ord.reverse()
        } else {
            ord
        }
    });
    sorted
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_torrent(id: i64, name: &str) -> TorrentData {
        TorrentData {
            id,
            name: name.to_owned(),
            status: 6,
            percent_done: 1.0,
            ..Default::default()
        }
    }

    fn make_list() -> Vec<TorrentData> {
        vec![
            TorrentData {
                id: 1,
                name: "charlie".into(),
                status: 6,
                total_size: 300,
                rate_download: 30,
                rate_upload: 3,
                eta: 30,
                upload_ratio: 0.3,
                percent_done: 0.3,
                ..Default::default()
            },
            TorrentData {
                id: 2,
                name: "alpha".into(),
                status: 0,
                total_size: 100,
                rate_download: 10,
                rate_upload: 1,
                eta: 10,
                upload_ratio: 0.1,
                percent_done: 0.1,
                ..Default::default()
            },
            TorrentData {
                id: 3,
                name: "bravo".into(),
                status: 4,
                total_size: 200,
                rate_download: 20,
                rate_upload: 2,
                eta: 20,
                upload_ratio: 0.2,
                percent_done: 0.2,
                ..Default::default()
            },
        ]
    }

    /// 10.1 – Empty list returns empty vec for any sort.
    #[test]
    fn sort_empty_list() {
        let torrents: Vec<TorrentData> = vec![];
        assert!(sort_torrents(&torrents, SortColumn::Name, SortDir::Asc).is_empty());
    }

    /// 10.2 – Single-element list is a no-op.
    #[test]
    fn sort_single_element() {
        let torrents = vec![make_torrent(1, "only")];
        let result = sort_torrents(&torrents, SortColumn::Name, SortDir::Asc);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
    }

    /// 10.3 – Ascending sort by Name.
    #[test]
    fn sort_by_name_asc() {
        let list = make_list();
        let result = sort_torrents(&list, SortColumn::Name, SortDir::Asc);
        let names: Vec<&str> = result.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["alpha", "bravo", "charlie"]);
    }

    /// 10.4 – Descending sort by Name.
    #[test]
    fn sort_by_name_desc() {
        let list = make_list();
        let result = sort_torrents(&list, SortColumn::Name, SortDir::Desc);
        let names: Vec<&str> = result.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["charlie", "bravo", "alpha"]);
    }

    /// 10.5 – Ascending sort by Status.
    #[test]
    fn sort_by_status_asc() {
        let list = make_list();
        let result = sort_torrents(&list, SortColumn::Status, SortDir::Asc);
        let ids: Vec<i64> = result.iter().map(|t| t.id).collect();
        // status: 0, 4, 6 → ids 2, 3, 1
        assert_eq!(ids, [2, 3, 1]);
    }

    /// 10.6 – Ascending sort by Size.
    #[test]
    fn sort_by_size_asc() {
        let list = make_list();
        let result = sort_torrents(&list, SortColumn::Size, SortDir::Asc);
        let ids: Vec<i64> = result.iter().map(|t| t.id).collect();
        assert_eq!(ids, [2, 3, 1]);
    }

    /// 10.7 – Ascending sort by SpeedDown.
    #[test]
    fn sort_by_speed_down_asc() {
        let list = make_list();
        let result = sort_torrents(&list, SortColumn::SpeedDown, SortDir::Asc);
        let ids: Vec<i64> = result.iter().map(|t| t.id).collect();
        assert_eq!(ids, [2, 3, 1]);
    }

    /// 10.8 – Ascending sort by SpeedUp.
    #[test]
    fn sort_by_speed_up_asc() {
        let list = make_list();
        let result = sort_torrents(&list, SortColumn::SpeedUp, SortDir::Asc);
        let ids: Vec<i64> = result.iter().map(|t| t.id).collect();
        assert_eq!(ids, [2, 3, 1]);
    }

    /// 10.9 – Ascending sort by ETA.
    #[test]
    fn sort_by_eta_asc() {
        let list = make_list();
        let result = sort_torrents(&list, SortColumn::Eta, SortDir::Asc);
        let ids: Vec<i64> = result.iter().map(|t| t.id).collect();
        assert_eq!(ids, [2, 3, 1]);
    }

    /// 10.10 – Unknown ETA (-1) sorts to end.
    #[test]
    fn sort_by_eta_unknown_last() {
        let mut list = make_list();
        list[0].eta = -1; // id=1 has unknown ETA
        let result = sort_torrents(&list, SortColumn::Eta, SortDir::Asc);
        assert_eq!(result.last().unwrap().id, 1);
    }

    /// 10.11 – Ascending sort by Ratio.
    #[test]
    fn sort_by_ratio_asc() {
        let list = make_list();
        let result = sort_torrents(&list, SortColumn::Ratio, SortDir::Asc);
        let ids: Vec<i64> = result.iter().map(|t| t.id).collect();
        assert_eq!(ids, [2, 3, 1]);
    }

    /// 10.12 – Ascending sort by Progress.
    #[test]
    fn sort_by_progress_asc() {
        let list = make_list();
        let result = sort_torrents(&list, SortColumn::Progress, SortDir::Asc);
        let ids: Vec<i64> = result.iter().map(|t| t.id).collect();
        assert_eq!(ids, [2, 3, 1]);
    }

    /// 10.13 – Descending sort reverses any column (spot-check with Size).
    #[test]
    fn sort_by_size_desc() {
        let list = make_list();
        let result = sort_torrents(&list, SortColumn::Size, SortDir::Desc);
        let ids: Vec<i64> = result.iter().map(|t| t.id).collect();
        assert_eq!(ids, [1, 3, 2]);
    }
}
