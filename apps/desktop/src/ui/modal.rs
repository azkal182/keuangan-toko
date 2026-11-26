//! Modern Enterprise Modal Dialog Container
//! Menampilkan dialog modal modern dengan border halus, header bersih,
//! tombol [X], tombol [ Batal (Esc) ] dan tombol aksi utama bernuansa enterprise.

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
            .bg(Theme::hsla_alpha(Theme::MODAL_BACKDROP, 0.55))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w(px(self.width_px))
                    .when_some(self.height_px, |d, h| d.h(px(h)))
                    .bg(rgb(0xFFFFFF))
                    .border_1()
                    .border_color(rgb(Theme::MODAL_BORDER))
                    .rounded(px(8.0))
                    .shadow_2xl()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    // Header Modal Modern
                    .child(
                        div()
                            .h(px(42.0))
                            .bg(rgb(Theme::MODAL_HEADER_BG))
                            .border_b_1()
                            .border_color(rgb(Theme::BORDER_DEFAULT))
                            .px(px(14.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(13.5))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(Theme::TEXT_PRIMARY))
                                    .child(self.title),
                            ),
                    )
                    // Isi Form Modal
                    .child(div().flex_1().p(px(16.0)).overflow_hidden().child(body))
                    // Footer Modal
                    .child(footer),
            )
    }
}
