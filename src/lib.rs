pub mod net;

pub use net::fetch_net_stats;
pub use net::format;
pub use net::net::{InterfaceSet, InterfaceStats, InterfaceType};
pub use net::tracker::{DeltaTracker, NetDelta};
pub use net::wan::snmp::{fetch_wan_stats, is_snmp_available};

