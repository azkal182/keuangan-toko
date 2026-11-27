//! Keuangan Toko - Native Windows Desktop Application (GPUI)
//! Arsitektur UI/UX Native Desktop yang presisi dan kontekstual:
//! - System Menu Bar (26px) di puncak jendela + Left Explorer Navigation Pane (210px)
//! - Contextual Page Command Bar (38px) yang HANYA menampilkan tombol aksi sesuai halaman/tab aktif
//! - Native DBGrid dengan kolom indikator baris (`#` / `▶`), garis sel tegas, dan format akuntansi `(Rp ...)`
//! - Seluruh data, aturan bisnis, dan otorisasi selaras 100% dengan spesifikasi final (ADR-007, API_SPEC, FEATURE_SPECIFICATIONS)
#![allow(clippy::get_first, clippy::too_many_arguments, clippy::collapsible_if)]

mod mock;
pub mod store;
pub mod ui;

use chrono::{Local, NaiveDate};
use gpui::{
    App, Application, Bounds, Context, Div, FocusHandle, FontWeight, KeyDownEvent, Window,
    WindowBounds, WindowOptions, div, prelude::*, px, rgb, size,
};
use store::mock_store::{MockDataAdapter, compute_line_total_half_up};
use store::*;
use ui::db_grid::{
    ColumnAlign, GridColumn, grid_badge, grid_cell, grid_cell_styled, grid_row_indicator,
    render_empty_state, render_grid_header,
};
use ui::menu_bar::{
    render_left_sidebar_container, render_system_menu_bar, sidebar_nav_item,
    sidebar_section_header, system_menu_item,
};
use ui::modal::ModalDialog;
use ui::status_bar::StatusBar;
use ui::theme::{Theme, format_accounting_rupiah, format_rupiah};
use ui::tool_bar::{
    accent_action_button, primary_action_button, render_toolbar_container, secondary_action_button,
    tool_separator,
};

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
    EditCustomer {
        customer_id: String,
    },
    MergeCustomer {
        source_id: String,
    },
    RecordSale,
    RecordPayment,
    ApplyDeposit,
    ReallocatePayment {
        payment_id: String,
    },
    ReverseTransaction {
        customer_id: String,
        entry_id: String,
    },
    FilterLedgerDate,
    NewUser,
    EditUser {
        user_id: String,
    },
    ResetUserPassword {
        user_id: String,
        username: String,
    },
    ChangeAsOfDate,
    Notice {
        title: &'static str,
        message: String,
    },
}

pub struct Desktop {
    store: Box<dyn TokoStoreAdapter>,
    session: Option<SessionUser>,
    current_view: CurrentView,
    selected_customer_id: Option<String>,
    customer_tab: CustomerTab,
    query: String,
    customer_active_only: bool,
    ledger_sort_desc: bool,
    ledger_from: Option<NaiveDate>,
    ledger_to: Option<NaiveDate>,
    status_msg: String,
    active_modal: Option<ActiveModal>,
    form_fields: Vec<(&'static str, String)>,
    form_active: usize,
    form_error: String,
    // State khusus form penjualan multi-item
    sale_items: Vec<(String, String, String)>, // (uraian, qty, harga_satuan)
    sale_active_field: (usize, usize), // (row_index, col_index: 0=desc, 1=qty, 2=price, 99=header_fields)
    // State khusus aging report
    aging_as_of: NaiveDate,
    aging_selected_bucket: Option<usize>, // None=Semua, 0=0-15, 1=16-30, 2=31-45, 3=46-60, 4=>60, 5=unknown, 6=future
    focus: FocusHandle,
}

impl Desktop {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut store = build_store_from_env();
        let session = store.restore_session();
        let today = Local::now().date_naive();
        let first_cust_id = if session.is_some() {
            store
                .list_customers(&CustomerQuery {
                    search: None,
                    active_only: false,
                })
                .first()
                .map(|c| c.id.clone())
        } else {
            None
        };

        let mut this = Self {
            store,
            session,
            current_view: CurrentView::CustomerList,
            selected_customer_id: first_cust_id,
            customer_tab: CustomerTab::Ledger,
            query: String::new(),
            customer_active_only: false,
            ledger_sort_desc: true, // Default sesuai Web: Terbaru di Atas
            ledger_from: None,
            ledger_to: None,
            status_msg: "Sistem siap. Gunakan panel navigasi kiri, tombol toolbar kontekstual, atau pintasan keyboard.".into(),
            active_modal: None,
            form_fields: Vec::new(),
            form_active: 0,
            form_error: String::new(),
            sale_items: vec![("".into(), "1".into(), "".into())],
            sale_active_field: (0, 0),
            aging_as_of: today,
            aging_selected_bucket: None,
            focus: cx.focus_handle(),
        };
        if this.session.is_none() {
            this.open_login_modal();
        }
        this
    }

    fn today_jakarta(&self) -> NaiveDate {
        Local::now().date_naive()
    }

    fn is_owner(&self) -> bool {
        self.session
            .as_ref()
            .map(|s| s.role == UserRole::Owner)
            .unwrap_or(false)
    }

    fn can_write(&self) -> bool {
        self.session
            .as_ref()
            .map(|s| s.role == UserRole::Owner || s.role == UserRole::Operator)
            .unwrap_or(false)
    }

