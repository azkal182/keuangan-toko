//! Native Windows System Menu Strip & Left Explorer Navigation Pane
//! Menggantikan navbar gaya web dengan:
//! 1. Baris menu sistem desktop tipis (26px) di bagian paling atas jendela
//! 2. Panel navigasi dok kiri (Left Explorer Pane) bergaya aplikasi desktop native

use crate::ui::theme::Theme;
use gpui::{Div, FontWeight, div, prelude::*, px, rgb};

/// Baris Menu Sistem Native (26px) di puncak jendela aplikasi
pub fn render_system_menu_bar() -> Div {
    div()
        .h(px(26.0))
        .bg(rgb(Theme::BG_SYSTEM_MENU))
        .border_b_1()
        .border_color(rgb(Theme::BORDER_DEFAULT))
        .px(px(8.0))
        .flex()
        .items_center()
        .justify_between()
}

/// Item teks pada System Menu Bar (contoh: `Berkas`, `Navigasi`, `Akun`)
pub fn system_menu_item(label: &'static str, shortcut: &'static str, is_active: bool) -> Div {
    div()
        .h(px(20.0))
        .px(px(7.0))
        .rounded(px(2.0))
        .bg(rgb(if is_active {
            Theme::BG_SIDEBAR_ACTIVE
        } else {
            Theme::BG_SYSTEM_MENU
        }))
        .flex()
        .items_center()
        .gap(px(5.0))
        .cursor_pointer()
        .hover(|d| d.bg(rgb(0xCBD5E1)))
        .child(
            div()
                .text_size(px(11.0))
                .font_weight(if is_active {
                    FontWeight::BOLD
                } else {
                    FontWeight::MEDIUM
                })
                .text_color(rgb(Theme::TEXT_PRIMARY))
                .child(label),
        )
        .when(!shortcut.is_empty(), |d| {
            d.child(
                div()
                    .text_size(px(9.5))
                    .text_color(rgb(Theme::TEXT_MUTED))
                    .child(shortcut),
            )
        })
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
    icon: &'static str,
    label: &'static str,
    shortcut: &'static str,
    is_active: bool,
) -> Div {
    div()
        .h(px(28.0))
        .mx(px(6.0))
        .px(px(8.0))
        .rounded(px(3.0))
        .bg(rgb(if is_active {
            Theme::BG_SIDEBAR_ACTIVE
        } else {
            Theme::BG_SIDEBAR
        }))
        .border_1()
        .border_color(rgb(if is_active {
            Theme::PRIMARY_BORDER
        } else {
            Theme::BG_SIDEBAR
        }))
        .flex()
        .items_center()
        .justify_between()
        .cursor_pointer()
        .when(!is_active, |d| d.hover(|d| d.bg(rgb(0xE2E8F0))))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(7.0))
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(rgb(if is_active {
                            Theme::PRIMARY
                        } else {
                            Theme::TEXT_MUTED
                        }))
                        .child(icon),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .font_weight(if is_active {
                            FontWeight::BOLD
                        } else {
                            FontWeight::MEDIUM
                        })
                        .text_color(rgb(if is_active {
                            Theme::PRIMARY_HOVER
                        } else {
                            Theme::TEXT_PRIMARY
                        }))
                        .child(label),
                ),
        )
        .when(!shortcut.is_empty(), |d| {
            d.child(
                div()
                    .px(px(4.0))
                    .py(px(1.0))
                    .rounded(px(2.0))
                    .bg(rgb(if is_active { 0xBFDBFE } else { 0xE2E8F0 }))
                    .text_size(px(9.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(if is_active {
                        Theme::PRIMARY_HOVER
                    } else {
                        Theme::TEXT_MUTED
                    }))
                    .child(shortcut),
            )
        })
}
