#[macro_use]
mod declarative_ui;

use declarative_ui::{Color as DColor, Element as DElement, Style as DStyle};
use gpui::{
    div, prelude::*, px, rgb, size, uniform_list, App, Application, AnyElement, AsyncApp, Bounds,
    Context, FontWeight, Global, Timer, Window, WindowBounds, WindowOptions,
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
    fn render_element(&self, el: DElement, _cx: &mut Context<Self>) -> AnyElement {
        let mut gpui_el = div();
        let mut has_click = false;

        // Apply styles
        for style in &el.styles {
            gpui_el = match style {
                DStyle::Flex => gpui_el.flex(),
                DStyle::FlexCol => gpui_el.flex_col(),
                DStyle::FlexRow => gpui_el.flex_row(),
                DStyle::FlexGrow => gpui_el.flex_grow(),
                DStyle::JustifyCenter => gpui_el.justify_center(),
                DStyle::JustifyBetween => gpui_el.justify_between(),
                DStyle::ItemsCenter => gpui_el.items_center(),
                DStyle::Gap(p) => gpui_el.gap(px(*p)),
                DStyle::Padding(p) => gpui_el.p(px(*p)),
                DStyle::Width(w) => gpui_el.w(px(*w)),
                DStyle::Height(h) => gpui_el.h(px(*h)),
                DStyle::Size(s) => gpui_el.size(px(*s)),
                DStyle::SizeFull => gpui_el.size_full(),
                DStyle::Background(color) => gpui_el.bg(self.convert_color(color.clone())),
                DStyle::TextColor(color) => gpui_el.text_color(self.convert_color(color.clone())),
                DStyle::TextSize(s) => gpui_el.text_size(px(*s)),
                DStyle::FontWeightBold => gpui_el.font_weight(FontWeight::BOLD),
                DStyle::CursorPointer => {
                    has_click = true;
                    gpui_el.cursor_pointer()
                }
            };
        }

        // Special handling for the Interfaces button - needs to open a window
        let is_interfaces_button = el.content.as_deref() == Some("⚙ Interfaces");
        if is_interfaces_button {
            gpui_el = gpui_el
                .cursor_pointer()
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
        } else if let Some(on_click) = el.on_click.clone() {
            // Add click handler if element has on_click
            gpui_el = gpui_el.on_mouse_down(gpui::MouseButton::Left, move |_ev, _window, cx| {
                on_click(&mut ());
                cx.refresh_windows();
            });
            if !has_click {
                gpui_el = gpui_el.cursor_pointer();
            }
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

        // Clone data to avoid borrow issues
        let interfaces = state.interfaces.clone();
        let snmp_available = state.snmp_available;

        // Build interface cards dynamically
        let mut interface_cards: Vec<DElement> = Vec::new();
        for metric in &interfaces {
            interface_cards.push(self.build_interface_card(metric));
        }

        // SNMP status indicator
        let snmp_status = if snmp_available {
            "SNMP: ✓"
        } else {
            "SNMP: ✗"
        };

        // Build the UI tree using ui! macro
        let mut root = ui! {
            div["flex col bg-dark size-full"] {
                // Header
                div["flex row items-center justify-between p-4 bg-light-gray"] {
                    div["text-xl bold text-white"] { text["NetGauge"] }
                    div["flex row gap-4"] {
                        div["text-sm text-dim"] { text[snmp_status] }
                        div["text-sm text-blue cursor-pointer"] { text["⚙ Interfaces"] }
                    }
                }
                // Main Content
                div["flex col gap-4 p-4"] {
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
// Interface Selector View
// ============================================================================

struct InterfaceSelectorView;

impl InterfaceSelectorView {
    fn render_element(&self, el: DElement, _cx: &mut Context<Self>) -> AnyElement {
        // Handle list elements specially - they become uniform_list
        if el.tag == "list" {
            if let Some(list_config) = el.list_config {
                let renderer = list_config.item_renderer.clone();
                let styles = el.styles.clone();
                let id: &'static str = Box::leak(list_config.id.into_boxed_str());

                let mut list_el = uniform_list(
                    id,
                    list_config.item_count,
                    move |range: std::ops::Range<usize>, _window, _cx| {
                        range
                            .map(|ix| {
                                let item_el = renderer(ix);
                                Self::render_element_static(item_el)
                            })
                            .collect()
                    },
                );

                // Apply styles to the list
                for style in &styles {
                    list_el = match style {
                        DStyle::Flex => list_el.flex(),
                        DStyle::FlexCol => list_el.flex_col(),
                        DStyle::FlexRow => list_el.flex_row(),
                        DStyle::FlexGrow => list_el.flex_grow(),
                        DStyle::JustifyCenter => list_el.justify_center(),
                        DStyle::JustifyBetween => list_el.justify_between(),
                        DStyle::ItemsCenter => list_el.items_center(),
                        DStyle::Gap(p) => list_el.gap(px(*p)),
                        DStyle::Padding(p) => list_el.p(px(*p)),
                        DStyle::Width(w) => list_el.w(px(*w)),
                        DStyle::Height(h) => list_el.h(px(*h)),
                        DStyle::Size(s) => list_el.size(px(*s)),
                        DStyle::SizeFull => list_el.size_full(),
                        DStyle::Background(color) => list_el.bg(Self::convert_color_static(color)),
                        DStyle::TextColor(color) => {
                            list_el.text_color(Self::convert_color_static(color))
                        }
                        DStyle::TextSize(s) => list_el.text_size(px(*s)),
                        DStyle::FontWeightBold => list_el.font_weight(FontWeight::BOLD),
                        DStyle::CursorPointer => list_el.cursor_pointer(),
                    };
                }

                return list_el.into_any_element();
            }
        }

        // For non-list elements, render normally but handle list children
        let mut gpui_el = div();
        let mut has_cursor = false;

        for style in &el.styles {
            gpui_el = match style {
                DStyle::Flex => gpui_el.flex(),
                DStyle::FlexCol => gpui_el.flex_col(),
                DStyle::FlexRow => gpui_el.flex_row(),
                DStyle::FlexGrow => gpui_el.flex_grow(),
                DStyle::JustifyCenter => gpui_el.justify_center(),
                DStyle::JustifyBetween => gpui_el.justify_between(),
                DStyle::ItemsCenter => gpui_el.items_center(),
                DStyle::Gap(p) => gpui_el.gap(px(*p)),
                DStyle::Padding(p) => gpui_el.p(px(*p)),
                DStyle::Width(w) => gpui_el.w(px(*w)),
                DStyle::Height(h) => gpui_el.h(px(*h)),
                DStyle::Size(s) => gpui_el.size(px(*s)),
                DStyle::SizeFull => gpui_el.size_full(),
                DStyle::Background(color) => gpui_el.bg(Self::convert_color_static(color)),
                DStyle::TextColor(color) => gpui_el.text_color(Self::convert_color_static(color)),
                DStyle::TextSize(s) => gpui_el.text_size(px(*s)),
                DStyle::FontWeightBold => gpui_el.font_weight(FontWeight::BOLD),
                DStyle::CursorPointer => {
                    has_cursor = true;
                    gpui_el.cursor_pointer()
                }
            };
        }

        // Handle on_click callback
        if let Some(on_click) = el.on_click.clone() {
            gpui_el = gpui_el.on_mouse_down(gpui::MouseButton::Left, move |_ev, _window, cx| {
                on_click(&mut ());
                cx.refresh_windows();
            });
            if !has_cursor {
                gpui_el = gpui_el.cursor_pointer();
            }
        }

        // Recursively render children - use render_element to handle list children
        for child in el.children {
            gpui_el = gpui_el.child(self.render_element(child, _cx));
        }

        if let Some(content) = el.content {
            gpui_el = gpui_el.child(content);
        }

        gpui_el.into_any_element()
    }

    fn render_element_static(el: DElement) -> AnyElement {
        let mut gpui_el = div();
        let mut has_cursor = false;

        for style in &el.styles {
            gpui_el = match style {
                DStyle::Flex => gpui_el.flex(),
                DStyle::FlexCol => gpui_el.flex_col(),
                DStyle::FlexRow => gpui_el.flex_row(),
                DStyle::FlexGrow => gpui_el.flex_grow(),
                DStyle::JustifyCenter => gpui_el.justify_center(),
                DStyle::JustifyBetween => gpui_el.justify_between(),
                DStyle::ItemsCenter => gpui_el.items_center(),
                DStyle::Gap(p) => gpui_el.gap(px(*p)),
                DStyle::Padding(p) => gpui_el.p(px(*p)),
                DStyle::Width(w) => gpui_el.w(px(*w)),
                DStyle::Height(h) => gpui_el.h(px(*h)),
                DStyle::Size(s) => gpui_el.size(px(*s)),
                DStyle::SizeFull => gpui_el.size_full(),
                DStyle::Background(color) => gpui_el.bg(Self::convert_color_static(color)),
                DStyle::TextColor(color) => gpui_el.text_color(Self::convert_color_static(color)),
                DStyle::TextSize(s) => gpui_el.text_size(px(*s)),
                DStyle::FontWeightBold => gpui_el.font_weight(FontWeight::BOLD),
                DStyle::CursorPointer => {
                    has_cursor = true;
                    gpui_el.cursor_pointer()
                }
            };
        }

        // Handle on_click callback
        if let Some(on_click) = el.on_click.clone() {
            gpui_el = gpui_el.on_mouse_down(gpui::MouseButton::Left, move |_ev, _window, cx| {
                on_click(&mut ());
                cx.refresh_windows();
            });
            if !has_cursor {
                gpui_el = gpui_el.cursor_pointer();
            }
        }

        for child in el.children {
            gpui_el = gpui_el.child(Self::render_element_static(child));
        }

        if let Some(content) = el.content {
            gpui_el = gpui_el.child(content);
        }

        gpui_el.into_any_element()
    }

    fn convert_color_static(color: &DColor) -> gpui::Hsla {
        match color {
            DColor::Hex(h) => rgb(*h).into(),
            DColor::Name("red") => gpui::red(),
            DColor::Name("green") => gpui::green(),
            DColor::Name("blue") => rgb(0x4a90e2).into(),
            DColor::Rgb(r, g, b) => {
                gpui::rgb((*r as u32) << 16 | (*g as u32) << 8 | (*b as u32)).into()
            }
            _ => rgb(0x000000).into(),
        }
    }
}

impl Render for InterfaceSelectorView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = cx.global::<NetGaugeState>();
        let available = state.available_interfaces.clone();
        let selected_lock = state.selected_interfaces.clone();
        let item_count = available.len();

        // Create the item renderer closure
        let selected_for_render = selected_lock.clone();
        let available_for_render = available.clone();
        let item_renderer = move |ix: usize| {
            let available = available_for_render.clone();
            let selected_lock = selected_for_render.clone();
            let selected = selected_lock.read().unwrap().clone();

            let iface = available[ix].clone();
            let is_selected = selected.contains(&iface);
            let checkbox = if is_selected { "☑" } else { "☐" };
            let label = format!("{} {}", checkbox, iface);

            let iface_clone = iface.clone();
            let selected_clone = selected_lock.clone();

            jsx! {
                <div class={"flex row items-center gap-2 p-2 bg-gray text-white cursor-pointer"} onclick={
                    move |_: &mut dyn std::any::Any| {
                        let mut sel = selected_clone.write().unwrap();
                        if sel.contains(&iface_clone) {
                            sel.remove(&iface_clone);
                        } else {
                            sel.insert(iface_clone.clone());
                        }
                    }
                }> {
                    <text>{label}</text>
                } </div>
            }
        };

        // Build the full UI using jsx! macro with list element
        let root = jsx! {
            <div class={"flex col bg-dark size-full"}> {
                <div class={"flex row items-center justify-between p-4 bg-light-gray bold text-white"}> {
                    <text>{"Select Interfaces"}</text>
                } </div>
                <list id={"interface-list"} count={item_count} class={"flex-grow p-4"} render={item_renderer} />
            } </div>
        };

        self.render_element(root, cx)
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