    fn active_module_name(&self) -> &'static str {
        match self.current_view {
            CurrentView::CustomerList => "Ringkasan Pelanggan",
            CurrentView::CustomerDetail => match self.customer_tab {
                CustomerTab::Ledger => "Detail Pelanggan > Buku Piutang",
                CustomerTab::Payments => "Detail Pelanggan > Riwayat Pembayaran",
            },
            CurrentView::AgingReport => "Laporan Umur Piutang",
            CurrentView::UserManagement => "Manajemen Pengguna",
        }
    }

    // =========================================================================
    // Pembuka Modal Kontekstual
    // =========================================================================

    pub fn open_new_customer_modal(&mut self) {
        if !self.can_write() {
            self.status_msg =
                "Akses ditolak: Hanya Pemilik (Owner) dan Operator yang dapat menambah pelanggan."
                    .into();
            return;
        }
        self.form_fields = vec![
            (
                "Kode Pelanggan (Opsional, unik setelah trim/uppercase)",
                String::new(),
            ),
            ("Nama Pelanggan (Wajib)", String::new()),
            ("Alamat Pelanggan (Opsional)", String::new()),
        ];
        self.form_active = 1;
        self.form_error.clear();
        self.active_modal = Some(ActiveModal::NewCustomer);
    }

    pub fn open_edit_customer_modal(&mut self) {
        if !self.can_write() {
            self.status_msg =
                "Akses ditolak: Hanya Pemilik (Owner) dan Operator yang dapat mengubah pelanggan."
                    .into();
            return;
        }
        let Some(cid) = self.selected_customer_id.clone() else {
            self.status_msg = "Pilih pelanggan terlebih dahulu.".into();
            return;
        };
        let Ok(detail) = self.store.get_customer(&cid) else {
            self.status_msg = "Pelanggan tidak ditemukan.".into();
            return;
        };
        self.form_fields = vec![
            (
                "Kode Pelanggan (Kosongkan untuk menghapus kode)",
                detail.code.unwrap_or_default(),
            ),
            ("Nama Pelanggan (Wajib)", detail.name),
            (
                "Alamat Pelanggan (Opsional)",
                detail.address.unwrap_or_default(),
            ),
            (
                "Status Aktif (ketik: aktif / nonaktif)",
                if detail.is_active {
                    "aktif".into()
                } else {
                    "nonaktif".into()
                },
            ),
        ];
        self.form_active = 1;
        self.form_error.clear();
        self.active_modal = Some(ActiveModal::EditCustomer { customer_id: cid });
    }

    pub fn open_merge_customer_modal(&mut self) {
        if !self.is_owner() {
            self.status_msg =
                "Akses ditolak: Penggabungan (merge) pelanggan hanya dapat dilakukan oleh Pemilik (Owner)."
                    .into();
            return;
        }
        let Some(cid) = self.selected_customer_id.clone() else {
            self.status_msg = "Pilih pelanggan sumber terlebih dahulu.".into();
            return;
        };
        let targets = self.store.list_customers(&CustomerQuery {
            search: None,
            active_only: true,
        });
        let first_target = targets
            .iter()
            .find(|c| c.id != cid)
            .and_then(|c| c.code.clone().or_else(|| Some(c.name.clone())))
            .unwrap_or_default();

        self.form_fields = vec![
            (
                "Kode atau Nama Pelanggan Tujuan (Target Aktif)",
                first_target,
            ),
            (
                "Alasan Penggabungan Pelanggan (Wajib untuk Audit)",
                "Duplikasi data pelanggan".into(),
            ),
        ];
        self.form_active = 0;
        self.form_error.clear();
        self.active_modal = Some(ActiveModal::MergeCustomer { source_id: cid });
    }

    pub fn open_record_sale_modal(&mut self) {
        if !self.can_write() {
            self.status_msg =
                "Akses ditolak: Hanya Pemilik (Owner) dan Operator yang dapat mencatat penjualan kredit."
                    .into();
            return;
        }
        let Some(cid) = &self.selected_customer_id else {
            self.status_msg = "Pilih pelanggan terlebih dahulu.".into();
            return;
        };
        if let Ok(cust) = self.store.get_customer(cid) {
            if !cust.is_active {
                self.status_msg =
                    "Pelanggan ini berstatus Nonaktif. Aktifkan terlebih dahulu untuk mencatat penjualan."
                        .into();
                return;
            }
        }
        self.form_fields = vec![
            (
                "Tanggal Transaksi (YYYY-MM-DD)",
                self.today_jakarta().to_string(),
            ),
            ("No. Referensi / Nota (Opsional)", String::new()),
            (
                "Alasan Tanggal Lampau / Backdate (Wajib & khusus Owner jika tanggal < hari ini)",
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
                "Akses ditolak: Hanya Pemilik (Owner) dan Operator yang dapat mencatat pembayaran."
                    .into();
            return;
        }
        if self.selected_customer_id.is_none() {
            self.status_msg = "Pilih pelanggan terlebih dahulu.".into();
            return;
        }
        self.form_fields = vec![
            (
                "Nominal Pembayaran Diterima (Rp - Bilangan Bulat > 0)",
                String::new(),
            ),
            (
                "Tanggal Diterima (YYYY-MM-DD)",
                self.today_jakarta().to_string(),
            ),
            (
                "Metode Pembayaran (ketik: Tunai / Transfer Bank)",
                "Tunai".into(),
            ),
            ("Referensi Bukti / No. Transfer (Opsional)", String::new()),
            ("Catatan Tambahan (Opsional)", String::new()),
        ];
        self.form_active = 0;
        self.form_error.clear();
        self.active_modal = Some(ActiveModal::RecordPayment);
    }

    pub fn open_apply_deposit_modal_for_payment(&mut self, preferred_payment_id: Option<String>) {
        if !self.can_write() {
            self.status_msg =
                "Akses ditolak: Hanya Pemilik (Owner) dan Operator yang dapat menerapkan saldo kredit."
                    .into();
            return;
        }
        let Some(cid) = &self.selected_customer_id else {
            self.status_msg = "Pilih pelanggan terlebih dahulu.".into();
            return;
        };

        let payments = self.store.list_payments(cid).unwrap_or_default();
        let credit_payment = if let Some(pref_id) = preferred_payment_id {
            payments
                .iter()
                .find(|p| p.id == pref_id && p.unallocated_credit_idr > 0)
        } else {
            payments.iter().find(|p| p.unallocated_credit_idr > 0)
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
            .map(|page| page.entries)
            .unwrap_or_default();
        let open_debit = ledger
            .iter()
            .find(|e| e.remaining_idr > 0 && !e.is_reversed);

        let pay_id_default = credit_payment.map(|p| p.id.clone()).unwrap_or_default();
        let debit_id_default = open_debit.map(|e| e.id.clone()).unwrap_or_default();
        let suggested_amount = match (credit_payment, open_debit) {
            (Some(p), Some(d)) => p.unallocated_credit_idr.min(d.remaining_idr).to_string(),
            _ => String::new(),
        };

        self.form_fields = vec![
            ("ID Pembayaran Sumber Saldo Kredit", pay_id_default),
            (
                "ID Transaksi Debet (Piutang Terbuka) Tujuan",
                debit_id_default,
            ),
            ("Nominal Kredit Diterapkan (Rp)", suggested_amount),
            (
                "Alasan Penerapan Saldo Kredit (Wajib untuk Audit)",
                "Penerapan saldo lebih pembayaran ke nota terbuka".into(),
            ),
        ];
        self.form_active = 2;
        self.form_error.clear();
        self.active_modal = Some(ActiveModal::ApplyDeposit);
    }

    pub fn open_reallocate_payment_modal(&mut self, payment_id: &str) {
        if !self.is_owner() {
            self.status_msg =
                "Akses ditolak: Realokasi pembayaran hanya dapat dilakukan oleh Pemilik (Owner)."
                    .into();
            return;
        }
        let Some(cid) = &self.selected_customer_id else {
            return;
        };
        let payments = self.store.list_payments(cid).unwrap_or_default();
        let Some(payment) = payments.iter().find(|p| p.id == payment_id) else {
            return;
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
            .map(|page| page.entries)
            .unwrap_or_default();
        let first_debit = ledger
            .iter()
            .find(|e| e.debit_idr > 0 && !e.is_reversed)
            .map(|e| e.id.clone())
            .unwrap_or_default();

        self.form_fields = vec![
            (
                "ID Transaksi Debet Tujuan Baru (Kosongkan untuk melepas semua ke Saldo Kredit)",
                first_debit,
            ),
            (
                "Nominal Alokasi Baru (Rp, 0 s/d Nominal Pembayaran)",
                payment.amount_idr.to_string(),
            ),
            (
                "Alasan Realokasi Pembayaran (Wajib untuk Audit)",
                "Koreksi alokasi nota oleh Owner".into(),
            ),
        ];
        self.form_active = 1;
        self.form_error.clear();
        self.active_modal = Some(ActiveModal::ReallocatePayment {
            payment_id: payment_id.to_string(),
        });
    }

    pub fn open_reverse_modal(&mut self, entry_id: &str) {
        if !self.is_owner() {
            self.status_msg =
                "Akses ditolak: Pembalikan (reversal) transaksi hanya diizinkan untuk Pemilik (Owner)."
                    .into();
            return;
        }
        if let Some(cid) = &self.selected_customer_id {
            self.form_fields = vec![(
                "Alasan Pembalikan / Reversal Transaksi (Wajib untuk Audit)",
                String::new(),
            )];
            self.form_active = 0;
            self.form_error.clear();
            self.active_modal = Some(ActiveModal::ReverseTransaction {
                customer_id: cid.clone(),
                entry_id: entry_id.to_string(),
            });
        }
    }

    pub fn open_filter_ledger_date_modal(&mut self) {
        self.form_fields = vec![
            (
                "Dari Tanggal (YYYY-MM-DD, kosongkan untuk dari awal)",
                self.ledger_from.map(|d| d.to_string()).unwrap_or_default(),
            ),
            (
                "Sampai Tanggal (YYYY-MM-DD, kosongkan untuk sampai akhir)",
                self.ledger_to.map(|d| d.to_string()).unwrap_or_default(),
            ),
        ];
        self.form_active = 0;
        self.form_error.clear();
        self.active_modal = Some(ActiveModal::FilterLedgerDate);
    }

    pub fn open_new_user_modal(&mut self) {
        if !self.is_owner() {
            self.status_msg =
                "Akses ditolak: Hanya Pemilik (Owner) yang dapat mengelola akun pengguna.".into();
            return;
        }
        self.form_fields = vec![
            ("Nama Pengguna / Username (Wajib)", String::new()),
            (
                "Peran Hak Akses (owner / operator / viewer)",
                "operator".into(),
            ),
            (
                "Kata Sandi Sementara (Wajib Minimal 15 Karakter)",
                "sandisementara123".into(),
            ),
        ];
        self.form_active = 0;
        self.form_error.clear();
        self.active_modal = Some(ActiveModal::NewUser);
    }

    pub fn open_edit_user_modal(&mut self, user: &UserItem) {
        if !self.is_owner() {
            return;
        }
        self.form_fields = vec![
            ("Nama Pengguna / Username", user.username.clone()),
            (
                "Peran Hak Akses (owner / operator / viewer)",
                user.role.as_str().to_string(),
            ),
            (
                "Status Keaktifan (aktif / nonaktif)",
                if user.is_active {
                    "aktif".into()
                } else {
                    "nonaktif".into()
                },
            ),
        ];
        self.form_active = 0;
        self.form_error.clear();
        self.active_modal = Some(ActiveModal::EditUser {
            user_id: user.id.clone(),
        });
    }

    pub fn open_reset_user_password_modal(&mut self, user: &UserItem) {
        if !self.is_owner() {
            return;
        }
        self.form_fields = vec![(
            "Kata Sandi Sementara Baru (Wajib Minimal 15 Karakter)",
            "resetpassword12345".into(),
        )];
        self.form_active = 0;
        self.form_error.clear();
        self.active_modal = Some(ActiveModal::ResetUserPassword {
            user_id: user.id.clone(),
            username: user.username.clone(),
        });
    }

    pub fn open_login_modal(&mut self) {
        let sim = self.store.is_simulation();
        self.form_fields = vec![
            (
                if sim {
                    "Username (owner / operator_kasir / auditor_viewer)"
                } else {
                    "Nama Pengguna"
                },
                if sim { "owner".into() } else { String::new() },
            ),
            (
                if sim {
                    "Kata Sandi (default: password1234567)"
                } else {
                    "Kata Sandi"
                },
                if sim {
                    "password1234567".into()
                } else {
                    String::new()
                },
            ),
        ];
        self.form_active = 0;
        self.form_error.clear();
        self.active_modal = Some(ActiveModal::Login);
    }

    pub fn open_change_password_modal(&mut self) {
        self.form_fields = vec![
            ("Kata Sandi Saat Ini", String::new()),
            ("Kata Sandi Baru (Wajib Minimal 15 Karakter)", String::new()),
        ];
        self.form_active = 0;
        self.form_error.clear();
        self.active_modal = Some(ActiveModal::ChangePassword);
    }

    pub fn open_change_as_of_modal(&mut self) {
        self.form_fields = vec![(
            "Tanggal Acuan Laporan Umur Piutang (YYYY-MM-DD, maks hari ini)",
            self.aging_as_of.to_string(),
        )];
        self.form_active = 0;
        self.form_error.clear();
        self.active_modal = Some(ActiveModal::ChangeAsOfDate);
    }

    fn submit_modal(&mut self, cx: &mut Context<Self>) {
        let Some(modal) = self.active_modal.clone() else {
            return;
        };

        match modal {
            ActiveModal::Login => {
                let u = self.form_fields.get(0).map(|f| f.1.as_str()).unwrap_or("");
                let p = self.form_fields.get(1).map(|f| f.1.as_str()).unwrap_or("");
                match self.store.login(u, p) {
                    Ok(sess) => {
                        let must_change = sess.must_change_password;
                        self.status_msg = format!(
                            "Berhasil masuk sebagai {} ({}).",
                            sess.username,
                            sess.role.label()
                        );
                        if sess.role != UserRole::Owner
                            && self.current_view == CurrentView::UserManagement
                        {
                            self.current_view = CurrentView::CustomerList;
                        }
                        self.session = Some(sess);
                        self.active_modal = None;
                        if must_change {
                            self.open_change_password_modal();
                            self.form_error = "Akun menggunakan kata sandi sementara. Anda wajib menggantinya sebelum melanjutkan (AC-019).".into();
                        }
                    }
                    Err(e) => self.form_error = e.0,
                }
            }
            ActiveModal::ChangePassword => {
                let old_pw = self.form_fields.get(0).map(|f| f.1.as_str()).unwrap_or("");
                let new_pw = self.form_fields.get(1).map(|f| f.1.as_str()).unwrap_or("");
                match self.store.change_password(old_pw, new_pw) {
                    Ok(()) => {
                        self.session = self.store.current_session();
                        self.status_msg =
                            "Kata sandi berhasil diperbarui dan sesi lain telah dicabut.".into();
                        self.active_modal = None;
                    }
                    Err(e) => self.form_error = e.0,
                }
            }
            ActiveModal::NewCustomer => {
                let code_raw = self.form_fields.get(0).map(|f| f.1.trim()).unwrap_or("");
                let name = self
                    .form_fields
                    .get(1)
                    .map(|f| f.1.trim().to_string())
                    .unwrap_or_default();
                let addr_raw = self.form_fields.get(2).map(|f| f.1.trim()).unwrap_or("");

                let input = CreateCustomerInput {
                    code: if code_raw.is_empty() {
                        None
                    } else {
                        Some(code_raw.to_string())
                    },
                    name,
                    address: if addr_raw.is_empty() {
                        None
                    } else {
                        Some(addr_raw.to_string())
                    },
                };

                match self.store.create_customer(&input) {
                    Ok(detail) => {
                        self.status_msg =
                            format!("Pelanggan '{}' berhasil ditambahkan.", detail.name);
                        self.selected_customer_id = Some(detail.id);
                        self.current_view = CurrentView::CustomerDetail;
                        self.active_modal = None;
                    }
                    Err(e) => self.form_error = e.0,
                }
            }
            ActiveModal::EditCustomer { customer_id } => {
                let code_raw = self.form_fields.get(0).map(|f| f.1.trim()).unwrap_or("");
                let name = self
                    .form_fields
                    .get(1)
                    .map(|f| f.1.trim().to_string())
                    .unwrap_or_default();
                let addr_raw = self.form_fields.get(2).map(|f| f.1.trim()).unwrap_or("");
                let status_raw = self
                    .form_fields
                    .get(3)
                    .map(|f| f.1.trim().to_lowercase())
                    .unwrap_or_else(|| "aktif".into());
                let is_active =
                    status_raw != "nonaktif" && status_raw != "false" && status_raw != "0";

                let input = UpdateCustomerInput {
                    code: if code_raw.is_empty() {
                        None
                    } else {
                        Some(code_raw.to_string())
                    },
                    name,
                    address: if addr_raw.is_empty() {
                        None
                    } else {
                        Some(addr_raw.to_string())
                    },
                    is_active,
                };

                match self.store.update_customer(&customer_id, &input) {
                    Ok(detail) => {
                        self.status_msg = format!(
                            "Perubahan data pelanggan '{}' (v{}) berhasil disimpan.",
                            detail.name, detail.version
                        );
                        self.active_modal = None;
                    }
                    Err(e) => self.form_error = e.0,
                }
            }
            ActiveModal::MergeCustomer { source_id } => {
                let target_key = self
                    .form_fields
                    .get(0)
                    .map(|f| f.1.trim().to_lowercase())
                    .unwrap_or_default();
                let reason = self
                    .form_fields
                    .get(1)
                    .map(|f| f.1.trim().to_string())
                    .unwrap_or_default();

                let all_custs = self.store.list_customers(&CustomerQuery {
                    search: None,
                    active_only: false,
                });
                let found_target = all_custs.iter().find(|c| {
                    c.id != source_id
                        && (c.name.to_lowercase() == target_key
                            || c.code
                                .as_deref()
                                .map(|code| code.to_lowercase() == target_key)
                                .unwrap_or(false)
                            || c.id.to_lowercase() == target_key)
                });

                let Some(target) = found_target else {
                    self.form_error =
                        "Pelanggan tujuan tidak ditemukan. Masukkan Kode (misal P002) atau Nama Pelanggan tujuan yang tepat."
                            .into();
                    cx.notify();
                    return;
                };
                let target_id = target.id.clone();
                let target_name = target.name.clone();

                match self.store.merge_customers(&source_id, &target_id, &reason) {
                    Ok(()) => {
                        self.selected_customer_id = Some(target_id);
                        self.status_msg = format!(
                            "Pelanggan berhasil digabungkan ke '{}'. Sumber telah dinonaktifkan.",
                            target_name
                        );
                        self.active_modal = None;
                    }
                    Err(e) => self.form_error = e.0,
                }
            }
            ActiveModal::RecordSale => {
                let Some(cid) = self.selected_customer_id.clone() else {
                    return;
                };
                let date_str = self.form_fields.get(0).map(|f| f.1.trim()).unwrap_or("");
                let Ok(effective_date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") else {
                    self.form_error = "Format Tanggal Transaksi harus YYYY-MM-DD.".into();
                    cx.notify();
                    return;
                };
                let ref_raw = self.form_fields.get(1).map(|f| f.1.trim()).unwrap_or("");
                let reason_raw = self.form_fields.get(2).map(|f| f.1.trim()).unwrap_or("");

                let mut items = Vec::new();
                for (desc, qty, price_str) in &self.sale_items {
                    if desc.trim().is_empty() && price_str.trim().is_empty() {
                        continue;
                    }
                    let clean_price: String =
                        price_str.chars().filter(|c| c.is_ascii_digit()).collect();
                    let unit_price_idr: i64 = clean_price.parse().unwrap_or(0);
                    items.push(SaleItemInput {
                        description: desc.trim().to_string(),
                        quantity: qty.trim().to_string(),
                        unit_price_idr,
                    });
                }

                let input = InstantSaleInput {
                    effective_date,
                    reference_no: if ref_raw.is_empty() {
                        None
                    } else {
                        Some(ref_raw.to_string())
                    },
                    backdate_reason: if reason_raw.is_empty() {
                        None
                    } else {
                        Some(reason_raw.to_string())
                    },
                    items,
                };

                match self.store.create_instant_sale(&cid, &input) {
                    Ok(total) => {
                        self.status_msg = format!(
                            "Penjualan kredit sebesar {} berhasil diposting ke Buku Piutang.",
                            format_rupiah(total)
                        );
                        self.customer_tab = CustomerTab::Ledger;
                        self.active_modal = None;
                    }
                    Err(e) => self.form_error = e.0,
                }
            }
            ActiveModal::RecordPayment => {
                let Some(cid) = self.selected_customer_id.clone() else {
                    return;
                };
                let amt_raw = self.form_fields.get(0).map(|f| f.1.trim()).unwrap_or("");
                let clean_amt: String = amt_raw.chars().filter(|c| c.is_ascii_digit()).collect();
                let amount_idr: i64 = clean_amt.parse().unwrap_or(0);

                let date_str = self.form_fields.get(1).map(|f| f.1.trim()).unwrap_or("");
                let Ok(received_date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") else {
                    self.form_error = "Format Tanggal Diterima harus YYYY-MM-DD.".into();
                    cx.notify();
                    return;
                };

                let method = self
                    .form_fields
                    .get(2)
                    .map(|f| f.1.trim().to_string())
                    .unwrap_or_else(|| "Tunai".into());
                let ref_raw = self.form_fields.get(3).map(|f| f.1.trim()).unwrap_or("");
                let note_raw = self.form_fields.get(4).map(|f| f.1.trim()).unwrap_or("");

                let input = RecordPaymentInput {
                    received_date,
                    amount_idr,
                    method,
                    reference: if ref_raw.is_empty() {
                        None
                    } else {
                        Some(ref_raw.to_string())
                    },
                    note: if note_raw.is_empty() {
                        None
                    } else {
                        Some(note_raw.to_string())
                    },
                };

                match self.store.record_payment(&cid, &input) {
                    Ok(pay) => {
                        self.status_msg = format!(
                            "Pembayaran {} dicatat (Teralokasi FIFO: {}, Saldo Lebih/Kredit: {}).",
                            format_rupiah(pay.amount_idr),
                            format_rupiah(pay.allocated_idr),
                            format_rupiah(pay.unallocated_credit_idr)
                        );
                        self.active_modal = None;
                    }
                    Err(e) => self.form_error = e.0,
                }
            }
            ActiveModal::ApplyDeposit => {
                let Some(cid) = self.selected_customer_id.clone() else {
                    return;
                };
                let pay_id = self
                    .form_fields
                    .get(0)
                    .map(|f| f.1.trim().to_string())
                    .unwrap_or_default();
                let debit_id = self
                    .form_fields
                    .get(1)
                    .map(|f| f.1.trim().to_string())
                    .unwrap_or_default();
                let amt_raw = self.form_fields.get(2).map(|f| f.1.trim()).unwrap_or("");
                let clean_amt: String = amt_raw.chars().filter(|c| c.is_ascii_digit()).collect();
                let amount_idr: i64 = clean_amt.parse().unwrap_or(0);
                let reason = self
                    .form_fields
                    .get(3)
                    .map(|f| f.1.trim().to_string())
                    .unwrap_or_default();

                let input = ApplyCreditInput {
                    payment_id: pay_id,
                    debit_entry_id: debit_id,
                    amount_idr,
                    reason,
                };

                match self.store.apply_credit(&cid, &input) {
                    Ok(()) => {
                        self.status_msg = format!(
                            "Saldo kredit sebesar {} berhasil diterapkan ke nota piutang.",
                            format_rupiah(amount_idr)
                        );
                        self.active_modal = None;
                    }
                    Err(e) => self.form_error = e.0,
                }
            }
            ActiveModal::ReallocatePayment { payment_id } => {
                let Some(cid) = self.selected_customer_id.clone() else {
                    return;
                };
                let debit_raw = self
                    .form_fields
                    .get(0)
                    .map(|f| f.1.trim().to_string())
                    .unwrap_or_default();
                let amt_raw = self.form_fields.get(1).map(|f| f.1.trim()).unwrap_or("");
                let clean_amt: String = amt_raw.chars().filter(|c| c.is_ascii_digit()).collect();
                let amount_idr: i64 = clean_amt.parse().unwrap_or(0);
                let reason = self
                    .form_fields
                    .get(2)
                    .map(|f| f.1.trim().to_string())
                    .unwrap_or_default();

                let input = ReallocatePaymentInput {
                    payment_id,
                    debit_entry_id: if debit_raw.is_empty() {
                        None
                    } else {
                        Some(debit_raw)
                    },
                    amount_idr,
                    reason,
                };

                match self.store.reallocate_payment(&cid, &input) {
                    Ok(()) => {
                        self.status_msg =
                            "Realokasi pembayaran berhasil disimpan secara atomik.".into();
                        self.active_modal = None;
                    }
                    Err(e) => self.form_error = e.0,
                }
            }
            ActiveModal::ReverseTransaction {
                customer_id,
                entry_id,
            } => {
                let reason = self
                    .form_fields
                    .get(0)
                    .map(|f| f.1.trim().to_string())
                    .unwrap_or_default();
                match self
                    .store
                    .reverse_transaction(&customer_id, &entry_id, &reason)
                {
                    Ok(()) => {
                        self.status_msg =
                            "Transaksi berhasil dibalik (reversed) dengan kontra-entry dan alokasi dipulihkan."
                                .into();
                        self.active_modal = None;
                    }
                    Err(e) => self.form_error = e.0,
                }
            }
            ActiveModal::FilterLedgerDate => {
                let from_raw = self.form_fields.get(0).map(|f| f.1.trim()).unwrap_or("");
                let to_raw = self.form_fields.get(1).map(|f| f.1.trim()).unwrap_or("");

                let from_date = if from_raw.is_empty() {
                    None
                } else {
                    match NaiveDate::parse_from_str(from_raw, "%Y-%m-%d") {
                        Ok(d) => Some(d),
                        Err(_) => {
                            self.form_error = "Format Dari Tanggal harus YYYY-MM-DD.".into();
                            cx.notify();
                            return;
                        }
                    }
                };
                let to_date = if to_raw.is_empty() {
                    None
                } else {
                    match NaiveDate::parse_from_str(to_raw, "%Y-%m-%d") {
                        Ok(d) => Some(d),
                        Err(_) => {
                            self.form_error = "Format Sampai Tanggal harus YYYY-MM-DD.".into();
                            cx.notify();
                            return;
                        }
                    }
                };
                if let (Some(f), Some(t)) = (from_date, to_date) {
                    if f > t {
                        self.form_error =
                            "Dari Tanggal tidak boleh lebih besar dari Sampai Tanggal.".into();
                        cx.notify();
                        return;
                    }
                }
                self.ledger_from = from_date;
                self.ledger_to = to_date;
                self.status_msg = "Filter rentang tanggal buku piutang diterapkan.".into();
                self.active_modal = None;
            }
            ActiveModal::NewUser => {
                let username = self
                    .form_fields
                    .get(0)
                    .map(|f| f.1.trim().to_string())
                    .unwrap_or_default();
                let role_str = self
                    .form_fields
                    .get(1)
                    .map(|f| f.1.trim().to_lowercase())
                    .unwrap_or_default();
                let temp_pw = self
                    .form_fields
                    .get(2)
                    .map(|f| f.1.clone())
                    .unwrap_or_default();

                let role = match role_str.as_str() {
                    "owner" | "pemilik" => UserRole::Owner,
                    "viewer" | "pembaca" => UserRole::Viewer,
                    _ => UserRole::Operator,
                };

                let input = CreateUserInput {
                    username,
                    role,
                    temporary_password: temp_pw,
                };

                match self.store.create_user(&input) {
                    Ok(u) => {
                        self.status_msg = format!(
                            "Akun '{}' ({}) berhasil dibuat. Wajib ganti kata sandi saat login pertama.",
                            u.username,
                            u.role.label()
                        );
                        self.active_modal = None;
                    }
                    Err(e) => self.form_error = e.0,
                }
            }
            ActiveModal::EditUser { user_id } => {
                let username = self
                    .form_fields
                    .get(0)
                    .map(|f| f.1.trim().to_string())
                    .unwrap_or_default();
                let role_str = self
                    .form_fields
                    .get(1)
                    .map(|f| f.1.trim().to_lowercase())
                    .unwrap_or_default();
                let status_str = self
                    .form_fields
                    .get(2)
                    .map(|f| f.1.trim().to_lowercase())
                    .unwrap_or_else(|| "aktif".into());

                let role = match role_str.as_str() {
                    "owner" | "pemilik" => UserRole::Owner,
                    "viewer" | "pembaca" => UserRole::Viewer,
                    _ => UserRole::Operator,
                };
                let is_active =
                    status_str != "nonaktif" && status_str != "false" && status_str != "0";

                match self.store.update_user(
                    &user_id,
                    &UpdateUserInput {
                        username,
                        role,
                        is_active,
                    },
                ) {
                    Ok(u) => {
                        self.status_msg = format!("Akun '{}' berhasil diperbarui.", u.username);
                        self.active_modal = None;
                    }
                    Err(e) => self.form_error = e.0,
                }
            }
            ActiveModal::ResetUserPassword { user_id, username } => {
                let temp_pw = self
                    .form_fields
                    .get(0)
                    .map(|f| f.1.clone())
                    .unwrap_or_default();
                match self.store.reset_password(&user_id, &temp_pw) {
                    Ok(()) => {
                        self.status_msg = format!(
                            "Kata sandi akun '{}' berhasil direset. Pengguna wajib mengganti sandi saat login.",
                            username
                        );
                        self.active_modal = None;
                    }
                    Err(e) => self.form_error = e.0,
                }
            }
            ActiveModal::ChangeAsOfDate => {
                let date_str = self.form_fields.get(0).map(|f| f.1.trim()).unwrap_or("");
                match NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                    Ok(d) => {
                        if d > self.today_jakarta() {
                            self.form_error =
                                "Tanggal acuan tidak boleh berada di masa depan (API-501).".into();
                        } else {
                            self.aging_as_of = d;
                            self.status_msg =
                                format!("Tanggal acuan laporan aging diubah ke {}.", d);
                            self.active_modal = None;
                        }
                    }
                    Err(_) => {
                        self.form_error = "Format tanggal harus YYYY-MM-DD.".into();
                    }
                }
            }
            ActiveModal::Notice { .. } => {
                self.active_modal = None;
            }
        }
        self.session = self.store.current_session();
        cx.notify();
    }

    fn key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke;

        // 1. Penanganan input saat Modal Dialog terbuka
        if let Some(modal) = &self.active_modal {
            match key.key.as_str() {
                "escape" => {
                    self.active_modal = None;
                    cx.notify();
                    return;
                }
                "enter" => {
                    self.submit_modal(cx);
                    return;
                }
                "tab" => {
                    if *modal == ActiveModal::RecordSale {
                        let (r, c) = self.sale_active_field;
                        if r < self.sale_items.len() {
                            if c < 2 {
                                self.sale_active_field = (r, c + 1);
                            } else if r + 1 < self.sale_items.len() {
                                self.sale_active_field = (r + 1, 0);
                            } else {
                                self.sale_active_field = (self.sale_items.len(), 0);
                            }
                        } else if !self.form_fields.is_empty() {
                            let next_f = (c + 1) % self.form_fields.len();
                            if next_f == 0 {
                                self.sale_active_field = (0, 0);
                            } else {
                                self.sale_active_field = (self.sale_items.len(), next_f);
                            }
                        }
                    } else if !self.form_fields.is_empty() {
                        self.form_active = (self.form_active + 1) % self.form_fields.len();
                    }
                    cx.notify();
                    return;
                }
                "backspace" => {
                    if *modal == ActiveModal::RecordSale {
                        let (r, c) = self.sale_active_field;
                        if r < self.sale_items.len() {
                            if let Some(item) = self.sale_items.get_mut(r) {
                                match c {
                                    0 => {
                                        item.0.pop();
                                    }
                                    1 => {
                                        item.1.pop();
                                    }
                                    _ => {
                                        item.2.pop();
                                    }
                                }
                            }
                        } else if let Some(field) = self.form_fields.get_mut(c) {
                            field.1.pop();
                        }
                    } else if let Some(field) = self.form_fields.get_mut(self.form_active) {
                        field.1.pop();
                    }
                    cx.notify();
                    return;
                }
                _ => {
                    if let Some(ch) = &key.key_char {
                        if !ch.chars().any(char::is_control) {
                            if *modal == ActiveModal::RecordSale {
                                let (r, c) = self.sale_active_field;
                                if r < self.sale_items.len() {
                                    if let Some(item) = self.sale_items.get_mut(r) {
                                        match c {
                                            0 => item.0.push_str(ch),
                                            1 => item.1.push_str(ch),
                                            _ => item.2.push_str(ch),
                                        }
                                    }
                                } else if let Some(field) = self.form_fields.get_mut(c) {
                                    field.1.push_str(ch);
                                }
                            } else if let Some(field) = self.form_fields.get_mut(self.form_active) {
                                field.1.push_str(ch);
                            }
                            cx.notify();
                            return;
                        }
                    } else if key.key == "space" {
                        if *modal == ActiveModal::RecordSale {
                            let (r, c) = self.sale_active_field;
                            if r < self.sale_items.len() {
                                if let Some(item) = self.sale_items.get_mut(r) {
                                    if c == 0 {
                                        item.0.push(' ');
                                    }
                                }
                            } else if let Some(field) = self.form_fields.get_mut(c) {
                                field.1.push(' ');
                            }
                        } else if let Some(field) = self.form_fields.get_mut(self.form_active) {
                            field.1.push(' ');
                        }
                        cx.notify();
                        return;
                    }
                }
            }
            return;
        }

        // 2. Pintasan Global & Kontekstual Halaman
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
        if key.modifiers.control && key.key == "n" && self.current_view == CurrentView::CustomerList
        {
            self.open_new_customer_modal();
            cx.notify();
            return;
        }
        if key.modifiers.control
            && key.key == "e"
            && self.current_view == CurrentView::CustomerDetail
        {
            self.open_edit_customer_modal();
            cx.notify();
            return;
        }
        if key.modifiers.control
            && key.key == "m"
            && self.current_view == CurrentView::CustomerDetail
        {
            self.open_merge_customer_modal();
            cx.notify();
            return;
        }

        match key.key.as_str() {
            "f2" => {
                self.current_view = CurrentView::CustomerList;
                self.status_msg = "Modul: Buku Ringkasan Saldo Pelanggan.".into();
                cx.notify();
            }
            "f3" => {
                if self.current_view == CurrentView::CustomerDetail {
                    self.open_record_sale_modal();
                    cx.notify();
                }
            }
            "f4" => {
                if self.current_view == CurrentView::CustomerDetail {
                    self.open_record_payment_modal();
                    cx.notify();
                }
            }
            "f5" => {
                if self.current_view == CurrentView::CustomerDetail {
                    self.open_apply_deposit_modal_for_payment(None);
                    cx.notify();
                }
            }
            "f6" => {
                self.current_view = CurrentView::AgingReport;
                self.status_msg = "Modul: Laporan Analisis Umur Piutang (Aging Report).".into();
                cx.notify();
            }
            "f9" => {
                self.store.invalidate_cache();
                self.session = self.store.restore_session();
                if self.session.is_some() {
                    self.status_msg = "Data dimuat ulang dari sumber (cache dibersihkan).".into();
                } else {
                    self.status_msg = "Sesi berakhir. Silakan masuk kembali.".into();
                    self.open_login_modal();
                }
                cx.notify();
            }
            "f8" => {
                if self.is_owner() {
                    self.current_view = CurrentView::UserManagement;
                    self.status_msg = "Modul: Manajemen Akun & Otorisasi Pengguna.".into();
                } else {
                    self.status_msg =
                        "Akses ditolak: Manajemen Pengguna hanya untuk Pemilik (Owner).".into();
                }
                cx.notify();
            }
            "enter" => {
                if self.current_view == CurrentView::CustomerList
                    && self.selected_customer_id.is_some()
                {
                    self.current_view = CurrentView::CustomerDetail;
                    self.status_msg = "Membuka detail buku piutang pelanggan terpilih.".into();
                    cx.notify();
                }
            }
            "up" | "down" => {
                if self.current_view == CurrentView::CustomerList {
                    let list = self.store.list_customers(&CustomerQuery {
                        search: if self.query.is_empty() {
                            None
                        } else {
                            Some(self.query.clone())
                        },
                        active_only: self.customer_active_only,
                    });
                    if !list.is_empty() {
                        let cur_idx = self
                            .selected_customer_id
                            .as_ref()
                            .and_then(|id| list.iter().position(|c| &c.id == id))
                            .unwrap_or(0);
                        let next_idx = if key.key == "down" {
                            (cur_idx + 1).min(list.len() - 1)
                        } else {
                            cur_idx.saturating_sub(1)
                        };
                        self.selected_customer_id = Some(list[next_idx].id.clone());
                        cx.notify();
                    }
                }
            }
            "escape" => {
                if self.current_view == CurrentView::CustomerDetail {
                    self.current_view = CurrentView::CustomerList;
                    self.status_msg = "Kembali ke Buku Ringkasan Saldo Pelanggan.".into();
                } else if self.current_view == CurrentView::CustomerList && !self.query.is_empty() {
                    self.query.clear();
                    self.status_msg = "Pencarian pelanggan dibersihkan.".into();
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
        if let Some(e) = self.store.take_last_error() {
            self.status_msg = e;
        }
        let is_sim = self.store.is_simulation();
        let is_owner = self.is_owner();
        let must_change_pw = self
            .session
            .as_ref()
            .map(|s| s.must_change_password)
            .unwrap_or(false);

        let user_display = match &self.session {
            Some(u) => format!("{} [{}]", u.username, u.role.label()),
            None => "Belum Masuk".into(),
        };

        let date_label = format!("Asia/Jakarta · {}", self.today_jakarta().format("%d %b %Y"));

        let selected_cust_summary = self
            .selected_customer_id
            .as_ref()
            .and_then(|id| self.store.get_customer(id).ok());

        div()
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key_down))
            .size_full()
            .bg(rgb(Theme::BG_APP))
            .flex()
            .flex_col()
            .overflow_hidden()
            // 1. NATIVE WINDOWS SYSTEM MENU STRIP (26px)
            .child(
                render_system_menu_bar()
                    // Left: Traditional Native System Menus
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(2.0))
                            .child(
                                system_menu_item("Ganti Akun", "Ctrl+L", false)
                                    .id("sys-menu-login")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.open_login_modal();
                                        cx.notify();
                                    })),
                            )
                            .child(
                                system_menu_item("Ubah Sandi", "Ctrl+P", false)
                                    .id("sys-menu-pw")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.open_change_password_modal();
                                        cx.notify();
                                    })),
                            )
                            .child(tool_separator())
                            .child(
                                system_menu_item(
                                    "Pelanggan",
                                    "F2",
                                    self.current_view == CurrentView::CustomerList
                                        || self.current_view == CurrentView::CustomerDetail,
                                )
                                .id("sys-menu-cust")
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.current_view = CurrentView::CustomerList;
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                system_menu_item(
                                    "Laporan Aging",
                                    "F6",
                                    self.current_view == CurrentView::AgingReport,
                                )
                                .id("sys-menu-aging")
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.current_view = CurrentView::AgingReport;
                                        cx.notify();
                                    },
                                )),
                            )
                            .when(is_owner, |d| {
                                d.child(
                                    system_menu_item(
                                        "Manajemen Akun",
                                        "F8",
                                        self.current_view == CurrentView::UserManagement,
                                    )
                                    .id("sys-menu-users")
                                    .on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.current_view = CurrentView::UserManagement;
                                            cx.notify();
                                        },
                                    )),
                                )
                            }),
                    )
                    // Right: Quick Role Switcher (untuk simulasi izin Owner / Operator / Viewer)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .text_color(rgb(Theme::TEXT_MUTED))
                                    .child(if is_sim {
                                        "Simulasi Hak Akses:".to_string()
                                    } else {
                                        format!("Terhubung: {}", self.store.adapter_label())
                                    }),
                            )
                            .child(self.render_role_switch_chip(
                                "role-sw-owner",
                                "Owner",
                                UserRole::Owner,
                                cx,
                            ))
                            .child(self.render_role_switch_chip(
                                "role-sw-op",
                                "Operator",
                                UserRole::Operator,
                                cx,
                            ))
                            .child(self.render_role_switch_chip(
                                "role-sw-view",
                                "Viewer",
                                UserRole::Viewer,
                                cx,
                            )),
                    ),
            )
            // 2. MAIN WORKSPACE: LEFT NATIVE EXPLORER SIDEBAR + RIGHT PAGE PANEL
            .child(
                div()
                    .flex_1()
                    .flex()
                    .overflow_hidden()
                    // Left Docked Explorer Sidebar (210px)
                    .child(
                        render_left_sidebar_container()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(2.0))
                                    .child(sidebar_section_header("MODUL UTAMA"))
                                    .child(
                                        sidebar_nav_item(
                                            "▤",
                                            "Ringkasan Pelanggan",
                                            "F2",
                                            self.current_view == CurrentView::CustomerList,
                                        )
                                        .id("side-nav-cust-list")
                                        .on_click(
                                            cx.listener(|this, _, _, cx| {
                                                this.current_view = CurrentView::CustomerList;
                                                cx.notify();
                                            }),
                                        ),
                                    )
                                    // Sub-node pelanggan yang sedang aktif/terpilih
                                    .when_some(selected_cust_summary, |d, cust| {
                                        let is_detail =
                                            self.current_view == CurrentView::CustomerDetail;
                                        let label = match &cust.code {
                                            Some(c) => format!("└ [{}] {}", c, cust.name),
                                            None => format!("└ {}", cust.name),
                                        };
                                        d.child(
                                            div()
                                                .id("side-nav-cust-detail")
                                                .h(px(26.0))
                                                .mx(px(6.0))
                                                .pl(px(18.0))
                                                .pr(px(8.0))
                                                .rounded(px(3.0))
                                                .bg(rgb(if is_detail {
                                                    Theme::BG_SIDEBAR_ACTIVE
                                                } else {
                                                    Theme::BG_SIDEBAR
                                                }))
                                                .border_1()
                                                .border_color(rgb(if is_detail {
                                                    Theme::PRIMARY_BORDER
                                                } else {
                                                    Theme::BG_SIDEBAR
                                                }))
                                                .flex()
                                                .items_center()
                                                .overflow_hidden()
                                                .whitespace_nowrap()
                                                .cursor_pointer()
                                                .hover(|el| el.bg(rgb(0xE2E8F0)))
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.current_view = CurrentView::CustomerDetail;
                                                    cx.notify();
                                                }))
                                                .child(
                                                    div()
                                                        .text_size(px(11.0))
                                                        .font_weight(if is_detail {
                                                            FontWeight::BOLD
                                                        } else {
                                                            FontWeight::MEDIUM
                                                        })
                                                        .text_color(rgb(if is_detail {
                                                            Theme::PRIMARY_HOVER
                                                        } else {
                                                            Theme::TEXT_SECONDARY
                                                        }))
                                                        .child(label),
                                                ),
                                        )
                                    })
                                    .child(
                                        sidebar_nav_item(
                                            "◷",
                                            "Laporan Umur Piutang",
                                            "F6",
                                            self.current_view == CurrentView::AgingReport,
                                        )
                                        .id("side-nav-aging")
                                        .on_click(
                                            cx.listener(|this, _, _, cx| {
                                                this.current_view = CurrentView::AgingReport;
                                                cx.notify();
                                            }),
                                        ),
                                    )
                                    .child(sidebar_section_header("ADMINISTRASI & SESI"))
                                    .when(is_owner, |d| {
                                        d.child(
                                            sidebar_nav_item(
                                                "⚙",
                                                "Manajemen Pengguna",
                                                "F8",
                                                self.current_view == CurrentView::UserManagement,
                                            )
                                            .id("side-nav-users")
                                            .on_click(
                                                cx.listener(|this, _, _, cx| {
                                                    this.current_view = CurrentView::UserManagement;
                                                    cx.notify();
                                                }),
                                            ),
                                        )
                                    })
                                    .child(
                                        sidebar_nav_item("🔑", "Ubah Kata Sandi", "Ctrl+P", false)
                                            .id("side-nav-pw")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.open_change_password_modal();
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        sidebar_nav_item(
                                            "⇄",
                                            "Ganti Akun / Login",
                                            "Ctrl+L",
                                            false,
                                        )
                                        .id("side-nav-login")
                                        .on_click(
                                            cx.listener(|this, _, _, cx| {
                                                this.open_login_modal();
                                                cx.notify();
                                            }),
                                        ),
                                    ),
                            )
                            // Bottom Docked Session & Shortcut Panel
                            .child(self.render_sidebar_footer_panel()),
                    )
                    // Right Workspace Area (Contextual Toolbar + Content)
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .child(if must_change_pw {
                                self.render_password_change_required_view(cx)
                            } else {
                                match self.current_view {
                                    CurrentView::CustomerList => self.render_customer_list_view(cx),
                                    CurrentView::CustomerDetail => {
                                        self.render_customer_detail_view(cx)
                                    }
                                    CurrentView::AgingReport => self.render_aging_report_view(cx),
                                    CurrentView::UserManagement => {
                                        self.render_user_management_view(cx)
                                    }
                                }
                            }),
                    ),
            )
            // 3. NATIVE DESKTOP STATUS BAR (24px)
            .child(
                StatusBar::new(
                    &self.status_msg,
                    self.active_module_name(),
                    &date_label,
                    &user_display,
                )
                .adapter(self.store.adapter_label())
                .render(),
            )
            // 4. MODAL DIALOGS OVERLAY
            .children(self.render_modal_overlay(cx))
    }
}

