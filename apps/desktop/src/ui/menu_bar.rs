//! Native Menu Bar Component
//! Menyediakan menu bar desktop klasik: Berkas, Pelanggan, Transaksi, Laporan, Pengaturan.

use crate::ui::theme::Theme;
use gpui::{Div, div, prelude::*, px, rgb};

pub fn render_menu_bar_container() -> Div {
    div()
        .h(px(26.0))
        .bg(rgb(Theme::BG_MENU_BAR))
        .border_b_1()
        .border_color(rgb(Theme::BORDER_LIGHT))
        .flex()
        .items_center()
        .px(px(8.0))
        .gap(px(4.0))
        .text_size(px(12.0))
        .text_color(rgb(Theme::TEXT_PRIMARY))
}

pub fn menu_item(title: &'static str, is_active: bool) -> Div {
    div()
        .px(px(8.0))
        .py(px(2.0))
        .rounded(px(3.0))
        .cursor_pointer()
        .when(is_active, |d| d.bg(rgb(Theme::BG_ROW_HOVER)))
        .hover(|d| d.bg(rgb(Theme::BG_ROW_HOVER)))
        .child(title)
}
