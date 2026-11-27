//! Contextual Page Command Bar (Native Desktop Toolbar)
//! Menyediakan command bar (36px) yang ditempatkan di bagian atas area kerja (workspace)
//! dan HANYA menampilkan tombol aksi yang relevan dengan halaman dan status data yang sedang aktif.

use crate::ui::theme::Theme;
use gpui::{Div, FontWeight, div, prelude::*, px, rgb};

pub fn render_toolbar_container() -> Div {
    div()
        .h(px(38.0))
        .bg(rgb(Theme::BG_TOOLBAR))
        .border_b_1()
        .border_color(rgb(Theme::BORDER_DEFAULT))
        .px(px(10.0))
        .flex()
        .items_center()
        .justify_between()
}

pub fn primary_action_button(label: &'static str, shortcut: &'static str) -> Div {
    div()
        .h(px(26.0))
        .px(px(10.0))
        .bg(rgb(Theme::PRIMARY))
        .border_1()
        .border_color(rgb(Theme::PRIMARY_HOVER))
        .rounded(px(3.0))
        .flex()
        .items_center()
        .gap(px(6.0))
        .cursor_pointer()
        .hover(|d| d.bg(rgb(Theme::PRIMARY_HOVER)))
        .child(
            div()
                .text_size(px(11.5))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(0xFFFFFF))
                .child(label),
        )
        .when(!shortcut.is_empty(), |d| {
            d.child(
                div()
                    .px(px(4.0))
                    .rounded(px(2.0))
                    .bg(rgb(Theme::PRIMARY_HOVER))
                    .text_size(px(9.5))
                    .text_color(rgb(0xDBEAFE))
                    .child(shortcut),
            )
        })
}

pub fn secondary_action_button(label: &'static str, shortcut: &'static str) -> Div {
    div()
        .h(px(26.0))
        .px(px(9.0))
        .bg(rgb(Theme::BG_CARD))
        .border_1()
        .border_color(rgb(Theme::BORDER_STRONG))
        .rounded(px(3.0))
        .flex()
        .items_center()
        .gap(px(6.0))
        .cursor_pointer()
        .hover(|d| d.bg(rgb(Theme::BG_ROW_HOVER)))
        .child(
            div()
                .text_size(px(11.5))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(Theme::TEXT_PRIMARY))
                .child(label),
        )
        .when(!shortcut.is_empty(), |d| {
            d.child(
                div()
                    .px(px(4.0))
                    .rounded(px(2.0))
                    .bg(rgb(Theme::BG_SIDEBAR))
                    .text_size(px(9.5))
                    .text_color(rgb(Theme::TEXT_MUTED))
                    .child(shortcut),
            )
        })
}

pub fn accent_action_button(
    label: &'static str,
    shortcut: &'static str,
    text_hex: u32,
    bg_hex: u32,
    border_hex: u32,
) -> Div {
    div()
        .h(px(26.0))
        .px(px(9.0))
        .bg(rgb(bg_hex))
        .border_1()
        .border_color(rgb(border_hex))
        .rounded(px(3.0))
        .flex()
        .items_center()
        .gap(px(6.0))
        .cursor_pointer()
        .hover(|d| d.bg(rgb(border_hex)))
        .child(
            div()
                .text_size(px(11.5))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(text_hex))
                .child(label),
        )
        .when(!shortcut.is_empty(), |d| {
            d.child(
                div()
                    .text_size(px(9.5))
                    .text_color(rgb(text_hex))
                    .child(shortcut),
            )
        })
}

pub fn tool_separator() -> Div {
    div()
        .w(px(1.0))
        .h(px(18.0))
        .bg(rgb(Theme::BORDER_DEFAULT))
        .mx(px(4.0))
}
