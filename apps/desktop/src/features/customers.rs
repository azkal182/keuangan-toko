use crate::*;
use gpui_kit::component::{
    button::{Button, ButtonVariants},
    input::Input,
    scroll::ScrollbarMode,
    table::{DataTable, TableSelection},
};

impl Desktop {
    pub(crate) fn render_customer_list_view(&self, cx: &mut Context<Self>) -> Div {
        let all = match &self.customers_result {
            Some(Ok(customers)) => customers,
            Some(Err(error)) => {
                return div()
                    .p(px(24.0))
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .child("Daftar pelanggan tidak dapat dimuat.")
                    .child(error.0.clone())
                    .child(
                        Button::new("retry-customers")
                            .outline()
                            .label("Coba lagi")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.store.invalidate_cache();
                                this.invalidate_aging();
                                cx.notify();
                            })),
                    );
            }
            None => {
                return div()
                    .p(px(24.0))
                    .child(render_loading_state("Memuat daftar pelanggan..."));
            }
        };
        let term = self.query.trim().to_lowercase();
        let customers: Vec<_> = all
            .iter()
            .filter(|c| !self.customer_active_only || c.is_active)
            .filter(|c| {
                term.is_empty()
                    || c.name.to_lowercase().contains(&term)
                    || c.code
                        .as_deref()
                        .is_some_and(|v| v.to_lowercase().contains(&term))
                    || c.address
                        .as_deref()
                        .is_some_and(|v| v.to_lowercase().contains(&term))
            })
            .cloned()
            .collect();
        let total_customers = customers.len();
        let total_piutang: i64 = customers.iter().map(|c| c.outstanding_idr).sum();
        let total_deposit: i64 = customers.iter().map(|c| c.customer_credit_idr).sum();
        let total_over_60: i64 = customers.iter().map(|c| c.aging.days_over_60).sum();

        // Pagination slice
        let page_size = self.page_size;
        let page = crate::ui::db_grid::bounded_page(self.customer_page, total_customers, page_size);
        let start_idx = page * page_size;
        let end_idx = (start_idx + page_size).min(total_customers);
        let paged_customers: Vec<_> = customers[start_idx..end_idx].to_vec();

        self.customer_table.update(cx, |table, cx| {
            if table.delegate_mut().replace(paged_customers.clone()) {
                table.set_selection(TableSelection::None, cx);
                table.refresh(cx);
            }
        });
        div()
            .flex_1()
            .flex()
            .flex_col()
            .h_full()
            .overflow_hidden()
            // CONTEXTUAL COMMAND BAR: HANYA AKSI DAFTAR PELANGGAN
            .child(
                render_toolbar_container()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .text_size(px(12.5))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(Theme::TEXT_PRIMARY))
                                    .child("Buku Ringkasan Saldo Pelanggan"),
                            )
                            .child(tool_separator())
                            .when(self.can_write(), |d| {
                                d.child(
                                    Button::new("tool-cust-add")
                                        .primary()
                                        .label("Tambah Pelanggan")
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.open_new_customer_modal();
                                            window.focus(&this.focus, cx);
                                            cx.notify();
                                        })),
                                )
                            })
                            .when(
                                self.customer_table
                                    .read(cx)
                                    .selected_row()
                                    .and_then(|row| paged_customers.get(row))
                                    .is_some_and(|c| {
                                        Some(&c.id) == self.selected_customer_id.as_ref()
                                    }),
                                |d| {
                                    d.child(
                                        Button::new("tool-cust-open")
                                            .outline()
                                            .label("Buka Detail")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.current_view = CurrentView::CustomerDetail;
                                                cx.notify();
                                            })),
                                    )
                                },
                            )
                            .child(
                                secondary_action_button(
                                    if self.customer_active_only {
                                        "Filter: Hanya Aktif"
                                    } else {
                                        "Filter: Semua Status"
                                    },
                                    "",
                                )
                                .id("tool-cust-filter-active")
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.customer_active_only = !this.customer_active_only;
                                        this.customer_page = 0;
                                        this.selected_customer_id = None;
                                        cx.notify();
                                    },
                                )),
                            ),
                    )
                    // Komponen input GPUI Kit menangani editing, clipboard, dan IME.
                    .child(
                        div()
                            .w(px(240.0))
                            .min_w_0()
                            .child(Input::new(&self.search_input)),
                    ),
            )
            // WORKSPACE BODY
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .p(px(10.0))
                    .gap(px(8.0))
                    .overflow_hidden()
                    // 4 Native Summary Boxes (Sesuai Web CustomerList)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(self.native_summary_box(
                                "TOTAL PELANGGAN TERDAFTAR",
                                format!("{} Pelanggan", total_customers),
                                "Aktif dan terpantau dalam buku piutang",
                                Theme::TEXT_PRIMARY,
                                Theme::BORDER_STRONG,
                            ))
                            .child(self.native_summary_box(
                                "TOTAL PIUTANG TERBUKA",
                                format_rupiah(total_piutang),
                                "Total piutang belum terbayar",
                                Theme::TEXT_PRIMARY,
                                Theme::PRIMARY,
                            ))
                            .child(self.native_summary_box(
                                "SALDO KREDIT (DEPOSIT)",
                                format_rupiah(total_deposit),
                                "Kelebihan bayar belum dialokasikan",
                                Theme::BADGE_DEPOSIT_TEXT,
                                Theme::BADGE_DEPOSIT_BORDER,
                            ))
                            .child(self.native_summary_box(
                                "PIUTANG KRITIS (>60 HARI)",
                                format_rupiah(total_over_60),
                                "Memerlukan penagihan intensif",
                                Theme::BADGE_DEBIT_TEXT,
                                Theme::BADGE_DEBIT_BORDER,
                            )),
                    )
                    // DataTable virtual dengan scrollbar horizontal dan sel dapat dibaca.
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .min_h(px(150.))
                            .flex()
                            .flex_col()
                            .child(if self.is_loading {
                                render_loading_state(&self.loading_label).into_any_element()
                            } else if paged_customers.is_empty() {
                                render_empty_state("Tidak ada pelanggan yang ditemukan.")
                                    .into_any_element()
                            } else {
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(
                                        DataTable::new(&self.customer_table)
                                            .scrollbar_mode(ScrollbarMode::Always)
                                            .accessibility_label("Daftar pelanggan"),
                                    )
                                    .into_any_element()
                            })
                            .child(render_pagination_bar(
                                page,
                                total_customers,
                                page_size,
                                "btn-cust-prev",
                                "btn-cust-next",
                                cx.listener(move |this, _, _, cx| {
                                    if page > 0 {
                                        this.customer_page = page - 1;
                                        this.selected_customer_id = None;
                                        cx.notify();
                                    }
                                }),
                                cx.listener(move |this, _, _, cx| {
                                    if (page + 1) * page_size < total_customers {
                                        this.customer_page = page + 1;
                                        this.selected_customer_id = None;
                                        cx.notify();
                                    }
                                }),
                            )),
                    ),
            )
    }
}
