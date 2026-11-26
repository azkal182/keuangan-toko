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
