use crate::store::AgingReportCustomerItem;
use crate::ui::theme::format_rupiah;
use gpui::{App, Context, IntoElement, ParentElement, Window, div, px};
use gpui_kit::component::{
    label::Label,
    table::{Column, TableDelegate, TableState},
};

#[derive(Default)]
pub(crate) struct AgingRows {
    pub rows: Vec<AgingReportCustomerItem>,
    signature: String,
}

impl AgingRows {
    pub fn replace(&mut self, rows: Vec<AgingReportCustomerItem>) -> bool {
        let signature = format!("{rows:?}");
        if self.signature == signature {
            return false;
        }
        self.rows = rows;
        self.signature = signature;
        true
    }
}

impl TableDelegate for AgingRows {
    fn columns_count(&self, _: &App) -> usize {
        10
    }
    fn rows_count(&self, _: &App) -> usize {
        self.rows.len()
    }
    fn column(&self, ix: usize, _: &App) -> Column {
        let (key, title, width) = [
            ("code", "Kode", 90.),
            ("name", "Nama pelanggan", 220.),
            ("total", "Total terbuka", 150.),
            ("d0", "0–15 hari", 130.),
            ("d1", "16–30 hari", 130.),
            ("d2", "31–45 hari", 130.),
            ("d3", "46–60 hari", 130.),
            ("d4", ">60 hari", 130.),
            ("unknown", "Tanggal tidak diketahui", 180.),
            ("future", "Tanggal masa depan", 170.),
        ][ix];
        let column = Column::new(key, title).width(px(width)).movable(false);
        if ix >= 2 { column.text_right() } else { column }
    }
    fn render_td(
        &mut self,
        row: usize,
        col: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let c = &self.rows[row];
        let value = match col {
            0 => c.customer_code.clone().unwrap_or_else(|| "–".into()),
            1 => c.customer_name.clone(),
            2 => format_rupiah(c.total_outstanding_idr),
            3 => format_rupiah(c.days_0_15),
            4 => format_rupiah(c.days_16_30),
            5 => format_rupiah(c.days_31_45),
            6 => format_rupiah(c.days_46_60),
            7 => format_rupiah(c.days_over_60),
            8 => format_rupiah(c.unknown_date),
            _ => format_rupiah(c.future_date),
        };
        div().child(Label::new(value))
    }
}
