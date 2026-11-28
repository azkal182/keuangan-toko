//! Native Windows System Menu Strip & Left Explorer Navigation Pane
//! Menggantikan navbar gaya web dengan:
//! 1. Baris menu sistem desktop tipis (26px) di bagian paling atas jendela
//! 2. Panel navigasi dok kiri (Left Explorer Pane) bergaya aplikasi desktop native

use crate::ui::theme::Theme;
use gpui::{Div, FontWeight, div, prelude::*, px, rgb};

/// Baris Menu Sistem Native (26px) di puncak jendela aplikasi
pub fn render_system_menu_bar() -> Div {
    div()
        .h(px(34.0))
        .bg(rgb(Theme::BG_SYSTEM_MENU))
        .border_b_1()
        .border_color(rgb(Theme::BORDER_DEFAULT))
        .px(px(8.0))
        .flex()
        .items_center()
        .justify_between()
}

/// Kontainer Panel Navigasi Dok Kiri (Native Explorer Sidebar - lebar 210px)
pub fn render_left_sidebar_container() -> Div {
    div()
        .w(px(210.0))
        .h_full()
        .flex_shrink_0()
        .bg(rgb(Theme::BG_SIDEBAR))
        .border_r_1()
        .border_color(rgb(Theme::BORDER_DEFAULT))
        .flex()
        .flex_col()
        .justify_between()
}

/// Judul grup modul pada panel navigasi kiri
pub fn sidebar_section_header(title: &'static str) -> Div {
    div()
        .px(px(10.0))
        .pt(px(10.0))
        .pb(px(4.0))
        .text_size(px(10.0))
        .font_weight(FontWeight::BOLD)
        .text_color(rgb(Theme::TEXT_LIGHT))
        .child(title)
}

/// Item modul pada panel navigasi kiri
pub fn sidebar_nav_item(
    id: &'static str,
    icon: &'static str,
    label: &'static str,
    shortcut: &'static str,
    is_active: bool,
) -> gpui_kit::component::button::Button {
    use gpui_kit::component::button::{Button, ButtonVariants};
    let text = if shortcut.is_empty() {
        format!("{icon}  {label}")
    } else {
        format!("{icon}  {label}  {shortcut}")
    };
    let button = Button::new(id).label(text);
    if is_active {
        button.primary()
    } else {
        button.ghost()
    }
}
