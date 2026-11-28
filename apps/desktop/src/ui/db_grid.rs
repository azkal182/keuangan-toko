//! Native Desktop DBGrid Component
//! Komponen tabel data grid bergaya aplikasi desktop native:
//! - Kolom indikator baris di sisi paling kiri (nomor baris / penanda aktif `▶`)
//! - Garis batas sel vertikal dan horizontal yang tegas (1px gridlines)
//! - Header kolom padat (28px) dengan kontras tinggi
//! - Dukungan sel berwarna khusus untuk nilai debet, kredit, saldo negatif `(Rp ...)`, dan aging kritis

use crate::ui::theme::Theme;
use gpui::{Div, FontWeight, div, prelude::*, px, rgb};
use gpui_kit::component::{empty::Empty, spinner::Spinner};
use gpui_kit::{base::Disableable, component::button::Button};

/// A changed filter or customer can leave the previous page outside the new data set.
pub fn bounded_page(page: usize, total: usize, page_size: usize) -> usize {
    page.min(total.saturating_sub(1) / page_size.max(1))
}

pub fn render_empty_state(message: &'static str) -> Div {
    div()
        .h(px(140.0))
        .w_full()
        .bg(rgb(Theme::BG_CARD))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(6.0))
        .child(
            Empty::new().child(
                div()
                    .text_size(px(12.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(Theme::TEXT_MUTED))
                    .child(message),
            ),
        )
}

/// Indikator Loading Data di DBGrid
pub fn render_loading_state(message: &str) -> Div {
    div()
        .h(px(140.0))
        .w_full()
        .bg(rgb(Theme::BG_CARD))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(8.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(Spinner::new().color(rgb(Theme::PRIMARY).into()))
                .child(
                    div()
                        .text_size(px(12.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(Theme::PRIMARY))
                        .child(if message.is_empty() {
                            "Memuat data...".to_string()
                        } else {
                            message.to_string()
                        }),
                ),
        )
}

/// Bar Pagination DBGrid Native Desktop
pub fn render_pagination_bar<F1, F2>(
    current_page: usize,
    total_items: usize,
    page_size: usize,
    prev_id: &'static str,
    next_id: &'static str,
    on_prev: F1,
    on_next: F2,
) -> Div
where
    F1: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    F2: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
{
    let total_pages = if total_items == 0 {
        1
    } else {
        total_items.div_ceil(page_size)
    };
    let current_page = bounded_page(current_page, total_items, page_size);
    let page_display = current_page + 1;
    let has_prev = current_page > 0;
    let has_next = page_display < total_pages;

    let start_idx = if total_items == 0 {
        0
    } else {
        current_page * page_size + 1
    };
    let end_idx = (start_idx + page_size - 1).min(total_items);

    div()
        .h(px(42.0))
        .flex_shrink_0()
        .bg(rgb(Theme::BG_HEADER))
        .border_t_1()
        .border_color(rgb(Theme::BORDER_STRONG))
        .px(px(10.0))
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(4.0))
                .text_size(px(11.0))
                .text_color(rgb(Theme::TEXT_SECONDARY))
                .child(div().font_weight(FontWeight::BOLD).child(format!(
                    "Menampilkan {}-{} dari {} baris",
                    start_idx, end_idx, total_items
                ))),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    Button::new(prev_id)
                        .outline()
                        .label("Sebelumnya")
                        .disabled(!has_prev)
                        .on_click(on_prev),
                )
                .child(
                    div()
                        .px(px(6.0))
                        .text_size(px(11.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(Theme::TEXT_PRIMARY))
                        .child(format!("Hal. {} / {}", page_display, total_pages)),
                )
                .child(
                    Button::new(next_id)
                        .outline()
                        .label("Berikutnya")
                        .disabled(!has_next)
                        .on_click(on_next),
                ),
        )
}

#[cfg(test)]
mod pagination_tests {
    use super::bounded_page;

    #[test]
    fn page_is_clamped_when_result_set_shrinks_or_becomes_empty() {
        assert_eq!(bounded_page(2, 101, 50), 2);
        assert_eq!(bounded_page(2, 51, 50), 1);
        assert_eq!(bounded_page(2, 50, 50), 0);
        assert_eq!(bounded_page(2, 0, 50), 0);
    }
}
