#[macro_use]
mod declarative_ui;

use declarative_ui::{Color as DColor, Element as DElement, Style as DStyle};
use gpui::{
    div, prelude::*, px, rgb, size, App, Application, AnyElement, AsyncApp, Bounds, Context,
    FontWeight, Global, Timer, Window, WindowBounds, WindowOptions,
};
use netgauge::{
    fetch_net_stats, fetch_wan_stats, format, is_snmp_available, DeltaTracker, InterfaceSet,
    InterfaceType,
};
use std::time::Duration;

// ============================================================================
// SNMP Configuration (customize for your router)
// ============================================================================

const SNMP_TARGET: &str = "192.168.1.1:161";
const SNMP_COMMUNITY: &[u8] = b"public";
const SNMP_IF_INDEX: u32 = 26; // WAN interface index on your router
const SNMP_IF_NAME: &str = "WAN"; // Display name for WAN interface

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
}

impl Global for NetGaugeState {}

// ============================================================================
// App View
// ============================================================================

struct AppView;

impl AppView {
    fn render_element(&self, el: DElement, _cx: &mut Context<Self>) -> AnyElement {
        let mut gpui_el = div();

        // Apply styles
        for style in el.styles {
            gpui_el = match style {
                DStyle::Flex => gpui_el.flex(),
                DStyle::FlexCol => gpui_el.flex_col(),
                DStyle::FlexRow => gpui_el.flex_row(),
                DStyle::JustifyCenter => gpui_el.justify_center(),
                DStyle::JustifyBetween => gpui_el.justify_between(),
                DStyle::ItemsCenter => gpui_el.items_center(),
                DStyle::Gap(p) => gpui_el.gap(px(p)),
                DStyle::Padding(p) => gpui_el.p(px(p)),
                DStyle::Width(w) => gpui_el.w(px(w)),
                DStyle::Height(h) => gpui_el.h(px(h)),
                DStyle::Size(s) => gpui_el.size(px(s)),
                DStyle::SizeFull => gpui_el.size_full(),
                DStyle::Background(color) => gpui_el.bg(self.convert_color(color)),
                DStyle::TextColor(color) => gpui_el.text_color(self.convert_color(color)),
                DStyle::TextSize(s) => gpui_el.text_size(px(s)),
                DStyle::FontWeightBold => gpui_el.font_weight(FontWeight::BOLD),
            };
        }

        // Add children recursively
        for child in el.children {
            gpui_el = gpui_el.child(self.render_element(child, _cx));
        }

        // Add content if any
        if let Some(content) = el.content {
            gpui_el = gpui_el.child(content);
        }

        gpui_el.into_any_element()
    }

    fn convert_color(&self, color: DColor) -> gpui::Hsla {
        match color {
            DColor::Hex(h) => rgb(h).into(),
            DColor::Name("red") => gpui::red(),
            DColor::Name("green") => gpui::green(),
            DColor::Name("blue") => rgb(0x4a90e2).into(),
            DColor::Rgb(r, g, b) => {
                gpui::rgb((r as u32) << 16 | (g as u32) << 8 | (b as u32)).into()
            }
            _ => rgb(0x000000).into(),
        }
    }

    fn build_interface_card(&self, metric: &InterfaceMetric) -> DElement {
        // Use different background for WAN vs LAN
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

        ui! {
            div[bg_style] {
                div["bold text-white"] { text[label] }
                div["flex col gap-1"] {
                    div["flex row gap-2 text-white"] {
                        text["↓"]
                        text[metric.rx_speed.clone()]
                    }
                    div["flex row gap-2 text-white"] {
                        text["↑"]
                        text[metric.tx_speed.clone()]
                    }
                }
            }
        }
    }
}

impl Render for AppView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = cx.global::<NetGaugeState>();

        // Build interface cards dynamically
        let mut interface_cards: Vec<DElement> = Vec::new();
        for metric in &state.interfaces {
            interface_cards.push(self.build_interface_card(metric));
        }

        // SNMP status indicator
        let snmp_status = if state.snmp_available {
            "SNMP: ✓"
        } else {
            "SNMP: ✗"
        };

        // Build the UI tree
        let mut root = ui! {
            div["flex col bg-dark size-full"] {
                // Header
                div["flex row items-center justify-between p-4 bg-light-gray"] {
                    div["text-xl bold text-white"] { text["NetGauge"] }
                    div["flex row gap-4"] {
                        div["text-sm text-dim"] { text[snmp_status] }
                        div["text-sm text-dim"] { text["Network Monitor"] }
                    }
                }
                // Main Content
                div["flex col gap-4 p-4"] {
                }
                // Footer
                div["p-2 bg-footer text-xs text-dim"] {
                    text["Refreshing every 1s"]
                }
            }
        };

        // Insert interface cards into the main content area (second child)
        if root.children.len() >= 2 {
            for card in interface_cards {
                root.children[1] = root.children[1].clone().child(card);
            }
        }

        self.render_element(root, cx)
    }
}

// ============================================================================
// Main
// ============================================================================

fn main() {
    Application::new().run(|cx: &mut App| {
        // Check SNMP availability
        let snmp_available = is_snmp_available(SNMP_TARGET, SNMP_COMMUNITY);

        // Initialize global state
        cx.set_global(NetGaugeState {
            interfaces: vec![InterfaceMetric {
                name: "Loading...".to_string(),
                rx_speed: "-- B/s".to_string(),
                tx_speed: "-- B/s".to_string(),
                is_wan: false,
            }],
            snmp_available,
        });

        // Spawn background polling task
        cx.spawn(async move |cx: &mut AsyncApp| {
            let selected: InterfaceSet = ["eth0", "wlan0", "en0", "Wi-Fi", "Ethernet"]
                .iter()
                .map(|s| s.to_string())
                .collect();

            let mut tracker = DeltaTracker::new();

            loop {
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

                // Fetch WAN stats via SNMP if available
                if snmp_available {
                    let wan_stats =
                        fetch_wan_stats(SNMP_TARGET, SNMP_COMMUNITY, SNMP_IF_INDEX, SNMP_IF_NAME);
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

                // Update global state
                let _ = cx.update_global::<NetGaugeState, _>(|state, _cx| {
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
                });

                Timer::after(Duration::from_secs(1)).await;
            }
        })
        .detach();

        // Open window
        let bounds = Bounds::centered(None, size(px(400.), px(500.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_window, cx| cx.new(|_cx| AppView),
        )
        .expect("Failed to open window");
    });
}
