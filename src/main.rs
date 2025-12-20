mod net;

use net::wan::snmp::{fetch_wan_stats, is_snmp_available};
use net::{fetch_net_stats, format, net::InterfaceType, tracker::DeltaTracker};
use std::collections::HashSet;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let selected: HashSet<String> = ["eth0", "wlan0", "en0", "Wi-Fi"]
        .iter()
        .map(|s| s.to_string())
        .collect();

    let snmp_ok = is_snmp_available("192.168.1.1:161", b"public");
    let mut tracker = DeltaTracker::new();

    loop {
        let net_stats = fetch_net_stats(&selected);
        let mut all_stats = net_stats;

        if snmp_ok {
            let wan_stats = fetch_wan_stats("192.168.1.1:161", b"public", 26, "WAN");
            all_stats.push(wan_stats);
        } else {
            println!("SNMP unavailable!");
        }

        let deltas = tracker.update(&all_stats);
        for d in deltas {
            let label = match d.kind {
                InterfaceType::Net => "Net interface",
                InterfaceType::Wan => "WAN interface",
            };
            println!(
                "{} ({}) → RX: {} ({}) TX: {} ({})",
                d.interface,
                label,
                format::human_bytes_per_sec(d.rx_delta),
                format::human_bits_per_sec(d.rx_delta),
                format::human_bytes_per_sec(d.tx_delta),
                format::human_bits_per_sec(d.tx_delta),
            );
        }

        sleep(Duration::from_secs(1));
    }
}
