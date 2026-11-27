//! Native Desktop DBGrid Component
//! Komponen tabel data grid bergaya aplikasi desktop native:
//! - Kolom indikator baris di sisi paling kiri (nomor baris / penanda aktif `▶`)
//! - Garis batas sel vertikal dan horizontal yang tegas (1px gridlines)
//! - Header kolom padat (28px) dengan kontras tinggi
//! - Dukungan sel berwarna khusus untuk nilai debet, kredit, saldo negatif `(Rp ...)`, dan aging kritis

use crate::ui::theme::Theme;
use gpui::{Div, FontWeight, SharedString, div, prelude::*, px, rgb};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone)]
pub struct GridColumn {
    pub title: &'static str,
    pub width_px: f32,
    pub align: ColumnAlign,
}

pub fn render_grid_header(columns: &[GridColumn]) -> Div {
    let mut header_row = div()
        .h(px(28.0))
        .bg(rgb(Theme::BG_HEADER))
        .border_b_1()
        .border_color(rgb(Theme::BORDER_STRONG))
        .flex()
        .items_center()
        // Kolom indikator kiri khas DBGrid native (#)
        .child(
            div()
                .h_full()
                .w(px(34.0))
                .flex_shrink_0()
                .bg(rgb(0xE2E8F0))
                .border_r_1()
                .border_color(rgb(Theme::BORDER_DEFAULT))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(10.0))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(Theme::TEXT_MUTED))
                .child("#"),
        );

    for col in columns {
        let mut col_div = div()
            .h_full()
            .w(px(col.width_px))
            .flex_shrink_0()
            .border_r_1()
            .border_color(rgb(Theme::BORDER_DEFAULT))
            .px(px(7.0))
            .flex()
            .items_center()
            .text_size(px(10.5))
            .font_weight(FontWeight::BOLD)
            .text_color(rgb(Theme::TEXT_SECONDARY));

        match col.align {
            ColumnAlign::Left => col_div = col_div.justify_start(),
            ColumnAlign::Center => col_div = col_div.justify_center(),
            ColumnAlign::Right => col_div = col_div.justify_end(),
        }

        header_row = header_row.child(col_div.child(col.title));
    }

    header_row
}

/// Sel indikator nomor baris di sisi kiri DBGrid
pub fn grid_row_indicator(row_num: usize, is_selected: bool) -> Div {
    div()
        .h_full()
        .w(px(34.0))
        .flex_shrink_0()
        .bg(rgb(if is_selected {
            Theme::PRIMARY
        } else {
            0xF1F5F9
        }))
        .border_r_1()
        .border_color(rgb(Theme::BORDER_DEFAULT))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(10.0))
        .font_weight(FontWeight::BOLD)
        .text_color(rgb(if is_selected {
            0xFFFFFF
        } else {
            Theme::TEXT_LIGHT
        }))
        .child(if is_selected {
            "▶".to_string()
        } else {
            row_num.to_string()
        })
}

pub fn grid_cell(
    text: impl Into<SharedString>,
    width_px: f32,
    align: ColumnAlign,
    is_selected: bool,
) -> Div {
    let text_color = if is_selected {
        Theme::PRIMARY_HOVER
    } else {
        Theme::TEXT_PRIMARY
    };
    grid_cell_styled(
        text,
        width_px,
        align,
        text_color,
        if is_selected {
            FontWeight::SEMIBOLD
        } else {
            FontWeight::NORMAL
        },
        None,
    )
}

pub fn grid_cell_styled(
    text: impl Into<SharedString>,
    width_px: f32,
    align: ColumnAlign,
    text_color: u32,
    weight: FontWeight,
    bg_tint: Option<u32>,
) -> Div {
    let mut cell_div = div()
        .h_full()
        .w(px(width_px))
        .flex_shrink_0()
        .when_some(bg_tint, |d, bg| d.bg(rgb(bg)))
        .border_r_1()
        .border_color(rgb(Theme::GRID_LINE))
        .px(px(7.0))
        .flex()
        .items_center()
        .overflow_hidden()
        .whitespace_nowrap()
        .text_size(px(11.5))
        .font_weight(weight)
        .text_color(rgb(text_color));

    match align {
        ColumnAlign::Left => cell_div = cell_div.justify_start(),
        ColumnAlign::Center => cell_div = cell_div.justify_center(),
        ColumnAlign::Right => cell_div = cell_div.justify_end().text_right(),
    }

    cell_div.child(text.into())
}

pub fn grid_badge(
    text: &'static str,
    color_hex: u32,
    bg_hex: u32,
    border_hex: u32,
    width_px: f32,
) -> Div {
    div()
        .h_full()
        .w(px(width_px))
        .flex_shrink_0()
        .border_r_1()
        .border_color(rgb(Theme::GRID_LINE))
        .px(px(4.0))
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .px(px(6.0))
                .py(px(1.0))
                .rounded(px(3.0))
                .bg(rgb(bg_hex))
                .border_1()
                .border_color(rgb(border_hex))
                .text_size(px(10.0))
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(color_hex))
                .child(text),
        )
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
            div()
                .text_size(px(12.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(Theme::TEXT_MUTED))
                .child(message),
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
                .child(
                    div()
                        .w(px(12.0))
                        .h(px(12.0))
                        .rounded(px(6.0))
                        .border_2()
                        .border_color(rgb(Theme::PRIMARY)),
                )
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
        .h(px(30.0))
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
                    div()
                        .id(prev_id)
                        .h(px(22.0))
                        .px(px(8.0))
                        .bg(rgb(if has_prev { 0xFFFFFF } else { 0xF1F5F9 }))
                        .border_1()
                        .border_color(rgb(if has_prev {
                            Theme::BORDER_DEFAULT
                        } else {
                            Theme::BORDER_LIGHT
                        }))
                        .rounded(px(2.0))
                        .flex()
                        .items_center()
                        .when(has_prev, |d| d.cursor_pointer().on_click(on_prev))
                        .child(
                            div()
                                .text_size(px(10.5))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(if has_prev {
                                    Theme::TEXT_PRIMARY
                                } else {
                                    Theme::TEXT_LIGHT
                                }))
                                .child("◀ Sebelumnya"),
                        ),
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
                    div()
                        .id(next_id)
                        .h(px(22.0))
                        .px(px(8.0))
                        .bg(rgb(if has_next { 0xFFFFFF } else { 0xF1F5F9 }))
                        .border_1()
                        .border_color(rgb(if has_next {
                            Theme::BORDER_DEFAULT
                        } else {
                            Theme::BORDER_LIGHT
                        }))
                        .rounded(px(2.0))
                        .flex()
                        .items_center()
                        .when(has_next, |d| d.cursor_pointer().on_click(on_next))
                        .child(
                            div()
                                .text_size(px(10.5))
                                .font_weight(FontWeight::BOLD)
                                .text_color(rgb(if has_next {
                                    Theme::TEXT_PRIMARY
                                } else {
                                    Theme::TEXT_LIGHT
                                }))
                                .child("Berikutnya ▶"),
                        ),
                ),
        )
}
