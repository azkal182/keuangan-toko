//! Native Windowed Modal Dialog Container
//! Menampilkan dialog modal bergaya jendela native Windows dengan titlebar tegas,
//! border kontras, area isi terstruktur, dan bar tombol aksi di bagian bawah.

use crate::ui::theme::Theme;
use gpui::{AnyElement, Div, FontWeight, div, prelude::*, px, rgb};
use gpui_kit::component::dialog::{DialogContent, DialogFooter, DialogHeader, DialogTitle};
use gpui_kit::component::scroll::ScrollableElement;

pub struct ModalDialog {
    pub title: &'static str,
    pub subtitle: Option<String>,
    pub width_px: f32,
    pub height_px: Option<f32>,
}

impl ModalDialog {
    pub fn new(title: &'static str, width_px: f32) -> Self {
        Self {
            title,
            subtitle: None,
            width_px,
            height_px: None,
        }
    }

    pub fn subtitle(mut self, sub: impl Into<String>) -> Self {
        self.subtitle = Some(sub.into());
        self
    }

    pub fn height(mut self, h: f32) -> Self {
        self.height_px = Some(h);
        self
    }

    pub fn frame(self, body: AnyElement, footer: Div) -> Div {
        div().w_full().flex().flex_col().child(
            div()
                .w_full()
                .when_some(self.height_px, |d, h| d.h(px(h)))
                .bg(rgb(0xFFFFFF))
                .flex()
                .flex_col()
                .overflow_hidden()
                // Native Dialog Titlebar
                .child(
                    DialogHeader::new()
                        .h(px(48.0))
                        .flex_shrink_0()
                        .bg(rgb(Theme::BG_CARD))
                        .border_b_1()
                        .border_color(rgb(Theme::BORDER_DEFAULT))
                        .px(px(16.0))
                        .flex_row()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .child(
                                    DialogTitle::new()
                                        .text_size(px(14.0))
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(rgb(Theme::TEXT_PRIMARY))
                                        .child(self.title),
                                )
                                .when_some(self.subtitle, |d, sub| {
                                    d.child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(rgb(Theme::TEXT_MUTED))
                                            .child(format!("— {}", sub)),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(rgb(Theme::TEXT_MUTED))
                                .child("[Esc] Tutup"),
                        ),
                )
                // Isi Form Dialog
                .child(
                    DialogContent::new()
                        .flex_1()
                        .min_h(px(0.0))
                        .p(px(14.0))
                        .child(
                            div()
                                .id("modal-body-scroll")
                                .flex_1()
                                .min_h(px(0.0))
                                .overflow_scrollbar()
                                .child(body),
                        ),
                )
                // Footer Dialog
                .child(
                    DialogFooter::new()
                        .w_full()
                        .flex_shrink_0()
                        .child(footer.w_full()),
                ),
        )
    }
}
