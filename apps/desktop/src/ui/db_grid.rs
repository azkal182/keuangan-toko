//! Modern Enterprise DBGrid Component
//! Menghadirkan tabel data grid kelas enterprise:
//! - Header halus dengan border bawah bersih dan pemisah kolom lembut
//! - Tinggi baris yang nyaman (32px), tipografi kontras dan mudah dibaca
//! - Seleksi baris modern: soft blue highlight dengan teks tetap tajam
//! - Badge status berbentuk pill modern dengan bingkai halus
//! - Angka nominal rupiah rata kanan dengan format tabular rapi

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
        .h(px(32.0))
        .bg(rgb(Theme::BG_HEADER))
        .border_b_1()
        .border_color(rgb(Theme::BORDER_DEFAULT))
        .flex()
        .items_center();

    for col in columns {
        let mut col_div = div()
            .h_full()
            .w(px(col.width_px))
            .flex_shrink_0()
            .border_r_1()
            .border_color(rgb(Theme::BORDER_LIGHT))
            .px(px(8.0))
            .flex()
            .items_center()
            .text_size(px(11.5))
            .font_weight(FontWeight::BOLD)
            .text_color(rgb(Theme::TEXT_MUTED));

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
        Theme::PRIMARY_HOVER
    } else {
        Theme::TEXT_PRIMARY
    };

    let mut cell_div = div()
        .h_full()
        .w(px(width_px))
        .flex_shrink_0()
        .border_r_1()
        .border_color(rgb(Theme::BORDER_LIGHT))
        .px(px(8.0))
        .flex()
        .items_center()
        .overflow_hidden()
        .whitespace_nowrap()
        .text_size(px(12.0))
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
        .border_color(rgb(Theme::BORDER_LIGHT))
        .px(px(6.0))
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .px(px(8.0))
                .py(px(2.0))
                .rounded(px(4.0))
                .bg(rgb(bg_hex))
                .border_1()
                .border_color(rgb(border_hex))
                .text_size(px(10.5))
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
                .text_size(px(20.0))
                .text_color(rgb(Theme::TEXT_LIGHT))
                .child("📋"),
        )
        .child(
            div()
                .text_size(px(12.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(Theme::TEXT_MUTED))
                .child(message),
        )
}
