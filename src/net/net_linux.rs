use std::fs::read_to_string;

use crate::{InterfaceSet, InterfaceStats, InterfaceType};

pub fn fetch_net_stats(selected: &InterfaceSet) -> Vec<InterfaceStats> {
    let mut stats = Vec::new();

    let content = match read_to_string("/proc/net/dev") {
        Ok(c) => c,
        Err(_) => return stats,
    };

    for line in content.lines().skip(2) {
        // example: "eth0: 123 0 0 0 0 0 0 0 456 0 0 0 0 0 0 0"
        let line = line.trim();
        let mut parts = line.split(':');

        let iface = match parts.next() {
            Some(i) => i.trim(),
            None => continue,
        };

        if !selected.matches(iface) {
            continue;
        }

        let data = match parts.next() {
            Some(d) => d.split_whitespace().collect::<Vec<_>>(),
            None => continue,
        };

        if data.len() < 16 {
            continue;
        }

        let rx_bytes = data[0].parse::<u64>().unwrap_or(0);
        let tx_bytes = data[8].parse::<u64>().unwrap_or(0);

        stats.push(InterfaceStats {
            interface: iface.to_string(),
            rx_bytes,
            tx_bytes,
            kind: InterfaceType::Net,
        });
    }

    stats
}
