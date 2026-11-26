//! Keuangan Toko - Native Desktop Application (GPUI)
#![allow(clippy::get_first, clippy::too_many_arguments)]
//! Implementasi aplikasi pencatatan keuangan/piutang toko dengan antarmuka native desktop sejati:
//! - Menu Bar, Quick Action Toolbar, 3-panel Status Bar
//! - Native DBGrid dengan grid lines tajam, pemilihan baris Windows Blue, format angka tabular rata kanan
//! - Navigasi keyboard responsif (F2-F8, Ctrl+N/P/L, Esc, Enter, Tab)
//! - Beroperasi melalui TokoStoreAdapter dengan in-memory MockDataAdapter
//! - Fitur lengkap: Direktori Pelanggan, Buku Piutang & Riwayat Pembayaran, Penjualan Kredit Instan Multi-Item,
//!   Pembayaran FIFO, Penerapan Saldo Kredit (Deposit), Laporan Aging & Drill-down, Manajemen Pengguna

mod mock;
pub mod store;
pub mod ui;

use chrono::{Local, NaiveDate};
use gpui::{
    App, Application, Bounds, Context, Div, FocusHandle, FontWeight, KeyDownEvent, Window,
    WindowBounds, WindowOptions, div, prelude::*, px, rgb, size,
};
use store::mock_store::MockDataAdapter;
use store::*;
use ui::db_grid::{
    ColumnAlign, GridColumn, grid_badge, grid_cell, render_empty_state, render_grid_header,
};
use ui::menu_bar::render_menu_bar_container;
use ui::modal::ModalDialog;
use ui::status_bar::StatusBar;
use ui::theme::{Theme, format_idr};
use ui::tool_bar::{render_toolbar_container, tool_button, tool_separator};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentView {
    CustomerList,
    CustomerDetail,
    AgingReport,
    UserManagement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustomerTab {
    Ledger,
    Payments,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveModal {
    Login,
    ChangePassword,
    NewCustomer,
    EditCustomer(String),
    RecordSale,
    RecordPayment,
    ApplyDeposit,
    ReverseTransaction {
        customer_id: String,
        entry_id: String,
    },
    NewUser,
    ChangeAsOfDate,
    Notice {
        title: &'static str,
        message: String,
    },
}

pub struct Desktop {
    pub store: MockDataAdapter,
    pub session: Option<SessionUser>,
    pub current_view: CurrentView,
    pub selected_customer_id: Option<String>,
    pub customer_tab: CustomerTab,
    pub query: String,
    pub status_msg: String,
    pub active_modal: Option<ActiveModal>,
    pub form_fields: Vec<(&'static str, String)>,
    pub form_active: usize,
    pub form_error: String,
    pub sale_items: Vec<(String, String, String)>, // (deskripsi, qty, harga)
    pub sale_active_field: (usize, usize),         // (baris, kolom: 0=desc, 1=qty, 2=price)
    pub aging_as_of: NaiveDate,
    pub aging_selected_bucket: Option<usize>,
    pub focus: FocusHandle,
}

impl Desktop {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let store = MockDataAdapter::new();
        let session = store.current_session();
        let today = Local::now().date_naive();

        Self {
            store,
            session,
            current_view: CurrentView::CustomerList,
            selected_customer_id: None,
            customer_tab: CustomerTab::Ledger,
            query: String::new(),
            status_msg: "Sistem siap. Gunakan mouse atau tombol pintas keyboard (F2-F8).".into(),
            active_modal: None,
            form_fields: Vec::new(),
            form_active: 0,
            form_error: String::new(),
            sale_items: vec![("".into(), "1".into(), "".into())],
            sale_active_field: (0, 0),
            aging_as_of: today,
            aging_selected_bucket: None,
            focus: cx.focus_handle(),
        }
    }

    pub fn today_jakarta(&self) -> NaiveDate {
        Local::now().date_naive()
    }

    pub fn can_write(&self) -> bool {
        self.session
            .as_ref()
            .map(|u| u.role == UserRole::Owner || u.role == UserRole::Operator)
            .unwrap_or(false)
    }

    pub fn is_owner(&self) -> bool {
        self.session
            .as_ref()
            .map(|u| u.role == UserRole::Owner)
            .unwrap_or(false)
    }

    // Helper pembuka dialog modal
    pub fn open_new_customer_modal(&mut self) {
        if !self.can_write() {
            self.status_msg =
                "Akses ditolak: Hanya Owner dan Operator yang dapat menambah pelanggan.".into();
            return;
        }
        self.form_fields = vec![
            ("Nama Pelanggan (Wajib)", String::new()),
            ("Kode Pelanggan (Opsional)", String::new()),
            ("Alamat Lengkap (Opsional)", String::new()),
            ("Nomor Telepon / HP (Opsional)", String::new()),
        ];
        self.form_active = 0;
        self.form_error.clear();
        self.active_modal = Some(ActiveModal::NewCustomer);
    }

    pub fn open_record_sale_modal(&mut self) {
        if !self.can_write() {
            self.status_msg =
                "Akses ditolak: Hanya Owner dan Operator yang dapat mencatat penjualan.".into();
            return;
        }
        if self.selected_customer_id.is_none() {
            self.status_msg =
                "Pilih pelanggan terlebih dahulu untuk mencatat penjualan kredit.".into();
            return;
        }
        self.form_fields = vec![
            (
                "Tanggal Transaksi (YYYY-MM-DD)",
                self.today_jakarta().to_string(),
            ),
            (
                "Alasan Tanggal Lampau (Wajib jika masa lalu)",
                String::new(),
            ),
        ];
        self.sale_items = vec![("".into(), "1".into(), "".into())];
        self.sale_active_field = (0, 0);
        self.form_active = 0;
        self.form_error.clear();
        self.active_modal = Some(ActiveModal::RecordSale);
    }

    pub fn open_record_payment_modal(&mut self) {
        if !self.can_write() {
            self.status_msg =
                "Akses ditolak: Hanya Owner dan Operator yang dapat mencatat pembayaran.".into();
            return;
        }
        if self.selected_customer_id.is_none() {
            self.status_msg = "Pilih pelanggan terlebih dahulu untuk mencatat pembayaran.".into();
            return;
        }
        self.form_fields = vec![
            (
                "Tanggal Pembayaran (YYYY-MM-DD)",
                self.today_jakarta().to_string(),
            ),
            ("Jumlah Pembayaran Rupiah (Contoh: 150000)", String::new()),
            (
                "Metode Pembayaran (Tunai / Transfer Bank / QRIS)",
                "Tunai".into(),
            ),
            ("Nomor Referensi Transfer / Bukti (Opsional)", String::new()),
            ("Catatan Tambahan (Opsional)", String::new()),
            (
                "Alasan Tanggal Lampau (Wajib jika masa lalu)",
                String::new(),
            ),
        ];
        self.form_active = 1;
        self.form_error.clear();
        self.active_modal = Some(ActiveModal::RecordPayment);
    }

    pub fn open_apply_deposit_modal(&mut self) {
        if !self.can_write() {
            self.status_msg =
                "Akses ditolak: Hanya Owner dan Operator yang dapat menerapkan saldo kredit."
                    .into();
            return;
        }
        let Some(cid) = &self.selected_customer_id else {
            self.status_msg = "Pilih pelanggan terlebih dahulu.".into();
            return;
        };

        let payments = self.store.list_payments(cid).unwrap_or_default();
        let ledger = self
            .store
            .get_ledger(
                cid,
                &LedgerQuery {
                    from: None,
                    to: None,
                    order_desc: false,
                },
            )
            .unwrap_or_default();

        let unallocated_payments: Vec<_> = payments
            .iter()
            .filter(|p| p.unallocated_credit_idr > 0)
            .collect();
        let open_debits: Vec<_> = ledger
            .iter()
            .filter(|e| e.debit_idr > 0 && e.remaining_idr > 0 && !e.is_reversed)
            .collect();

        if unallocated_payments.is_empty() {
            self.active_modal = Some(ActiveModal::Notice {
                title: "Saldo Kredit Kosong",
                message: "Pelanggan ini tidak memiliki saldo pembayaran lebih (deposit) yang dapat diterapkan.".into(),
            });
            return;
        }

        if open_debits.is_empty() {
            self.active_modal = Some(ActiveModal::Notice {
                title: "Tidak Ada Nota Terbuka",
                message: "Semua nota debet pelanggan sudah lunas. Saldo kredit tetap tersimpan sebagai deposit.".into(),
            });
            return;
        }

        let first_pay = unallocated_payments[0];
        let first_deb = open_debits[0];
        let max_app = first_pay
            .unallocated_credit_idr
            .min(first_deb.remaining_idr);

        self.form_fields = vec![
            ("ID Pembayaran Sumber", first_pay.id.clone()),
            ("ID Nota Debet Tujuan", first_deb.id.clone()),
            ("Jumlah Penerapan (Rp)", max_app.to_string()),
            (
                "Alasan Penerapan (Wajib diisi)",
                "Penerapan saldo deposit pelanggan ke nota terbuka".into(),
            ),
        ];
        self.form_active = 2;
        self.form_error.clear();
        self.active_modal = Some(ActiveModal::ApplyDeposit);
    }

    pub fn open_reverse_modal(&mut self, entry_id: &str) {
        if !self.can_write() {
            self.status_msg =
                "Akses ditolak: Hanya Owner dan Operator yang dapat membatalkan mutasi.".into();
            return;
        }
        let Some(cid) = &self.selected_customer_id else {
            return;
        };
        self.form_fields = vec![(
            "Alasan Koreksi / Pembatalan (Wajib)",
            "Salah input transaksi".into(),
        )];
        self.form_active = 0;
        self.form_error.clear();
        self.active_modal = Some(ActiveModal::ReverseTransaction {
            customer_id: cid.clone(),
            entry_id: entry_id.to_string(),
        });
    }

    pub fn open_change_password_modal(&mut self) {
        self.form_fields = vec![
            ("Kata Sandi Saat Ini", String::new()),
            ("Kata Sandi Baru (Min. 8 karakter)", String::new()),
            ("Konfirmasi Kata Sandi Baru", String::new()),
        ];
        self.form_active = 0;
        self.form_error.clear();
        self.active_modal = Some(ActiveModal::ChangePassword);
    }

    pub fn open_login_modal(&mut self) {
        self.form_fields = vec![
            ("Username Akun", "azka".into()),
            ("Kata Sandi", "owner123".into()),
        ];
        self.form_active = 1;
        self.form_error.clear();
        self.active_modal = Some(ActiveModal::Login);
    }

    pub fn open_new_user_modal(&mut self) {
        if !self.is_owner() {
            self.status_msg =
                "Akses ditolak: Hanya Owner yang dapat menambah akun pengguna.".into();
            return;
        }
        self.form_fields = vec![
            ("Username (Huruf kecil, tanpa spasi)", String::new()),
            ("Nama Lengkap", String::new()),
            ("Peran Akun (Owner / Operator / Viewer)", "Operator".into()),
            ("Kata Sandi Awal", "password123".into()),
        ];
        self.form_active = 0;
        self.form_error.clear();
        self.active_modal = Some(ActiveModal::NewUser);
    }

    pub fn open_change_as_of_modal(&mut self) {
        self.form_fields = vec![(
            "Tanggal Acuan Laporan Aging (YYYY-MM-DD)",
            self.aging_as_of.to_string(),
        )];
        self.form_active = 0;
        self.form_error.clear();
        self.active_modal = Some(ActiveModal::ChangeAsOfDate);
    }

    // Submit form modal aktif
    pub fn submit_modal(&mut self, cx: &mut Context<Self>) {
        let Some(modal) = self.active_modal.clone() else {
            return;
        };
        let fields: Vec<String> = self
            .form_fields
            .iter()
            .map(|(_, val)| val.trim().to_string())
            .collect();

        match modal {
            ActiveModal::Notice { .. } => {
                self.active_modal = None;
            }
            ActiveModal::Login => {
                let username = fields.get(0).cloned().unwrap_or_default();
                let password = fields.get(1).cloned().unwrap_or_default();
                match self.store.login(&username, &password) {
                    Ok(user) => {
                        let must_change = user.must_change_password;
                        self.session = Some(user);
                        self.active_modal = None;
                        self.status_msg = format!("Berhasil masuk sebagai {}.", username);
                        if must_change {
                            self.open_change_password_modal();
                        }
                    }
                    Err(e) => {
                        self.form_error = e.0;
                    }
                }
            }
            ActiveModal::ChangePassword => {
                let current = fields.get(0).cloned().unwrap_or_default();
                let new_pw = fields.get(1).cloned().unwrap_or_default();
                let conf_pw = fields.get(2).cloned().unwrap_or_default();

                if new_pw.len() < 8 {
                    self.form_error = "Kata sandi baru minimal 8 karakter.".into();
                    return;
                }
                if new_pw != conf_pw {
                    self.form_error = "Konfirmasi kata sandi tidak cocok.".into();
                    return;
                }
                match self.store.change_password(&current, &new_pw) {
                    Ok(_) => {
                        if let Some(s) = &mut self.session {
                            s.must_change_password = false;
                        }
                        self.active_modal = Some(ActiveModal::Notice {
                            title: "Kata Sandi Berhasil Diperbarui",
                            message: "Kata sandi akun Anda telah sukses diubah.".into(),
                        });
                        self.status_msg = "Kata sandi telah diperbarui.".into();
                    }
                    Err(e) => {
                        self.form_error = e.0;
                    }
                }
            }
            ActiveModal::NewCustomer => {
                let name = fields.get(0).cloned().unwrap_or_default();
                let code = fields.get(1).filter(|s| !s.is_empty()).cloned();
                let address = fields.get(2).filter(|s| !s.is_empty()).cloned();
                let phone = fields.get(3).filter(|s| !s.is_empty()).cloned();

                let input = CreateCustomerInput {
                    name,
                    code,
                    address,
                    phone,
                };
                match self.store.create_customer(&input) {
                    Ok(detail) => {
                        self.selected_customer_id = Some(detail.id.clone());
                        self.current_view = CurrentView::CustomerDetail;
                        self.active_modal = None;
                        self.status_msg =
                            format!("Pelanggan {} berhasil ditambahkan.", detail.name);
                    }
                    Err(e) => {
                        self.form_error = e.0;
                    }
                }
            }
            ActiveModal::EditCustomer(id) => {
                let name = fields.get(0).cloned().unwrap_or_default();
                let code = fields.get(1).filter(|s| !s.is_empty()).cloned();
                let address = fields.get(2).filter(|s| !s.is_empty()).cloned();
                let phone = fields.get(3).filter(|s| !s.is_empty()).cloned();

                let input = UpdateCustomerInput {
                    name,
                    code,
                    address,
                    phone,
                    active: true,
                };
                match self.store.update_customer(&id, &input) {
                    Ok(_) => {
                        self.active_modal = None;
                        self.status_msg = "Data pelanggan berhasil diperbarui.".into();
                    }
                    Err(e) => {
                        self.form_error = e.0;
                    }
                }
            }
            ActiveModal::RecordSale => {
                let date_str = fields.get(0).cloned().unwrap_or_default();
                let reason_str = fields.get(1).cloned().unwrap_or_default();

                let effective_date = match NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                    Ok(d) => d,
                    Err(_) => {
                        self.form_error = "Format tanggal salah. Gunakan YYYY-MM-DD.".into();
                        return;
                    }
                };

                let today = self.today_jakarta();
                if effective_date > today {
                    self.form_error = "Tanggal transaksi tidak boleh di masa depan.".into();
                    return;
                }
                if effective_date < today && reason_str.trim().is_empty() {
                    self.form_error = "Tanggal lampau wajib menyertakan alasan.".into();
                    return;
                }

                let mut items = Vec::new();
                for (desc, qty, pr) in &self.sale_items {
                    if !desc.trim().is_empty() {
                        let pr_val = match toko_money::rupiah(pr) {
                            Some(val) if val > 0 => val,
                            _ => {
                                self.form_error =
                                    format!("Harga satuan untuk item '{}' tidak valid.", desc);
                                return;
                            }
                        };
                        items.push(SaleItemInput {
                            description: desc.trim().to_string(),
                            quantity: if qty.trim().is_empty() {
                                "1".into()
                            } else {
                                qty.trim().to_string()
                            },
                            unit_price_idr: pr_val,
                        });
                    }
                }

                if items.is_empty() {
                    self.form_error = "Minimal harus ada satu baris barang yang terisi.".into();
                    return;
                }

                let input = InstantSaleInput {
                    effective_date,
                    reference_no: None,
                    items,
                };

                let cid = self.selected_customer_id.clone().unwrap_or_default();
                match self.store.create_instant_sale(&cid, &input) {
                    Ok(total) => {
                        self.active_modal = None;
                        self.status_msg = format!(
                            "Penjualan kredit Rp {} berhasil dicatat.",
                            format_idr(total)
                        );
                    }
                    Err(e) => {
                        self.form_error = e.0;
                    }
                }
            }
            ActiveModal::RecordPayment => {
                let date_str = fields.get(0).cloned().unwrap_or_default();
                let amount_str = fields.get(1).cloned().unwrap_or_default();
                let method = fields.get(2).cloned().unwrap_or_default();
                let reference = fields.get(3).filter(|s| !s.is_empty()).cloned();
                let note = fields.get(4).filter(|s| !s.is_empty()).cloned();
                let reason_str = fields.get(5).cloned().unwrap_or_default();

                let received_date = match NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                    Ok(d) => d,
                    Err(_) => {
                        self.form_error = "Format tanggal salah. Gunakan YYYY-MM-DD.".into();
                        return;
                    }
                };

                let today = self.today_jakarta();
                if received_date > today {
                    self.form_error = "Tanggal pembayaran tidak boleh di masa depan.".into();
                    return;
                }
                if received_date < today && reason_str.trim().is_empty() {
                    self.form_error = "Tanggal lampau wajib menyertakan alasan.".into();
                    return;
                }

                let amount_idr = match toko_money::rupiah(&amount_str) {
                    Some(val) if val > 0 => val,
                    _ => {
                        self.form_error = "Nominal pembayaran harus berupa angka positif.".into();
                        return;
                    }
                };

                let input = RecordPaymentInput {
                    received_date,
                    amount_idr,
                    method,
                    reference,
                    note,
                };

                let cid = self.selected_customer_id.clone().unwrap_or_default();
                match self.store.record_payment(&cid, &input) {
                    Ok(payment) => {
                        self.active_modal = None;
                        self.status_msg = format!(
                            "Pembayaran Rp {} berhasil dicatat (Teralokasi: Rp {}, Deposit: Rp {}).",
                            format_idr(payment.amount_idr),
                            format_idr(payment.allocated_idr),
                            format_idr(payment.unallocated_credit_idr)
                        );
                    }
                    Err(e) => {
                        self.form_error = e.0;
                    }
                }
            }
            ActiveModal::ApplyDeposit => {
                let payment_id = fields.get(0).cloned().unwrap_or_default();
                let debit_entry_id = fields.get(1).cloned().unwrap_or_default();
                let amount_str = fields.get(2).cloned().unwrap_or_default();
                let reason = fields.get(3).cloned().unwrap_or_default();

                let amount_idr = match toko_money::rupiah(&amount_str) {
                    Some(val) if val > 0 => val,
                    _ => {
                        self.form_error = "Nominal penerapan harus berupa angka positif.".into();
                        return;
                    }
                };

                let input = ApplyCreditInput {
                    payment_id,
                    debit_entry_id,
                    amount_idr,
                    reason,
                };

                let cid = self.selected_customer_id.clone().unwrap_or_default();
                match self.store.apply_credit(&cid, &input) {
                    Ok(()) => {
                        self.active_modal = None;
                        self.status_msg = format!(
                            "Saldo kredit Rp {} sukses diterapkan.",
                            format_idr(amount_idr)
                        );
                    }
                    Err(e) => {
                        self.form_error = e.0;
                    }
                }
            }
            ActiveModal::ReverseTransaction {
                customer_id,
                entry_id,
            } => {
                let reason = fields.get(0).cloned().unwrap_or_default();
                match self
                    .store
                    .reverse_transaction(&customer_id, &entry_id, &reason)
                {
                    Ok(_) => {
                        self.active_modal = None;
                        self.status_msg = "Mutasi buku piutang telah dibatalkan/dibalik.".into();
                    }
                    Err(e) => {
                        self.form_error = e.0;
                    }
                }
            }
            ActiveModal::NewUser => {
                let username = fields.get(0).cloned().unwrap_or_default();
                let full_name = fields.get(1).cloned().unwrap_or_default();
                let role_str = fields.get(2).cloned().unwrap_or_default();
                let temp_pw = fields.get(3).cloned().unwrap_or_default();

                let role = match role_str.to_lowercase().as_str() {
                    "owner" => UserRole::Owner,
                    "viewer" => UserRole::Viewer,
                    _ => UserRole::Operator,
                };

                let input = CreateUserInput {
                    username,
                    full_name,
                    role,
                    temporary_password: temp_pw,
                };

                match self.store.create_user(&input) {
                    Ok(u) => {
                        self.active_modal = None;
                        self.status_msg = format!(
                            "Pengguna {} ({}) berhasil didaftarkan.",
                            u.username, u.full_name
                        );
                    }
                    Err(e) => {
                        self.form_error = e.0;
                    }
                }
            }
            ActiveModal::ChangeAsOfDate => {
                let date_str = fields.get(0).cloned().unwrap_or_default();
                match NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                    Ok(d) => {
                        self.aging_as_of = d;
                        self.active_modal = None;
                        self.status_msg = format!("Tanggal acuan laporan aging diubah ke {}.", d);
                    }
                    Err(_) => {
                        self.form_error = "Format tanggal harus YYYY-MM-DD.".into();
                    }
                }
            }
        }

        cx.notify();
    }

    // Penanganan Event Keyboard Global
    fn key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke;

        // Jika modal sedang terbuka
        if let Some(modal) = &self.active_modal {
            if key.key == "escape" {
                self.active_modal = None;
                cx.notify();
                return;
            }

            if key.key == "tab" && !self.form_fields.is_empty() {
                self.form_active = (self.form_active + 1) % self.form_fields.len();
                cx.notify();
                return;
            }

            if key.key == "enter" {
                self.submit_modal(cx);
                return;
            }

            // Input teks ke modal
            if matches!(modal, ActiveModal::RecordSale) {
                let (row, col) = self.sale_active_field;
                if row < self.sale_items.len() {
                    let target = match col {
                        0 => &mut self.sale_items[row].0,
                        1 => &mut self.sale_items[row].1,
                        _ => &mut self.sale_items[row].2,
                    };
                    if key.key == "backspace" {
                        target.pop();
                    } else if let Some(ch) = &key.key_char {
                        if !ch.chars().any(char::is_control) && target.len() + ch.len() <= 60 {
                            target.push_str(ch);
                        }
                    } else if key.key == "space" {
                        target.push(' ');
                    }
                }
            } else if !self.form_fields.is_empty() && self.form_active < self.form_fields.len() {
                let target = &mut self.form_fields[self.form_active].1;
                if key.key == "backspace" {
                    target.pop();
                } else if let Some(ch) = &key.key_char {
                    if !ch.chars().any(char::is_control) && target.len() + ch.len() <= 80 {
                        target.push_str(ch);
                    }
                } else if key.key == "space" {
                    target.push(' ');
                }
            }

            cx.notify();
            return;
        }

        // Navigasi & Pintasan Keyboard Utama (F2 - F8, Ctrl+N/P/L, Esc)
        if key.modifiers.control && key.key == "n" {
            self.open_new_customer_modal();
            cx.notify();
            return;
        }
        if key.modifiers.control && key.key == "p" {
            self.open_change_password_modal();
            cx.notify();
            return;
        }
        if key.modifiers.control && key.key == "l" {
            self.open_login_modal();
            cx.notify();
            return;
        }

        match key.key.as_str() {
            "f2" => {
                self.current_view = CurrentView::CustomerList;
                self.status_msg = "Tampilan Direktori Pelanggan.".into();
                cx.notify();
            }
            "f3" => {
                self.open_record_sale_modal();
                cx.notify();
            }
            "f4" => {
                self.open_record_payment_modal();
                cx.notify();
            }
            "f5" => {
                self.open_apply_deposit_modal();
                cx.notify();
            }
            "f6" => {
                self.current_view = CurrentView::AgingReport;
                self.status_msg = "Laporan Analisis Umur Piutang (Aging).".into();
                cx.notify();
            }
            "f8" => {
                if self.is_owner() {
                    self.current_view = CurrentView::UserManagement;
                    self.status_msg = "Manajemen Pengguna Aplikasi.".into();
                    cx.notify();
                } else {
                    self.status_msg = "Akses ditolak: Menu ini khusus Owner.".into();
                    cx.notify();
                }
            }
            "escape" => {
                if self.current_view != CurrentView::CustomerList {
                    self.current_view = CurrentView::CustomerList;
                    self.status_msg = "Kembali ke Direktori Pelanggan.".into();
                } else if !self.query.is_empty() {
                    self.query.clear();
                    self.status_msg = "Pencarian dibersihkan.".into();
                }
                cx.notify();
            }
            "backspace" => {
                if self.current_view == CurrentView::CustomerList {
                    self.query.pop();
                    cx.notify();
                }
            }
            _ => {
                if self.current_view == CurrentView::CustomerList {
                    if let Some(ch) = &key.key_char {
                        if !ch.chars().any(char::is_control) && self.query.len() + ch.len() <= 60 {
                            self.query.push_str(ch);
                            cx.notify();
                        }
                    } else if key.key == "space" {
                        self.query.push(' ');
                        cx.notify();
                    }
                }
            }
        }
    }
}

