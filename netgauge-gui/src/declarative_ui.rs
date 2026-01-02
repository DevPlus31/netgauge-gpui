//! Declarative UI layer for GPUI
//!
//! This module provides Tailwind-inspired style parsing and macros
//! that work directly with GPUI's Div type - no intermediate Element struct.

use std::sync::Arc;
use gpui::{div, prelude::*, px, rgb, uniform_list, AnyElement, Div, FontWeight, Styled};

/// Apply a single style string to a Div
pub fn apply_style(el: Div, style: &str) -> Div {
    match style {
        "flex" => el.flex(),
        "flex-col" | "col" => el.flex_col(),
        "flex-row" | "row" => el.flex_row(),
        "flex-grow" => el.flex_grow(),
        "justify-center" => el.justify_center(),
        "justify-between" => el.justify_between(),
        "items-center" => el.items_center(),
        "font-bold" | "bold" => el.font_weight(FontWeight::BOLD),
        "cursor-pointer" => el.cursor_pointer(),
        "size-full" => el.size_full(),
        "bg-gray" => el.bg(rgb(0x333333)),
        "bg-dark" => el.bg(rgb(0x1e1e1e)),
        "bg-light-gray" => el.bg(rgb(0x444444)),
        "bg-blue" => el.bg(rgb(0x4a90e2)),
        "bg-footer" => el.bg(rgb(0x252525)),
        "bg-wan" => el.bg(rgb(0x2d5a3d)),
        "text-white" => el.text_color(rgb(0xffffff)),
        "text-gray" => el.text_color(rgb(0xcccccc)),
        "text-dim" => el.text_color(rgb(0x666666)),
        "text-blue" => el.text_color(rgb(0x4a90e2)),
        "text-xl" => el.text_size(px(20.0)),
        "text-lg" => el.text_size(px(18.0)),
        "text-sm" => el.text_size(px(14.0)),
        "text-xs" => el.text_size(px(12.0)),
        "text-2xl" => el.text_size(px(24.0)),
        s if s.starts_with("gap-") => {
            if let Ok(v) = s["gap-".len()..].parse::<f32>() { el.gap(px(v)) } else { el }
        }
        s if s.starts_with("p-") => {
            if let Ok(v) = s["p-".len()..].parse::<f32>() { el.p(px(v)) } else { el }
        }
        s if s.starts_with("w-") => {
            if let Ok(v) = s["w-".len()..].parse::<f32>() { el.w(px(v)) } else { el }
        }
        s if s.starts_with("h-") => {
            if let Ok(v) = s["h-".len()..].parse::<f32>() { el.h(px(v)) } else { el }
        }
        s if s.starts_with("size-") => {
            if let Ok(v) = s["size-".len()..].parse::<f32>() { el.size(px(v)) } else { el }
        }
        s if s.starts_with("text-") && s.len() > 5 && s.chars().nth(5).map(|c| c.is_ascii_digit()).unwrap_or(false) => {
            if let Ok(v) = s["text-".len()..].parse::<f32>() { el.text_size(px(v)) } else { el }
        }
        _ => el,
    }
}

/// Apply multiple space-separated styles to a Div
pub fn apply_styles(mut el: Div, styles: &str) -> Div {
    for style in styles.split_whitespace() {
        el = apply_style(el, style);
    }
    el
}

/// Create a styled div from a style string
pub fn styled_div(styles: &str) -> Div {
    apply_styles(div(), styles)
}

/// Create a uniform_list with styling
pub fn styled_list<F>(
    id: &'static str,
    count: usize,
    styles: &str,
    renderer: F,
) -> AnyElement
where
    F: Fn(usize) -> AnyElement + Send + Sync + 'static,
{
    let renderer = Arc::new(renderer);
    let mut list = uniform_list(id, count, move |range, _window, _cx| {
        range.map(|ix| renderer(ix)).collect()
    });

    // Apply styles to the list
    for style in styles.split_whitespace() {
        list = match style {
            "flex-grow" => list.flex_grow(),
            "size-full" => list.size_full(),
            s if s.starts_with("p-") => {
                if let Ok(v) = s["p-".len()..].parse::<f32>() { list.p(px(v)) } else { list }
            }
            s if s.starts_with("gap-") => {
                if let Ok(v) = s["gap-".len()..].parse::<f32>() { list.gap(px(v)) } else { list }
            }
            _ => list,
        };
    }

    list.into_any_element()
}

// ============================================================================
// Macros - work directly with GPUI Div
// ============================================================================

