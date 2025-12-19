use crate::net::net::{InterfaceSet, InterfaceStats};

use libc::*;
use std::collections::HashMap;
use std::ffi::CStr;

#[cfg(target_os = "macos")]
pub fn fetch_net_stats(selected: &InterfaceSet) -> Vec<InterfaceStats> {
    unsafe {
        let mut ifap: *mut ifaddrs = std::ptr::null_mut();
        if getifaddrs(&mut ifap) != 0 {
            return vec![];
        }

        let mut acc: HashMap<String, (u64, u64)> = HashMap::new();
        let mut cur = ifap;

        while !cur.is_null() {
            let ifa = &*cur;

            if !ifa.ifa_data.is_null() {
                let name = CStr::from_ptr(ifa.ifa_name).to_string_lossy().into_owned();

                if selected.contains(&name) {
                    let data = &*(ifa.ifa_data as *const if_data);
                    let entry = acc.entry(name).or_insert((0, 0));
                    entry.0 += data.ifi_ibytes as u64;
                    entry.1 += data.ifi_obytes as u64;
                }
            }

            cur = ifa.ifa_next;
        }

        freeifaddrs(ifap);

        acc.into_iter()
            .map(|(iface, (rx, tx))| InterfaceStats {
                interface: iface,
                rx_bytes: rx,
                tx_bytes: tx,
                kind: super::net::InterfaceType::Net,
            })
            .collect()
    }
}
