use crate::store::CustomerSummary;
use crate::ui::theme::format_rupiah;
use gpui::{App, Context, IntoElement, ParentElement, Window, div, px};
use gpui_kit::component::{
    label::Label,
    table::{Column, TableDelegate, TableState},
};

#[derive(Default)]
pub(crate) struct CustomerRows {
    pub rows: Vec<CustomerSummary>,
    signature: String,
}

impl CustomerRows {
    pub fn replace(&mut self, rows: Vec<CustomerSummary>) -> bool {
        // Used only in memory to detect changes; never logged or persisted.
        let signature = format!("{rows:?}");
        if self.signature == signature {
            return false;
        }
        self.rows = rows;
        self.signature = signature;
        true
    }
}

impl TableDelegate for CustomerRows {
    fn columns_count(&self, _: &App) -> usize {
        11
    }
    fn rows_count(&self, _: &App) -> usize {
        self.rows.len()
    }
    fn column(&self, ix: usize, _: &App) -> Column {
        let (key, title, width) = [
            ("code", "Kode", 90.),
            ("name", "Nama pelanggan", 220.),
            ("status", "Status", 100.),
            ("outstanding", "Total piutang", 150.),
            ("credit", "Saldo kredit", 150.),
            ("aging0", "0-15 hari", 130.),
            ("aging1", "16-30 hari", 130.),
            ("aging2", "31-45 hari", 130.),
            ("aging3", "46-60 hari", 130.),
            ("aging4", ">60 hari", 130.),
            ("exception", "Perlu ditinjau", 150.),
        ][ix];
        let col = Column::new(key, title).width(px(width)).movable(false);
        if ix >= 3 { col.text_right() } else { col }
    }
    fn render_td(
        &mut self,
        row: usize,
        col: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let customer = &self.rows[row];
        let value = match col {
            0 => customer.code.clone().unwrap_or_else(|| "-".into()),
            1 => customer.name.clone(),
            2 => if customer.is_active {
                "Aktif"
            } else {
                "Nonaktif"
            }
            .into(),
            3 => format_rupiah(customer.outstanding_idr),
            4 => format_rupiah(customer.customer_credit_idr),
            5 => format_rupiah(customer.aging.days_0_15),
            6 => format_rupiah(customer.aging.days_16_30),
            7 => format_rupiah(customer.aging.days_31_45),
            8 => format_rupiah(customer.aging.days_46_60),
            9 => format_rupiah(customer.aging.days_over_60),
            _ => format_rupiah(customer.aging.exception_total()),
        };
        div().child(Label::new(value))
    }
}
