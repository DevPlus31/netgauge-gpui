#[macro_use]
mod declarative_ui;

use declarative_ui::styled_div;
use gpui::{
    prelude::*, px, size, App, Application, AnyElement, AsyncApp, Bounds,
    Context, Global, Timer, Window, WindowBounds, WindowOptions,
};
use netgauge::{
    detect_interface_index, fetch_net_stats, fetch_wan_stats, format, is_snmp_available,
    list_interfaces, DeltaTracker, InterfaceSet, InterfaceType,
};
use std::sync::{Arc, RwLock};
use std::time::Duration;

// ============================================================================
// SNMP Configuration (customize for your router)
// ============================================================================

const SNMP_TARGET: &str = "192.168.1.1:161";
const SNMP_COMMUNITY: &[u8] = b"public";
const SNMP_IF_PATTERN: &str = "ppp"; // Pattern to search for WAN interface (e.g., "ppp", "wan")

// ============================================================================
// Global State
// ============================================================================

#[derive(Clone, Debug)]
struct InterfaceMetric {
    name: String,
    rx_speed: String,
    tx_speed: String,
    is_wan: bool,
}

#[derive(Clone)]
struct NetGaugeState {
    interfaces: Vec<InterfaceMetric>,
    snmp_available: bool,
    available_interfaces: Vec<String>,
    selected_interfaces: Arc<RwLock<InterfaceSet>>,
}

impl Global for NetGaugeState {}

// ============================================================================
// App View
// ============================================================================

struct AppView;

impl AppView {
    fn build_interface_card(&self, metric: &InterfaceMetric) -> gpui::Div {
        let bg_style = if metric.is_wan {
            "flex row items-center justify-between bg-wan p-4 gap-4"
        } else {
            "flex row items-center justify-between bg-gray p-4 gap-4"
        };

        let label = if metric.is_wan {
            format!("🌐 {}", metric.name)
        } else {
            metric.name.clone()
        };

        let rx = metric.rx_speed.clone();
        let tx = metric.tx_speed.clone();

        ui! {
            div[bg_style] {
                div["bold text-white"] { text[label] }
                div["flex col gap-1"] {
                    div["flex row gap-2 text-white"] {
                        text["↓"]
                        text[rx]
                    }
                    div["flex row gap-2 text-white"] {
                        text["↑"]
                        text[tx]
                    }
                }
            }
        }
    }
}

impl Render for AppView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = cx.global::<NetGaugeState>();
        let interfaces = state.interfaces.clone();
        let snmp_available = state.snmp_available;

        let snmp_status = if snmp_available { "SNMP: ✓" } else { "SNMP: ✗" };

        // Build interface cards
        let cards: Vec<_> = interfaces.iter().map(|m| self.build_interface_card(m)).collect();

        // Settings button with click handler
        let settings_btn = styled_div("text-sm text-blue cursor-pointer")
            .child("⚙ Interfaces")
            .on_mouse_down(gpui::MouseButton::Left, |_ev, _window, cx| {
                let bounds = Bounds::centered(None, size(px(450.), px(400.)), cx);
                cx.open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        is_resizable: true,
                        ..Default::default()
                    },
                    |_window, cx| cx.new(|_cx| InterfaceSelectorView),
                )
                .expect("Failed to open interface selector window");
            });

        // Build content with cards
        let mut content = styled_div("flex col gap-4 p-4");
        for card in cards {
            content = content.child(card);
        }

        ui! {
            div["flex col bg-dark size-full"] {
                div["flex row items-center justify-between p-4 bg-gray"] {
                    div["text-xl bold text-white"] { text["NetGauge"] }
                    div["flex row gap-4"] {
                        div["text-sm text-dim"] { text[snmp_status] }
                        { settings_btn }
                    }
                }
                { content }
            }
        }
    }
}

// ============================================================================
// Interface Selector View
// ============================================================================

struct InterfaceSelectorView;

