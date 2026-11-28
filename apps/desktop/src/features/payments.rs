use crate::*;
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
        div()
            .flex_1()
            .min_w_0()
            .min_h(px(150.))
            .flex()
            .flex_col()
            .child(if self.is_loading {
                div()
                    .flex_1()
                    .child(render_loading_state(&self.loading_label))
                    .into_any_element()
            } else if total == 0 {
                div()
                    .flex_1()
                    .child(render_empty_state(
                        "Belum ada riwayat pembayaran untuk pelanggan ini.",
                    ))
                    .into_any_element()
            } else {
                div()
                    .flex_1()
                    .min_w_0()
                    .child(
                        DataTable::new(&self.payment_table)
                            .scrollbar_mode(ScrollbarMode::Always)
                            .accessibility_label("Riwayat pembayaran pelanggan"),
                    )
                    .into_any_element()
            })
            .child(render_pagination_bar(
                page,
                total,
                size,
                "btn-pay-prev",
                "btn-pay-next",
                cx.listener(move |this, _, _, cx| {
                    if page > 0 {
                        this.payment_page_idx = page - 1;
                        cx.notify();
                    }
                }),
                cx.listener(move |this, _, _, cx| {
                    if (page + 1) * size < total {
                        this.payment_page_idx = page + 1;
                        cx.notify();
                    }
                }),
            ))
    }
}