/// Main declarative UI macro using bracket syntax
///
/// Example:
/// ```
/// ui! {
///     div["flex col bg-dark size-full"] {
///         div["text-xl bold text-white"] { text["Hello World"] }
///         div["flex row gap-2"] {
///             text["Label:"]
///             text[some_variable]
///         }
///     }
/// }
/// ```
#[macro_export]
macro_rules! ui {
    // div["styles"] { children... }
    ( div [ $styles:expr ] { $($children:tt)* } ) => {
        {
            let mut el = $crate::declarative_ui::styled_div($styles);
            $crate::ui_children!(el, $($children)*);
            el
        }
    };

    // div["styles"] - no children
    ( div [ $styles:expr ] ) => {
        $crate::declarative_ui::styled_div($styles)
    };
}

/// Helper macro to collect children
#[macro_export]
macro_rules! ui_children {
    // Base case - no more children
    ($el:ident, ) => {};

    // text[content] - text element
    ($el:ident, text [ $content:expr ] $($rest:tt)* ) => {
        $el = $el.child($content);
        $crate::ui_children!($el, $($rest)*);
    };

    // div with children
    ($el:ident, div [ $styles:expr ] { $($body:tt)* } $($rest:tt)* ) => {
        $el = $el.child($crate::ui! { div [ $styles ] { $($body)* } });
        $crate::ui_children!($el, $($rest)*);
    };

    // div without children
    ($el:ident, div [ $styles:expr ] $($rest:tt)* ) => {
        $el = $el.child($crate::ui! { div [ $styles ] });
        $crate::ui_children!($el, $($rest)*);
    };

    // for loop - iterate and add children
    ($el:ident, for $item:ident in $iter:expr => { $($body:tt)* } $($rest:tt)* ) => {
        for $item in $iter {
            $el = $el.child($crate::ui! { $($body)* });
        }
        $crate::ui_children!($el, $($rest)*);
    };

    // Arbitrary expression in braces { expr }
    ($el:ident, { $child:expr } $($rest:tt)* ) => {
        $el = $el.child($child);
        $crate::ui_children!($el, $($rest)*);
    };
}

/// JSX-like macro syntax
///
/// Example:
/// ```
/// jsx! {
///     <div class={"flex col bg-dark"}> {
///         <div class={"text-white"}> { "Hello" } </div>
///     } </div>
/// }
/// ```
#[macro_export]
macro_rules! jsx {
    // div with class and children
    ( <div class={ $styles:expr }> { $($body:tt)* } </div> ) => {
        {
            let mut el = $crate::declarative_ui::styled_div($styles);
            $crate::jsx_children!(el, $($body)*);
            el
        }
    };

    // div with class, onclick and children
    ( <div class={ $styles:expr } onclick={ $handler:expr }> { $($body:tt)* } </div> ) => {
        {
            let mut el = $crate::declarative_ui::styled_div($styles);
            let handler = $handler;
            el = el.cursor_pointer().on_mouse_down(gpui::MouseButton::Left, move |_ev, _window, cx| {
                handler();
                cx.refresh_windows();
            });
            $crate::jsx_children!(el, $($body)*);
            el
        }
    };

    // Self-closing div
    ( <div class={ $styles:expr } /> ) => {
        $crate::declarative_ui::styled_div($styles)
    };

    // Text element
    ( <text> { $content:expr } </text> ) => {
        $content
    };

    // List element
    ( <list id={ $id:expr } count={ $count:expr } class={ $styles:expr } render={ $renderer:expr } /> ) => {
        $crate::declarative_ui::styled_list($id, $count, $styles, $renderer)
    };
}

/// Helper macro to collect JSX children
#[macro_export]
macro_rules! jsx_children {
    // Base case
    ($el:ident, ) => {};

    // text element
    ($el:ident, <text> { $content:expr } </text> $($rest:tt)* ) => {
        $el = $el.child($content);
        $crate::jsx_children!($el, $($rest)*);
    };

    // Self-closing list
    ($el:ident, <list id={ $id:expr } count={ $count:expr } class={ $styles:expr } render={ $renderer:expr } /> $($rest:tt)* ) => {
        $el = $el.child($crate::declarative_ui::styled_list($id, $count, $styles, $renderer));
        $crate::jsx_children!($el, $($rest)*);
    };

    // Self-closing div
    ($el:ident, <div class={ $styles:expr } /> $($rest:tt)* ) => {
        $el = $el.child($crate::declarative_ui::styled_div($styles));
        $crate::jsx_children!($el, $($rest)*);
    };

    // div with children
    ($el:ident, <div class={ $styles:expr }> { $($body:tt)* } </div> $($rest:tt)* ) => {
        $el = $el.child($crate::jsx! { <div class={ $styles }> { $($body)* } </div> });
        $crate::jsx_children!($el, $($rest)*);
    };

    // div with onclick and children
    ($el:ident, <div class={ $styles:expr } onclick={ $handler:expr }> { $($body:tt)* } </div> $($rest:tt)* ) => {
        $el = $el.child($crate::jsx! { <div class={ $styles } onclick={ $handler }> { $($body)* } </div> });
        $crate::jsx_children!($el, $($rest)*);
    };
}