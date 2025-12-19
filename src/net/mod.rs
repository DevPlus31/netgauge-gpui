pub mod format;
pub mod net;
pub mod tracker;
pub mod wan;

#[cfg(target_os = "macos")]
mod net_macos;

#[cfg(target_os = "macos")]
pub use net_macos::fetch_net_stats;

#[cfg(target_os = "windows")]
mod net_windows;

#[cfg(target_os = "windows")]
pub use net_windows::fetch_net_stats;

#[cfg(target_os = "linux")]
mod net_linux;

#[cfg(target_os = "linux")]
pub use net_linux::fetch_net_stats;