impl Desktop {
    fn render_role_switch_chip(
        &self,
        id: &'static str,
        label: &'static str,
        role: UserRole,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_current = self
            .session
            .as_ref()
            .map(|s| s.role == role)
            .unwrap_or(false);
        div()
            .id(id)
            .when(!self.store.is_simulation(), |d| d.hidden())
            .h(px(19.0))
            .px(px(7.0))
            .rounded(px(2.0))
            .bg(rgb(if is_current {
                Theme::PRIMARY
            } else {
                Theme::BG_CARD
            }))
            .border_1()
            .border_color(rgb(if is_current {
                Theme::PRIMARY_HOVER
            } else {
                Theme::BORDER_STRONG
            }))
            .flex()
            .items_center()
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.store.switch_session_role(role);
                this.session = this.store.current_session();
                if role != UserRole::Owner && this.current_view == CurrentView::UserManagement {
                    this.current_view = CurrentView::CustomerList;
                }
                this.status_msg = format!("Beralih simulasi sesi ke peran {}.", role.label());
                cx.notify();
            }))
            .child(
                div()
                    .text_size(px(10.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(if is_current {
                        0xFFFFFF
                    } else {
                        Theme::TEXT_SECONDARY
                    }))
                    .child(label),
            )
    }

    fn render_sidebar_footer_panel(&self) -> Div {
        let shortcut_hint = match self.current_view {
            CurrentView::CustomerList => "Ketik: Cari • ↑↓: Pilih • Enter: Detail • Ctrl+N: Baru",
            CurrentView::CustomerDetail => "F3: Belanja • F4: Bayar • F5: Kredit • Esc: Kembali",
            CurrentView::AgingReport => "Klik kartu umur untuk filter rincian pelanggan",
            CurrentView::UserManagement => "Kelola pengguna, peran, dan reset sandi sementara",
        };

        div()
            .m(px(6.0))
            .p(px(8.0))
            .bg(rgb(Theme::BG_CARD))
            .border_1()
            .border_color(rgb(Theme::BORDER_DEFAULT))
            .rounded(px(3.0))
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(
                div()
                    .text_size(px(10.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(Theme::TEXT_MUTED))
                    .child("PETUNJUK HALAMAN AKTIF"),
            )
            .child(
                div()
                    .text_size(px(10.5))
                    .text_color(rgb(Theme::TEXT_SECONDARY))
                    .child(shortcut_hint),
            )
    }

    fn render_password_change_required_view(&self, cx: &mut Context<Self>) -> Div {
        div()
            .flex_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .p(px(24.0))
            .gap(px(12.0))
            .child(
                div()
                    .w(px(460.0))
                    .p(px(20.0))
                    .bg(rgb(Theme::BG_CARD))
                    .border_1()
                    .border_color(rgb(Theme::BADGE_WARNING_BORDER))
                    .rounded(px(4.0))
                    .flex()
                    .flex_col()
                    .gap(px(10.0))
                    .child(
                        div()
                            .text_size(px(14.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(Theme::BADGE_WARNING_TEXT))
                            .child("Penggantian Kata Sandi Sementara Diwajibkan (AC-019)"),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(rgb(Theme::TEXT_SECONDARY))
                            .child("Akun Anda menggunakan kata sandi sementara setelah pembuatan atau reset. Sesuai kebijakan keamanan, akses ke modul bisnis dikunci hingga Anda mengganti kata sandi (minimal 15 karakter)."),
                    )
                    .child(
                        primary_action_button("Ganti Kata Sandi Sekarang", "Ctrl+P")
                            .id("btn-force-change-pw")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_change_password_modal();
                                cx.notify();
                            })),
                    ),
            )
    }

    // =========================================================================
    // HALAMAN 1: BUKU RINGKASAN SALDO PELANGGAN (CustomerList)
    // Toolbar kontekstual: HANYA tombol terkait direktori pelanggan & pencarian
    // =========================================================================
    fn render_customer_list_view(&self, cx: &mut Context<Self>) -> Div {
        let q = CustomerQuery {
            search: if self.query.is_empty() {
                None
            } else {
                Some(self.query.clone())
            },
            active_only: self.customer_active_only,
        };
        let customers = self.store.list_customers(&q);
        let total_customers = customers.len();
        let total_piutang: i64 = customers.iter().map(|c| c.outstanding_idr).sum();
        let total_deposit: i64 = customers.iter().map(|c| c.customer_credit_idr).sum();
        let total_over_60: i64 = customers.iter().map(|c| c.aging.days_over_60).sum();

        // Kolom persis seperti Buku Ringkasan Saldo Pelanggan di Web & CustomerSummary Contract
        let columns = vec![
            GridColumn {
                title: "KODE",
                width_px: 72.0,
                align: ColumnAlign::Center,
            },
            GridColumn {
                title: "NAMA PELANGGAN",
                width_px: 195.0,
                align: ColumnAlign::Left,
            },
            GridColumn {
                title: "STATUS",
                width_px: 72.0,
                align: ColumnAlign::Center,
            },
            GridColumn {
                title: "TOTAL PIUTANG",
                width_px: 120.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "SALDO KREDIT",
                width_px: 110.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "0–15 HARI",
                width_px: 95.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "16–30 HARI",
                width_px: 95.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "31–45 HARI",
                width_px: 95.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "46–60 HARI",
                width_px: 95.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: ">60 HARI",
                width_px: 110.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "PERLU DITINJAU",
                width_px: 110.0,
                align: ColumnAlign::Right,
            },
        ];

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
                                    primary_action_button("+ Tambah Pelanggan", "Ctrl+N")
                                        .id("tool-cust-add")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.open_new_customer_modal();
                                            cx.notify();
                                        })),
                                )
                            })
                            .when(self.selected_customer_id.is_some(), |d| {
                                d.child(
                                    secondary_action_button("Buka Detail Buku Piutang", "Enter")
                                        .id("tool-cust-open")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.current_view = CurrentView::CustomerDetail;
                                            cx.notify();
                                        })),
                                )
                            })
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
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.customer_active_only = !this.customer_active_only;
                                    cx.notify();
                                })),
                            ),
                    )
                    // Right: Contextual Search Box
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .w(px(260.0))
                                    .h(px(26.0))
                                    .bg(rgb(Theme::BG_CARD))
                                    .border_1()
                                    .border_color(rgb(if !self.query.is_empty() {
                                        Theme::FOCUS_BORDER
                                    } else {
                                        Theme::BORDER_STRONG
                                    }))
                                    .when(!self.query.is_empty(), |d| {
                                        d.shadow(vec![gpui::BoxShadow {
                                            color: gpui::rgba(0x2563EB33).into(),
                                            offset: gpui::point(px(0.0), px(0.0)),
                                            blur_radius: px(2.5),
                                            spread_radius: px(1.5),
                                        }])
                                    })
                                    .rounded(px(3.0))
                                    .px(px(8.0))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(if self.query.is_empty() {
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.0))
                                            .child(
                                                div()
                                                    .text_size(px(10.5))
                                                    .text_color(rgb(Theme::TEXT_MUTED))
                                                    .child("🔍"),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(11.0))
                                                    .text_color(rgb(Theme::TEXT_LIGHT))
                                                    .child("Cari nama/kode (ketik langsung)..."),
                                            )
                                    } else {
                                        div()
                                            .flex()
                                            .items_center()
                                            .child(
                                                div()
                                                    .text_size(px(11.0))
                                                    .font_weight(FontWeight::BOLD)
                                                    .text_color(rgb(Theme::PRIMARY))
                                                    .child(self.query.clone()),
                                            )
                                            .child(
                                                div()
                                                    .w(px(2.0))
                                                    .h(px(14.0))
                                                    .bg(rgb(Theme::CURSOR_COLOR))
                                                    .ml(px(1.5)),
                                            )
                                    }),
                            )
                            .when(!self.query.is_empty(), |d| {
                                d.child(
                                    secondary_action_button("Reset Cari", "Esc")
                                        .id("tool-cust-clear-search")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.query.clear();
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
                    // Native DBGrid Container
                    .child(
                        div()
                            .flex_1()
                            .bg(rgb(Theme::BG_CARD))
                            .border_1()
                            .border_color(rgb(Theme::BORDER_STRONG))
                            .rounded(px(3.0))
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .child(render_grid_header(&columns))
                            .child(
                                div()
                                    .id("cust-list-scroll")
                                    .flex_1()
                                    .children(if customers.is_empty() {
                                        vec![
                                            render_empty_state(
                                                "Tidak ada pelanggan yang ditemukan.",
                                            )
                                            .into_any_element(),
                                        ]
                                    } else {
                                        customers
                                            .into_iter()
                                            .enumerate()
                                            .map(|(idx, cust)| {
                                                let is_selected =
                                                    self.selected_customer_id.as_deref()
                                                        == Some(&cust.id);
                                                let bg_color = if is_selected {
                                                    Theme::BG_ROW_SELECTED
                                                } else if idx % 2 == 1 {
                                                    Theme::BG_ROW_ALT
                                                } else {
                                                    Theme::BG_CARD
                                                };

                                                let cust_id = cust.id.clone();
                                                let cust_name = cust.name.clone();
                                                let code_str = cust
                                                    .code
                                                    .clone()
                                                    .unwrap_or_else(|| "—".into());
                                                let exc_total = cust.aging.exception_total();

                                                div()
                                                    .id(("cust-row", idx))
                                                    .h(px(28.0))
                                                    .bg(rgb(bg_color))
                                                    .border_b_1()
                                                    .border_color(rgb(Theme::GRID_LINE))
                                                    .flex()
                                                    .items_center()
                                                    .cursor_pointer()
                                                    .when(!is_selected, |d| {
                                                        d.hover(|d| d.bg(rgb(Theme::BG_ROW_HOVER)))
                                                    })
                                                    .on_click(cx.listener(
                                                        move |this, _, _, cx| {
                                                            this.selected_customer_id =
                                                                Some(cust_id.clone());
                                                            this.current_view =
                                                                CurrentView::CustomerDetail;
                                                            this.status_msg = format!(
                                                                "Membuka rincian buku piutang pelanggan '{}'.",
                                                                cust_name
                                                            );
                                                            cx.notify();
                                                        },
                                                    ))
                                                    .child(grid_row_indicator(
                                                        idx + 1,
                                                        is_selected,
                                                    ))
                                                    .child(grid_cell(
                                                        code_str,
                                                        72.0,
                                                        ColumnAlign::Center,
                                                        is_selected,
                                                    ))
                                                    .child(grid_cell_styled(
                                                        cust.name,
                                                        195.0,
                                                        ColumnAlign::Left,
                                                        Theme::TEXT_PRIMARY,
                                                        FontWeight::BOLD,
                                                        None,
                                                    ))
                                                    .child(if cust.is_active {
                                                        grid_badge(
                                                            "Aktif",
                                                            Theme::BADGE_CREDIT_TEXT,
                                                            Theme::BADGE_CREDIT_BG,
                                                            Theme::BADGE_CREDIT_BORDER,
                                                            72.0,
                                                        )
                                                    } else {
                                                        grid_badge(
                                                            "Nonaktif",
                                                            Theme::TEXT_MUTED,
                                                            0xF1F5F9,
                                                            0xCBD5E1,
                                                            72.0,
                                                        )
                                                    })
                                                    .child(grid_cell_styled(
                                                        format_rupiah(cust.outstanding_idr),
                                                        120.0,
                                                        ColumnAlign::Right,
                                                        if cust.outstanding_idr > 0 {
                                                            Theme::TEXT_PRIMARY
                                                        } else {
                                                            Theme::TEXT_LIGHT
                                                        },
                                                        FontWeight::BOLD,
                                                        None,
                                                    ))
                                                    .child(grid_cell_styled(
                                                        if cust.customer_credit_idr > 0 {
                                                            format_rupiah(cust.customer_credit_idr)
                                                        } else {
                                                            "Rp 0".into()
                                                        },
                                                        110.0,
                                                        ColumnAlign::Right,
                                                        if cust.customer_credit_idr > 0 {
                                                            Theme::BADGE_DEPOSIT_TEXT
                                                        } else {
                                                            Theme::TEXT_LIGHT
                                                        },
                                                        if cust.customer_credit_idr > 0 {
                                                            FontWeight::BOLD
                                                        } else {
                                                            FontWeight::NORMAL
                                                        },
                                                        None,
                                                    ))
                                                    .child(self.aging_amount_cell(
                                                        cust.aging.days_0_15,
                                                        95.0,
                                                        Theme::TEXT_PRIMARY,
                                                        None,
                                                    ))
                                                    .child(self.aging_amount_cell(
                                                        cust.aging.days_16_30,
                                                        95.0,
                                                        Theme::TEXT_PRIMARY,
                                                        None,
                                                    ))
                                                    .child(self.aging_amount_cell(
                                                        cust.aging.days_31_45,
                                                        95.0,
                                                        Theme::BADGE_WARNING_TEXT,
                                                        None,
                                                    ))
                                                    .child(self.aging_amount_cell(
                                                        cust.aging.days_46_60,
                                                        95.0,
                                                        Theme::BADGE_WARNING_TEXT,
                                                        None,
                                                    ))
                                                    .child(self.aging_amount_cell(
                                                        cust.aging.days_over_60,
                                                        110.0,
                                                        Theme::BADGE_DEBIT_TEXT,
                                                        if cust.aging.days_over_60 > 0 {
                                                            Some(Theme::BADGE_DEBIT_BG)
                                                        } else {
                                                            None
                                                        },
                                                    ))
                                                    .child(self.aging_amount_cell(
                                                        exc_total,
                                                        110.0,
                                                        Theme::BADGE_EXCEPTION_TEXT,
                                                        if exc_total > 0 {
                                                            Some(Theme::BADGE_EXCEPTION_BG)
                                                        } else {
                                                            None
                                                        },
                                                    ))
                                                    .into_any_element()
                                            })
                                            .collect()
                                    }),
                            ),
                    ),
            )
    }

    fn aging_amount_cell(
        &self,
        amount: i64,
        width: f32,
        active_color: u32,
        bg_tint: Option<u32>,
    ) -> Div {
        grid_cell_styled(
            format_rupiah(amount),
            width,
            ColumnAlign::Right,
            if amount > 0 {
                active_color
            } else {
                Theme::TEXT_LIGHT
            },
            if amount > 0 {
                FontWeight::BOLD
            } else {
                FontWeight::NORMAL
            },
            bg_tint,
        )
    }

    // =========================================================================
    // HALAMAN 2: DETAIL PELANGGAN (Buku Piutang & Riwayat Pembayaran)
    // Toolbar kontekstual: HANYA tombol transaksi & identitas pelanggan ini
    // =========================================================================
    fn render_customer_detail_view(&self, cx: &mut Context<Self>) -> Div {
        let Some(cid) = &self.selected_customer_id else {
            return div().p(px(16.0)).child("Pelanggan belum dipilih.");
        };

        let detail = match self.store.get_customer(cid) {
            Ok(d) => d,
            Err(_) => return div().p(px(16.0)).child("Pelanggan tidak ditemukan."),
        };

        let ledger_page = self
            .store
            .get_ledger(
                cid,
                &LedgerQuery {
                    from: self.ledger_from,
                    to: self.ledger_to,
                    order_desc: self.ledger_sort_desc,
                },
            )
            .unwrap_or(LedgerPage {
                entries: Vec::new(),
                summary: LedgerSummary {
                    outstanding_idr: detail.outstanding_idr,
                    customer_credit_idr: detail.customer_credit_idr,
                    ledger_balance_idr: detail.outstanding_idr - detail.customer_credit_idr,
                },
            });
        let payments = self.store.list_payments(cid).unwrap_or_default();

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
                                        primary_action_button("+ Catat Belanja / Piutang", "F3")
                                            .id("tool-det-sale")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.open_record_sale_modal();
                                                cx.notify();
                                            })),
                                    )
                                    .child(
                                        secondary_action_button("+ Catat Pembayaran", "F4")
                                            .id("tool-det-pay")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.open_record_payment_modal();
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
                                                this.open_apply_deposit_modal_for_payment(None);
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
                                                    .child(detail.name),
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
                                            format!(
                                                "Riwayat Pembayaran & Alokasi FIFO [{}]",
                                                payments.len()
                                            ),
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
                        CustomerTab::Payments => self.render_payments_grid(&payments, cx),
                    }),
            )
    }

    fn render_ledger_grid(&self, ledger: &[LedgerEntry], cx: &mut Context<Self>) -> Div {
        let is_owner = self.is_owner();
        let columns = vec![
            GridColumn {
                title: "TANGGAL",
                width_px: 92.0,
                align: ColumnAlign::Center,
            },
            GridColumn {
                title: "URAIAN TRANSAKSI",
                width_px: 230.0,
                align: ColumnAlign::Left,
            },
            GridColumn {
                title: "JUMLAH (QTY)",
                width_px: 80.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "HARGA SATUAN",
                width_px: 105.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "DEBET (RP)",
                width_px: 115.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "KREDIT (RP)",
                width_px: 115.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "SALDO (RP)",
                width_px: 125.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "LUNAS",
                width_px: 72.0,
                align: ColumnAlign::Center,
            },
            GridColumn {
                title: "SISA",
                width_px: 105.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "UMUR PIUTANG",
                width_px: 95.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: if is_owner {
                    "KOREKSI (OWNER)"
                } else {
                    "STATUS"
                },
                width_px: 100.0,
                align: ColumnAlign::Center,
            },
        ];

        div()
            .flex_1()
            .bg(rgb(Theme::BG_CARD))
            .border_1()
            .border_color(rgb(Theme::BORDER_STRONG))
            .rounded(px(3.0))
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
                                "Belum ada transaksi buku piutang pada periode yang dipilih.",
                            )
                            .into_any_element(),
                        ]
                    } else {
                        ledger
                            .iter()
                            .enumerate()
                            .map(|(idx, entry)| {
                                let bg_color = if idx % 2 == 1 {
                                    Theme::BG_ROW_ALT
                                } else {
                                    Theme::BG_CARD
                                };
                                let entry_id = entry.id.clone();
                                let is_debit = entry.debit_idr > 0;
                                let is_credit = entry.credit_idr > 0;
                                let is_settled = entry.remaining_idr == 0;
                                let is_rev = entry.is_reversed;
                                // FR-007: Reversal HANYA untuk Owner pada entry non-reversed dan bukan kontra-entry
                                let can_rev = is_owner && !is_rev && entry.entry_type != "reversal";

                                let date_str = entry
                                    .effective_date
                                    .map(|d| d.format("%d %b %Y").to_string())
                                    .unwrap_or_else(|| "Tidak Diketahui".into());
                                let desc_str = if is_rev {
                                    format!("{} [DIBALIK]", entry.description)
                                } else {
                                    entry.description.clone()
                                };
                                let qty_str = entry.quantity.clone().unwrap_or_else(|| "—".into());
                                let price_str = entry
                                    .unit_price_idr
                                    .map(format_rupiah)
                                    .unwrap_or_else(|| "—".into());

                                let age_str = if !is_debit || is_settled || is_rev {
                                    "—".to_string()
                                } else {
                                    match entry.age_days {
                                        None => "Tgl tidak diketahui".to_string(),
                                        Some(d) if d < 0 => "Masa depan".to_string(),
                                        Some(d) if d > 60 => "60 > Hari".to_string(),
                                        Some(d) => format!("{} hari", d),
                                    }
                                };

                                div()
                                    .id(("ledg-row", idx))
                                    .h(px(28.0))
                                    .bg(rgb(bg_color))
                                    .border_b_1()
                                    .border_color(rgb(Theme::GRID_LINE))
                                    .flex()
                                    .items_center()
                                    .child(grid_row_indicator(idx + 1, false))
                                    .child(grid_cell(date_str, 92.0, ColumnAlign::Center, false))
                                    .child(grid_cell_styled(
                                        desc_str,
                                        230.0,
                                        ColumnAlign::Left,
                                        if is_rev {
                                            Theme::TEXT_LIGHT
                                        } else {
                                            Theme::TEXT_PRIMARY
                                        },
                                        FontWeight::MEDIUM,
                                        None,
                                    ))
                                    .child(grid_cell(qty_str, 80.0, ColumnAlign::Right, false))
                                    .child(grid_cell(price_str, 105.0, ColumnAlign::Right, false))
                                    .child(grid_cell_styled(
                                        if is_debit {
                                            format_rupiah(entry.debit_idr)
                                        } else {
                                            "—".into()
                                        },
                                        115.0,
                                        ColumnAlign::Right,
                                        if is_debit {
                                            Theme::BADGE_WARNING_TEXT
                                        } else {
                                            Theme::TEXT_LIGHT
                                        },
                                        if is_debit {
                                            FontWeight::BOLD
                                        } else {
                                            FontWeight::NORMAL
                                        },
                                        None,
                                    ))
                                    .child(grid_cell_styled(
                                        if is_credit {
                                            format_rupiah(entry.credit_idr)
                                        } else {
                                            "—".into()
                                        },
                                        115.0,
                                        ColumnAlign::Right,
                                        if is_credit {
                                            Theme::BADGE_CREDIT_TEXT
                                        } else {
                                            Theme::TEXT_LIGHT
                                        },
                                        if is_credit {
                                            FontWeight::BOLD
                                        } else {
                                            FontWeight::NORMAL
                                        },
                                        None,
                                    ))
                                    // Saldo berjalan: format kurung akuntansi (Rp ...) merah jika negatif (FR-004)
                                    .child(grid_cell_styled(
                                        format_accounting_rupiah(entry.running_balance_idr),
                                        125.0,
                                        ColumnAlign::Right,
                                        if entry.running_balance_idr < 0 {
                                            Theme::BADGE_DEBIT_TEXT
                                        } else {
                                            Theme::TEXT_PRIMARY
                                        },
                                        FontWeight::BOLD,
                                        if entry.running_balance_idr < 0 {
                                            Some(Theme::BADGE_DEBIT_BG)
                                        } else {
                                            None
                                        },
                                    ))
                                    // Kolom Lunas (persis seperti Web & Excel Gambar 1)
                                    .child(if !is_debit || is_rev {
                                        grid_cell("—", 72.0, ColumnAlign::Center, false)
                                    } else if is_settled {
                                        grid_badge(
                                            "Lunas",
                                            Theme::BADGE_LUNAS_TEXT,
                                            Theme::BADGE_LUNAS_BG,
                                            Theme::BADGE_LUNAS_BORDER,
                                            72.0,
                                        )
                                    } else {
                                        grid_badge(
                                            "Belum",
                                            Theme::BADGE_DEBIT_TEXT,
                                            Theme::BADGE_DEBIT_BG,
                                            Theme::BADGE_DEBIT_BORDER,
                                            72.0,
                                        )
                                    })
                                    // Kolom Sisa
                                    .child(grid_cell_styled(
                                        if !is_debit || is_settled || is_rev {
                                            "—".into()
                                        } else {
                                            format_rupiah(entry.remaining_idr)
                                        },
                                        105.0,
                                        ColumnAlign::Right,
                                        if !is_debit || is_settled || is_rev {
                                            Theme::TEXT_LIGHT
                                        } else {
                                            Theme::BADGE_DEBIT_TEXT
                                        },
                                        if !is_debit || is_settled || is_rev {
                                            FontWeight::NORMAL
                                        } else {
                                            FontWeight::BOLD
                                        },
                                        None,
                                    ))
                                    // Kolom Umur Piutang
                                    .child(grid_cell_styled(
                                        age_str,
                                        95.0,
                                        ColumnAlign::Right,
                                        match entry.age_days {
                                            Some(d) if d > 60 && !is_settled => {
                                                Theme::BADGE_DEBIT_TEXT
                                            }
                                            Some(d) if d > 30 && !is_settled => {
                                                Theme::BADGE_WARNING_TEXT
                                            }
                                            Some(d) if d < 0 && !is_settled => {
                                                Theme::BADGE_EXCEPTION_TEXT
                                            }
                                            None if is_debit && !is_settled => {
                                                Theme::BADGE_EXCEPTION_TEXT
                                            }
                                            _ => Theme::TEXT_MUTED,
                                        },
                                        FontWeight::SEMIBOLD,
                                        None,
                                    ))
                                    // Kolom Aksi Koreksi (Owner Only sesuai FR-007)
                                    .child(
                                        div()
                                            .w(px(100.0))
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
                                                        .bg(rgb(Theme::BG_CARD))
                                                        .border_1()
                                                        .border_color(rgb(
                                                            Theme::BADGE_DEBIT_BORDER,
                                                        ))
                                                        .rounded(px(2.0))
                                                        .cursor_pointer()
                                                        .hover(|d| d.bg(rgb(Theme::BADGE_DEBIT_BG)))
                                                        .on_click(cx.listener(
                                                            move |this, _, _, cx| {
                                                                this.open_reverse_modal(&entry_id);
                                                                cx.notify();
                                                            },
                                                        ))
                                                        .child(
                                                            div()
                                                                .text_size(px(10.0))
                                                                .font_weight(FontWeight::BOLD)
                                                                .text_color(rgb(
                                                                    Theme::BADGE_DEBIT_TEXT,
                                                                ))
                                                                .child("Balik (Reverse)"),
                                                        ),
                                                )
                                            })
                                            .when(is_rev, |d| {
                                                d.child(
                                                    div()
                                                        .text_size(px(10.0))
                                                        .font_weight(FontWeight::BOLD)
                                                        .text_color(rgb(Theme::BADGE_DEBIT_TEXT))
                                                        .child("Dibalik"),
                                                )
                                            })
                                            .when(!can_rev && !is_rev, |d| {
                                                d.child(
                                                    div()
                                                        .text_size(px(10.0))
                                                        .text_color(rgb(Theme::TEXT_LIGHT))
                                                        .child("Posted"),
                                                )
                                            }),
                                    )
                                    .into_any_element()
                            })
                            .collect()
                    }),
            )
    }

    fn render_payments_grid(&self, payments: &[PaymentRow], cx: &mut Context<Self>) -> Div {
        let can_write = self.can_write();
        let is_owner = self.is_owner();
        let columns = vec![
            GridColumn {
                title: "TGL DITERIMA",
                width_px: 95.0,
                align: ColumnAlign::Center,
            },
            GridColumn {
                title: "METODE",
                width_px: 105.0,
                align: ColumnAlign::Center,
            },
            GridColumn {
                title: "STATUS",
                width_px: 75.0,
                align: ColumnAlign::Center,
            },
            GridColumn {
                title: "REFERENSI",
                width_px: 125.0,
                align: ColumnAlign::Left,
            },
            GridColumn {
                title: "NOMINAL BAYAR",
                width_px: 125.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "TERALOKASI (FIFO)",
                width_px: 125.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "SALDO LEBIH (KREDIT)",
                width_px: 140.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "CATATAN & RINCIAN ALOKASI",
                width_px: 230.0,
                align: ColumnAlign::Left,
            },
            GridColumn {
                title: "AKSI PEMBAYARAN",
                width_px: 170.0,
                align: ColumnAlign::Center,
            },
        ];

        div()
            .flex_1()
            .bg(rgb(Theme::BG_CARD))
            .border_1()
            .border_color(rgb(Theme::BORDER_STRONG))
            .rounded(px(3.0))
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
                            render_empty_state(
                                "Belum ada riwayat pembayaran untuk pelanggan ini.",
                            )
                            .into_any_element(),
                        ]
                    } else {
                        payments
                            .iter()
                            .enumerate()
                            .map(|(idx, p)| {
                                let bg_color = if idx % 2 == 1 {
                                    Theme::BG_ROW_ALT
                                } else {
                                    Theme::BG_CARD
                                };
                                let date_str = p.received_date.format("%d %b %Y").to_string();
                                let method_str = p.method_label().to_string();
                                let ref_str = p.reference.clone().unwrap_or_else(|| "—".into());
                                let alloc_summary = if p.allocations.is_empty() {
                                    p.note.clone().unwrap_or_else(|| "Belum dialokasikan".into())
                                } else {
                                    let count = p.allocations.len();
                                    match &p.note {
                                        Some(n) => format!("{} ({} nota)", n, count),
                                        None => format!("Dialokasikan ke {} nota debet", count),
                                    }
                                };
                                let has_surplus = p.unallocated_credit_idr > 0;
                                let is_posted = p.status == "posted";
                                let pay_id_apply = p.id.clone();
                                let pay_id_realloc = p.id.clone();

                                div()
                                    .id(("pay-row", idx))
                                    .h(px(28.0))
                                    .bg(rgb(bg_color))
                                    .border_b_1()
                                    .border_color(rgb(Theme::GRID_LINE))
                                    .flex()
                                    .items_center()
                                    .child(grid_row_indicator(idx + 1, false))
                                    .child(grid_cell(date_str, 95.0, ColumnAlign::Center, false))
                                    .child(grid_cell(method_str, 105.0, ColumnAlign::Center, false))
                                    .child(if is_posted {
                                        grid_badge(
                                            "Posted",
                                            Theme::BADGE_CREDIT_TEXT,
                                            Theme::BADGE_CREDIT_BG,
                                            Theme::BADGE_CREDIT_BORDER,
                                            75.0,
                                        )
                                    } else {
                                        grid_badge(
                                            "Reversed",
                                            Theme::BADGE_DEBIT_TEXT,
                                            Theme::BADGE_DEBIT_BG,
                                            Theme::BADGE_DEBIT_BORDER,
                                            75.0,
                                        )
                                    })
                                    .child(grid_cell(ref_str, 125.0, ColumnAlign::Left, false))
                                    .child(grid_cell_styled(
                                        format_rupiah(p.amount_idr),
                                        125.0,
                                        ColumnAlign::Right,
                                        Theme::BADGE_CREDIT_TEXT,
                                        FontWeight::BOLD,
                                        None,
                                    ))
                                    .child(grid_cell(
                                        format_rupiah(p.allocated_idr),
                                        125.0,
                                        ColumnAlign::Right,
                                        false,
                                    ))
                                    .child(grid_cell_styled(
                                        format_rupiah(p.unallocated_credit_idr),
                                        140.0,
                                        ColumnAlign::Right,
                                        if has_surplus {
                                            Theme::BADGE_DEPOSIT_TEXT
                                        } else {
                                            Theme::TEXT_LIGHT
                                        },
                                        if has_surplus {
                                            FontWeight::BOLD
                                        } else {
                                            FontWeight::NORMAL
                                        },
                                        if has_surplus {
                                            Some(Theme::BADGE_DEPOSIT_BG)
                                        } else {
                                            None
                                        },
                                    ))
                                    .child(grid_cell(
                                        alloc_summary,
                                        230.0,
                                        ColumnAlign::Left,
                                        false,
                                    ))
                                    .child(
                                        div()
                                            .w(px(170.0))
                                            .h_full()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .gap(px(4.0))
                                            .when(can_write && has_surplus && is_posted, |d| {
                                                d.child(
                                                    div()
                                                        .id(("btn-pay-apply", idx))
                                                        .px(px(6.0))
                                                        .py(px(1.0))
                                                        .bg(rgb(Theme::BADGE_DEPOSIT_BG))
                                                        .border_1()
                                                        .border_color(rgb(
                                                            Theme::BADGE_DEPOSIT_BORDER,
                                                        ))
                                                        .rounded(px(2.0))
                                                        .cursor_pointer()
                                                        .on_click(cx.listener(
                                                            move |this, _, _, cx| {
                                                                this.open_apply_deposit_modal_for_payment(Some(pay_id_apply.clone()));
                                                                cx.notify();
                                                            },
                                                        ))
                                                        .child(
                                                            div()
                                                                .text_size(px(10.0))
                                                                .font_weight(FontWeight::BOLD)
                                                                .text_color(rgb(
                                                                    Theme::BADGE_DEPOSIT_TEXT,
                                                                ))
                                                                .child("Terapkan Kredit"),
                                                        ),
                                                )
                                            })
                                            .when(is_owner && is_posted, |d| {
                                                d.child(
                                                    div()
                                                        .id(("btn-pay-realloc", idx))
                                                        .px(px(6.0))
                                                        .py(px(1.0))
                                                        .bg(rgb(Theme::BG_CARD))
                                                        .border_1()
                                                        .border_color(rgb(Theme::BORDER_STRONG))
                                                        .rounded(px(2.0))
                                                        .cursor_pointer()
                                                        .hover(|d| d.bg(rgb(Theme::BG_ROW_HOVER)))
                                                        .on_click(cx.listener(
                                                            move |this, _, _, cx| {
                                                                this.open_reallocate_payment_modal(
                                                                    &pay_id_realloc,
                                                                );
                                                                cx.notify();
                                                            },
                                                        ))
                                                        .child(
                                                            div()
                                                                .text_size(px(10.0))
                                                                .font_weight(FontWeight::SEMIBOLD)
                                                                .text_color(rgb(
                                                                    Theme::TEXT_PRIMARY,
                                                                ))
                                                                .child("Realokasi"),
                                                        ),
                                                )
                                            }),
                                    )
                                    .into_any_element()
                            })
                            .collect()
                    }),
            )
    }

    // =========================================================================
    // HALAMAN 3: LAPORAN ANALISIS UMUR PIUTANG (AgingReport - FR-005 / API-501..502)
    // Toolbar kontekstual: HANYA kontrol tanggal acuan (as_of) & filter bucket aging
    // =========================================================================
    fn render_aging_report_view(&self, cx: &mut Context<Self>) -> Div {
        let report = match self.store.get_aging_report(self.aging_as_of) {
            Ok(r) => r,
            Err(e) => return div().p(px(16.0)).child(e.0),
        };

        let columns = vec![
            GridColumn {
                title: "KODE",
                width_px: 75.0,
                align: ColumnAlign::Center,
            },
            GridColumn {
                title: "NAMA PELANGGAN",
                width_px: 210.0,
                align: ColumnAlign::Left,
            },
            GridColumn {
                title: "TOTAL TERBUKA",
                width_px: 130.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "0–15 HARI",
                width_px: 105.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "16–30 HARI",
                width_px: 105.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "31–45 HARI",
                width_px: 105.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "46–60 HARI",
                width_px: 105.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: ">60 HARI",
                width_px: 115.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "TGL TDK DIKETAHUI",
                width_px: 120.0,
                align: ColumnAlign::Right,
            },
            GridColumn {
                title: "TGL MASA DEPAN",
                width_px: 115.0,
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
                Some(5) => c.unknown_date > 0,
                Some(6) => c.future_date > 0,
                _ => true,
            })
            .collect();

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
                                primary_action_button("Ubah Tanggal Acuan (As Of)...", "")
                                    .id("tool-aging-date")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.open_change_as_of_modal();
                                        cx.notify();
                                    })),
                            )
                            .when(self.aging_as_of != self.today_jakarta(), |d| {
                                d.child(
                                    secondary_action_button("Reset ke Hari Ini", "")
                                        .id("tool-aging-reset-date")
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
                                    secondary_action_button("Tampilkan Semua Kelompok Umur", "")
                                        .id("tool-aging-clear-bucket")
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
                            )),
                    )
                    // DBGrid Drill-Down
                    .child(
                        div()
                            .flex_1()
                            .bg(rgb(Theme::BG_CARD))
                            .border_1()
                            .border_color(rgb(Theme::BORDER_STRONG))
                            .rounded(px(3.0))
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .child(render_grid_header(&columns))
                            .child(
                                div()
                                    .id("aging-drill-scroll")
                                    .flex_1()
                                    .children(if filtered_customers.is_empty() {
                                        vec![
                                            render_empty_state(
                                                "Tidak ada piutang terbuka pada kelompok umur ini.",
                                            )
                                            .into_any_element(),
                                        ]
                                    } else {
                                        filtered_customers
                                            .into_iter()
                                            .enumerate()
                                            .map(|(idx, c)| {
                                                let bg_color = if idx % 2 == 1 {
                                                    Theme::BG_ROW_ALT
                                                } else {
                                                    Theme::BG_CARD
                                                };
                                                let cid = c.customer_id.clone();
                                                let cname = c.customer_name.clone();
                                                let code_str =
                                                    c.customer_code.unwrap_or_else(|| "—".into());

                                                div()
                                                    .id(("ag-row", idx))
                                                    .h(px(28.0))
                                                    .bg(rgb(bg_color))
                                                    .border_b_1()
                                                    .border_color(rgb(Theme::GRID_LINE))
                                                    .flex()
                                                    .items_center()
                                                    .cursor_pointer()
                                                    .hover(|d| d.bg(rgb(Theme::BG_ROW_HOVER)))
                                                    .on_click(cx.listener(
                                                        move |this, _, _, cx| {
                                                            this.selected_customer_id =
                                                                Some(cid.clone());
                                                            this.current_view =
                                                                CurrentView::CustomerDetail;
                                                            this.status_msg = format!(
                                                                "Membuka buku piutang pelanggan '{}'.",
                                                                cname
                                                            );
                                                            cx.notify();
                                                        },
                                                    ))
                                                    .child(grid_row_indicator(idx + 1, false))
                                                    .child(grid_cell(
                                                        code_str,
                                                        75.0,
                                                        ColumnAlign::Center,
                                                        false,
                                                    ))
                                                    .child(grid_cell_styled(
                                                        c.customer_name,
                                                        210.0,
                                                        ColumnAlign::Left,
                                                        Theme::TEXT_PRIMARY,
                                                        FontWeight::BOLD,
                                                        None,
                                                    ))
                                                    .child(grid_cell_styled(
                                                        format_rupiah(c.total_outstanding_idr),
                                                        130.0,
                                                        ColumnAlign::Right,
                                                        Theme::TEXT_PRIMARY,
                                                        FontWeight::BOLD,
                                                        None,
                                                    ))
                                                    .child(self.aging_amount_cell(
                                                        c.days_0_15,
                                                        105.0,
                                                        Theme::TEXT_PRIMARY,
                                                        None,
                                                    ))
                                                    .child(self.aging_amount_cell(
                                                        c.days_16_30,
                                                        105.0,
                                                        Theme::TEXT_PRIMARY,
                                                        None,
                                                    ))
                                                    .child(self.aging_amount_cell(
                                                        c.days_31_45,
                                                        105.0,
                                                        Theme::BADGE_WARNING_TEXT,
                                                        None,
                                                    ))
                                                    .child(self.aging_amount_cell(
                                                        c.days_46_60,
                                                        105.0,
                                                        Theme::BADGE_WARNING_TEXT,
                                                        None,
                                                    ))
                                                    .child(self.aging_amount_cell(
                                                        c.days_over_60,
                                                        115.0,
                                                        Theme::BADGE_DEBIT_TEXT,
                                                        if c.days_over_60 > 0 {
                                                            Some(Theme::BADGE_DEBIT_BG)
                                                        } else {
                                                            None
                                                        },
                                                    ))
                                                    .child(self.aging_amount_cell(
                                                        c.unknown_date,
                                                        120.0,
                                                        Theme::BADGE_EXCEPTION_TEXT,
                                                        if c.unknown_date > 0 {
                                                            Some(Theme::BADGE_EXCEPTION_BG)
                                                        } else {
                                                            None
                                                        },
                                                    ))
                                                    .child(self.aging_amount_cell(
                                                        c.future_date,
                                                        115.0,
                                                        Theme::BADGE_EXCEPTION_TEXT,
                                                        if c.future_date > 0 {
                                                            Some(Theme::BADGE_EXCEPTION_BG)
                                                        } else {
                                                            None
                                                        },
                                                    ))
                                                    .into_any_element()
                                            })
                                            .collect()
                                    }),
                            ),
                    ),
            )
    }

    // =========================================================================
    // HALAMAN 4: MANAJEMEN AKUN & OTORISASI PENGGUNA (Owner Only - FR-007)
    // Toolbar kontekstual: HANYA tombol tambah pengguna & kebijakan keamanan
    // =========================================================================
    fn render_user_management_view(&self, cx: &mut Context<Self>) -> Div {
        let users = self.store.list_users().unwrap_or_default();
        let current_uid = self
            .session
            .as_ref()
            .map(|s| s.id.clone())
            .unwrap_or_default();

        let total_users = users.len();
        let total_owners = users.iter().filter(|u| u.role == UserRole::Owner).count();
        let total_operators = users
            .iter()
            .filter(|u| u.role == UserRole::Operator)
            .count();
        let total_active = users.iter().filter(|u| u.is_active).count();

        let columns = vec![
            GridColumn {
                title: "USERNAME",
                width_px: 185.0,
                align: ColumnAlign::Left,
            },
            GridColumn {
                title: "PERAN (ROLE)",
                width_px: 155.0,
                align: ColumnAlign::Center,
            },
            GridColumn {
                title: "STATUS AKUN",
                width_px: 115.0,
                align: ColumnAlign::Center,
            },
            GridColumn {
                title: "KEWAJIBAN SANDI",
                width_px: 150.0,
                align: ColumnAlign::Center,
            },
            GridColumn {
                title: "TANGGAL DIBUAT",
                width_px: 135.0,
                align: ColumnAlign::Center,
            },
            GridColumn {
                title: "AKSI KELOLA AKUN",
                width_px: 260.0,
                align: ColumnAlign::Center,
            },
        ];

        div()
            .flex_1()
            .flex()
            .flex_col()
            .h_full()
            .overflow_hidden()
            // CONTEXTUAL COMMAND BAR: HANYA AKSI MANAJEMEN PENGGUNA
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
                                    .child("Manajemen Akun & Otorisasi Pengguna (Owner Only)"),
                            )
                            .child(tool_separator())
                            .child(
                                primary_action_button("+ Tambah Pengguna Baru", "")
                                    .id("tool-user-add")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.open_new_user_modal();
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(rgb(Theme::TEXT_MUTED))
                            .child("Kebijakan: Kata Sandi Sementara Min. 15 Karakter • Minimal 1 Owner Aktif"),
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
                    // 4 KPI Boxes matching Web UserManagementPage
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(self.native_summary_box(
                                "TOTAL PENGGUNA",
                                format!("{} Akun", total_users),
                                "Terdaftar dalam sistem toko",
                                Theme::TEXT_PRIMARY,
                                Theme::BORDER_STRONG,
                            ))
                            .child(self.native_summary_box(
                                "PERAN PEMILIK (OWNER)",
                                format!("{} Akun", total_owners),
                                "Hak akses administratif penuh",
                                Theme::PRIMARY,
                                Theme::PRIMARY_BORDER,
                            ))
                            .child(self.native_summary_box(
                                "OPERATOR KASIR",
                                format!("{} Akun", total_operators),
                                "Penjualan kredit & pencatatan bayar",
                                Theme::TEXT_PRIMARY,
                                Theme::BORDER_STRONG,
                            ))
                            .child(self.native_summary_box(
                                "STATUS KEAKTIFAN",
                                format!("{} Aktif", total_active),
                                "Akun yang memiliki izin masuk",
                                Theme::BADGE_CREDIT_TEXT,
                                Theme::BADGE_CREDIT_BORDER,
                            )),
                    )
                    // DBGrid Users
                    .child(
                        div()
                            .flex_1()
                            .bg(rgb(Theme::BG_CARD))
                            .border_1()
                            .border_color(rgb(Theme::BORDER_STRONG))
                            .rounded(px(3.0))
                            .flex()
                            .flex_col()
                            .overflow_hidden()
                            .child(render_grid_header(&columns))
                            .child(
                                div().id("user-mgr-scroll").flex_1().children(
                                    users
                                        .into_iter()
                                        .enumerate()
                                        .map(|(idx, u)| {
                                            let bg_color = if idx % 2 == 1 {
                                                Theme::BG_ROW_ALT
                                            } else {
                                                Theme::BG_CARD
                                            };
                                            let is_self = u.id == current_uid;
                                            let u_edit = u.clone();
                                            let u_reset = u.clone();
                                            let u_toggle = u.clone();
                                            let uname_label = if is_self {
                                                format!("{} (Akun Anda)", u.username)
                                            } else {
                                                u.username.clone()
                                            };
                                            let date_str =
                                                u.created_at.format("%d %b %Y").to_string();

                                            div()
                                                .id(("u-row", idx))
                                                .h(px(28.0))
                                                .bg(rgb(bg_color))
                                                .border_b_1()
                                                .border_color(rgb(Theme::GRID_LINE))
                                                .flex()
                                                .items_center()
                                                .child(grid_row_indicator(idx + 1, is_self))
                                                .child(grid_cell_styled(
                                                    uname_label,
                                                    185.0,
                                                    ColumnAlign::Left,
                                                    Theme::TEXT_PRIMARY,
                                                    FontWeight::BOLD,
                                                    None,
                                                ))
                                                .child(match u.role {
                                                    UserRole::Owner => grid_badge(
                                                        "Pemilik (Owner)",
                                                        Theme::BADGE_CREDIT_TEXT,
                                                        Theme::BADGE_CREDIT_BG,
                                                        Theme::BADGE_CREDIT_BORDER,
                                                        155.0,
                                                    ),
                                                    UserRole::Operator => grid_badge(
                                                        "Operator Kasir",
                                                        Theme::PRIMARY,
                                                        Theme::PRIMARY_LIGHT,
                                                        Theme::PRIMARY_BORDER,
                                                        155.0,
                                                    ),
                                                    UserRole::Viewer => grid_badge(
                                                        "Pembaca (Viewer)",
                                                        Theme::TEXT_MUTED,
                                                        0xF1F5F9,
                                                        0xCBD5E1,
                                                        155.0,
                                                    ),
                                                })
                                                .child(if u.is_active {
                                                    grid_badge(
                                                        "Aktif",
                                                        Theme::BADGE_CREDIT_TEXT,
                                                        Theme::BADGE_CREDIT_BG,
                                                        Theme::BADGE_CREDIT_BORDER,
                                                        115.0,
                                                    )
                                                } else {
                                                    grid_badge(
                                                        "Nonaktif",
                                                        Theme::TEXT_MUTED,
                                                        0xF1F5F9,
                                                        0xCBD5E1,
                                                        115.0,
                                                    )
                                                })
                                                .child(if u.must_change_password {
                                                    grid_badge(
                                                        "Wajib Ganti Sandi",
                                                        Theme::BADGE_WARNING_TEXT,
                                                        Theme::BADGE_WARNING_BG,
                                                        Theme::BADGE_WARNING_BORDER,
                                                        150.0,
                                                    )
                                                } else {
                                                    grid_cell(
                                                        "Terverifikasi",
                                                        150.0,
                                                        ColumnAlign::Center,
                                                        false,
                                                    )
                                                })
                                                .child(grid_cell(
                                                    date_str,
                                                    135.0,
                                                    ColumnAlign::Center,
                                                    false,
                                                ))
                                                .child(
                                                    div()
                                                        .w(px(260.0))
                                                        .h_full()
                                                        .flex()
                                                        .items_center()
                                                        .justify_center()
                                                        .gap(px(5.0))
                                                        .child(
                                                            div()
                                                                .id(("edit-u-btn", idx))
                                                                .px(px(7.0))
                                                                .py(px(1.0))
                                                                .bg(rgb(Theme::BG_CARD))
                                                                .border_1()
                                                                .border_color(rgb(
                                                                    Theme::BORDER_STRONG,
                                                                ))
                                                                .rounded(px(2.0))
                                                                .cursor_pointer()
                                                                .hover(|d| {
                                                                    d.bg(rgb(Theme::BG_ROW_HOVER))
                                                                })
                                                                .on_click(cx.listener(
                                                                    move |this, _, _, cx| {
                                                                        this.open_edit_user_modal(
                                                                            &u_edit,
                                                                        );
                                                                        cx.notify();
                                                                    },
                                                                ))
                                                                .child(
                                                                    div()
                                                                        .text_size(px(10.0))
                                                                        .font_weight(
                                                                            FontWeight::SEMIBOLD,
                                                                        )
                                                                        .text_color(rgb(
                                                                            Theme::TEXT_PRIMARY,
                                                                        ))
                                                                        .child("Ubah"),
                                                                ),
                                                        )
                                                        .when(!is_self && u.is_active, |d| {
                                                            d.child(
                                                                div()
                                                                    .id(("rst-u-btn", idx))
                                                                    .px(px(7.0))
                                                                    .py(px(1.0))
                                                                    .bg(rgb(
                                                                        Theme::BADGE_WARNING_BG,
                                                                    ))
                                                                    .border_1()
                                                                    .border_color(rgb(
                                                                        Theme::BADGE_WARNING_BORDER,
                                                                    ))
                                                                    .rounded(px(2.0))
                                                                    .cursor_pointer()
                                                                    .on_click(cx.listener(
                                                                        move |this, _, _, cx| {
                                                                            this.open_reset_user_password_modal(&u_reset);
                                                                            cx.notify();
                                                                        },
                                                                    ))
                                                                    .child(
                                                                        div()
                                                                            .text_size(px(10.0))
                                                                            .font_weight(
                                                                                FontWeight::BOLD,
                                                                            )
                                                                            .text_color(rgb(
                                                                                Theme::BADGE_WARNING_TEXT,
                                                                            ))
                                                                            .child("Reset Sandi"),
                                                                    ),
                                                            )
                                                        })
                                                        .when(!is_self, |d| {
                                                            let is_act = u.is_active;
                                                            d.child(
                                                                div()
                                                                    .id(("tgl-u-btn", idx))
                                                                    .px(px(7.0))
                                                                    .py(px(1.0))
                                                                    .bg(rgb(if is_act {
                                                                        Theme::BADGE_DEBIT_BG
                                                                    } else {
                                                                        Theme::BADGE_CREDIT_BG
                                                                    }))
                                                                    .border_1()
                                                                    .border_color(rgb(if is_act {
                                                                        Theme::BADGE_DEBIT_BORDER
                                                                    } else {
                                                                        Theme::BADGE_CREDIT_BORDER
                                                                    }))
                                                                    .rounded(px(2.0))
                                                                    .cursor_pointer()
                                                                    .on_click(cx.listener(
                                                                        move |this, _, _, cx| {
                                                                            let up =
                                                                                UpdateUserInput {
                                                                                    username: u_toggle.username.clone(),
                                                                                    role: u_toggle.role,
                                                                                    is_active: !u_toggle.is_active,
                                                                                };
                                                                            match this.store.update_user(&u_toggle.id, &up) {
                                                                                Ok(updated) => {
                                                                                    this.status_msg = format!(
                                                                                        "Status akun '{}' diubah menjadi {}.",
                                                                                        updated.username,
                                                                                        if updated.is_active { "Aktif" } else { "Nonaktif" }
                                                                                    );
                                                                                }
                                                                                Err(e) => {
                                                                                    this.status_msg = e.0;
                                                                                }
                                                                            }
                                                                            cx.notify();
                                                                        },
                                                                    ))
                                                                    .child(
                                                                        div()
                                                                            .text_size(px(10.0))
                                                                            .font_weight(
                                                                                FontWeight::BOLD,
                                                                            )
                                                                            .text_color(rgb(
                                                                                if is_act {
                                                                                    Theme::BADGE_DEBIT_TEXT
                                                                                } else {
                                                                                    Theme::BADGE_CREDIT_TEXT
                                                                                },
                                                                            ))
                                                                            .child(if is_act {
                                                                                "Nonaktifkan"
                                                                            } else {
                                                                                "Aktifkan"
                                                                            }),
                                                                    ),
                                                            )
                                                        }),
                                                )
                                                .into_any_element()
                                        })
                                        .collect::<Vec<_>>(),
                                ),
                            ),
                    ),
            )
    }

    // =========================================================================
    // Helper Visual Native
    // =========================================================================

    fn native_summary_box(
        &self,
        title: &'static str,
        value: String,
        subtitle: &'static str,
        color_hex: u32,
        border_left_hex: u32,
    ) -> Div {
        div()
            .flex_1()
            .h(px(58.0))
            .bg(rgb(Theme::BG_CARD))
            .border_1()
            .border_color(rgb(Theme::BORDER_STRONG))
            .border_l_4()
            .border_color(rgb(border_left_hex))
            .px(px(10.0))
            .py(px(6.0))
            .rounded(px(3.0))
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
                            .text_size(px(10.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(Theme::TEXT_MUTED))
                            .child(title),
                    )
                    .child(
                        div()
                            .text_size(px(14.0))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(color_hex))
                            .child(value),
                    ),
            )
            .child(
                div()
                    .text_size(px(10.0))
                    .text_color(rgb(Theme::TEXT_LIGHT))
                    .child(subtitle),
            )
    }

    fn detail_financial_badge(
        &self,
        label: &'static str,
        amount_str: String,
        status_label: &'static str,
        text_hex: u32,
        bg_hex: u32,
        border_hex: u32,
    ) -> Div {
        div()
            .px(px(10.0))
            .py(px(5.0))
            .bg(rgb(bg_hex))
            .border_1()
            .border_color(rgb(border_hex))
            .rounded(px(3.0))
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(9.5))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(Theme::TEXT_MUTED))
                            .child(label),
                    )
                    .child(
                        div()
                            .text_size(px(9.5))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(text_hex))
                            .child(status_label),
                    ),
            )
            .child(
                div()
                    .text_size(px(14.0))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(text_hex))
                    .child(amount_str),
            )
    }

    fn native_folder_tab(
        &self,
        id: &'static str,
        label: String,
        is_active: bool,
    ) -> gpui::Stateful<Div> {
        div()
            .id(id)
            .h(px(28.0))
            .px(px(12.0))
            .bg(rgb(if is_active {
                Theme::BG_CARD
            } else {
                Theme::BG_SIDEBAR
            }))
            .border_1()
            .border_color(rgb(if is_active {
                Theme::PRIMARY
            } else {
                Theme::BORDER_STRONG
            }))
            .border_t_2()
            .rounded(px(3.0))
            .flex()
            .items_center()
            .cursor_pointer()
            .child(
                div()
                    .text_size(px(11.5))
                    .font_weight(if is_active {
                        FontWeight::BOLD
                    } else {
                        FontWeight::MEDIUM
                    })
                    .text_color(rgb(if is_active {
                        Theme::PRIMARY_HOVER
                    } else {
                        Theme::TEXT_SECONDARY
                    }))
                    .child(label),
            )
    }

    fn sort_toggle_button(
        &self,
        id: &'static str,
        label: &'static str,
        is_active: bool,
    ) -> gpui::Stateful<Div> {
        div()
            .id(id)
            .h(px(25.0))
            .px(px(9.0))
            .bg(rgb(if is_active {
                Theme::PRIMARY
            } else {
                Theme::BG_CARD
            }))
            .border_1()
            .border_color(rgb(if is_active {
                Theme::PRIMARY_HOVER
            } else {
                Theme::BORDER_STRONG
            }))
            .rounded(px(3.0))
            .flex()
            .items_center()
            .cursor_pointer()
            .child(
                div()
                    .text_size(px(10.5))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(if is_active {
                        0xFFFFFF
                    } else {
                        Theme::TEXT_SECONDARY
                    }))
                    .child(label),
            )
    }

    fn native_aging_bucket_card(
        &self,
        id: &'static str,
        label: &'static str,
        desc: &'static str,
        amount: String,
        is_active: bool,
        color_hex: u32,
        border_hex: u32,
        bucket: Option<usize>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .id(id)
            .flex_1()
            .h(px(62.0))
            .bg(rgb(if is_active {
                Theme::BG_ROW_SELECTED
            } else {
                Theme::BG_CARD
            }))
            .border_1()
            .border_color(rgb(if is_active {
                Theme::PRIMARY
            } else {
                border_hex
            }))
            .border_t_2()
            .p(px(7.0))
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
                    .text_size(px(9.5))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(Theme::TEXT_MUTED))
                    .child(label),
            )
            .child(
                div()
                    .text_size(px(12.5))
                    .font_weight(FontWeight::BOLD)
                    .text_color(rgb(color_hex))
                    .child(amount),
            )
            .child(
                div()
                    .text_size(px(9.5))
                    .text_color(rgb(Theme::TEXT_LIGHT))
                    .child(desc),
            )
    }

    // =========================================================================
    // Modal Dialogs Overlay
    // =========================================================================
    fn render_modal_overlay(&self, cx: &mut Context<Self>) -> Option<Div> {
        let modal = self.active_modal.as_ref()?;

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
                        primary_action_button("Tutup (Enter / Esc)", "")
                            .id("btn-notice-ok")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.active_modal = None;
                                cx.notify();
                            })),
                    );
                Some(dialog.frame(body, footer))
            }

            ActiveModal::RecordSale => {
                let cust_name = self
                    .selected_customer_id
                    .as_ref()
                    .and_then(|id| self.store.get_customer(id).ok())
                    .map(|c| c.name)
                    .unwrap_or_else(|| "Pelanggan".into());

                let dialog = ModalDialog::new("Catat Belanja Kredit / Piutang Baru", 740.0)
                    .subtitle(cust_name);

                let mut grand_total = 0_i64;
                for (_, qty_str, price_str) in &self.sale_items {
                    let clean_p: String =
                        price_str.chars().filter(|c| c.is_ascii_digit()).collect();
                    let p: i64 = clean_p.parse().unwrap_or(0);
                    if let Ok(line) = compute_line_total_half_up(qty_str, p) {
                        grand_total += line;
                    }
                }

                let active_pos = self.sale_active_field;
                let item_count = self.sale_items.len();

                let body = div()
                    .flex()
                    .flex_col()
                    .gap(px(10.0))
                    // Header Form: Tanggal, Referensi, & Backdate Reason
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .children(self.form_fields.iter().enumerate().map(
                                |(f_idx, (label, val))| {
                                    let is_focused = active_pos == (item_count, f_idx);
                                    div()
                                        .id(("sale-hdr-f", f_idx))
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .gap(px(3.0))
                                        .cursor_pointer()
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.sale_active_field =
                                                (this.sale_items.len(), f_idx);
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
                                        .child(self.render_native_text_box(
                                            val,
                                            "—",
                                            is_focused,
                                            false,
                                            28.0,
                                        ))
                                },
                            )),
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
                                    .h(px(26.0))
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
                                    .child(
                                        div()
                                            .w(px(95.0))
                                            .text_right()
                                            .pr(px(8.0))
                                            .child("QTY (MAKS 3 DES)"),
                                    )
                                    .child(
                                        div()
                                            .w(px(130.0))
                                            .text_right()
                                            .pr(px(8.0))
                                            .child("HARGA SATUAN (RP)"),
                                    )
                                    .child(
                                        div()
                                            .w(px(130.0))
                                            .text_right()
                                            .pr(px(8.0))
                                            .child("SUBTOTAL (HALF-UP)"),
                                    )
                                    .child(div().w(px(45.0)).text_center().child("HAPUS")),
                            )
                            .children(self.sale_items.iter().enumerate().map(
                                |(r_idx, (desc, qty, price))| {
                                    let clean_p: String =
                                        price.chars().filter(|c| c.is_ascii_digit()).collect();
                                    let p_num: i64 = clean_p.parse().unwrap_or(0);
                                    let subtotal =
                                        compute_line_total_half_up(qty, p_num).unwrap_or(0);

                                    let f_desc = active_pos == (r_idx, 0);
                                    let f_qty = active_pos == (r_idx, 1);
                                    let f_price = active_pos == (r_idx, 2);

                                    div()
                                        .h(px(32.0))
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
                                                .child(self.render_native_text_box(
                                                    desc,
                                                    "Ketik nama barang...",
                                                    f_desc,
                                                    false,
                                                    24.0,
                                                )),
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
                                                .child(self.render_native_text_box(
                                                    qty,
                                                    "1",
                                                    f_qty,
                                                    false,
                                                    24.0,
                                                )),
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
                                                .child(self.render_native_text_box(
                                                    if price.is_empty() {
                                                        ""
                                                    } else {
                                                        price
                                                    },
                                                    "0",
                                                    f_price,
                                                    false,
                                                    24.0,
                                                )),
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
                                                .child(format_rupiah(subtotal)),
                                        )
                                        // Tombol Hapus Baris
                                        .child(
                                            div()
                                                .id(("s-del", r_idx))
                                                .w(px(45.0))
                                                .flex()
                                                .justify_center()
                                                .cursor_pointer()
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    if this.sale_items.len() > 1 {
                                                        this.sale_items.remove(r_idx);
                                                        this.sale_active_field = (0, 0);
                                                        cx.notify();
                                                    }
                                                }))
                                                .child(
                                                    div()
                                                        .text_size(px(11.0))
                                                        .font_weight(FontWeight::BOLD)
                                                        .text_color(rgb(Theme::BADGE_DEBIT_TEXT))
                                                        .child("✕"),
                                                ),
                                        )
                                },
                            ))
                            // Footer Tabel: Tambah Baris + Total
                            .child(
                                div()
                                    .h(px(34.0))
                                    .bg(rgb(Theme::BG_HEADER))
                                    .px(px(8.0))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        secondary_action_button("+ Tambah Baris Barang Lagi", "")
                                            .id("btn-sale-add-row")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                let next_idx = this.sale_items.len();
                                                this.sale_items.push((
                                                    "".into(),
                                                    "1".into(),
                                                    "".into(),
                                                ));
                                                this.sale_active_field = (next_idx, 0);
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
                                                    .child(format_rupiah(grand_total)),
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
                            .child("Catatan BR-006: Saldo kredit pelanggan tidak dipotong otomatis pada penjualan baru. Gunakan tombol 'Terapkan Saldo Kredit' untuk mengalokasikan deposit."),
                    )
                    .into_any_element();

                let footer =
                    self.render_modal_footer("Simpan & Posting ke Buku Piutang (Enter)", cx);
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
                        "Konfirmasi & Catat Pembayaran (Enter)",
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
                            .map(|(idx, (label, val))| {
                                let is_active = idx == active_idx;
                                let is_pw = label.to_lowercase().contains("sandi");

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
                                    .child(self.render_native_text_box(
                                        val,
                                        "Ketik nilai...",
                                        is_active,
                                        is_pw,
                                        30.0,
                                    ))
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
        div()
            .h(px(42.0))
            .bg(rgb(Theme::BG_HEADER))
            .border_t_1()
            .border_color(rgb(Theme::BORDER_DEFAULT))
            .px(px(14.0))
            .flex()
            .items_center()
            .justify_end()
            .gap(px(8.0))
            .child(
                secondary_action_button("Batal (Esc)", "")
                    .id("btn-modal-cancel")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.active_modal = None;
                        cx.notify();
                    })),
            )
            .child(
                primary_action_button(submit_label, "")
                    .id("btn-modal-submit")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.submit_modal(cx);
                    })),
            )
    }

    /// Helper render input field bergaya native desktop dengan indikator Focus dan Kursor aktif yang jelas.
    fn render_native_text_box(
        &self,
        value: &str,
        placeholder: &str,
        is_focused: bool,
        is_password: bool,
        height_px: f32,
    ) -> Div {
        let make_cursor = move || {
            div()
                .w(px(2.0))
                .h(px((height_px - 10.0).max(12.0)))
                .bg(rgb(Theme::CURSOR_COLOR))
                .ml(px(1.0))
        };

        div()
            .h(px(height_px))
            .px(px(8.0))
            .bg(rgb(if is_focused {
                Theme::FOCUS_BG
            } else {
                0xFFFFFF
            }))
            .border_1()
            .border_color(rgb(if is_focused {
                Theme::FOCUS_BORDER
            } else {
                Theme::BORDER_STRONG
            }))
            .when(is_focused, |d| {
                // Outline ring native yang kontras agar field fokus langsung terlihat mencolok
                d.shadow(vec![gpui::BoxShadow {
                    color: gpui::rgba(0x2563EB44).into(),
                    offset: gpui::point(px(0.0), px(0.0)),
                    blur_radius: px(3.0),
                    spread_radius: px(2.0),
                }])
            })
            .rounded(px(3.0))
            .flex()
            .items_center()
            .text_size(px(12.0))
            .child(if value.is_empty() {
                div()
                    .flex()
                    .items_center()
                    .when(is_focused, |d| d.child(make_cursor()))
                    .child(
                        div()
                            .text_color(rgb(Theme::TEXT_LIGHT))
                            .child(placeholder.to_string()),
                    )
            } else if is_password {
                div()
                    .flex()
                    .items_center()
                    .text_color(rgb(Theme::TEXT_PRIMARY))
                    .child("•".repeat(value.len()))
                    .when(is_focused, |d| d.child(make_cursor()))
            } else {
                div()
                    .flex()
                    .items_center()
                    .font_weight(if is_focused {
                        FontWeight::SEMIBOLD
                    } else {
                        FontWeight::MEDIUM
                    })
                    .text_color(rgb(Theme::TEXT_PRIMARY))
                    .child(value.to_string())
                    .when(is_focused, |d| d.child(make_cursor()))
            })
    }
}

/// Pilih sumber data: API nyata (default) atau mock lokal (`TOKO_DESKTOP_ADAPTER=mock`).
/// Base URL API dapat diubah lewat `TOKO_API_BASE_URL`.
fn build_store_from_env() -> Box<dyn TokoStoreAdapter> {
    let mode = std::env::var("TOKO_DESKTOP_ADAPTER")
        .unwrap_or_default()
        .to_lowercase();
    if mode == "mock" {
        return Box::new(MockDataAdapter::new());
    }
    let base = std::env::var("TOKO_API_BASE_URL")
        .unwrap_or_else(|_| "https://piutang.banza.web.id".into());
    Box::new(HttpApiAdapter::new(base))
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1240.0), px(760.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("Keuangan Toko — Aplikasi Desktop Native".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| {
                window.activate_window();
                cx.new(Desktop::new)
            },
        )
        .expect("gagal membuka jendela aplikasi desktop");
        cx.activate(true);
    });
}
