//! SNMP interface discovery tool
//! Run with: cargo run --bin snmp_discover

use snmp2::{Oid, SyncSession, Value};
use std::time::Duration;

const SNMP_TARGET: &str = "192.168.1.1:161";
const SNMP_COMMUNITY: &[u8] = b"public";

fn main() {
    println!("Discovering SNMP interfaces on {}...\n", SNMP_TARGET);

    let timeout = Duration::from_secs(5);
    let mut sess = match SyncSession::new_v2c(SNMP_TARGET, SNMP_COMMUNITY, Some(timeout), 0) {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to connect: {:?}", e);
            println!("\nMake sure:");
            println!("  1. SNMP is enabled on your router");
            println!("  2. Community string is correct (try 'public' or 'private')");
            println!("  3. Router IP is correct ({})", SNMP_TARGET);
            return;
        }
    };

    // Walk ifDescr (1.3.6.1.2.1.2.2.1.2) to get interface names
    // Walk ifInOctets (1.3.6.1.2.1.2.2.1.10) to get RX bytes
    // Walk ifOutOctets (1.3.6.1.2.1.2.2.1.16) to get TX bytes

    println!("{:<6} {:<30} {:>15} {:>15}", "Index", "Interface Name", "RX Bytes", "TX Bytes");
    println!("{}", "-".repeat(70));

    // Try indexes 1-50 (most routers have fewer interfaces)
    for idx in 1u64..=50 {
        let descr_oid = Oid::from(&[1, 3, 6, 1, 2, 1, 2, 2, 1, 2, idx]).unwrap();
        let rx_oid = Oid::from(&[1, 3, 6, 1, 2, 1, 2, 2, 1, 10, idx]).unwrap();
        let tx_oid = Oid::from(&[1, 3, 6, 1, 2, 1, 2, 2, 1, 16, idx]).unwrap();

        // Get interface description
        let name = match sess.get(&descr_oid) {
            Ok(resp) => match resp.varbinds.into_iter().next() {
                Some((_, Value::OctetString(bytes))) => {
                    String::from_utf8_lossy(&bytes).to_string()
                }
                _ => continue, // No interface at this index
            },
            Err(_) => continue,
        };

        // Get RX bytes
        let rx = match sess.get(&rx_oid) {
            Ok(resp) => match resp.varbinds.into_iter().next() {
                Some((_, Value::Counter32(v))) => v as u64,
                Some((_, Value::Counter64(v))) => v,
                _ => 0,
            },
            Err(_) => 0,
        };

        // Get TX bytes
        let tx = match sess.get(&tx_oid) {
            Ok(resp) => match resp.varbinds.into_iter().next() {
                Some((_, Value::Counter32(v))) => v as u64,
                Some((_, Value::Counter64(v))) => v,
                _ => 0,
            },
            Err(_) => 0,
        };

        // Print interface info
        println!("{:<6} {:<30} {:>15} {:>15}", idx, name, rx, tx);
    }

    println!("\n** Look for WAN/Internet/ppp/eth interfaces with high byte counts **");
    println!("** Use that index number in SNMP_IF_INDEX in main.rs **");
}

