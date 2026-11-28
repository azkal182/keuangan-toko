use crate::*;
use gpui_kit::component::Disableable;
use gpui_kit::component::date_picker::DatePicker;
use gpui_kit::component::input::{Input, InputContentType};

impl Desktop {
    pub(crate) fn render_login_page(&self, cx: &mut Context<Self>) -> Div {
        let mut form = div().flex().flex_col().gap(px(14.0));
        for (index, (label, _)) in self.form_fields.iter().enumerate() {
            form = form.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(5.0))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(rgb(Theme::TEXT_SECONDARY))
                            .child(*label),
                    )
                    .child(if index == 1 {
                        Input::new(&self.login_inputs[index])
                            .content_type(InputContentType::Password)
                            .disabled(self.is_loading)
                    } else {
                        Input::new(&self.login_inputs[index])
                            .content_type(InputContentType::Username)
                            .disabled(self.is_loading)
                    }),
            );
        }
        div()
            .size_full()
            .bg(rgb(0xEEF2F8))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .w(px(460.0))
                    .max_w(gpui::relative(0.94))
                    .bg(rgb(Theme::BG_CARD))
                    .border_1()
                    .border_color(rgb(Theme::BORDER_DEFAULT))
                    .rounded(px(12.0))
                    .overflow_hidden()
                    .shadow_2xl()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .bg(rgb(0x101F42))
                            .p(px(28.0))
                            .flex()
                            .items_center()
                            .gap(px(16.0))
                            .child(
                                gpui::img("brand/piutang-reza-putra.png")
                                    .w(px(62.0))
                                    .h(px(62.0))
                                    .rounded(px(12.0)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(5.0))
                                    .child(
                                        div()
                                            .text_size(px(20.0))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(rgb(0xFFFFFF))
                                            .child("Piutang Reza Putra"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(rgb(0xBDCCE5))
                                            .child("Pencatatan piutang yang lebih tertata"),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .p(px(28.0))
                            .flex()
                            .flex_col()
                            .gap(px(18.0))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(5.0))
                                    .child(
                                        div()
                                            .text_size(px(19.0))
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(rgb(Theme::TEXT_PRIMARY))
                                            .child("Selamat datang"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .text_color(rgb(Theme::TEXT_SECONDARY))
                                            .child("Masuk dengan akun Anda untuk melanjutkan."),
                                    ),
                            )
                            .child(form)
                            .when(!self.form_error.is_empty(), |d| {
                                d.child(
                                    div()
                                        .p(px(10.0))
                                        .rounded(px(6.0))
                                        .bg(rgb(Theme::BADGE_DEBIT_BG))
                                        .text_size(px(12.0))
                                        .text_color(rgb(Theme::BADGE_DEBIT_TEXT))
                                        .child(self.form_error.clone()),
                                )
                            })
                            .child(
                                Button::new("login-submit")
                                    .primary()
                                    .label(if self.is_loading {
                                        "Sedang masuk..."
                                    } else {
                                        "Masuk"
                                    })
                                    .disabled(self.is_loading)
                                    .on_click(cx.listener(|this, _, _, cx| this.submit_modal(cx))),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(rgb(Theme::TEXT_LIGHT))
                                    .child("Pastikan perangkat terhubung ke internet."),
                            ),
                    ),
            )
    }
    pub(crate) fn render_modal_overlay(&self, cx: &mut Context<Self>) -> Option<Div> {
        let modal = self.active_modal.as_ref()?;

        if self.reviewing_transaction
            && matches!(modal, ActiveModal::RecordSale | ActiveModal::RecordPayment)
        {
            let is_sale = matches!(modal, ActiveModal::RecordSale);
            let customer = self
                .current_detail()
                .map(|data| data.customer.name.clone())
                .unwrap_or_else(|| "Pelanggan terpilih".into());
            let review_row = |label: &'static str, value: String| {
                div()
                    .flex()
                    .items_start()
                    .gap(px(16.0))
                    .child(
                        div()
                            .w(px(100.0))
                            .flex_shrink_0()
                            .text_color(rgb(Theme::TEXT_SECONDARY))
                            .child(label),
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_color(rgb(Theme::TEXT_PRIMARY))
                            .child(value),
                    )
            };
            let mut body = div()
                .flex()
                .flex_col()
                .gap(px(12.))
                .child(
                    div()
                        .text_color(rgb(Theme::TEXT_SECONDARY))
                        .child("Periksa kembali data sebelum menyimpan."),
                )
                .child(review_row("Pelanggan", customer));
            if is_sale {
                body = body
                    .child(review_row("Tanggal", self.form_fields[0].1.clone()))
                    .child(review_row(
                        "Referensi",
                        if self.form_fields[1].1.is_empty() {
                            "-"
                        } else {
                            &self.form_fields[1].1
                        }
                        .to_string(),
                    ));
                let mut total = 0_i64;
                let mut items = div().flex().flex_col().gap(px(8.0));
                for (description, qty, price) in &self.sale_items {
                    if description.trim().is_empty() && price.trim().is_empty() {
                        continue;
                    }
                    let price = parse_money_input(price).unwrap_or(0);
                    let line = compute_line_total_half_up(qty, price).unwrap_or(0);
                    total = total.saturating_add(line);
                    items = items.child(
                        div()
                            .flex()
                            .justify_between()
                            .gap(px(12.0))
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.0))
                                    .child(description.clone())
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(rgb(Theme::TEXT_SECONDARY))
                                            .child(format!("{} × {}", qty, format_rupiah(price))),
                                    ),
                            )
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(format_rupiah(line)),
                            ),
                    );
                }
                body = body
                    .child(
                        div()
                            .p(px(12.0))
                            .rounded(px(4.0))
                            .bg(rgb(Theme::BG_HEADER))
                            .flex()
                            .flex_col()
                            .gap(px(10.0))
                            .child(div().font_weight(FontWeight::BOLD).child("Rincian barang"))
                            .child(items),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .font_weight(FontWeight::BOLD)
                            .child("Total belanja")
                            .child(format_rupiah(total)),
                    );
            } else {
                let amount = parse_money_input(&self.form_fields[1].1).unwrap_or(0);
                body = body
                    .child(review_row("Tanggal", self.form_fields[0].1.clone()))
                    .child(review_row("Metode", self.form_fields[2].1.clone()))
                    .child(review_row(
                        "Referensi",
                        if self.form_fields[3].1.is_empty() {
                            "-"
                        } else {
                            &self.form_fields[3].1
                        }
                        .to_string(),
                    ))
                    .child(review_row(
                        "Catatan",
                        if self.form_fields[4].1.is_empty() {
                            "-"
                        } else {
                            &self.form_fields[4].1
                        }
                        .to_string(),
                    ))
                    .child(
                        div()
                            .p(px(12.0))
                            .rounded(px(4.0))
                            .bg(rgb(Theme::BG_HEADER))
                            .flex()
                            .justify_between()
                            .font_weight(FontWeight::BOLD)
                            .child("Total pembayaran")
                            .child(format_rupiah(amount)),
                    );
            }
            if !self.form_error.is_empty() {
                body = body.child(
                    div()
                        .text_color(rgb(Theme::BADGE_DEBIT_TEXT))
                        .child(self.form_error.clone()),
                );
            }
            let footer = div()
                .h(px(54.))
                .px(px(14.))
                .flex()
                .items_center()
                .justify_end()
                .gap(px(8.))
                .child(
                    Button::new("review-back")
                        .outline()
                        .label("Kembali")
                        .disabled(self.is_loading)
                        .on_click(cx.listener(|this, _, window, cx| {
                            if this.is_loading {
                                return;
                            }
                            this.reviewing_transaction = false;
                            this.form_error.clear();
                            this.focus_active_modal_field(window, cx);
                            cx.notify();
                        })),
                )
                .child(
                    Button::new("review-confirm")
                        .primary()
                        .label(if self.is_loading {
                            "Menyimpan..."
                        } else {
                            "Konfirmasi dan simpan"
                        })
                        .disabled(self.is_loading || self.transaction_uncertain)
                        .on_click(cx.listener(|this, _, _, cx| {
                            if !this.is_loading {
                                this.submit_modal(cx);
                            }
                        })),
                );
            return Some(
                ModalDialog::new(
                    if is_sale {
                        "Tinjau belanja"
                    } else {
                        "Tinjau pembayaran"
                    },
                    560.,
                )
                .frame(
                    div()
                        .id("review-content")
                        .max_h(px(350.))
                        .overflow_y_scroll()
                        .child(body)
                        .into_any_element(),
                    footer,
                ),
            );
        }

        match modal {
            ActiveModal::Notice { title, message } => {
                let dialog = ModalDialog::new(title, 420.0);
                let body = div()
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .child(
                        div()
                            .text_size(px(12.5))
                            .text_color(rgb(Theme::TEXT_PRIMARY))
                            .child(message.clone()),
                    )
                    .into_any_element();

                let footer = div()
                    .h(px(42.0))
                    .bg(rgb(Theme::BG_HEADER))
                    .border_t_1()
                    .border_color(rgb(Theme::BORDER_DEFAULT))
                    .px(px(14.0))
                    .flex()
                    .items_center()
                    .justify_end()
                    .child(
                        Button::new("btn-notice-ok")
                            .primary()
                            .label("Tutup")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.active_modal = None;
                                cx.notify();
                            })),
                    );
                Some(dialog.frame(body, footer))
            }

            ActiveModal::RecordSale => {
                let cust_name = self
                    .current_detail()
                    .map(|data| data.customer.name.clone())
                    .unwrap_or_else(|| "Pelanggan".into());

                let dialog = ModalDialog::new("Catat Belanja Kredit / Piutang Baru", 800.0)
                    .subtitle(cust_name);

                let grand_total =
                    self.sale_items
                        .iter()
                        .try_fold(0_i64, |sum, (desc, qty, price)| {
                            if desc.trim().is_empty() && price.trim().is_empty() {
                                return Some(sum);
                            }
                            let line =
                                compute_line_total_half_up(qty, parse_money_input(price)?).ok()?;
                            sum.checked_add(line)
                        });

                let active_pos = self.sale_active_field;
                let item_count = self.sale_items.len();

                let body = div()
                    .flex()
                    .flex_col()
                    .min_w(px(720.0))
                    .gap(px(14.0))
                    // Header form: tanggal dan referensi.
                    .child(
                        div().flex().items_start().gap(px(12.0)).children(
                            self.form_fields
                                .iter()
                                .enumerate()
                                .map(|(f_idx, (label, _val))| {
                                    let is_focused = active_pos == (item_count, f_idx);
                                    div()
                                        .id(("sale-hdr-f", f_idx))
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .gap(px(3.0))
                                        .cursor_pointer()
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.sale_active_field = (this.sale_items.len(), f_idx);
                                            cx.notify();
                                        }))
                                        .child(
                                            div()
                                                .text_size(px(10.0))
                                                .font_weight(if is_focused {
                                                    FontWeight::BOLD
                                                } else {
                                                    FontWeight::SEMIBOLD
                                                })
                                                .text_color(rgb(if is_focused {
                                                    Theme::PRIMARY
                                                } else {
                                                    Theme::TEXT_SECONDARY
                                                }))
                                                .child(format!(
                                                    "{}{}",
                                                    *label,
                                                    if is_focused { " ◀" } else { "" }
                                                )),
                                        )
                                        .child(if f_idx == 0 {
                                            DatePicker::new(&self.date_pickers[0])
                                                .placeholder("Pilih tanggal")
                                                .into_any_element()
                                        } else {
                                            Input::new(&self.customer_form_inputs[f_idx])
                                                .disabled(self.is_loading)
                                                .into_any_element()
                                        })
                                }),
                        ),
                    )
                    // Tabel Faktur Multi-Item
                    .child(
                        div()
                            .border_1()
                            .border_color(rgb(Theme::BORDER_STRONG))
                            .rounded(px(3.0))
                            .overflow_hidden()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .h(px(34.0))
                                    .bg(rgb(Theme::BG_HEADER))
                                    .border_b_1()
                                    .border_color(rgb(Theme::BORDER_STRONG))
                                    .px(px(8.0))
                                    .flex()
                                    .items_center()
                                    .text_size(px(10.5))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(Theme::TEXT_SECONDARY))
                                    .child(div().w(px(32.0)).child("NO"))
                                    .child(div().w(px(260.0)).child("NAMA BARANG / URAIAN"))
                                    .child(div().w(px(95.0)).text_right().pr(px(8.0)).child("QTY"))
                                    .child(
                                        div()
                                            .w(px(130.0))
                                            .text_right()
                                            .pr(px(8.0))
                                            .child("HARGA SATUAN"),
                                    )
                                    .child(
                                        div()
                                            .w(px(130.0))
                                            .text_right()
                                            .pr(px(8.0))
                                            .child("SUBTOTAL"),
                                    )
                                    .child(div().w(px(45.0)).text_center().child("HAPUS")),
                            )
                            .children(self.sale_items.iter().enumerate().map(
                                |(r_idx, (_desc, qty, price))| {
                                    let subtotal = parse_money_input(price)
                                        .and_then(|p| compute_line_total_half_up(qty, p).ok());

                                    div()
                                        .h(px(44.0))
                                        .px(px(8.0))
                                        .bg(rgb(if r_idx % 2 == 1 {
                                            Theme::BG_ROW_ALT
                                        } else {
                                            0xFFFFFF
                                        }))
                                        .border_b_1()
                                        .border_color(rgb(Theme::GRID_LINE))
                                        .flex()
                                        .items_center()
                                        .child(
                                            div()
                                                .w(px(32.0))
                                                .text_size(px(11.0))
                                                .text_color(rgb(Theme::TEXT_MUTED))
                                                .child(format!("{}", r_idx + 1)),
                                        )
                                        // Kolom Uraian
                                        .child(
                                            div()
                                                .id(("s-desc", r_idx))
                                                .w(px(252.0))
                                                .mr(px(8.0))
                                                .cursor_pointer()
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.sale_active_field = (r_idx, 0);
                                                    cx.notify();
                                                }))
                                                .child(
                                                    Input::new(&self.sale_inputs[r_idx][0])
                                                        .disabled(self.is_loading),
                                                ),
                                        )
                                        // Kolom Qty
                                        .child(
                                            div()
                                                .id(("s-qty", r_idx))
                                                .w(px(87.0))
                                                .mr(px(8.0))
                                                .cursor_pointer()
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.sale_active_field = (r_idx, 1);
                                                    cx.notify();
                                                }))
                                                .child(
                                                    Input::new(&self.sale_inputs[r_idx][1])
                                                        .disabled(self.is_loading),
                                                ),
                                        )
                                        // Kolom Harga Satuan
                                        .child(
                                            div()
                                                .id(("s-price", r_idx))
                                                .w(px(122.0))
                                                .mr(px(8.0))
                                                .cursor_pointer()
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.sale_active_field = (r_idx, 2);
                                                    cx.notify();
                                                }))
                                                .child(
                                                    Input::new(&self.sale_inputs[r_idx][2])
                                                        .disabled(self.is_loading),
                                                ),
                                        )
                                        // Subtotal
                                        .child(
                                            div()
                                                .w(px(130.0))
                                                .pr(px(8.0))
                                                .text_right()
                                                .text_size(px(11.5))
                                                .font_weight(FontWeight::BOLD)
                                                .text_color(rgb(Theme::TEXT_PRIMARY))
                                                .child(
                                                    subtotal
                                                        .map(format_rupiah)
                                                        .unwrap_or_else(|| "—".into()),
                                                ),
                                        )
                                        // Tombol Hapus Baris
                                        .child(
                                            Button::new(("s-del", r_idx))
                                                .ghost()
                                                .label("×")
                                                .accessibility_label(format!(
                                                    "Hapus barang {}",
                                                    r_idx + 1
                                                ))
                                                .disabled(self.sale_items.len() <= 1)
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    if this.sale_items.len() > 1 {
                                                        this.sale_items.remove(r_idx);
                                                        this.sale_active_field = (0, 0);
                                                        this.sale_inputs_initialized = false;
                                                        cx.notify();
                                                    }
                                                })),
                                        )
                                },
                            ))
                            // Footer Tabel: Tambah Baris + Total
                            .child(
                                div()
                                    .h(px(48.0))
                                    .bg(rgb(Theme::BG_HEADER))
                                    .px(px(8.0))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        Button::new("btn-sale-add-row")
                                            .outline()
                                            .label("Tambah barang")
                                            .track_focus(&self.sale_action_focus[0])
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                let next_idx = this.sale_items.len();
                                                this.sale_items.push((
                                                    "".into(),
                                                    "1".into(),
                                                    "".into(),
                                                ));
                                                this.sale_active_field = (next_idx, 0);
                                                this.sale_inputs_initialized = false;
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(8.0))
                                            .child(
                                                div()
                                                    .text_size(px(11.5))
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(rgb(Theme::TEXT_SECONDARY))
                                                    .child("TOTAL SELURUH BELANJA:"),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(14.5))
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(rgb(Theme::BADGE_DEBIT_TEXT))
                                                    .child(
                                                        grand_total
                                                            .map(format_rupiah)
                                                            .unwrap_or_else(|| {
                                                                "Periksa jumlah dan harga".into()
                                                            }),
                                                    ),
                                            ),
                                    ),
                            ),
                    )
                    .when(!self.form_error.is_empty(), |d| {
                        d.child(
                            div()
                                .p(px(8.0))
                                .bg(rgb(Theme::BADGE_DEBIT_BG))
                                .border_1()
                                .border_color(rgb(Theme::BADGE_DEBIT_BORDER))
                                .rounded(px(3.0))
                                .text_size(px(11.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(Theme::BADGE_DEBIT_TEXT))
                                .child(self.form_error.clone()),
                        )
                    })
                    .child(
                        div()
                            .text_size(px(10.5))
                            .text_color(rgb(Theme::TEXT_MUTED))
                            .child(
                                "Saldo kredit yang tersedia diterapkan otomatis ke belanja baru.",
                            ),
                    )
                    .into_any_element();

                let footer = self.render_modal_footer("Tinjau belanja", cx);
                Some(dialog.frame(body, footer))
            }

            _ => {
                let (title, submit_label, width) = match modal {
                    ActiveModal::Login => {
                        ("Masuk Sesi Pengguna (Login)", "Masuk Sesi (Enter)", 440.0)
                    }
                    ActiveModal::ChangePassword => (
                        "Ubah Kata Sandi Akun",
                        "Simpan Kata Sandi Baru (Enter)",
                        460.0,
                    ),
                    ActiveModal::NewCustomer => {
                        ("Tambah Pelanggan Baru", "Simpan Pelanggan (Enter)", 480.0)
                    }
                    ActiveModal::EditCustomer { .. } => (
                        "Ubah Identitas & Status Pelanggan",
                        "Simpan Perubahan (Enter)",
                        480.0,
                    ),
                    ActiveModal::MergeCustomer { .. } => (
                        "Gabungkan Pelanggan (Merge - Owner Only)",
                        "Konfirmasi Gabungkan (Enter)",
                        500.0,
                    ),
                    ActiveModal::RecordPayment => (
                        "Catat Penerimaan Pembayaran & Alokasi FIFO",
                        "Tinjau pembayaran",
                        500.0,
                    ),
                    ActiveModal::ApplyDeposit => (
                        "Terapkan Saldo Kredit Pelanggan ke Piutang",
                        "Terapkan Saldo Kredit (Enter)",
                        520.0,
                    ),
                    ActiveModal::ReallocatePayment { .. } => (
                        "Realokasi Pembayaran (Owner Only - API-305)",
                        "Simpan Realokasi (Enter)",
                        520.0,
                    ),
                    ActiveModal::ReverseTransaction { .. } => (
                        "Pembalikan Transaksi / Reversal (Owner Only)",
                        "Konfirmasi Pembalikan (Enter)",
                        480.0,
                    ),
                    ActiveModal::FilterLedgerDate => (
                        "Filter Rentang Tanggal Buku Piutang",
                        "Terapkan Filter (Enter)",
                        440.0,
                    ),
                    ActiveModal::NewUser => (
                        "Tambah Akun Pengguna Baru (Owner Only)",
                        "Buat Akun Pengguna (Enter)",
                        480.0,
                    ),
                    ActiveModal::EditUser { .. } => (
                        "Ubah Peran & Status Akun Pengguna",
                        "Simpan Perubahan Akun (Enter)",
                        480.0,
                    ),
                    ActiveModal::ResetUserPassword { .. } => (
                        "Reset Kata Sandi Sementara Pengguna",
                        "Reset Kata Sandi (Enter)",
                        480.0,
                    ),
                    ActiveModal::ChangeAsOfDate => (
                        "Ubah Tanggal Acuan Laporan Umur Piutang",
                        "Terapkan Tanggal Acuan (Enter)",
                        440.0,
                    ),
                    _ => ("Form Dialog", "Simpan (Enter)", 460.0),
                };

                let dialog = ModalDialog::new(title, width);
                let active_idx = self.form_active;

                let body = div()
                    .flex()
                    .flex_col()
                    .gap(px(10.0))
                    .children(
                        self.form_fields
                            .iter()
                            .enumerate()
                            .map(|(idx, (label, _val))| {
                                let is_active = idx == active_idx;

                                div()
                                    .id(("modal-field", idx))
                                    .flex()
                                    .flex_col()
                                    .gap(px(3.0))
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.form_active = idx;
                                        cx.notify();
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .font_weight(if is_active {
                                                FontWeight::BOLD
                                            } else {
                                                FontWeight::SEMIBOLD
                                            })
                                            .text_color(rgb(if is_active {
                                                Theme::PRIMARY
                                            } else {
                                                Theme::TEXT_SECONDARY
                                            }))
                                            .child(format!(
                                                "{}{}",
                                                *label,
                                                if is_active { " ◀" } else { "" }
                                            )),
                                    )
                                    .child(
                                        if let Some(slot) = (0..self.date_pickers.len())
                                            .find(|slot| self.date_field_index(*slot) == Some(idx))
                                        {
                                            DatePicker::new(&self.date_pickers[slot])
                                                .placeholder("Pilih tanggal")
                                                .cleanable(matches!(
                                                    modal,
                                                    ActiveModal::FilterLedgerDate
                                                ))
                                                .into_any_element()
                                        } else {
                                            Input::new(&self.customer_form_inputs[idx])
                                                .disabled(self.is_loading)
                                                .into_any_element()
                                        },
                                    )
                            }),
                    )
                    .when(!self.form_error.is_empty(), |d| {
                        d.child(
                            div()
                                .p(px(8.0))
                                .bg(rgb(Theme::BADGE_DEBIT_BG))
                                .border_1()
                                .border_color(rgb(Theme::BADGE_DEBIT_BORDER))
                                .rounded(px(3.0))
                                .text_size(px(11.5))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(Theme::BADGE_DEBIT_TEXT))
                                .child(self.form_error.clone()),
                        )
                    })
                    .child(
                        div()
                            .text_size(px(10.5))
                            .text_color(rgb(Theme::TEXT_LIGHT))
                            .child(
                                "Pintasan: [Tab] Pindah Kolom  •  [Enter] Eksekusi  •  [Esc] Batal",
                            ),
                    )
                    .into_any_element();

                let footer = self.render_modal_footer(submit_label, cx);
                Some(dialog.frame(body, footer))
            }
        }
    }

    fn render_modal_footer(&self, submit_label: &'static str, cx: &mut Context<Self>) -> Div {
        let is_loading = self.is_loading;
        let submit_text = if is_loading {
            "Memproses ke Server..."
        } else {
            submit_label
        };

        div()
            .h(px(54.0))
            .bg(rgb(Theme::BG_HEADER))
            .border_t_1()
            .border_color(rgb(Theme::BORDER_DEFAULT))
            .px(px(14.0))
            .flex()
            .items_center()
            .justify_end()
            .gap(px(8.0))
            .child(
                Button::new("btn-modal-cancel")
                    .outline()
                    .label("Batal")
                    .when(self.active_modal == Some(ActiveModal::RecordSale), |b| {
                        b.track_focus(&self.sale_action_focus[2])
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        if !this.is_loading {
                            this.active_modal = None;
                            cx.notify();
                        }
                    })),
            )
            .child(
                Button::new("btn-modal-submit")
                    .primary()
                    .label(submit_text)
                    .when(self.active_modal == Some(ActiveModal::RecordSale), |b| {
                        b.track_focus(&self.sale_action_focus[1])
                    })
                    .disabled(
                        is_loading
                            || (self.active_modal == Some(ActiveModal::ApplyDeposit)
                                && self.transaction_uncertain),
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        if !this.is_loading {
                            this.submit_modal(cx);
                        }
                    })),
            )
    }
}
