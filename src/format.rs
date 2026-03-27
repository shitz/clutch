//! Shared formatting helpers for torrent data values.
//!
//! All functions return `"—"` for sentinel values (negative or zero where
//! semantically absent).

/// Format a byte count as a human-readable string (e.g. `"1.00 GB"`).
///
/// Returns `"—"` for negative values (sentinel for unavailable).
pub fn format_size(bytes: i64) -> String {
    if bytes < 0 {
        return "—".to_owned();
    }
    let bytes = bytes as u64;
    const GIB: u64 = 1 << 30;
    const MIB: u64 = 1 << 20;
    const KIB: u64 = 1 << 10;
    if bytes >= GIB {
        format!("{:.2} GB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.2} MB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.2} KB", bytes as f64 / KIB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Format a bytes-per-second rate (e.g. `"1.00 MB/s"`).
///
/// Returns `"—"` for zero or negative values (idle / unavailable).
pub fn format_speed(bps: i64) -> String {
    if bps <= 0 {
        return "—".to_owned();
    }
    let bps = bps as u64;
    const MIB: u64 = 1 << 20;
    const KIB: u64 = 1 << 10;
    if bps >= MIB {
        format!("{:.2} MB/s", bps as f64 / MIB as f64)
    } else if bps >= KIB {
        format!("{:.2} KB/s", bps as f64 / KIB as f64)
    } else {
        format!("{bps} B/s")
    }
}

/// Format an ETA in seconds to a human-readable duration string.
///
/// Returns `"—"` when `secs` is negative (Transmission sentinel for unknown).
pub fn format_eta(secs: i64) -> String {
    if secs < 0 {
        return "—".to_owned();
    }
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        let m = secs / 60;
        let s = secs % 60;
        format!("{m}m {s}s")
    } else {
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        format!("{h}h {m}m")
    }
}

/// Format a Unix timestamp as a relative "time ago" string.
pub fn format_ago(unix_secs: i64) -> String {
    if unix_secs <= 0 {
        return "Never".to_owned();
    }
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let diff = now.saturating_sub(unix_secs);
    if diff < 60 {
        format!("{diff}s ago")
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else {
        format!("{}d ago", diff / 86400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_size_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn format_size_kib() {
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(2048), "2.00 KB");
    }

    #[test]
    fn format_size_mib() {
        assert_eq!(format_size(1 << 20), "1.00 MB");
    }

    #[test]
    fn format_size_gib() {
        assert_eq!(format_size(1 << 30), "1.00 GB");
    }

    #[test]
    fn format_size_negative_sentinel() {
        assert_eq!(format_size(-1), "—");
    }

    #[test]
    fn format_speed_zero_is_dash() {
        assert_eq!(format_speed(0), "—");
    }

    #[test]
    fn format_speed_bps() {
        assert_eq!(format_speed(512), "512 B/s");
    }

    #[test]
    fn format_speed_kibps() {
        assert_eq!(format_speed(1024), "1.00 KB/s");
    }

    #[test]
    fn format_speed_mibps() {
        assert_eq!(format_speed(1 << 20), "1.00 MB/s");
    }

    #[test]
    fn format_eta_unknown() {
        assert_eq!(format_eta(-1), "—");
    }

    #[test]
    fn format_eta_zero() {
        assert_eq!(format_eta(0), "0s");
    }

    #[test]
    fn format_eta_seconds() {
        assert_eq!(format_eta(45), "45s");
    }

    #[test]
    fn format_eta_minutes() {
        assert_eq!(format_eta(90), "1m 30s");
        assert_eq!(format_eta(3599), "59m 59s");
    }

    #[test]
    fn format_eta_hours() {
        assert_eq!(format_eta(3600), "1h 0m");
        assert_eq!(format_eta(7200), "2h 0m");
    }
}