impl Render for InterfaceSelectorView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = cx.global::<NetGaugeState>();
        let available = state.available_interfaces.clone();
        let selected_lock = state.selected_interfaces.clone();

        // Create the item renderer for the list
        let selected_for_render = selected_lock.clone();
        let available_for_render = available.clone();
        let item_renderer = move |ix: usize| -> AnyElement {
            let available = available_for_render.clone();
            let selected_lock = selected_for_render.clone();
            let selected = selected_lock.read().unwrap().clone();

            let iface = available[ix].clone();
            let is_selected = selected.contains(&iface);
            let checkbox = if is_selected { "☑" } else { "☐" };
            let label = format!("{} {}", checkbox, iface);

            let iface_clone = iface.clone();
            let selected_clone = selected_lock.clone();

            styled_div("flex row items-center gap-2 p-2 bg-gray text-white cursor-pointer")
                .child(label)
                .on_mouse_down(gpui::MouseButton::Left, move |_ev, _window, cx| {
                    let mut sel = selected_clone.write().unwrap();
                    if sel.contains(&iface_clone) {
                        sel.remove(&iface_clone);
                    } else {
                        sel.insert(iface_clone.clone());
                    }
                    cx.refresh_windows();
                })
                .into_any_element()
        };

        // Build the list element
        let list = declarative_ui::styled_list(
            "interface-list",
            available.len(),
            "flex-grow p-4",
            item_renderer,
        );

        ui! {
            div["flex col bg-dark size-full"] {
                div["flex row items-center justify-between p-4 bg-gray bold text-white"] {
                    text["Select Interfaces"]
                }
                { list }
            }
        }
    }
}

// ============================================================================
// Main
// ============================================================================

fn main() {
    Application::new().run(|cx: &mut App| {
        // Check SNMP availability and auto-detect ppp interface
        let snmp_available = is_snmp_available(SNMP_TARGET, SNMP_COMMUNITY);
        let wan_interface = if snmp_available {
            detect_interface_index(SNMP_TARGET, SNMP_COMMUNITY, SNMP_IF_PATTERN)
        } else {
            None
        };

        // Log detected interface
        if let Some((idx, name)) = &wan_interface {
            println!("Auto-detected WAN interface: {} (index {})", name, idx);
        } else if snmp_available {
            println!("SNMP available but no '{}' interface found", SNMP_IF_PATTERN);
        }

        // Get available interfaces
        let available_interfaces = list_interfaces();

        // Default selected interfaces
        let default_selected: InterfaceSet = ["eth0", "wlan0", "en0", "WiFi", "Ethernet"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let selected_interfaces = Arc::new(RwLock::new(default_selected));

        // Initialize global state
        cx.set_global(NetGaugeState {
            interfaces: vec![InterfaceMetric {
                name: "Loading...".to_string(),
                rx_speed: "-- B/s".to_string(),
                tx_speed: "-- B/s".to_string(),
                is_wan: false,
            }],
            snmp_available,
            available_interfaces,
            selected_interfaces: selected_interfaces.clone(),
        });

        // Spawn background polling task
        let selected_for_task = selected_interfaces.clone();
        cx.spawn(async move |cx: &mut AsyncApp| {
            let mut tracker = DeltaTracker::new();

            loop {
                // Get current selected interfaces
                let selected = selected_for_task.read().unwrap().clone();

                // Fetch local interface stats
                let stats = fetch_net_stats(&selected);
                let deltas = tracker.update(&stats);

                let mut metrics: Vec<InterfaceMetric> = deltas
                    .iter()
                    .map(|d| InterfaceMetric {
                        name: d.interface.clone(),
                        rx_speed: format::human_bytes_per_sec(d.rx_delta),
                        tx_speed: format::human_bytes_per_sec(d.tx_delta),
                        is_wan: d.kind == InterfaceType::Wan,
                    })
                    .collect();

                // Fetch WAN stats via SNMP if available and interface detected
                if let Some((if_index, ref if_name)) = wan_interface {
                    let display_name = format!("{} (WAN)", if_name);
                    let wan_stats =
                        fetch_wan_stats(SNMP_TARGET, SNMP_COMMUNITY, if_index, &display_name);
                    let wan_deltas = tracker.update(&[wan_stats]);
                    for d in wan_deltas {
                        metrics.push(InterfaceMetric {
                            name: d.interface.clone(),
                            rx_speed: format::human_bytes_per_sec(d.rx_delta),
                            tx_speed: format::human_bytes_per_sec(d.tx_delta),
                            is_wan: true,
                        });
                    }
                }

                // Update global state and refresh windows
                let _ = cx.update_global::<NetGaugeState, _>(|state, cx| {
                    if metrics.is_empty() {
                        state.interfaces = vec![InterfaceMetric {
                            name: "No interfaces found".to_string(),
                            rx_speed: "-- B/s".to_string(),
                            tx_speed: "-- B/s".to_string(),
                            is_wan: false,
                        }];
                    } else {
                        state.interfaces = metrics;
                    }
                    // Trigger window redraw
                    cx.refresh_windows();
                });

                Timer::after(Duration::from_secs(1)).await;
            }
        })
        .detach();

        // Open window - compact height, non-resizable (disables maximize)
        let bounds = Bounds::centered(None, size(px(400.), px(300.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                is_resizable: false,
                ..Default::default()
            },
            |_window, cx| cx.new(|_cx| AppView),
        )
        .expect("Failed to open window");
    });
}
