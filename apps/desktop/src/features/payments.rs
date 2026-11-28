use crate::*;
use gpui_kit::base::Disableable;
use gpui_kit::component::{
    scroll::ScrollbarMode,
    table::{DataTable, TableSelection},
};

impl Desktop {
    pub(crate) fn render_payments_grid(
        &self,
        payments: &[PaymentRow],
        cx: &mut Context<Self>,
    ) -> Div {
        let total = payments.len();
        let size = self.page_size;
        let page = crate::ui::db_grid::bounded_page(self.payment_page_idx, total, size);
        let start = page * size;
        let end = (start + size).min(total);
        self.payment_table.update(cx, |table, cx| {
            if table.delegate_mut().replace(payments[start..end].to_vec()) {
                table.set_selection(TableSelection::None, cx);
                table.refresh(cx);
            }
        });
        let selected_payment = self
            .payment_table
            .read(cx)
            .selected_row()
            .and_then(|row| self.payment_table.read(cx).delegate().rows.get(row))
            .filter(|p| p.status == "posted" && p.unallocated_credit_idr > 0)
            .map(|p| p.id.clone());
        div().flex_1().min_w_0().min_h(px(150.)).flex().flex_col()
            .when(self.can_write() && total > 0, |d| d.child(
                div().flex().items_center().gap(px(10.)).pb(px(6.))
                    .child(div().text_size(px(11.)).text_color(rgb(Theme::TEXT_MUTED)).child("Pilih pembayaran yang memiliki saldo lebih untuk menerapkan kredit."))
                    .child(Button::new("payment-apply-credit").outline().label("Terapkan kredit")
                        .disabled(selected_payment.is_none() || self.is_loading)
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(id) = selected_payment.clone() {
                                this.open_apply_deposit_modal_for_payment(Some(id), cx);
                            }
                        })))
            ))
            .child(if self.is_loading {
                div().flex_1().child(render_loading_state(&self.loading_label)).into_any_element()
            } else if total == 0 {
                div().flex_1().child(render_empty_state("Belum ada riwayat pembayaran untuk pelanggan ini.")).into_any_element()
            } else {
                div().flex_1().min_w_0().child(DataTable::new(&self.payment_table)
                    .scrollbar_mode(ScrollbarMode::Always)
                    .accessibility_label("Riwayat pembayaran pelanggan")).into_any_element()
            })
            .child(render_pagination_bar(page, total, size, "btn-pay-prev", "btn-pay-next",
                cx.listener(move |this, _, _, cx| { if page > 0 { this.payment_page_idx = page - 1; cx.notify(); } }),
                cx.listener(move |this, _, _, cx| { if (page + 1) * size < total { this.payment_page_idx = page + 1; cx.notify(); } })))
    }
}
