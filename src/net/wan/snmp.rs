use crate::net::net::{InterfaceStats, InterfaceType};
use snmp2::{Oid, SyncSession, Value};

/// Fetch SNMP WAN interface counters
pub fn fetch_wan_stats(
    target: &str,
    community: &[u8],
    if_index: u32,
    iface_name: &str,
) -> InterfaceStats {
    // Convert if_index to u64 for OID
    let idx = if_index as u64;

    // Build OIDs for ifInOctets and ifOutOctets
    let rx_oid = Oid::from(&[1, 3, 6, 1, 2, 1, 2, 2, 1, 10, idx]).unwrap();
    let tx_oid = Oid::from(&[1, 3, 6, 1, 2, 1, 2, 2, 1, 16, idx]).unwrap();

    // Create SNMP v2c session
    let timeout = std::time::Duration::from_secs(2);
    let mut sess = SyncSession::new_v2c(target, community, Some(timeout), 0)
        .expect("Failed to create SNMP session");

    // Fetch RX bytes
    let rx_bytes = match sess.get(&rx_oid).unwrap().varbinds.next() {
        Some((_oid, Value::Counter32(v))) => v as u64,
        Some((_oid, Value::Counter64(v))) => v,
        _ => 0,
    };

    // Fetch TX bytes
    let tx_bytes = match sess.get(&tx_oid).unwrap().varbinds.next() {
        Some((_oid, Value::Counter32(v))) => v as u64,
        Some((_oid, Value::Counter64(v))) => v,
        _ => 0,
    };

    // Return unified InterfaceStats
    InterfaceStats {
        interface: iface_name.to_string(),
        rx_bytes,
        tx_bytes,
        kind: InterfaceType::Wan,
    }
}

/// Check if SNMP is available on a router
/// Returns true if a simple SNMP get succeeds
pub fn is_snmp_available(target: &str, community: &[u8]) -> bool {
    let timeout = std::time::Duration::from_secs(2);

    // Try creating a session
    let mut session = match SyncSession::new_v2c(target, community, Some(timeout), 0) {
        Ok(s) => s,
        Err(_) => return false,
    };

    // Try fetching a common OID (sysDescr.0)
    let sys_descr_oid = Oid::from(&[1, 3, 6, 1, 2, 1, 1, 1, 0]).unwrap();

    match session.get(&sys_descr_oid) {
        Ok(mut vb) => match vb.varbinds.next() {
            Some((_oid, Value::OctetString(_))) => true, // SNMP responds correctly
            Some((_oid, _)) => true,                     // SNMP responds with some value
            None => false,                               // No varbind returned
        },
        Err(_) => false, // SNMP get failed
    }
}
