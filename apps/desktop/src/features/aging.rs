use crate::*;
use gpui_kit::component::{
    scroll::{ScrollableElement, ScrollbarMode},
    table::{DataTable, TableSelection},
};

impl Desktop {
    // =========================================================================
    // HALAMAN 2: DETAIL PELANGGAN (Buku Piutang & Riwayat Pembayaran)
    // Toolbar kontekstual: HANYA tombol transaksi & identitas pelanggan ini
    // =========================================================================
    pub(crate) fn render_aging_report_view(&self, cx: &mut Context<Self>) -> Div {
        let report = match self
            .aging_result
            .as_ref()
            .filter(|(date, _)| *date == self.aging_as_of)
        {
            Some((_, Ok(report))) => report.clone(),
            Some((_, Err(error))) => {
                return div()
                    .p(px(24.0))
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .child("Laporan umur piutang tidak dapat dimuat.")
                    .child(error.0.clone())
                    .child(
                        Button::new("retry-aging")
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
                    .child(render_loading_state("Memuat laporan umur piutang..."));
            }
        };

        let filtered_customers: Vec<_> = report
            .customers
            .into_iter()
            .filter(|c| match self.aging_selected_bucket {
                Some(0) => c.days_0_15 > 0,
                Some(1) => c.days_16_30 > 0,
                Some(2) => c.days_31_45 > 0,
                Some(3) => c.days_46_60 > 0,
                Some(4) => c.days_over_60 > 0,
                Some(5) => c.unknown_date > 0,
                Some(6) => c.future_date > 0,
                _ => true,
            })
            .collect();

        self.aging_table.update(cx, |table, cx| {
            if table.delegate_mut().replace(filtered_customers.clone()) {
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
            // CONTEXTUAL COMMAND BAR: HANYA KONTROL LAPORAN AGING
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
                                    .child(format!(
                                        "Laporan Umur Piutang (Per Tanggal Acuan: {})",
                                        report.as_of.format("%Y-%m-%d")
                                    )),
                            )
                            .child(tool_separator())
                            .child(
                                Button::new("tool-aging-date").primary().label("Ubah tanggal acuan")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.open_change_as_of_modal();
                                        cx.notify();
                                    })),
                            )
                            .when(self.aging_as_of != self.today_jakarta(), |d| {
                                d.child(
                                    Button::new("tool-aging-reset-date").outline().label("Reset ke hari ini")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.aging_as_of = this.today_jakarta();
                                            this.status_msg =
                                                "Tanggal acuan laporan aging dikembalikan ke hari ini."
                                                    .into();
                                            cx.notify();
                                        })),
                                )
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .when(self.aging_selected_bucket.is_some(), |d| {
                                d.child(
                                    Button::new("tool-aging-clear-bucket").outline().label("Tampilkan semua kelompok umur")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.aging_selected_bucket = None;
                                            cx.notify();
                                        })),
                                )
                            })
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(rgb(Theme::TEXT_MUTED))
                                    .child("Klik kartu umur di bawah untuk memfilter rincian pelanggan"),
                            ),
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
                    // 7 Bucket Cards (Semua + 5 Bucket Kalender + 2 Exception sesuai API-501)
                    .child(
                        div()
                            .id("aging-cards-scroll")
                            .min_w_0()
                            .h(px(74.0))
                            .flex_shrink_0()
                            .overflow_x_scrollbar()
                            .child(div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(self.native_aging_bucket_card(
                                "ag-b-all",
                                "TOTAL PIUTANG TERBUKA",
                                "Seluruh bucket + pengecualian",
                                format_rupiah(report.total_open_idr),
                                self.aging_selected_bucket.is_none(),
                                Theme::TEXT_PRIMARY,
                                Theme::PRIMARY,
                                None,
                                cx,
                            ))
                            .child(self.native_aging_bucket_card(
                                "ag-b-0",
                                "0–15 HARI",
                                "Piutang baru (lancar)",
                                format_rupiah(report.days_0_15),
                                self.aging_selected_bucket == Some(0),
                                Theme::BADGE_CREDIT_TEXT,
                                Theme::BADGE_CREDIT_BORDER,
                                Some(0),
                                cx,
                            ))
                            .child(self.native_aging_bucket_card(
                                "ag-b-1",
                                "16–30 HARI",
                                "Jatuh tempo normal",
                                format_rupiah(report.days_16_30),
                                self.aging_selected_bucket == Some(1),
                                Theme::BADGE_WARNING_TEXT,
                                Theme::BADGE_WARNING_BORDER,
                                Some(1),
                                cx,
                            ))
                            .child(self.native_aging_bucket_card(
                                "ag-b-2",
                                "31–45 HARI",
                                "Mulai tertunda",
                                format_rupiah(report.days_31_45),
                                self.aging_selected_bucket == Some(2),
                                Theme::BADGE_WARNING_TEXT,
                                Theme::BADGE_WARNING_BORDER,
                                Some(2),
                                cx,
                            ))
                            .child(self.native_aging_bucket_card(
                                "ag-b-3",
                                "46–60 HARI",
                                "Perhatian khusus",
                                format_rupiah(report.days_46_60),
                                self.aging_selected_bucket == Some(3),
                                Theme::BADGE_DEBIT_TEXT,
                                Theme::BADGE_DEBIT_BORDER,
                                Some(3),
                                cx,
                            ))
                            .child(self.native_aging_bucket_card(
                                "ag-b-4",
                                "> 60 HARI",
                                "Kritis / macet",
                                format_rupiah(report.days_over_60),
                                self.aging_selected_bucket == Some(4),
                                Theme::BADGE_CRITICAL_TEXT,
                                Theme::BADGE_CRITICAL_BORDER,
                                Some(4),
                                cx,
                            ))
                            .child(self.native_aging_bucket_card(
                                "ag-b-5",
                                "TGL TIDAK DIKETAHUI",
                                "Historis tanpa tanggal",
                                format_rupiah(report.unknown_date),
                                self.aging_selected_bucket == Some(5),
                                Theme::BADGE_EXCEPTION_TEXT,
                                Theme::BADGE_EXCEPTION_BORDER,
                                Some(5),
                                cx,
                            ))
                            .child(self.native_aging_bucket_card(
                                "ag-b-6",
                                "TGL MASA DEPAN",
                                "Setelah tanggal acuan",
                                format_rupiah(report.future_date),
                                self.aging_selected_bucket == Some(6),
                                Theme::BADGE_EXCEPTION_TEXT,
                                Theme::BADGE_EXCEPTION_BORDER,
                                Some(6),
                                cx,
                            ))),
                    )
                    .child(div().text_size(px(11.0)).text_color(rgb(Theme::TEXT_MUTED)).child("Klik dua kali pelanggan untuk membuka detail."))
                    .child(
                        div().flex_1().min_w_0().min_h(px(150.))
                            .child(if filtered_customers.is_empty() {
                                render_empty_state("Tidak ada piutang terbuka pada kelompok umur ini.").into_any_element()
                            } else {
                                DataTable::new(&self.aging_table)
                                    .scrollbar_mode(ScrollbarMode::Always)
                                    .accessibility_label("Rincian umur piutang per pelanggan")
                                    .into_any_element()
                            }),
                    )
            )
    }

    // =========================================================================
    // HALAMAN 4: MANAJEMEN AKUN & OTORISASI PENGGUNA (Owner Only - FR-007)
    // Toolbar kontekstual: HANYA tombol tambah pengguna & kebijakan keamanan
    // =========================================================================
}
