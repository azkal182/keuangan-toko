//! Contextual Page Command Bar (Native Desktop Toolbar)
//! Menyediakan command bar (36px) yang ditempatkan di bagian atas area kerja (workspace)
//! dan HANYA menampilkan tombol aksi yang relevan dengan halaman dan status data yang sedang aktif.

use crate::ui::theme::Theme;
use gpui::{Div, div, prelude::*, px, rgb};

pub fn render_toolbar_container() -> Div {
    div()
        .h(px(48.0))
        .bg(rgb(Theme::BG_TOOLBAR))
        .border_b_1()
        .border_color(rgb(Theme::BORDER_DEFAULT))
        .px(px(10.0))
        .flex()
        .items_center()
        .justify_between()
}

pub fn tool_separator() -> Div {
    div()
        .w(px(1.0))
        .h(px(18.0))
        .bg(rgb(Theme::BORDER_DEFAULT))
        .mx(px(4.0))
}
