use crate::net::net::{InterfaceStats, InterfaceType};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct NetDelta {
    pub interface: String,
    pub rx_delta: u64,
    pub tx_delta: u64,
    pub kind: InterfaceType,
}

#[derive(Default)]
pub struct DeltaTracker {
    previous: HashMap<String, (u64, u64)>, // (rx, tx)
}

impl DeltaTracker {
    pub fn new() -> Self {
        Self {
            previous: HashMap::new(),
        }
    }

    pub fn update(&mut self, stats: &[InterfaceStats]) -> Vec<NetDelta> {
        let mut deltas = Vec::with_capacity(stats.len());

        for s in stats {
            let (prev_rx, prev_tx) = self
                .previous
                .get(&s.interface)
                .copied()
                .unwrap_or((s.rx_bytes, s.tx_bytes));

            let rx_delta = s.rx_bytes.saturating_sub(prev_rx);
            let tx_delta = s.tx_bytes.saturating_sub(prev_tx);

            self.previous
                .insert(s.interface.clone(), (s.rx_bytes as u64, s.tx_bytes as u64));

            deltas.push(NetDelta {
                interface: s.interface.clone(),
                rx_delta,
                tx_delta,
                kind: s.kind.clone(),
            });
        }

        deltas
    }
}
