//! Modern Enterprise Top Navigation Bar
//! Menggantikan menu bar klasik dengan Top App Bar modern kelas enterprise.

use crate::ui::theme::Theme;
use gpui::{Div, FontWeight, div, prelude::*, px, rgb};

pub fn render_top_app_bar_container() -> Div {
    div()
        .h(px(46.0))
        .bg(rgb(Theme::BG_NAVBAR))
        .px(px(14.0))
        .flex()
        .items_center()
        .justify_between()
        .shadow_md()
}

pub fn brand_logo_pill() -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(
            div()
                .w(px(24.0))
                .h(px(24.0))
                .rounded(px(5.0))
                .bg(rgb(Theme::PRIMARY))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(13.0))
                .text_color(rgb(0xFFFFFF))
                .font_weight(FontWeight::BOLD)
                .child("K"),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .text_size(px(13.5))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(Theme::TEXT_ON_DARK))
                        .child("KEUANGAN TOKO"),
                )
                .child(
                    div()
                        .px(px(5.0))
                        .py(px(1.0))
                        .rounded(px(3.0))
                        .bg(rgb(0x1E293B))
                        .border_1()
                        .border_color(rgb(0x334155))
                        .text_size(px(9.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(0x94A3B8))
                        .child("ENTERPRISE"),
                ),
        )
}

pub fn nav_tab_button(label: &'static str, shortcut: &'static str, is_active: bool) -> Div {
    let bg_color = if is_active {
        0x1E293B
    } else {
        Theme::BG_NAVBAR
    };

    let text_color = if is_active { 0xFFFFFF } else { 0x94A3B8 };

    div()
        .h(px(28.0))
        .px(px(10.0))
        .rounded(px(4.0))
        .bg(rgb(bg_color))
        .flex()
        .items_center()
        .gap(px(6.0))
        .cursor_pointer()
        .hover(|d| d.bg(rgb(0x1E293B)))
        .child(
            div()
                .text_size(px(12.0))
                .font_weight(if is_active {
                    FontWeight::BOLD
                } else {
                    FontWeight::MEDIUM
                })
                .text_color(rgb(text_color))
                .child(label),
        )
        .child(
            div()
                .text_size(px(9.5))
                .text_color(rgb(0x64748B))
                .child(shortcut),
        )
}
