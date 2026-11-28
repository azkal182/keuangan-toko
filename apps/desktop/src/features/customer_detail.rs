use crate::*;
use gpui_kit::component::button::{Button, ButtonVariants};

impl Desktop {
    pub(crate) fn render_customer_detail_view(&self, cx: &mut Context<Self>) -> Div {
        let Some(key) = self.detail_key() else {
            return div().p(px(16.0)).child("Pelanggan belum dipilih.");
        };
        let data = match self
            .detail_result
            .as_ref()
            .filter(|(loaded, _)| loaded == &key)
        {
            Some((_, Ok(data))) => data,
            Some((_, Err(error))) => {
                return div()
                    .p(px(24.0))
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .child("Detail pelanggan tidak dapat dimuat.")
                    .child(error.0.clone())
                    .child(
                        Button::new("retry-detail")
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
                    .child(render_loading_state("Memuat detail pelanggan..."));
            }
        };
        let detail = &data.customer;
        let ledger_page = &data.ledger;
        let payments = &data.payments;

        let code_str = detail.code.clone().unwrap_or_else(|| "Tanpa Kode".into());
        let addr_str = detail
            .address
            .clone()
            .unwrap_or_else(|| "Alamat belum tercatat".into());
        let is_lunas = detail.outstanding_idr == 0;
        let has_credit = detail.customer_credit_idr > 0;
        let has_date_filter = self.ledger_from.is_some() || self.ledger_to.is_some();

        div()
            .flex_1()
            .flex()
            .flex_col()
            .h_full()
            .overflow_hidden()
            // CONTEXTUAL COMMAND BAR: HANYA AKSI PELANGGAN TERPILIH
            .child(
                render_toolbar_container()
                    // Left: Back & Transaction actions for this customer
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                secondary_action_button("← Kembali ke Ringkasan", "Esc")
                                    .id("tool-det-back")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.current_view = CurrentView::CustomerList;
                                        cx.notify();
                                    })),
                            )
                            .when(self.can_write() && detail.is_active, |d| {
                                d.child(tool_separator())
                                    .child(
                                        Button::new("tool-det-sale")
                                            .primary()
                                            .label("Catat Belanja")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.open_record_sale_modal();
                                                window.focus(&this.focus, cx);
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        Button::new("tool-det-pay")
                                            .outline()
                                            .label("Catat Pembayaran")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.open_record_payment_modal();
                                                window.focus(&this.focus, cx);
                                                cx.notify();
                                            })),
                                    )
                                    // Tombol Terapkan Saldo Kredit HANYA muncul jika pelanggan punya saldo kredit > 0
                                    .when(has_credit, |d2| {
                                        d2.child(
                                            accent_action_button(
                                                "Terapkan Saldo Kredit",
                                                "F5",
                                                Theme::BADGE_DEPOSIT_TEXT,
                                                Theme::BADGE_DEPOSIT_BG,
                                                Theme::BADGE_DEPOSIT_BORDER,
                                            )
                                            .id("tool-det-apply-credit")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.open_apply_deposit_modal_for_payment(None, cx);
                                                cx.notify();
                                            })),
                                        )
                                    })
                            }),
                    )
                    // Right: Master Customer Actions (Ubah Data & Merge)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .when(self.can_write(), |d| {
                                d.child(
                                    secondary_action_button("Ubah Data Pelanggan", "Ctrl+E")
                                        .id("tool-det-edit-cust")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.open_edit_customer_modal();
                                            cx.notify();
                                        })),
                                )
                            })
                            .when(self.is_owner(), |d| {
                                d.child(
                                    secondary_action_button("Gabungkan (Merge)", "Ctrl+M")
                                        .id("tool-det-merge-cust")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.open_merge_customer_modal();
                                            cx.notify();
                                        })),
                                )
                            }),
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
                    // Panel Identitas & Ringkasan Posisi Keuangan Pelanggan
                    .child(
                        div()
                            .bg(rgb(Theme::BG_CARD))
                            .border_1()
                            .border_color(rgb(Theme::BORDER_STRONG))
                            .rounded(px(3.0))
                            .p(px(10.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            // Left: Customer Identity
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(4.0))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(8.0))
                                            .child(
                                                div()
                                                    .text_size(px(15.0))
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(rgb(Theme::TEXT_PRIMARY))
                                                    .child(detail.name.clone()),
                                            )
                                            .child(
                                                div()
                                                    .px(px(6.0))
                                                    .py(px(1.0))
                                                    .rounded(px(2.0))
                                                    .bg(rgb(Theme::PRIMARY_LIGHT))
                                                    .border_1()
                                                    .border_color(rgb(Theme::PRIMARY_BORDER))
                                                    .text_size(px(10.5))
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(rgb(Theme::PRIMARY))
                                                    .child(code_str),
                                            )
                                            .child(
                                                div()
                                                    .px(px(6.0))
                                                    .py(px(1.0))
                                                    .rounded(px(2.0))
                                                    .bg(rgb(if detail.is_active {
                                                        Theme::BADGE_CREDIT_BG
                                                    } else {
                                                        0xF1F5F9
                                                    }))
                                                    .border_1()
                                                    .border_color(rgb(if detail.is_active {
                                                        Theme::BADGE_CREDIT_BORDER
                                                    } else {
                                                        Theme::BORDER_DEFAULT
                                                    }))
                                                    .text_size(px(10.0))
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(rgb(if detail.is_active {
                                                        Theme::BADGE_CREDIT_TEXT
                                                    } else {
                                                        Theme::TEXT_MUTED
                                                    }))
                                                    .child(if detail.is_active {
                                                        "Aktif"
                                                    } else {
                                                        "Nonaktif"
                                                    }),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.5))
                                            .text_color(rgb(Theme::TEXT_MUTED))
                                            .child(format!(
                                                "{}  •  Versi Data: v{}",
                                                addr_str, detail.version
                                            )),
                                    ),
                            )
                            // Right: Financial Highlights (Total Piutang, Saldo Kredit, Saldo Bersih Buku)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(10.0))
                                    .child(self.detail_financial_badge(
                                        "TOTAL PIUTANG TERUTANG",
                                        format_rupiah(detail.outstanding_idr),
                                        if is_lunas { "Lunas" } else { "Belum Lunas" },
                                        if is_lunas {
                                            Theme::BADGE_CREDIT_TEXT
                                        } else {
                                            Theme::BADGE_DEBIT_TEXT
                                        },
                                        if is_lunas {
                                            Theme::BADGE_CREDIT_BG
                                        } else {
                                            Theme::BADGE_DEBIT_BG
                                        },
                                        if is_lunas {
                                            Theme::BADGE_CREDIT_BORDER
                                        } else {
                                            Theme::BADGE_DEBIT_BORDER
                                        },
                                    ))
                                    .child(self.detail_financial_badge(
                                        "SALDO KREDIT (DEPOSIT)",
                                        format_rupiah(detail.customer_credit_idr),
                                        if has_credit {
                                            "Ada Saldo Lebih"
                                        } else {
                                            "Tidak Ada Deposit"
                                        },
                                        if has_credit {
                                            Theme::BADGE_DEPOSIT_TEXT
                                        } else {
                                            Theme::TEXT_MUTED
                                        },
                                        if has_credit {
                                            Theme::BADGE_DEPOSIT_BG
                                        } else {
                                            0xF8FAFC
                                        },
                                        if has_credit {
                                            Theme::BADGE_DEPOSIT_BORDER
                                        } else {
                                            Theme::BORDER_DEFAULT
                                        },
                                    ))
                                    .child(self.detail_financial_badge(
                                        "SALDO BERSIH BUKU PIUTANG",
                                        format_accounting_rupiah(
                                            ledger_page.summary.ledger_balance_idr,
                                        ),
                                        "Debet − Kredit",
                                        if ledger_page.summary.ledger_balance_idr < 0 {
                                            Theme::BADGE_DEBIT_TEXT
                                        } else {
                                            Theme::TEXT_PRIMARY
                                        },
                                        0xF8FAFC,
                                        Theme::BORDER_STRONG,
                                    )),
                            ),
                    )
                    // SUB-TAB BAR & CONTEXTUAL SUB-TAB CONTROLS
                    .child(
                        div()
                            .h(px(32.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            // Left: Folder Tabs
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(4.0))
                                    .child(
                                        self.native_folder_tab(
                                            "tab-btn-ledger",
                                            format!(
                                                "Buku Piutang (Ledger Mutasi) [{}]",
                                                ledger_page.entries.len()
                                            ),
                                            self.customer_tab == CustomerTab::Ledger,
                                        )
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.customer_tab = CustomerTab::Ledger;
                                            cx.notify();
                                        })),
                                    )
                                    .child(
                                        self.native_folder_tab(
                                            "tab-btn-payments",
                                            if self.customer_tab == CustomerTab::Payments {
                                                format!("Riwayat Pembayaran & Alokasi FIFO [{}]", payments.len())
                                            } else {
                                                "Riwayat Pembayaran & Alokasi FIFO".to_string()
                                            },
                                            self.customer_tab == CustomerTab::Payments,
                                        )
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.customer_tab = CustomerTab::Payments;
                                            cx.notify();
                                        })),
                                    ),
                            )
                            // Right: Controls specific to the active sub-tab!
                            .child(match self.customer_tab {
                                CustomerTab::Ledger => div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.0))
                                    .child(
                                        secondary_action_button(
                                            if has_date_filter {
                                                "Filter Tanggal: Aktif"
                                            } else {
                                                "Filter Periode Tanggal..."
                                            },
                                            "",
                                        )
                                        .id("btn-ledger-date-filter")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.open_filter_ledger_date_modal();
                                            cx.notify();
                                        })),
                                    )
                                    .when(has_date_filter, |d| {
                                        d.child(
                                            secondary_action_button("Reset Tanggal", "")
                                                .id("btn-ledger-date-reset")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.ledger_from = None;
                                                    this.ledger_to = None;
                                                    this.ledger_page_idx = 0;
                                                    this.status_msg =
                                                        "Filter tanggal buku piutang direset."
                                                            .into();
                                                    cx.notify();
                                                })),
                                        )
                                    })
                                    .child(tool_separator())
                                    .child(
                                        self.sort_toggle_button(
                                            "btn-sort-desc",
                                            "Terbaru di Atas",
                                            self.ledger_sort_desc,
                                        )
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.ledger_sort_desc = true;
                                            this.ledger_page_idx = 0;
                                            cx.notify();
                                        })),
                                    )
                                    .child(
                                        self.sort_toggle_button(
                                            "btn-sort-asc",
                                            "Terlama di Atas (Kronologis)",
                                            !self.ledger_sort_desc,
                                        )
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.ledger_sort_desc = false;
                                            this.ledger_page_idx = 0;
                                            cx.notify();
                                        })),
                                    ),
                                CustomerTab::Payments => div()
                                    .flex()
                                    .items_center()
                                    .gap(px(8.0))
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(rgb(Theme::TEXT_MUTED))
                                            .child("Pembayaran otomatis melunasi piutang terlama (FIFO). Sisa menjadi Saldo Kredit."),
                                    ),
                            }),
                    )
                    // ACTIVE SUB-TAB DBGRID
                    .child(match self.customer_tab {
                        CustomerTab::Ledger => {
                            self.render_ledger_grid(&ledger_page.entries, cx)
                        }
                        CustomerTab::Payments => self.render_payments_grid(payments, cx),
                    }),
            )
    }
}
