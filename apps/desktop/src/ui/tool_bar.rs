//! Native Toolbar Component
//! Toolbar aksi cepat dengan tombol gaya desktop klasik.

use crate::ui::theme::Theme;
use gpui::{Div, FontWeight, div, prelude::*, px, rgb};

pub fn render_toolbar_container() -> Div {
    div()
        .h(px(36.0))
        .bg(rgb(Theme::BG_HEADER))
        .border_b_1()
        .border_color(rgb(Theme::GRID_LINE))
        .flex()
        .items_center()
        .px(px(8.0))
        .gap(px(6.0))
}

pub fn tool_button(label: &'static str, shortcut: &'static str) -> Div {
    div()
        .h(px(26.0))
        .px(px(8.0))
        .bg(rgb(Theme::BG_SURFACE))
        .border_1()
        .border_color(rgb(Theme::BORDER_DARK))
        .rounded(px(2.0))
        .flex()
        .items_center()
        .gap(px(4.0))
        .cursor_pointer()
        .hover(|d| d.bg(rgb(Theme::BG_ROW_HOVER)))
        .child(
            div()
                .text_size(px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(Theme::TEXT_PRIMARY))
                .child(label),
        )
        .child(
            div()
                .text_size(px(9.5))
                .text_color(rgb(Theme::TEXT_MUTED))
                .child(shortcut),
        )
}

pub fn tool_separator() -> Div {
    div()
        .w(px(1.0))
        .h(px(20.0))
        .bg(rgb(Theme::GRID_LINE))
        .mx(px(2.0))
}
