use crate::store::{LedgerEntry, PaymentRow};
use crate::ui::theme::{Theme, format_accounting_rupiah, format_rupiah};
use gpui::{App, Context, IntoElement, ParentElement, Styled, Window, div, px, rgb};
use gpui_kit::component::{
    label::Label,
    table::{Column, TableDelegate, TableState},
};

#[derive(Default)]
pub(crate) struct LedgerRows {
    pub rows: Vec<LedgerEntry>,
    signature: String,
}
impl LedgerRows {
    pub fn replace(&mut self, rows: Vec<LedgerEntry>) -> bool {
        let signature = format!("{rows:?}");
        if self.signature == signature {
            return false;
        }
        self.rows = rows;
        self.signature = signature;
        true
    }
}
impl TableDelegate for LedgerRows {
    fn columns_count(&self, _: &App) -> usize {
        11
    }
    fn rows_count(&self, _: &App) -> usize {
        self.rows.len()
    }
    fn column(&self, ix: usize, _: &App) -> Column {
        let (key, title, width) = [
            ("date", "Tanggal", 130.),
            ("description", "Uraian transaksi", 240.),
            ("qty", "Jumlah (qty)", 115.),
            ("price", "Harga satuan", 140.),
            ("debit", "Debet", 145.),
            ("credit", "Kredit", 145.),
            ("balance", "Saldo", 150.),
            ("settled", "Lunas", 100.),
            ("remaining", "Sisa", 145.),
            ("age", "Umur piutang", 150.),
            ("status", "Status", 120.),
        ][ix];
        let column = Column::new(key, title).width(px(width)).movable(false);
        if (2..=6).contains(&ix) || ix == 8 {
            column.text_right()
        } else {
            column
        }
    }
    fn render_td(
        &mut self,
        row: usize,
        col: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let e = &self.rows[row];
        let debit = e.debit_idr > 0;
        let settled = e.remaining_idr == 0;
        let dash = || "–".to_string();
        let value = match col {
            0 => e
                .effective_date
                .map(|d| d.format("%d %b %Y").to_string())
                .unwrap_or_else(|| "Tidak diketahui".into()),
            1 => {
                if e.is_reversed {
                    format!("{} [DIBALIK]", e.description)
                } else {
                    e.description.clone()
                }
            }
            2 => e.quantity.clone().unwrap_or_else(dash),
            3 => e.unit_price_idr.map(format_rupiah).unwrap_or_else(dash),
            4 => {
                if debit {
                    format_rupiah(e.debit_idr)
                } else {
                    dash()
                }
            }
            5 => {
                if e.credit_idr > 0 {
                    format_rupiah(e.credit_idr)
                } else {
                    dash()
                }
            }
            6 => format_accounting_rupiah(e.running_balance_idr),
            7 => {
                if !debit || e.is_reversed {
                    dash()
                } else if settled {
                    "Lunas".into()
                } else {
                    "Belum".into()
                }
            }
            8 => {
                if !debit || settled || e.is_reversed {
                    dash()
                } else {
                    format_rupiah(e.remaining_idr)
                }
            }
            9 => {
                if !debit || settled || e.is_reversed {
                    dash()
                } else {
                    match e.age_days {
                        None => "Tgl tidak diketahui".into(),
                        Some(d) if d < 0 => "Masa depan".into(),
                        Some(d) if d > 60 => ">60 hari".into(),
                        Some(d) => format!("{d} hari"),
                    }
                }
            }
            _ => {
                if e.is_reversed {
                    "Dibalik".into()
                } else {
                    "Posted".into()
                }
            }
        };
        let color = if (col == 6 && e.running_balance_idr < 0)
            || (col == 7 && debit && !settled && !e.is_reversed)
            || (col == 10 && e.is_reversed)
        {
            Theme::BADGE_DEBIT_TEXT
        } else if col == 5 && e.credit_idr > 0 {
            Theme::BADGE_CREDIT_TEXT
        } else {
            Theme::TEXT_PRIMARY
        };
        div().text_color(rgb(color)).child(Label::new(value))
    }
}

#[derive(Default)]
pub(crate) struct PaymentRows {
    pub rows: Vec<PaymentRow>,
    signature: String,
}
impl PaymentRows {
    pub fn replace(&mut self, rows: Vec<PaymentRow>) -> bool {
        let signature = format!("{rows:?}");
        if self.signature == signature {
            return false;
        }
        self.rows = rows;
        self.signature = signature;
        true
    }
}
impl TableDelegate for PaymentRows {
    fn columns_count(&self, _: &App) -> usize {
        8
    }
    fn rows_count(&self, _: &App) -> usize {
        self.rows.len()
    }
    fn column(&self, ix: usize, _: &App) -> Column {
        let (key, title, width) = [
            ("date", "Tanggal diterima", 150.),
            ("method", "Metode", 140.),
            ("status", "Status", 110.),
            ("reference", "Referensi", 150.),
            ("amount", "Nominal bayar", 160.),
            ("allocated", "Teralokasi (FIFO)", 170.),
            ("credit", "Saldo lebih (kredit)", 180.),
            ("note", "Catatan & alokasi", 250.),
        ][ix];
        let column = Column::new(key, title).width(px(width)).movable(false);
        if (4..=6).contains(&ix) {
            column.text_right()
        } else {
            column
        }
    }
    fn render_td(
        &mut self,
        row: usize,
        col: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let p = &self.rows[row];
        let value = match col {
            0 => p.received_date.format("%d %b %Y").to_string(),
            1 => p.method_label().into(),
            2 => {
                if p.status == "posted" {
                    "Posted".into()
                } else {
                    "Reversed".into()
                }
            }
            3 => p.reference.clone().unwrap_or_else(|| "–".into()),
            4 => format_rupiah(p.amount_idr),
            5 => format_rupiah(p.allocated_idr),
            6 => format_rupiah(p.unallocated_credit_idr),
            _ => {
                if p.allocations.is_empty() {
                    p.note
                        .clone()
                        .unwrap_or_else(|| "Belum dialokasikan".into())
                } else {
                    match &p.note {
                        Some(n) => format!("{} ({} nota)", n, p.allocations.len()),
                        None => format!("Dialokasikan ke {} nota debet", p.allocations.len()),
                    }
                }
            }
        };
        div().child(Label::new(value))
    }
}
