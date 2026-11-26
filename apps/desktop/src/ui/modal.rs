//! Native Modal Dialog Container
//! Menampilkan dialog modal desktop standar dengan titlebar biru/slate,
//! tombol [X], tombol [ Batal (Esc) ] dan [ Simpan/Aksi (Enter) ].

use crate::ui::theme::Theme;
use gpui::{AnyElement, Div, FontWeight, div, prelude::*, px, rgb};

pub struct ModalDialog {
    pub title: &'static str,
    pub width_px: f32,
    pub height_px: Option<f32>,
}

impl ModalDialog {
    pub fn new(title: &'static str, width_px: f32) -> Self {
        Self {
            title,
            width_px,
            height_px: None,
        }
    }

    pub fn height(mut self, h: f32) -> Self {
        self.height_px = Some(h);
        self
    }

    pub fn frame(self, body: AnyElement, footer: Div) -> Div {
        div()
            .absolute()
            .inset_0()
            .bg(Theme::hsla_alpha(0x000000, 0.45))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w(px(self.width_px))
                    .when_some(self.height_px, |d, h| d.h(px(h)))
                    .bg(rgb(Theme::BG_WINDOW))
                    .border_2()
                    .border_color(rgb(Theme::BORDER_DARK))
                    .rounded(px(4.0))
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(
                        div()
                            .h(px(32.0))
                            .bg(rgb(Theme::TITLEBAR_BG))
                            .px(px(10.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(Theme::TITLEBAR_TEXT))
                                    .child(self.title),
                            ),
                    )
                    .child(div().flex_1().p(px(14.0)).overflow_hidden().child(body))
                    .child(footer),
            )
    }
}