impl Render for Desktop {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_owner = self.is_owner();

        let user_label = match &self.session {
            Some(u) => format!("Operator: {} ({:?})", u.username, u.role),
            None => "Belum Masuk (Anonim)".into(),
        };

        let date_label = format!("Tanggal: {} | WIB", self.today_jakarta().format("%d %b %Y"));

        // Layout aplikasi desktop klasik
        div()
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key_down))
            .size_full()
            .bg(rgb(Theme::BG_WINDOW))
            .flex()
            .flex_col()
            .overflow_hidden()
            // 1. Native Menu Bar Klasik
            .child(
                render_menu_bar_container()
                    .child(
                        div()
                            .id("menu-berkas")
                            .px(px(8.0))
                            .py(px(2.0))
                            .rounded(px(2.0))
                            .cursor_pointer()
                            .hover(|d| d.bg(rgb(Theme::BG_ROW_HOVER)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_login_modal();
                                cx.notify();
                            }))
                            .child("Berkas (Ctrl+L)"),
                    )
                    .child(
                        div()
                            .id("menu-pelanggan")
                            .px(px(8.0))
                            .py(px(2.0))
                            .rounded(px(2.0))
                            .cursor_pointer()
                            .hover(|d| d.bg(rgb(Theme::BG_ROW_HOVER)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.current_view = CurrentView::CustomerList;
                                cx.notify();
                            }))
                            .child("Pelanggan (F2)"),
                    )
                    .child(
                        div()
                            .id("menu-transaksi")
                            .px(px(8.0))
                            .py(px(2.0))
                            .rounded(px(2.0))
                            .cursor_pointer()
                            .hover(|d| d.bg(rgb(Theme::BG_ROW_HOVER)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_record_sale_modal();
                                cx.notify();
                            }))
                            .child("Transaksi (F3)"),
                    )
                    .child(
                        div()
                            .id("menu-laporan")
                            .px(px(8.0))
                            .py(px(2.0))
                            .rounded(px(2.0))
                            .cursor_pointer()
                            .hover(|d| d.bg(rgb(Theme::BG_ROW_HOVER)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.current_view = CurrentView::AgingReport;
                                cx.notify();
                            }))
                            .child("Laporan (F6)"),
                    )
                    .when(is_owner, |d| {
                        d.child(
                            div()
                                .id("menu-pengaturan")
                                .px(px(8.0))
                                .py(px(2.0))
                                .rounded(px(2.0))
                                .cursor_pointer()
                                .hover(|d| d.bg(rgb(Theme::BG_ROW_HOVER)))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.current_view = CurrentView::UserManagement;
                                    cx.notify();
                                }))
                                .child("Pengaturan (F8)"),
                        )
                    }),
            )
            // 2. Action Toolbar Cepat Klasik
            .child(
                render_toolbar_container()
                    .child(
                        tool_button("+ Pelanggan", "Ctrl+N")
                            .id("tool-add-cust")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_new_customer_modal();
                                cx.notify();
                            })),
                    )
                    .child(
                        tool_button("+ Belanja Kredit", "F3")
                            .id("tool-sale")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_record_sale_modal();
                                cx.notify();
                            })),
                    )
                    .child(
                        tool_button("+ Pembayaran", "F4")
                            .id("tool-pay")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_record_payment_modal();
                                cx.notify();
                            })),
                    )
                    .child(
                        tool_button("Terapkan Saldo", "F5")
                            .id("tool-deposit")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_apply_deposit_modal();
                                cx.notify();
                            })),
                    )
                    .child(tool_separator())
                    .child(
                        tool_button("Daftar Piutang", "F2")
                            .id("tool-cust-list")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.current_view = CurrentView::CustomerList;
                                cx.notify();
                            })),
                    )
                    .child(
                        tool_button("Laporan Aging", "F6")
                            .id("tool-aging")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.current_view = CurrentView::AgingReport;
                                cx.notify();
                            })),
                    )
                    .when(is_owner, |d| {
                        d.child(tool_separator()).child(
                            tool_button("Manajemen Pengguna", "F8")
                                .id("tool-users")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.current_view = CurrentView::UserManagement;
                                    cx.notify();
                                })),
                        )
                    }),
            )
            // 3. Area Konten Utama
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child(match self.current_view {
                        CurrentView::CustomerList => self.render_customer_list_view(cx),
                        CurrentView::CustomerDetail => self.render_customer_detail_view(cx),
                        CurrentView::AgingReport => self.render_aging_report_view(cx),
                        CurrentView::UserManagement => self.render_user_management_view(cx),
                    }),
            )
            // 4. 3-Panel Status Bar Klasik
            .child(StatusBar::new(&self.status_msg, &date_label, &user_label).render())
            // 5. Modal Dialog Overlay
            .children(self.render_modal_overlay(cx))
    }
}

