//! Modern Enterprise Action Toolbar Component
//! Toolbar aksi cepat dengan tombol modern kelas enterprise.

use crate::ui::theme::Theme;
use gpui::{Div, FontWeight, div, prelude::*, px, rgb};

pub fn render_toolbar_container() -> Div {
    div()
        .h(px(42.0))
        .bg(rgb(Theme::BG_TOOLBAR))
        .border_b_1()
        .border_color(rgb(Theme::BORDER_DEFAULT))
        .px(px(12.0))
        .flex()
        .items_center()
        .justify_between()
}

pub fn primary_action_button(label: &'static str, shortcut: &'static str) -> Div {
    div()
        .h(px(28.0))
        .px(px(10.0))
        .bg(rgb(Theme::PRIMARY))
        .rounded(px(4.0))
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
        .child(
            div()
                .text_size(px(9.5))
                .text_color(rgb(0xBFDBFE))
                .child(shortcut),
        )
}

pub fn secondary_action_button(label: &'static str, shortcut: &'static str) -> Div {
    div()
        .h(px(28.0))
        .px(px(10.0))
        .bg(rgb(Theme::BG_CARD))
        .border_1()
        .border_color(rgb(Theme::BORDER_DEFAULT))
        .rounded(px(4.0))
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
        .h(px(18.0))
        .bg(rgb(Theme::BORDER_DEFAULT))
        .mx(px(4.0))
}
