//! Native DBGrid Component
//! Menghadirkan tabel data grid bergaya desktop sejati:
//! - Grid lines horizontal & vertikal tajam (classic 1px border)
//! - Header kolom bergaya 3D/inset dengan pemisah kolom
//! - Baris kompak (26px), perataan kanan untuk angka & rupiah
//! - Highlight baris terpilih bergaya Windows Blue dengan teks putih
//! - Zebra row striping halus untuk kenyamanan mata

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
        .border_color(rgb(Theme::GRID_LINE))
        .flex()
        .items_center();

    for col in columns {
        let mut col_div = div()
            .h_full()
            .w(px(col.width_px))
            .flex_shrink_0()
            .border_r_1()
            .border_color(rgb(Theme::GRID_LINE))
            .px(px(6.0))
            .flex()
            .items_center()
            .text_size(px(11.5))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(Theme::TEXT_PRIMARY));

        match col.align {
            ColumnAlign::Left => col_div = col_div.justify_start(),
            ColumnAlign::Center => col_div = col_div.justify_center(),
            ColumnAlign::Right => col_div = col_div.justify_end(),
        }

        header_row = header_row.child(col_div.child(col.title));
    }

    header_row
}

pub fn grid_cell(
    text: impl Into<SharedString>,
    width_px: f32,
    align: ColumnAlign,
    is_selected: bool,
) -> Div {
    let text_color = if is_selected {
        Theme::TEXT_SELECTED
    } else {
        Theme::TEXT_PRIMARY
    };

    let mut cell_div = div()
        .h_full()
        .w(px(width_px))
        .flex_shrink_0()
        .border_r_1()
        .border_color(rgb(Theme::GRID_LINE))
        .px(px(6.0))
        .flex()
        .items_center()
        .overflow_hidden()
        .whitespace_nowrap()
        .text_size(px(11.5))
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
    width_px: f32,
    is_selected: bool,
) -> Div {
    let (text_c, bg_c) = if is_selected {
        (Theme::TEXT_SELECTED, Theme::BG_ROW_SELECTED)
    } else {
        (color_hex, bg_hex)
    };

    div()
        .h_full()
        .w(px(width_px))
        .flex_shrink_0()
        .border_r_1()
        .border_color(rgb(Theme::GRID_LINE))
        .px(px(6.0))
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .px(px(6.0))
                .py(px(1.0))
                .rounded(px(3.0))
                .bg(rgb(bg_c))
                .text_size(px(10.5))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(text_c))
                .child(text),
        )
}

pub fn render_empty_state(message: &'static str) -> Div {
    div()
        .h(px(120.0))
        .w_full()
        .bg(rgb(Theme::BG_SURFACE))
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .text_size(px(12.0))
        .text_color(rgb(Theme::TEXT_MUTED))
        .child(message)
}