// Implementasi Render View Helpers
impl Desktop {
    fn render_customer_list_view(&self, cx: &mut Context<Self>) -> Div {
        let q = CustomerQuery {
            search: if self.query.is_empty() {
                None
            } else {
                Some(self.query.clone())
            },
            active_only: false,
        };
        let customers = self.store.list_customers(&q);
        let total_customers = customers.len();
        let total_piutang: i64 = customers.iter().map(|c| c.total_outstanding_idr).sum();
        let total_deposit: i64 = customers.iter().map(|c| c.customer_credit_idr).sum();

        let columns = vec![
            GridColumn {
                title: "Kode",
                width_px: 85.0,
                align: ColumnAlign::Center,
            },
            GridColumn {
                title: "Nama Pelanggan",
                width_px: 260.0,
                align: ColumnAlign::Left,
            },
            GridColumn {
                title: "Alamat / Wilayah",
                width_px: 280.0,
                align: ColumnAlign::Left,
            },
            GridColumn {
                title: "Total Piutang (Rp)",
                width_px: 155.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "Saldo Kredit (Rp)",
                width_px: 155.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "Status",
                width_px: 95.0,
                align: ColumnAlign::Center,
            },
        ];

        div()
            .flex_1()
            .flex()
            .flex_col()
            .h_full()
            .bg(rgb(Theme::BG_WINDOW))
            .p(px(10.0))
            .gap(px(8.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .child(self.kpi_card(
                        "Total Pelanggan Terdaftar",
                        format!("{} Pelanggan", total_customers),
                        Theme::TEXT_PRIMARY,
                    ))
                    .child(self.kpi_card(
                        "Total Piutang Berjalan",
                        format!("Rp {}", format_idr(total_piutang)),
                        Theme::COLOR_DEBIT,
                    ))
                    .child(self.kpi_card(
                        "Total Saldo Kredit (Deposit)",
                        format!("Rp {}", format_idr(total_deposit)),
                        Theme::COLOR_DEPOSIT,
                    )),
            )
            .child(
                div()
                    .h(px(34.0))
                    .bg(rgb(Theme::BG_SURFACE))
                    .border_1()
                    .border_color(rgb(Theme::GRID_LINE))
                    .px(px(8.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(Theme::TEXT_SECONDARY))
                                    .child("Cari Pelanggan:"),
                            )
                            .child(
                                div()
                                    .w(px(300.0))
                                    .h(px(24.0))
                                    .bg(rgb(0xFFFFFF))
                                    .border_1()
                                    .border_color(rgb(Theme::BORDER_DARK))
                                    .px(px(6.0))
                                    .flex()
                                    .items_center()
                                    .text_size(px(12.0))
                                    .child(if self.query.is_empty() {
                                        div()
                                            .text_color(rgb(Theme::TEXT_MUTED))
                                            .child("Ketik kode / nama (Esc untuk bersihkan)...")
                                    } else {
                                        div()
                                            .text_color(rgb(Theme::TEXT_PRIMARY))
                                            .child(self.query.clone())
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .id("btn-add-cust-list")
                            .h(px(24.0))
                            .px(px(10.0))
                            .bg(rgb(Theme::BTN_PRIMARY_BG))
                            .rounded(px(2.0))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .hover(|d| d.bg(rgb(0x106EBE)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_new_customer_modal();
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(rgb(0xFFFFFF))
                                    .child("+ Tambah Pelanggan Baru (Ctrl+N)"),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .bg(rgb(Theme::BG_SURFACE))
                    .border_1()
                    .border_color(rgb(Theme::GRID_LINE))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(render_grid_header(&columns))
                    .child(div().id("cust-list-scroll").flex_1().children(
                        if customers.is_empty() {
                            vec![
                                render_empty_state(
                                    "Tidak ada data pelanggan yang sesuai kriteria pencarian.",
                                )
                                .into_any_element(),
                            ]
                        } else {
                            customers
                                .into_iter()
                                .enumerate()
                                .map(|(idx, cust)| {
                                    let is_selected =
                                        self.selected_customer_id.as_deref() == Some(&cust.id);
                                    let bg_color = if is_selected {
                                        Theme::BG_ROW_SELECTED
                                    } else if idx % 2 == 1 {
                                        Theme::BG_ROW_ALT
                                    } else {
                                        Theme::BG_SURFACE
                                    };

                                    let cust_id = cust.id.clone();
                                    let cust_name = cust.name.clone();
                                    let code_str = cust.code.clone().unwrap_or_else(|| "—".into());
                                    let addr_str =
                                        cust.address.clone().unwrap_or_else(|| "—".into());

                                    div()
                                        .id(("cust-row", idx))
                                        .h(px(26.0))
                                        .bg(rgb(bg_color))
                                        .border_b_1()
                                        .border_color(rgb(Theme::GRID_LINE))
                                        .flex()
                                        .items_center()
                                        .cursor_pointer()
                                        .when(!is_selected, |d| {
                                            d.hover(|d| d.bg(rgb(Theme::BG_ROW_HOVER)))
                                        })
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.selected_customer_id = Some(cust_id.clone());
                                            this.current_view = CurrentView::CustomerDetail;
                                            this.status_msg = format!(
                                                "Membuka buku piutang pelanggan {}.",
                                                cust_name
                                            );
                                            cx.notify();
                                        }))
                                        .child(grid_cell(
                                            code_str,
                                            85.0,
                                            ColumnAlign::Center,
                                            is_selected,
                                        ))
                                        .child(grid_cell(
                                            cust.name,
                                            260.0,
                                            ColumnAlign::Left,
                                            is_selected,
                                        ))
                                        .child(grid_cell(
                                            addr_str,
                                            280.0,
                                            ColumnAlign::Left,
                                            is_selected,
                                        ))
                                        .child(grid_cell(
                                            format_idr(cust.total_outstanding_idr),
                                            155.0,
                                            ColumnAlign::Right,
                                            is_selected,
                                        ))
                                        .child(grid_cell(
                                            format_idr(cust.customer_credit_idr),
                                            155.0,
                                            ColumnAlign::Right,
                                            is_selected,
                                        ))
                                        .child(if cust.active {
                                            grid_badge(
                                                "Aktif",
                                                Theme::COLOR_CREDIT,
                                                0xDCFCE7,
                                                95.0,
                                                is_selected,
                                            )
                                        } else {
                                            grid_badge(
                                                "Nonaktif",
                                                Theme::TEXT_MUTED,
                                                0xF1F5F9,
                                                95.0,
                                                is_selected,
                                            )
                                        })
                                        .into_any_element()
                                })
                                .collect()
                        },
                    )),
            )
    }

    fn render_customer_detail_view(&self, cx: &mut Context<Self>) -> Div {
        let Some(cid) = &self.selected_customer_id else {
            return div().child("Pelanggan belum dipilih.");
        };

        let detail = match self.store.get_customer(cid) {
            Ok(d) => d,
            Err(_) => return div().child("Pelanggan tidak ditemukan."),
        };

        let ledger = self
            .store
            .get_ledger(
                cid,
                &LedgerQuery {
                    from: None,
                    to: None,
                    order_desc: false,
                },
            )
            .unwrap_or_default();
        let payments = self.store.list_payments(cid).unwrap_or_default();

        let detail_code_str = detail.code.clone().unwrap_or_else(|| "—".into());
        let detail_addr_str = detail
            .address
            .clone()
            .unwrap_or_else(|| "Alamat belum tercatat".into());

        div()
            .flex_1()
            .flex()
            .flex_col()
            .h_full()
            .bg(rgb(Theme::BG_WINDOW))
            .p(px(10.0))
            .gap(px(8.0))
            // Header Pelanggan
            .child(
                div()
                    .h(px(40.0))
                    .bg(rgb(Theme::BG_SURFACE))
                    .border_1()
                    .border_color(rgb(Theme::GRID_LINE))
                    .px(px(10.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(10.0))
                            .child(
                                div()
                                    .id("btn-back-to-list")
                                    .h(px(26.0))
                                    .px(px(8.0))
                                    .bg(rgb(Theme::BG_WINDOW))
                                    .border_1()
                                    .border_color(rgb(Theme::BORDER_DARK))
                                    .rounded(px(2.0))
                                    .flex()
                                    .items_center()
                                    .cursor_pointer()
                                    .hover(|d| d.bg(rgb(Theme::BG_ROW_HOVER)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.current_view = CurrentView::CustomerList;
                                        cx.notify();
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(11.5))
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(rgb(Theme::TEXT_PRIMARY))
                                            .child("← Kembali (Esc)"),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(14.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(Theme::TEXT_PRIMARY))
                                    .child(format!("{} ({})", detail.name, detail_code_str)),
                            )
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(rgb(Theme::TEXT_MUTED))
                                    .child(detail_addr_str),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .when(self.can_write(), |d| {
                                d.child(
                                    div()
                                        .id("btn-det-sale")
                                        .h(px(26.0))
                                        .px(px(10.0))
                                        .bg(rgb(Theme::BTN_PRIMARY_BG))
                                        .rounded(px(2.0))
                                        .flex()
                                        .items_center()
                                        .cursor_pointer()
                                        .hover(|d| d.bg(rgb(0x106EBE)))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.open_record_sale_modal();
                                            cx.notify();
                                        }))
                                        .child(
                                            div()
                                                .text_size(px(11.5))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(rgb(0xFFFFFF))
                                                .child("+ Penjualan Kredit (F3)"),
                                        ),
                                )
                                .child(
                                    div()
                                        .id("btn-det-pay")
                                        .h(px(26.0))
                                        .px(px(10.0))
                                        .bg(rgb(Theme::COLOR_CREDIT))
                                        .rounded(px(2.0))
                                        .flex()
                                        .items_center()
                                        .cursor_pointer()
                                        .hover(|d| d.bg(rgb(0x166534)))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.open_record_payment_modal();
                                            cx.notify();
                                        }))
                                        .child(
                                            div()
                                                .text_size(px(11.5))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(rgb(0xFFFFFF))
                                                .child("+ Catat Pembayaran (F4)"),
                                        ),
                                )
                                .child(
                                    div()
                                        .id("btn-det-deposit")
                                        .h(px(26.0))
                                        .px(px(10.0))
                                        .bg(rgb(Theme::COLOR_DEPOSIT))
                                        .rounded(px(2.0))
                                        .flex()
                                        .items_center()
                                        .cursor_pointer()
                                        .hover(|d| d.bg(rgb(0x1E40AF)))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.open_apply_deposit_modal();
                                            cx.notify();
                                        }))
                                        .child(
                                            div()
                                                .text_size(px(11.5))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(rgb(0xFFFFFF))
                                                .child("Terapkan Saldo (F5)"),
                                        ),
                                )
                            }),
                    ),
            )
            // Ringkasan Saldo Pelanggan
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .child(self.balance_card(
                        "Total Piutang Terutang",
                        format!("Rp {}", format_idr(detail.total_outstanding_idr)),
                        "Saldo kewajiban piutang dari buku piutang",
                        Theme::COLOR_DEBIT,
                    ))
                    .child(self.balance_card(
                        "Saldo Kredit Pelanggan (Deposit)",
                        format!("Rp {}", format_idr(detail.customer_credit_idr)),
                        "Kelebihan pembayaran yang dapat diterapkan ke transaksi piutang",
                        Theme::COLOR_DEPOSIT,
                    )),
            )
            // Tab Selector
            .child(
                div()
                    .h(px(30.0))
                    .border_b_1()
                    .border_color(rgb(Theme::GRID_LINE))
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(
                        div()
                            .id("tab-btn-ledger")
                            .h(px(30.0))
                            .px(px(12.0))
                            .bg(rgb(if self.customer_tab == CustomerTab::Ledger {
                                Theme::BG_SURFACE
                            } else {
                                Theme::BG_WINDOW
                            }))
                            .border_t_2()
                            .border_l_1()
                            .border_r_1()
                            .border_color(rgb(if self.customer_tab == CustomerTab::Ledger {
                                Theme::GRID_LINE
                            } else {
                                Theme::BG_WINDOW
                            }))
                            .rounded_t(px(3.0))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.customer_tab = CustomerTab::Ledger;
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .font_weight(if self.customer_tab == CustomerTab::Ledger {
                                        FontWeight::BOLD
                                    } else {
                                        FontWeight::MEDIUM
                                    })
                                    .text_color(rgb(if self.customer_tab == CustomerTab::Ledger {
                                        Theme::TEXT_PRIMARY
                                    } else {
                                        Theme::TEXT_SECONDARY
                                    }))
                                    .child("Buku Piutang (Ledger Mutasi)"),
                            ),
                    )
                    .child(
                        div()
                            .id("tab-btn-payments")
                            .h(px(30.0))
                            .px(px(12.0))
                            .bg(rgb(if self.customer_tab == CustomerTab::Payments {
                                Theme::BG_SURFACE
                            } else {
                                Theme::BG_WINDOW
                            }))
                            .border_t_2()
                            .border_l_1()
                            .border_r_1()
                            .border_color(rgb(if self.customer_tab == CustomerTab::Payments {
                                Theme::GRID_LINE
                            } else {
                                Theme::BG_WINDOW
                            }))
                            .rounded_t(px(3.0))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.customer_tab = CustomerTab::Payments;
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .font_weight(if self.customer_tab == CustomerTab::Payments {
                                        FontWeight::BOLD
                                    } else {
                                        FontWeight::MEDIUM
                                    })
                                    .text_color(rgb(
                                        if self.customer_tab == CustomerTab::Payments {
                                            Theme::TEXT_PRIMARY
                                        } else {
                                            Theme::TEXT_SECONDARY
                                        },
                                    ))
                                    .child("Riwayat Pembayaran"),
                            ),
                    ),
            )
            // Konten Tab
            .child(match self.customer_tab {
                CustomerTab::Ledger => self.render_ledger_grid(&ledger, cx),
                CustomerTab::Payments => self.render_payments_grid(&payments),
            })
    }

    fn render_ledger_grid(&self, ledger: &[LedgerEntry], cx: &mut Context<Self>) -> Div {
        let columns = vec![
            GridColumn {
                title: "Tanggal",
                width_px: 90.0,
                align: ColumnAlign::Center,
            },
            GridColumn {
                title: "Uraian Transaksi",
                width_px: 240.0,
                align: ColumnAlign::Left,
            },
            GridColumn {
                title: "Qty",
                width_px: 55.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "Harga (Rp)",
                width_px: 95.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "Debet (Rp)",
                width_px: 110.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "Kredit (Rp)",
                width_px: 110.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "Saldo (Rp)",
                width_px: 120.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "Status",
                width_px: 85.0,
                align: ColumnAlign::Center,
            },
            GridColumn {
                title: "Sisa (Rp)",
                width_px: 100.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "Umur",
                width_px: 90.0,
                align: ColumnAlign::Center,
            },
            GridColumn {
                title: "Aksi",
                width_px: 75.0,
                align: ColumnAlign::Center,
            },
        ];

        div()
            .flex_1()
            .bg(rgb(Theme::BG_SURFACE))
            .border_1()
            .border_color(rgb(Theme::GRID_LINE))
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(render_grid_header(&columns))
            .child(
                div()
                    .id("ledger-scroll")
                    .flex_1()
                    .children(if ledger.is_empty() {
                        vec![
                            render_empty_state(
                                "Belum ada mutasi buku piutang untuk pelanggan ini.",
                            )
                            .into_any_element(),
                        ]
                    } else {
                        ledger
                            .iter()
                            .enumerate()
                            .map(|(idx, entry)| {
                                let is_odd = idx % 2 == 1;
                                let bg_color = if is_odd {
                                    Theme::BG_ROW_ALT
                                } else {
                                    Theme::BG_SURFACE
                                };
                                let entry_id = entry.id.clone();
                                let is_rev = entry.is_reversed;
                                let can_rev = self.can_write() && !is_rev;

                                let desc_str = entry.description.clone();
                                let qty_str = entry.quantity.clone().unwrap_or_else(|| "—".into());
                                let price_str = entry
                                    .unit_price_idr
                                    .map(format_idr)
                                    .unwrap_or_else(|| "—".into());
                                let date_str = entry.effective_date.format("%d %b %Y").to_string();

                                div()
                                    .id(("ledg-row", idx))
                                    .h(px(26.0))
                                    .bg(rgb(bg_color))
                                    .border_b_1()
                                    .border_color(rgb(Theme::GRID_LINE))
                                    .flex()
                                    .items_center()
                                    .child(grid_cell(date_str, 90.0, ColumnAlign::Center, false))
                                    .child(grid_cell(desc_str, 240.0, ColumnAlign::Left, false))
                                    .child(grid_cell(qty_str, 55.0, ColumnAlign::Right, false))
                                    .child(grid_cell(price_str, 95.0, ColumnAlign::Right, false))
                                    .child(grid_cell(
                                        if entry.debit_idr > 0 {
                                            format_idr(entry.debit_idr)
                                        } else {
                                            "—".into()
                                        },
                                        110.0,
                                        ColumnAlign::Right,
                                        false,
                                    ))
                                    .child(grid_cell(
                                        if entry.credit_idr > 0 {
                                            format_idr(entry.credit_idr)
                                        } else {
                                            "—".into()
                                        },
                                        110.0,
                                        ColumnAlign::Right,
                                        false,
                                    ))
                                    .child(grid_cell(
                                        format_idr(entry.running_balance_idr),
                                        120.0,
                                        ColumnAlign::Right,
                                        false,
                                    ))
                                    .child(match entry.allocation_status {
                                        Some(AllocationStatus::Allocated) => grid_badge(
                                            "Lunas",
                                            Theme::COLOR_CREDIT,
                                            0xDCFCE7,
                                            85.0,
                                            false,
                                        ),
                                        Some(AllocationStatus::PartiallyAllocated) => grid_badge(
                                            "Sebagian",
                                            Theme::COLOR_WARNING,
                                            0xFEF3C7,
                                            85.0,
                                            false,
                                        ),
                                        Some(AllocationStatus::Unallocated) => grid_badge(
                                            "Belum",
                                            Theme::COLOR_DEBIT,
                                            0xFEE2E2,
                                            85.0,
                                            false,
                                        ),
                                        None => grid_cell(
                                            "—".to_string(),
                                            85.0,
                                            ColumnAlign::Center,
                                            false,
                                        ),
                                    })
                                    .child(grid_cell(
                                        if entry.debit_idr > 0 {
                                            format_idr(entry.remaining_idr)
                                        } else {
                                            "—".into()
                                        },
                                        100.0,
                                        ColumnAlign::Right,
                                        false,
                                    ))
                                    .child(grid_cell(
                                        match entry.age_days {
                                            Some(d) if d > 60 => "> 60 Hari".to_string(),
                                            Some(d) => match d {
                                                0..=15 => "0-15 Hari".to_string(),
                                                16..=30 => "16-30 Hari".to_string(),
                                                31..=45 => "31-45 Hari".to_string(),
                                                46..=60 => "46-60 Hari".to_string(),
                                                _ => "> 60 Hari".to_string(),
                                            },
                                            None => "—".to_string(),
                                        },
                                        90.0,
                                        ColumnAlign::Center,
                                        false,
                                    ))
                                    .child(
                                        div()
                                            .w(px(75.0))
                                            .h_full()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .when(can_rev, |d| {
                                                d.child(
                                                    div()
                                                        .id(("rev-btn", idx))
                                                        .px(px(6.0))
                                                        .py(px(1.0))
                                                        .bg(rgb(Theme::BG_WINDOW))
                                                        .border_1()
                                                        .border_color(rgb(Theme::BORDER_DARK))
                                                        .rounded(px(2.0))
                                                        .cursor_pointer()
                                                        .hover(|d| d.bg(rgb(0xFEE2E2)))
                                                        .on_click(cx.listener(
                                                            move |this, _, _, cx| {
                                                                this.open_reverse_modal(&entry_id);
                                                                cx.notify();
                                                            },
                                                        ))
                                                        .child(
                                                            div()
                                                                .text_size(px(10.0))
                                                                .text_color(rgb(Theme::COLOR_DEBIT))
                                                                .child("Koreksi"),
                                                        ),
                                                )
                                            })
                                            .when(is_rev, |d| {
                                                d.child(
                                                    div()
                                                        .text_size(px(10.0))
                                                        .text_color(rgb(Theme::TEXT_MUTED))
                                                        .child("(Batal)"),
                                                )
                                            }),
                                    )
                                    .into_any_element()
                            })
                            .collect()
                    }),
            )
    }

    fn render_payments_grid(&self, payments: &[PaymentRow]) -> Div {
        let columns = vec![
            GridColumn {
                title: "Tanggal Terima",
                width_px: 110.0,
                align: ColumnAlign::Center,
            },
            GridColumn {
                title: "Metode",
                width_px: 100.0,
                align: ColumnAlign::Left,
            },
            GridColumn {
                title: "Nomor Referensi",
                width_px: 140.0,
                align: ColumnAlign::Left,
            },
            GridColumn {
                title: "Nominal Bayar (Rp)",
                width_px: 145.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "Teralokasi (Rp)",
                width_px: 140.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "Sisa Saldo Kredit (Rp)",
                width_px: 160.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "Catatan Pembayaran",
                width_px: 260.0,
                align: ColumnAlign::Left,
            },
        ];

        div()
            .flex_1()
            .bg(rgb(Theme::BG_SURFACE))
            .border_1()
            .border_color(rgb(Theme::GRID_LINE))
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(render_grid_header(&columns))
            .child(
                div()
                    .id("payments-scroll")
                    .flex_1()
                    .children(if payments.is_empty() {
                        vec![
                            render_empty_state("Belum ada riwayat pembayaran untuk pelanggan ini.")
                                .into_any_element(),
                        ]
                    } else {
                        payments
                            .iter()
                            .enumerate()
                            .map(|(idx, p)| {
                                let is_odd = idx % 2 == 1;
                                let bg_color = if is_odd {
                                    Theme::BG_ROW_ALT
                                } else {
                                    Theme::BG_SURFACE
                                };

                                let date_str = p.received_date.format("%d %b %Y").to_string();
                                let method_str = p.method.clone();
                                let ref_str = p.reference.clone().unwrap_or_else(|| "—".into());
                                let note_str = p.note.clone().unwrap_or_else(|| "—".into());

                                div()
                                    .id(("pay-row", idx))
                                    .h(px(26.0))
                                    .bg(rgb(bg_color))
                                    .border_b_1()
                                    .border_color(rgb(Theme::GRID_LINE))
                                    .flex()
                                    .items_center()
                                    .child(grid_cell(date_str, 110.0, ColumnAlign::Center, false))
                                    .child(grid_cell(method_str, 100.0, ColumnAlign::Left, false))
                                    .child(grid_cell(ref_str, 140.0, ColumnAlign::Left, false))
                                    .child(grid_cell(
                                        format_idr(p.amount_idr),
                                        145.0,
                                        ColumnAlign::Right,
                                        false,
                                    ))
                                    .child(grid_cell(
                                        format_idr(p.allocated_idr),
                                        140.0,
                                        ColumnAlign::Right,
                                        false,
                                    ))
                                    .child(grid_cell(
                                        format_idr(p.unallocated_credit_idr),
                                        160.0,
                                        ColumnAlign::Right,
                                        false,
                                    ))
                                    .child(grid_cell(note_str, 260.0, ColumnAlign::Left, false))
                                    .into_any_element()
                            })
                            .collect()
                    }),
            )
    }

    fn render_aging_report_view(&self, cx: &mut Context<Self>) -> Div {
        let report = match self.store.get_aging_report(self.aging_as_of) {
            Ok(r) => r,
            Err(_) => return div().child("Gagal memuat laporan aging."),
        };

        let columns = vec![
            GridColumn {
                title: "Kode",
                width_px: 80.0,
                align: ColumnAlign::Center,
            },
            GridColumn {
                title: "Nama Pelanggan",
                width_px: 240.0,
                align: ColumnAlign::Left,
            },
            GridColumn {
                title: "Total Terutang (Rp)",
                width_px: 145.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "0 - 15 Hari",
                width_px: 115.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "16 - 30 Hari",
                width_px: 115.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "31 - 45 Hari",
                width_px: 115.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "46 - 60 Hari",
                width_px: 115.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "> 60 Hari",
                width_px: 120.0,
                align: ColumnAlign::Right,
            },
        ];

        let filtered_customers: Vec<_> = report
            .customers
            .into_iter()
            .filter(|c| match self.aging_selected_bucket {
                Some(0) => c.days_0_15 > 0,
                Some(1) => c.days_16_30 > 0,
                Some(2) => c.days_31_45 > 0,
                Some(3) => c.days_46_60 > 0,
                Some(4) => c.days_over_60 > 0,
                _ => true,
            })
            .collect();

        div()
            .flex_1()
            .flex()
            .flex_col()
            .h_full()
            .bg(rgb(Theme::BG_WINDOW))
            .p(px(10.0))
            .gap(px(8.0))
            .child(
                div()
                    .h(px(36.0))
                    .bg(rgb(Theme::BG_SURFACE))
                    .border_1()
                    .border_color(rgb(Theme::GRID_LINE))
                    .px(px(10.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(Theme::TEXT_PRIMARY))
                                    .child("Laporan Analisis Umur Piutang (Aging)"),
                            )
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .text_color(rgb(Theme::TEXT_MUTED))
                                    .child(format!(
                                        "Per Tanggal: {}",
                                        report.as_of.format("%d %B %Y")
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .id("btn-change-aging-date")
                            .h(px(24.0))
                            .px(px(8.0))
                            .bg(rgb(Theme::BG_WINDOW))
                            .border_1()
                            .border_color(rgb(Theme::BORDER_DARK))
                            .rounded(px(2.0))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .hover(|d| d.bg(rgb(Theme::BG_ROW_HOVER)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_change_as_of_modal();
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(rgb(Theme::TEXT_PRIMARY))
                                    .child("Ubah Tanggal Acuan..."),
                            ),
                    ),
            )
            // Bucket Cards
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(self.aging_card(
                        "aging-c-all",
                        "Total Terbuka",
                        format!("Rp {}", format_idr(report.total_open_idr)),
                        self.aging_selected_bucket.is_none(),
                        Theme::COLOR_DEBIT,
                        cx,
                        None,
                    ))
                    .child(self.aging_card(
                        "aging-c-0",
                        "0-15 Hari (Lancar)",
                        format!("Rp {}", format_idr(report.days_0_15)),
                        self.aging_selected_bucket == Some(0),
                        Theme::COLOR_CREDIT,
                        cx,
                        Some(0),
                    ))
                    .child(self.aging_card(
                        "aging-c-1",
                        "16-30 Hari",
                        format!("Rp {}", format_idr(report.days_16_30)),
                        self.aging_selected_bucket == Some(1),
                        Theme::COLOR_WARNING,
                        cx,
                        Some(1),
                    ))
                    .child(self.aging_card(
                        "aging-c-2",
                        "31-45 Hari",
                        format!("Rp {}", format_idr(report.days_31_45)),
                        self.aging_selected_bucket == Some(2),
                        Theme::COLOR_WARNING,
                        cx,
                        Some(2),
                    ))
                    .child(self.aging_card(
                        "aging-c-3",
                        "46-60 Hari",
                        format!("Rp {}", format_idr(report.days_46_60)),
                        self.aging_selected_bucket == Some(3),
                        Theme::COLOR_DEBIT,
                        cx,
                        Some(3),
                    ))
                    .child(self.aging_card(
                        "aging-c-4",
                        "> 60 Hari (Menunggak)",
                        format!("Rp {}", format_idr(report.days_over_60)),
                        self.aging_selected_bucket == Some(4),
                        0x7F1D1D,
                        cx,
                        Some(4),
                    )),
            )
            // DBGrid Drill-down
            .child(
                div()
                    .flex_1()
                    .bg(rgb(Theme::BG_SURFACE))
                    .border_1()
                    .border_color(rgb(Theme::GRID_LINE))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(render_grid_header(&columns))
                    .child(div().id("aging-drill-scroll").flex_1().children(
                        if filtered_customers.is_empty() {
                            vec![
                                render_empty_state(
                                    "Tidak ada piutang terbuka pada kategori bucket umur ini.",
                                )
                                .into_any_element(),
                            ]
                        } else {
                            filtered_customers
                                .into_iter()
                                .enumerate()
                                .map(|(idx, c)| {
                                    let is_odd = idx % 2 == 1;
                                    let bg_color = if is_odd {
                                        Theme::BG_ROW_ALT
                                    } else {
                                        Theme::BG_SURFACE
                                    };
                                    let cid = c.customer_id.clone();
                                    let cname = c.customer_name.clone();
                                    let code_str = c.customer_code.unwrap_or_else(|| "—".into());

                                    div()
                                        .id(("ag-row", idx))
                                        .h(px(26.0))
                                        .bg(rgb(bg_color))
                                        .border_b_1()
                                        .border_color(rgb(Theme::GRID_LINE))
                                        .flex()
                                        .items_center()
                                        .cursor_pointer()
                                        .hover(|d| d.bg(rgb(Theme::BG_ROW_HOVER)))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.selected_customer_id = Some(cid.clone());
                                            this.current_view = CurrentView::CustomerDetail;
                                            this.status_msg = format!(
                                                "Membuka buku piutang pelanggan {}.",
                                                cname
                                            );
                                            cx.notify();
                                        }))
                                        .child(grid_cell(
                                            code_str,
                                            80.0,
                                            ColumnAlign::Center,
                                            false,
                                        ))
                                        .child(grid_cell(
                                            c.customer_name,
                                            240.0,
                                            ColumnAlign::Left,
                                            false,
                                        ))
                                        .child(grid_cell(
                                            format_idr(c.total_outstanding_idr),
                                            145.0,
                                            ColumnAlign::Right,
                                            false,
                                        ))
                                        .child(grid_cell(
                                            if c.days_0_15 > 0 {
                                                format_idr(c.days_0_15)
                                            } else {
                                                "—".into()
                                            },
                                            115.0,
                                            ColumnAlign::Right,
                                            false,
                                        ))
                                        .child(grid_cell(
                                            if c.days_16_30 > 0 {
                                                format_idr(c.days_16_30)
                                            } else {
                                                "—".into()
                                            },
                                            115.0,
                                            ColumnAlign::Right,
                                            false,
                                        ))
                                        .child(grid_cell(
                                            if c.days_31_45 > 0 {
                                                format_idr(c.days_31_45)
                                            } else {
                                                "—".into()
                                            },
                                            115.0,
                                            ColumnAlign::Right,
                                            false,
                                        ))
                                        .child(grid_cell(
                                            if c.days_46_60 > 0 {
                                                format_idr(c.days_46_60)
                                            } else {
                                                "—".into()
                                            },
                                            115.0,
                                            ColumnAlign::Right,
                                            false,
                                        ))
                                        .child(grid_cell(
                                            if c.days_over_60 > 0 {
                                                format_idr(c.days_over_60)
                                            } else {
                                                "—".into()
                                            },
                                            120.0,
                                            ColumnAlign::Right,
                                            false,
                                        ))
                                        .into_any_element()
                                })
                                .collect()
                        },
                    )),
            )
    }

    fn render_user_management_view(&self, cx: &mut Context<Self>) -> Div {
        let users = self.store.list_users().unwrap_or_default();
        let columns = vec![
            GridColumn {
                title: "Username",
                width_px: 130.0,
                align: ColumnAlign::Left,
            },
            GridColumn {
                title: "Nama Lengkap",
                width_px: 240.0,
                align: ColumnAlign::Left,
            },
            GridColumn {
                title: "Peran / Otorisasi",
                width_px: 130.0,
                align: ColumnAlign::Center,
            },
            GridColumn {
                title: "Status",
                width_px: 95.0,
                align: ColumnAlign::Center,
            },
            GridColumn {
                title: "Perlu Ganti Sandi",
                width_px: 140.0,
                align: ColumnAlign::Center,
            },
            GridColumn {
                title: "Aksi Akun",
                width_px: 180.0,
                align: ColumnAlign::Center,
            },
        ];

        div()
            .flex_1()
            .flex()
            .flex_col()
            .h_full()
            .bg(rgb(Theme::BG_WINDOW))
            .p(px(10.0))
            .gap(px(8.0))
            .child(
                div()
                    .h(px(36.0))
                    .bg(rgb(Theme::BG_SURFACE))
                    .border_1()
                    .border_color(rgb(Theme::GRID_LINE))
                    .px(px(10.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(Theme::TEXT_PRIMARY))
                                    .child("Manajemen Pengguna Aplikasi (Owner Only)"),
                            )
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(rgb(Theme::TEXT_MUTED))
                                    .child("Kelola akun operator dan kewenangan akses sistem"),
                            ),
                    )
                    .child(
                        div()
                            .id("btn-add-user-mgr")
                            .h(px(24.0))
                            .px(px(10.0))
                            .bg(rgb(Theme::BTN_PRIMARY_BG))
                            .rounded(px(2.0))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .hover(|d| d.bg(rgb(0x106EBE)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_new_user_modal();
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(rgb(0xFFFFFF))
                                    .child("+ Tambah Pengguna Baru"),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .bg(rgb(Theme::BG_SURFACE))
                    .border_1()
                    .border_color(rgb(Theme::GRID_LINE))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(render_grid_header(&columns))
                    .child(
                        div()
                            .id("user-mgr-scroll")
                            .flex_1()
                            .children(users.into_iter().enumerate().map(|(idx, u)| {
                                let is_odd = idx % 2 == 1;
                                let bg_color = if is_odd { Theme::BG_ROW_ALT } else { Theme::BG_SURFACE };
                                let uid = u.id.clone();
                                let uid_toggle = u.id.clone();
                                let is_act = u.active;
                                let username_str = u.username.clone();

                                div()
                                    .id(("u-row", idx))
                                    .h(px(28.0))
                                    .bg(rgb(bg_color))
                                    .border_b_1()
                                    .border_color(rgb(Theme::GRID_LINE))
                                    .flex()
                                    .items_center()
                                    .child(grid_cell(u.username, 130.0, ColumnAlign::Left, false))
                                    .child(grid_cell(u.full_name.clone(), 240.0, ColumnAlign::Left, false))
                                    .child(grid_cell(match u.role {
                                        UserRole::Owner => "Owner",
                                        UserRole::Operator => "Operator",
                                        UserRole::Viewer => "Viewer",
                                    }, 130.0, ColumnAlign::Center, false))
                                    .child(if u.active {
                                        grid_badge("Aktif", Theme::COLOR_CREDIT, 0xDCFCE7, 95.0, false)
                                    } else {
                                        grid_badge("Nonaktif", Theme::COLOR_DEBIT, 0xFEE2E2, 95.0, false)
                                    })
                                    .child(grid_cell(if u.must_change_password { "Ya (Wajib)" } else { "Tidak" }, 140.0, ColumnAlign::Center, false))
                                    .child(
                                        div()
                                            .w(px(180.0))
                                            .h_full()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .gap(px(6.0))
                                            .child(
                                                div()
                                                    .id(("rst-btn", idx))
                                                    .px(px(6.0))
                                                    .py(px(2.0))
                                                    .bg(rgb(Theme::BG_WINDOW))
                                                    .border_1()
                                                    .border_color(rgb(Theme::BORDER_DARK))
                                                    .rounded(px(2.0))
                                                    .cursor_pointer()
                                                    .hover(|d| d.bg(rgb(Theme::BG_ROW_HOVER)))
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        match this.store.reset_password(&uid) {
                                                            Ok(pw) => {
                                                                this.active_modal = Some(ActiveModal::Notice {
                                                                    title: "Reset Kata Sandi Sukses",
                                                                    message: format!("Kata sandi sementara baru: {}", pw),
                                                                });
                                                            }
                                                            Err(e) => {
                                                                this.status_msg = e.0;
                                                            }
                                                        }
                                                        cx.notify();
                                                    }))
                                                    .child(div().text_size(px(10.5)).text_color(rgb(Theme::TEXT_PRIMARY)).child("Reset Sandi")),
                                            )
                                            .child(
                                                div()
                                                    .id(("tgl-btn", idx))
                                                    .px(px(6.0))
                                                    .py(px(2.0))
                                                    .bg(rgb(Theme::BG_WINDOW))
                                                    .border_1()
                                                    .border_color(rgb(Theme::BORDER_DARK))
                                                    .rounded(px(2.0))
                                                    .cursor_pointer()
                                                    .hover(|d| d.bg(rgb(Theme::BG_ROW_HOVER)))
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        let up = UpdateUserInput {
                                                            full_name: u.full_name.clone(),
                                                            role: u.role,
                                                            active: !is_act,
                                                        };
                                                        if let Err(e) = this.store.update_user(&uid_toggle, &up) {
                                                            this.status_msg = e.0;
                                                        } else {
                                                            this.status_msg = format!("Status akun {} telah diubah.", username_str);
                                                        }
                                                        cx.notify();
                                                    }))
                                                    .child(div().text_size(px(10.5)).text_color(rgb(if is_act { Theme::COLOR_DEBIT } else { Theme::COLOR_CREDIT })).child(if is_act { "Nonaktifkan" } else { "Aktifkan" })),
                                            ),
                                    )
                                    .into_any_element()
                            }).collect::<Vec<_>>()),
                    ),
            )
    }

    fn kpi_card(&self, title: &'static str, value: String, color_hex: u32) -> Div {
        div()
            .flex_1()
            .h(px(56.0))
            .bg(rgb(Theme::BG_SURFACE))
            .border_1()
            .border_color(rgb(Theme::GRID_LINE))
            .p(px(8.0))
            .flex()
            .flex_col()
            .justify_between()
            .child(
                div()
                    .text_size(px(11.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(Theme::TEXT_SECONDARY))
                    .child(title),
            )
            .child(
                div()
                    .text_size(px(16.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(color_hex))
                    .child(value),
            )
    }

    fn balance_card(
        &self,
        title: &'static str,
        amount: String,
        subtitle: &'static str,
        color_hex: u32,
    ) -> Div {
        div()
            .flex_1()
            .h(px(64.0))
            .bg(rgb(Theme::BG_SURFACE))
            .border_1()
            .border_color(rgb(Theme::GRID_LINE))
            .p(px(8.0))
            .flex()
            .flex_col()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(11.0))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(Theme::TEXT_SECONDARY))
                            .child(title),
                    )
                    .child(
                        div()
                            .text_size(px(16.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(color_hex))
                            .child(amount),
                    ),
            )
            .child(
                div()
                    .text_size(px(10.5))
                    .text_color(rgb(Theme::TEXT_MUTED))
                    .child(subtitle),
            )
    }

    fn aging_card(
        &self,
        id: &'static str,
        label: &'static str,
        amount: String,
        is_active: bool,
        color_hex: u32,
        cx: &mut Context<Self>,
        bucket: Option<usize>,
    ) -> impl IntoElement {
        div()
            .id(id)
            .flex_1()
            .h(px(52.0))
            .bg(rgb(if is_active {
                0xFFFFFF
            } else {
                Theme::BG_SURFACE
            }))
            .border_2()
            .border_color(rgb(if is_active {
                Theme::BG_ROW_SELECTED
            } else {
                Theme::GRID_LINE
            }))
            .p(px(6.0))
            .rounded(px(3.0))
            .flex()
            .flex_col()
            .justify_between()
            .cursor_pointer()
            .hover(|d| d.bg(rgb(Theme::BG_ROW_HOVER)))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.aging_selected_bucket = bucket;
                cx.notify();
            }))
            .child(
                div()
                    .text_size(px(10.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(Theme::TEXT_SECONDARY))
                    .child(label),
            )
            .child(
                div()
                    .text_size(px(13.5))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(color_hex))
                    .child(amount),
            )
    }

    fn render_modal_overlay(&self, cx: &mut Context<Self>) -> Option<Div> {
        let modal = self.active_modal.as_ref()?;

        match modal {
            ActiveModal::Notice { title, message } => {
                let msg = message.clone();
                let dialog = ModalDialog::new(title, 420.0);
                let body = div()
                    .p(px(8.0))
                    .text_size(px(12.0))
                    .text_color(rgb(Theme::TEXT_PRIMARY))
                    .child(msg)
                    .into_any_element();
                let footer = div()
                    .h(px(40.0))
                    .bg(rgb(Theme::BG_HEADER))
                    .border_t_1()
                    .border_color(rgb(Theme::GRID_LINE))
                    .px(px(10.0))
                    .flex()
                    .items_center()
                    .justify_end()
                    .child(
                        div()
                            .id("btn-notice-ok")
                            .h(px(26.0))
                            .px(px(14.0))
                            .bg(rgb(Theme::BTN_PRIMARY_BG))
                            .rounded(px(2.0))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .hover(|d| d.bg(rgb(0x106EBE)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.active_modal = None;
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(0xFFFFFF))
                                    .child("OK (Enter)"),
                            ),
                    );
                Some(dialog.frame(body, footer))
            }
            ActiveModal::RecordSale => {
                let cid = self.selected_customer_id.as_deref().unwrap_or_default();
                let cust = self.store.get_customer(cid).ok()?;
                let deposit_avail = cust.customer_credit_idr;

                let mut total_sales: i64 = 0;
                for (desc, qty, pr) in &self.sale_items {
                    if !desc.trim().is_empty() {
                        let q = toko_money::quantity_milli(qty).unwrap_or(0);
                        let p = toko_money::rupiah(pr).unwrap_or(0);
                        if let Some(sub) = toko_money::line_total(q, p) {
                            total_sales += sub;
                        }
                    }
                }

                let dialog = ModalDialog::new("Penjualan Kredit Baru (Multi-Item)", 620.0);
                let date_val = self
                    .form_fields
                    .get(0)
                    .map(|f| f.1.clone())
                    .unwrap_or_default();
                let reason_val = self
                    .form_fields
                    .get(1)
                    .map(|f| f.1.clone())
                    .unwrap_or_default();
                let cust_code_str = cust.code.unwrap_or_else(|| "—".into());

                let body = div()
                    .flex()
                    .flex_col()
                    .gap(px(10.0))
                    .child(
                        div()
                            .p(px(8.0))
                            .bg(rgb(Theme::BG_SURFACE))
                            .border_1()
                            .border_color(rgb(Theme::GRID_LINE))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(div().text_size(px(12.0)).font_weight(FontWeight::BOLD).text_color(rgb(Theme::TEXT_PRIMARY)).child(format!("Pelanggan: {} ({})", cust.name, cust_code_str)))
                            .child(div().text_size(px(11.5)).text_color(rgb(Theme::COLOR_DEPOSIT)).child(format!("Saldo Deposit: Rp {}", format_idr(deposit_avail)))),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.0))
                                    .child(div().text_size(px(11.0)).font_weight(FontWeight::SEMIBOLD).child("Tanggal (YYYY-MM-DD):"))
                                    .child(div().h(px(24.0)).bg(rgb(0xFFFFFF)).border_1().border_color(rgb(Theme::BORDER_DARK)).px(px(6.0)).flex().items_center().text_size(px(11.5)).child(date_val)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.0))
                                    .child(div().text_size(px(11.0)).font_weight(FontWeight::SEMIBOLD).child("Alasan (jika tanggal lampau):"))
                                    .child(div().h(px(24.0)).bg(rgb(0xFFFFFF)).border_1().border_color(rgb(Theme::BORDER_DARK)).px(px(6.0)).flex().items_center().text_size(px(11.5)).child(if reason_val.is_empty() { "Alasan tanggal lampau...".into() } else { reason_val })),
                            ),
                    )
                    // Tabel Baris Barang
                    .child(
                        div()
                            .border_1()
                            .border_color(rgb(Theme::GRID_LINE))
                            .bg(rgb(Theme::BG_SURFACE))
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .h(px(24.0))
                                    .bg(rgb(Theme::BG_HEADER))
                                    .border_b_1()
                                    .border_color(rgb(Theme::GRID_LINE))
                                    .flex()
                                    .items_center()
                                    .text_size(px(11.0))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(div().w(px(240.0)).px(px(6.0)).child("Nama Barang / Uraian"))
                                    .child(div().w(px(70.0)).px(px(6.0)).text_right().child("Qty"))
                                    .child(div().w(px(110.0)).px(px(6.0)).text_right().child("Harga (Rp)"))
                                    .child(div().w(px(110.0)).px(px(6.0)).text_right().child("Subtotal (Rp)"))
                                    .child(div().w(px(50.0)).child("")),
                            )
                            .children(self.sale_items.iter().enumerate().map(|(idx, (desc, qty, pr))| {
                                let q = toko_money::quantity_milli(qty).unwrap_or(0);
                                let p = toko_money::rupiah(pr).unwrap_or(0);
                                let sub = toko_money::line_total(q, p).unwrap_or(0);
                                let is_active_row = self.sale_active_field.0 == idx;

                                let desc_display = if desc.is_empty() { "Ketik nama...".to_string() } else { desc.clone() };
                                let qty_display = if qty.is_empty() { "1".to_string() } else { qty.clone() };
                                let price_display = if pr.is_empty() { "0".to_string() } else { format_idr(p) };

                                div()
                                    .id(("sale-edit-row", idx))
                                    .h(px(26.0))
                                    .border_b_1()
                                    .border_color(rgb(Theme::GRID_LINE))
                                    .bg(rgb(if is_active_row { 0xEFF6FF } else { 0xFFFFFF }))
                                    .flex()
                                    .items_center()
                                    .text_size(px(11.5))
                                    .child(div().w(px(240.0)).px(px(6.0)).child(desc_display))
                                    .child(div().w(px(70.0)).px(px(6.0)).text_right().child(qty_display))
                                    .child(div().w(px(110.0)).px(px(6.0)).text_right().child(price_display))
                                    .child(div().w(px(110.0)).px(px(6.0)).text_right().font_weight(FontWeight::SEMIBOLD).child(format_idr(sub)))
                                    .child(
                                        div()
                                            .w(px(50.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .when(self.sale_items.len() > 1, |d| {
                                                d.child(
                                                    div()
                                                        .id(("del-sale-line", idx))
                                                        .cursor_pointer()
                                                        .text_size(px(10.0))
                                                        .text_color(rgb(Theme::COLOR_DEBIT))
                                                        .on_click(cx.listener(move |this, _, _, cx| {
                                                            this.sale_items.remove(idx);
                                                            cx.notify();
                                                        }))
                                                        .child("Hapus"),
                                                )
                                            }),
                                    )
                            })),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .id("btn-add-sale-row")
                                    .h(px(22.0))
                                    .px(px(8.0))
                                    .bg(rgb(Theme::BG_WINDOW))
                                    .border_1()
                                    .border_color(rgb(Theme::BORDER_DARK))
                                    .rounded(px(2.0))
                                    .flex()
                                    .items_center()
                                    .cursor_pointer()
                                    .hover(|d| d.bg(rgb(Theme::BG_ROW_HOVER)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.sale_items.push(("".into(), "1".into(), "".into()));
                                        this.sale_active_field = (this.sale_items.len() - 1, 0);
                                        cx.notify();
                                    }))
                                    .child(div().text_size(px(10.5)).font_weight(FontWeight::MEDIUM).child("+ Tambah Baris Barang")),
                            )
                            .child(div().text_size(px(12.0)).font_weight(FontWeight::BOLD).child(format!("Total Penjualan: Rp {}", format_idr(total_sales)))),
                    )
                    .when(deposit_avail > 0 && total_sales > 0, |d| {
                        let auto_app = total_sales.min(deposit_avail);
                        d.child(
                            div()
                                .p(px(6.0))
                                .bg(rgb(0xEFF6FF))
                                .border_1()
                                .border_color(rgb(0xBFDBFE))
                                .rounded(px(2.0))
                                .text_size(px(11.0))
                                .text_color(rgb(Theme::COLOR_DEPOSIT))
                                .child(format!(
                                    "Saldo deposit sebesar Rp {} akan otomatis diterapkan melunasi nota ini saat disimpan!",
                                    format_idr(auto_app)
                                )),
                        )
                    })
                    .when(!self.form_error.is_empty(), |d| {
                        d.child(
                            div()
                                .p(px(6.0))
                                .bg(rgb(0xFEE2E2))
                                .border_1()
                                .border_color(rgb(0xFCA5A5))
                                .rounded(px(2.0))
                                .text_size(px(11.5))
                                .text_color(rgb(Theme::COLOR_DEBIT))
                                .child(self.form_error.clone()),
                        )
                    })
                    .into_any_element();

                let footer = div()
                    .h(px(40.0))
                    .bg(rgb(Theme::BG_HEADER))
                    .border_t_1()
                    .border_color(rgb(Theme::GRID_LINE))
                    .px(px(10.0))
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(8.0))
                    .child(
                        div()
                            .id("btn-sale-modal-cancel")
                            .h(px(26.0))
                            .px(px(12.0))
                            .bg(rgb(Theme::BG_SURFACE))
                            .border_1()
                            .border_color(rgb(Theme::BORDER_DARK))
                            .rounded(px(2.0))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .hover(|d| d.bg(rgb(Theme::BG_ROW_HOVER)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.active_modal = None;
                                cx.notify();
                            }))
                            .child(div().text_size(px(11.5)).child("Batal (Esc)")),
                    )
                    .child(
                        div()
                            .id("btn-sale-modal-save")
                            .h(px(26.0))
                            .px(px(14.0))
                            .bg(rgb(Theme::BTN_PRIMARY_BG))
                            .rounded(px(2.0))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .hover(|d| d.bg(rgb(0x106EBE)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.submit_modal(cx);
                            }))
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(0xFFFFFF))
                                    .child("Simpan Penjualan (Enter)"),
                            ),
                    );

                Some(dialog.frame(body, footer))
            }
            _ => {
                // Modal form umum (Login, Pelanggan Baru, Pembayaran, Terapkan Saldo, Tambah Pengguna, dll.)
                let title = match modal {
                    ActiveModal::Login => "Masuk / Ganti Pengguna",
                    ActiveModal::ChangePassword => "Ganti Kata Sandi Akun",
                    ActiveModal::NewCustomer => "Tambah Pelanggan Baru",
                    ActiveModal::EditCustomer(_) => "Ubah Data Pelanggan",
                    ActiveModal::RecordPayment => "Catat Pembayaran Piutang (FIFO)",
                    ActiveModal::ApplyDeposit => "Terapkan Saldo Kredit Pelanggan (Deposit)",
                    ActiveModal::ReverseTransaction { .. } => {
                        "Koreksi / Batalkan Mutasi Buku Piutang"
                    }
                    ActiveModal::NewUser => "Tambah Pengguna Baru (Owner Only)",
                    ActiveModal::ChangeAsOfDate => "Ubah Tanggal Acuan Laporan Aging",
                    _ => "Dialog Formulir",
                };

                let submit_label = match modal {
                    ActiveModal::Login => "Masuk (Enter)",
                    ActiveModal::RecordPayment => "Catat Pembayaran (Enter)",
                    ActiveModal::ApplyDeposit => "Terapkan Saldo (Enter)",
                    ActiveModal::ReverseTransaction { .. } => "Batalkan Mutasi (Enter)",
                    _ => "Simpan (Enter)",
                };

                let dialog = ModalDialog::new(title, 480.0);
                let body = div()
                    .flex()
                    .flex_col()
                    .gap(px(10.0))
                    .children(self.form_fields.iter().enumerate().map(|(idx, (label, val))| {
                        let is_active = idx == self.form_active;
                        let is_pw = label.to_lowercase().contains("sandi") || label.to_lowercase().contains("password");

                        div()
                            .flex()
                            .flex_col()
                            .gap(px(3.0))
                            .child(div().text_size(px(11.5)).font_weight(FontWeight::SEMIBOLD).text_color(rgb(Theme::TEXT_PRIMARY)).child(*label))
                            .child(
                                div()
                                    .h(px(26.0))
                                    .bg(rgb(0xFFFFFF))
                                    .border_1()
                                    .border_color(rgb(if is_active { Theme::BG_ROW_SELECTED } else { Theme::BORDER_DARK }))
                                    .px(px(6.0))
                                    .flex()
                                    .items_center()
                                    .text_size(px(12.0))
                                    .child(if val.is_empty() {
                                        div().text_color(rgb(Theme::TEXT_MUTED)).child("Ketik teks...")
                                    } else if is_pw {
                                        div().text_color(rgb(Theme::TEXT_PRIMARY)).child("•".repeat(val.len()))
                                    } else {
                                        div().text_color(rgb(Theme::TEXT_PRIMARY)).child(val.clone())
                                    }),
                            )
                    }))
                    .when(!self.form_error.is_empty(), |d| {
                        d.child(
                            div()
                                .p(px(6.0))
                                .bg(rgb(0xFEE2E2))
                                .border_1()
                                .border_color(rgb(0xFCA5A5))
                                .rounded(px(2.0))
                                .text_size(px(11.5))
                                .text_color(rgb(Theme::COLOR_DEBIT))
                                .child(self.form_error.clone()),
                        )
                    })
                    .child(div().text_size(px(10.5)).text_color(rgb(Theme::TEXT_MUTED)).child("Gunakan [Tab] untuk berpindah kolom, [Enter] untuk simpan, [Esc] untuk batal."))
                    .into_any_element();

                let footer = div()
                    .h(px(40.0))
                    .bg(rgb(Theme::BG_HEADER))
                    .border_t_1()
                    .border_color(rgb(Theme::GRID_LINE))
                    .px(px(10.0))
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(8.0))
                    .child(
                        div()
                            .id("btn-form-cancel")
                            .h(px(26.0))
                            .px(px(12.0))
                            .bg(rgb(Theme::BG_SURFACE))
                            .border_1()
                            .border_color(rgb(Theme::BORDER_DARK))
                            .rounded(px(2.0))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .hover(|d| d.bg(rgb(Theme::BG_ROW_HOVER)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.active_modal = None;
                                cx.notify();
                            }))
                            .child(div().text_size(px(11.5)).child("Batal (Esc)")),
                    )
                    .child(
                        div()
                            .id("btn-form-submit")
                            .h(px(26.0))
                            .px(px(14.0))
                            .bg(rgb(Theme::BTN_PRIMARY_BG))
                            .rounded(px(2.0))
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .hover(|d| d.bg(rgb(0x106EBE)))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.submit_modal(cx);
                            }))
                            .child(
                                div()
                                    .text_size(px(11.5))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(0xFFFFFF))
                                    .child(submit_label),
                            ),
                    );

                Some(dialog.frame(body, footer))
            }
        }
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1120.0), px(740.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(Desktop::new),
        )
        .expect("gagal membuka jendela aplikasi desktop");
        cx.activate(true);
    });
}
