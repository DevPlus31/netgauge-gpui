/// Converts bytes per second to human-readable string (B/s, KB/s, MB/s, GB/s)
pub fn human_bytes_per_sec(bytes: u64) -> String {
    let b = bytes as f64;
    if b < 1024.0 {
        format!("{:.0} B/s", b)
    } else if b < 1024.0 * 1024.0 {
        format!("{:.2} KB/s", b / 1024.0)
    } else if b < 1024.0 * 1024.0 * 1024.0 {
        format!("{:.2} MB/s", b / 1024.0 / 1024.0)
    } else {
        format!("{:.2} GB/s", b / 1024.0 / 1024.0 / 1024.0)
    }
}

/// Converts bytes per second to human-readable bits per second (bps, Kbps, Mbps, Gbps)
pub fn human_bits_per_sec(bytes: u64) -> String {
    let bps = bytes as f64 * 8.0;
    if bps < 1024.0 {
        format!("{:.0} bps", bps)
    } else if bps < 1024.0 * 1024.0 {
        format!("{:.2} Kbps", bps / 1024.0)
    } else if bps < 1024.0 * 1024.0 * 1024.0 {
        format!("{:.2} Mbps", bps / 1024.0 / 1024.0)
    } else {
        format!("{:.2} Gbps", bps / 1024.0 / 1024.0 / 1024.0)
    }
}
