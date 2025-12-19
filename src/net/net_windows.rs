use crate::net::net::{InterfaceStats, InterfaceType};
use std::collections::HashSet;
use windows::{
    Win32::Foundation::ERROR_SUCCESS,
    Win32::NetworkManagement::IpHelper::{GetIfTable2, MIB_IF_TABLE2},
};

#[cfg(target_os = "windows")]
pub fn fetch_net_stats(selected: &InterfaceSet) -> Vec<InterfaceStats> {
    let mut results = Vec::new();

    unsafe {
        let mut table: *mut MIB_IF_TABLE2 = std::ptr::null_mut();

        if GetIfTable2(&mut table) != ERROR_SUCCESS.0 || table.is_null() {
            return results; // empty vec on failure (same semantics as macOS)
        }

        let table_ref = &*table;

        for i in 0..table_ref.NumEntries {
            let row = &*table_ref.Table.offset(i as isize);

            let name = String::from_utf16_lossy(&row.Alias)
                .trim_end_matches('\0')
                .to_string();

            // If selected set is empty → collect all
            if !selected.is_empty() && !selected.contains(&name) {
                continue;
            }

            results.push(InterfaceStats {
                interface: name,
                rx_bytes: row.InOctets,
                tx_bytes: row.OutOctets,
                kind: InterfaceType::Net,
                snmp_available: false, // Windows local stats
            });
        }
    }

    results
}
