use std::collections::HashSet;

#[derive(Clone, Debug)]
pub enum InterfaceType {
    Net,
    Wan,
}

#[derive(Debug, Clone)]
pub struct InterfaceStats {
    pub interface: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub kind: InterfaceType,
}

pub type InterfaceSet = HashSet<String>;
