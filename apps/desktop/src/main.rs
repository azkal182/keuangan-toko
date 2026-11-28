//! Keuangan Toko - Native Windows Desktop Application (GPUI)
//! Arsitektur UI/UX Native Desktop yang presisi dan kontekstual:
//! - System Menu Bar (26px) di puncak jendela + Left Explorer Navigation Pane (210px)
//! - Contextual Page Command Bar (38px) yang HANYA menampilkan tombol aksi sesuai halaman/tab aktif
//! - Native DBGrid dengan kolom indikator baris (`#` / `▶`), garis sel tegas, dan format akuntansi `(Rp ...)`
//! - Seluruh data, aturan bisnis, dan otorisasi selaras 100% dengan spesifikasi final (ADR-007, API_SPEC, FEATURE_SPECIFICATIONS)
#![allow(clippy::get_first, clippy::too_many_arguments, clippy::collapsible_if)]

mod app;
mod features;
mod mock;
pub mod store;
pub mod ui;

use chrono::{Local, NaiveDate};
use features::aging_table::AgingRows;
use features::customer_table::CustomerRows;
use features::detail_data::{DetailData, DetailKey};
use features::detail_tables::{LedgerRows, PaymentRows};
use gpui::{
    Context, Div, Entity, FocusHandle, Focusable, FontWeight, KeyDownEvent, Subscription, Window,
    div, prelude::*, px, rgb,
};
use gpui_kit::component::WindowExt;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::calendar::Date;
use gpui_kit::component::date_picker::{DatePickerEvent, DatePickerState};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::table::{TableEvent, TableState};
use store::mock_store::compute_line_total_half_up;
use store::*;
use ui::db_grid::{
    ColumnAlign, GridColumn, grid_badge, grid_cell, grid_cell_styled, grid_row_indicator,
    render_empty_state, render_grid_header, render_loading_state, render_pagination_bar,
};
use ui::menu_bar::{
    render_left_sidebar_container, render_system_menu_bar, sidebar_nav_item, sidebar_section_header,
};
use ui::modal::ModalDialog;
use ui::status_bar::StatusBar;
use ui::theme::{Theme, format_accounting_rupiah, format_rupiah};
use ui::tool_bar::{
    accent_action_button, primary_action_button, render_toolbar_container, secondary_action_button,
    tool_separator,
};

use app::build_store_from_env;
use app::navigation::{ActiveModal, CurrentView, CustomerTab};

gpui::actions!(desktop, [ModalNextField, ModalPreviousField]);

fn parse_money_input(raw: &str) -> Option<i64> {
    let raw = raw.trim();
    if raw.is_empty() || !raw.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    let parts: Vec<_> = raw.split('.').collect();
    if parts.iter().any(|p| p.is_empty())
        || (parts.len() > 1 && (parts[0].len() > 3 || parts[1..].iter().any(|p| p.len() != 3)))
    {
        return None;
    }
    parts.concat().parse().ok()
}

enum OperationOutcome {
    PasswordChanged,
    CustomerCreated(CustomerDetail),
    CustomerUpdated(CustomerDetail),
    CustomerMerged { id: String, name: String },
    CreditApplied(i64),
    UserCreated(UserItem),
    UserUpdated(UserItem),
    PasswordReset(String),
}

pub struct Desktop {
    store: Box<dyn TokoStoreAdapter>,
    session: Option<SessionUser>,
    current_view: CurrentView,
    selected_customer_id: Option<String>,
    open_customers: Vec<(String, String)>,
    detail_result: Option<(DetailKey, Result<DetailData, AppError>)>,
    detail_pending: Option<(u64, DetailKey)>,
    detail_generation: u64,
    customer_tab: CustomerTab,
    query: String,
    search_input: Entity<InputState>,
    _search_subscription: Subscription,
    login_inputs: Vec<Entity<InputState>>,
    _login_subscriptions: Vec<Subscription>,
    login_initialized: bool,
    customer_form_inputs: Vec<Entity<InputState>>,
    _customer_form_subscriptions: Vec<Subscription>,
    customer_form_initialized: bool,
    sale_inputs: Vec<Vec<Entity<InputState>>>,
    sale_input_subscriptions: Vec<Subscription>,
    sale_inputs_initialized: bool,
    date_pickers: Vec<Entity<DatePickerState>>,
    _date_subscriptions: Vec<Subscription>,
    picker_modal: Option<ActiveModal>,
    customer_table: Entity<TableState<CustomerRows>>,
    _customer_table_subscription: Subscription,
    aging_table: Entity<TableState<AgingRows>>,
    _aging_table_subscription: Subscription,
    ledger_table: Entity<TableState<LedgerRows>>,
    payment_table: Entity<TableState<PaymentRows>>,
    _payment_table_subscription: Subscription,
    customer_active_only: bool,
    customers_result: Option<Result<Vec<CustomerSummary>, AppError>>,
    customers_pending: Option<u64>,
    customers_generation: u64,
    users_result: Option<Result<Vec<UserItem>, AppError>>,
    users_pending: Option<u64>,
    users_generation: u64,
    ledger_sort_desc: bool,
    ledger_from: Option<NaiveDate>,
    ledger_to: Option<NaiveDate>,
    status_msg: String,
    active_modal: Option<ActiveModal>,
    reviewing_transaction: bool,
    transaction_uncertain: bool,
    form_fields: Vec<(&'static str, String)>,
    form_active: usize,
    form_error: String,
    // State khusus form penjualan multi-item
    sale_items: Vec<(String, String, String)>, // (uraian, qty, harga_satuan)
    sale_active_field: (usize, usize), // (row_index, col_index: 0=desc, 1=qty, 2=price, 99=header_fields)
    // State khusus aging report
    aging_as_of: NaiveDate,
    aging_result: Option<(NaiveDate, Result<AgingReport, AppError>)>,
    aging_pending: Option<(u64, NaiveDate)>,
    aging_generation: u64,
    aging_selected_bucket: Option<usize>, // None=Semua, 0=0-15, 1=16-30, 2=31-45, 3=46-60, 4=>60, 5=unknown, 6=future
    // Pagination per modul
    customer_page: usize,
    ledger_page_idx: usize,
    payment_page_idx: usize,
    page_size: usize,
    // State loading UI
    is_loading: bool,
    loading_label: String,
    focus: FocusHandle,
    dialog_focus: FocusHandle,
    previous_reviewing: bool,
    dialog_view: Option<Entity<DesktopDialogView>>,
    kit_dialog_open: bool,
}

struct DesktopDialogView {
    desktop: gpui::WeakEntity<Desktop>,
    _subscription: Subscription,
}

impl Render for DesktopDialogView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(desktop) = self.desktop.upgrade() else {
            return div().into_any_element();
        };
        desktop.update(cx, |desktop, cx| {
            div()
                .key_context("DesktopModal")
                .track_focus(&desktop.dialog_focus)
                .on_action(cx.listener(|this, _: &ModalNextField, window, cx| {
                    if this.active_modal.is_some() && !this.reviewing_transaction {
                        this.next_modal_field();
                        this.focus_active_modal_field(window, cx);
                        cx.notify();
                    }
                    cx.stop_propagation();
                }))
                .on_action(cx.listener(|this, _: &ModalPreviousField, window, cx| {
                    if this.active_modal.is_some() && !this.reviewing_transaction {
                        this.previous_modal_field();
                        this.focus_active_modal_field(window, cx);
                        cx.notify();
                    }
                    cx.stop_propagation();
                }))
                .on_key_down(cx.listener(Desktop::key_down))
                .children(desktop.render_modal_overlay(cx))
                .into_any_element()
        })
    }
}

impl Desktop {
    fn uses_generic_inputs(&self) -> bool {
        matches!(self.active_modal, Some(ref modal) if !matches!(modal, ActiveModal::Login | ActiveModal::Notice { .. }))
    }
    fn date_field_index(&self, slot: usize) -> Option<usize> {
        match (&self.active_modal, slot) {
            (Some(ActiveModal::RecordSale), 0) => Some(0),
            (Some(ActiveModal::RecordPayment), 0) => Some(1),
            (Some(ActiveModal::ChangeAsOfDate), 0) => Some(0),
            (Some(ActiveModal::FilterLedgerDate), 0 | 1) => Some(slot),
            _ => None,
        }
    }

    fn focus_active_modal_field(&self, window: &mut Window, cx: &mut Context<Self>) {
        if self.active_modal == Some(ActiveModal::RecordSale) {
            let (row, col) = self.sale_active_field;
            if row < self.sale_inputs.len() {
                window.focus(&self.sale_inputs[row][col].read(cx).focus_handle(cx), cx);
                return;
            }
            if col != 0 {
                window.focus(
                    &self.customer_form_inputs[col].read(cx).focus_handle(cx),
                    cx,
                );
                return;
            }
        }
        if self.uses_generic_inputs()
            && !(0..self.date_pickers.len())
                .any(|slot| self.date_field_index(slot) == Some(self.form_active))
        {
            window.focus(
                &self.customer_form_inputs[self.form_active]
                    .read(cx)
                    .focus_handle(cx),
                cx,
            );
            return;
        }
        let active = if self.active_modal == Some(ActiveModal::RecordSale) {
            let (row, col) = self.sale_active_field;
            (row >= self.sale_items.len()).then_some(col)
        } else {
            Some(self.form_active)
        };
        if let Some(slot) = active.and_then(|index| {
            (0..self.date_pickers.len()).find(|slot| self.date_field_index(*slot) == Some(index))
        }) {
            window.focus(&self.date_pickers[slot].read(cx).focus_handle(cx), cx);
        } else {
            window.focus(&self.focus, cx);
        }
    }

    fn sync_date_pickers(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.picker_modal == self.active_modal {
            return;
        }
        self.picker_modal = self.active_modal.clone();
        for slot in 0..self.date_pickers.len() {
            if let Some(index) = self.date_field_index(slot) {
                let date = self
                    .form_fields
                    .get(index)
                    .and_then(|field| field.1.parse().ok());
                self.date_pickers[slot].update(cx, |picker, cx| {
                    picker.set_date(Date::Single(date), window, cx)
                });
            }
        }
    }
    fn run_operation<F>(
        &mut self,
        label: &'static str,
        financial: bool,
        cx: &mut Context<Self>,
        operation: F,
    ) where
        F: FnOnce(&mut dyn TokoStoreAdapter) -> Result<OperationOutcome, AppError> + Send + 'static,
    {
        if self.is_loading {
            return;
        }
        self.is_loading = true;
        self.loading_label = label.into();
        self.form_error.clear();
        cx.notify();
        let mut reader = self.store.background_reader();
        let request = cx.background_spawn(async move {
            let result = operation(reader.as_mut());
            (reader, result)
        });
        cx.spawn(async move |this, cx| {
            let (reader, result) = request.await;
            let _ = this.update(cx, |this, cx| {
                this.is_loading = false;
                match result {
                    Ok(outcome) => {
                        this.store = reader;
                        this.session = this.store.current_session();
                        this.invalidate_aging();
                        this.active_modal = None;
                        this.status_msg = match outcome {
                            OperationOutcome::PasswordChanged => "Kata sandi berhasil diperbarui.".into(),
                            OperationOutcome::CustomerCreated(c) => {
                                this.selected_customer_id = Some(c.id);
                                this.current_view = CurrentView::CustomerDetail;
                                format!("Pelanggan '{}' berhasil ditambahkan.", c.name)
                            }
                            OperationOutcome::CustomerUpdated(c) => {
                                if let Some((_, name)) = this.open_customers.iter_mut().find(|(id, _)| id == &c.id) {
                                    *name = c.name.clone();
                                }
                                format!("Perubahan data pelanggan '{}' (v{}) berhasil disimpan.", c.name, c.version)
                            },
                            OperationOutcome::CustomerMerged { id, name } => {
                                this.selected_customer_id = Some(id);
                                format!("Pelanggan berhasil digabungkan ke '{}'.", name)
                            }
                            OperationOutcome::CreditApplied(amount) => format!("Saldo kredit sebesar {} berhasil diterapkan.", format_rupiah(amount)),
                            OperationOutcome::UserCreated(user) => format!("Akun '{}' ({}) berhasil dibuat. Wajib ganti kata sandi saat login pertama.", user.username, user.role.label()),
                            OperationOutcome::UserUpdated(user) => format!("Akun '{}' berhasil diperbarui.", user.username),
                            OperationOutcome::PasswordReset(username) => format!("Kata sandi akun '{}' berhasil direset.", username),
                        };
                    }
                    Err(error) => {
                        if financial {
                            this.transaction_uncertain = true;
                            this.form_error = format!("{} Periksa buku piutang sebelum mencoba lagi.", error.0);
                        } else {
                            this.form_error = error.0;
                        }
                    }
                }
                cx.notify();
            });
        }).detach();
    }
    fn next_modal_field(&mut self) {
        if self.active_modal == Some(ActiveModal::RecordSale) {
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
                let next = (c + 1) % self.form_fields.len();
                self.sale_active_field = if next == 0 {
                    (0, 0)
                } else {
                    (self.sale_items.len(), next)
                };
            }
        } else if !self.form_fields.is_empty() {
            self.form_active = (self.form_active + 1) % self.form_fields.len();
        }
    }

    fn previous_modal_field(&mut self) {
        if self.active_modal == Some(ActiveModal::RecordSale) {
            let (row, col) = self.sale_active_field;
            if row < self.sale_items.len() {
                if col > 0 {
                    self.sale_active_field = (row, col - 1);
                } else if row > 0 {
                    self.sale_active_field = (row - 1, 2);
                } else if !self.form_fields.is_empty() {
                    self.sale_active_field = (self.sale_items.len(), self.form_fields.len() - 1);
                }
            } else if col > 0 {
                self.sale_active_field = (self.sale_items.len(), col - 1);
            } else if !self.sale_items.is_empty() {
                self.sale_active_field = (self.sale_items.len() - 1, 2);
            }
        } else if !self.form_fields.is_empty() {
            self.form_active =
                (self.form_active + self.form_fields.len() - 1) % self.form_fields.len();
        }
    }
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let date_pickers: Vec<_> = (0..2)
            .map(|_| cx.new(|cx| DatePickerState::new(window, cx).date_format("%d %b %Y")))
            .collect();
        let date_subscriptions = date_pickers
            .iter()
            .enumerate()
            .map(|(slot, picker)| {
                cx.subscribe_in(picker, window, move |this: &mut Self, _, event, _, cx| {
                    let DatePickerEvent::Change(value) = event;
                    if let Some(index) = this.date_field_index(slot) {
                        if let Some(field) = this.form_fields.get_mut(index) {
                            field.1 = value
                                .start()
                                .map(|date| date.date().to_string())
                                .unwrap_or_default();
                            this.form_error.clear();
                            cx.notify();
                        }
                    }
                })
            })
            .collect();
        let customer_table = cx.new(|cx| TableState::new(CustomerRows::default(), window, cx));
        let aging_table = cx.new(|cx| TableState::new(AgingRows::default(), window, cx));
        let ledger_table = cx.new(|cx| TableState::new(LedgerRows::default(), window, cx));
        let payment_table = cx.new(|cx| TableState::new(PaymentRows::default(), window, cx));
        let payment_subscription = cx
            .subscribe(&payment_table, |_: &mut Self, _, _: &TableEvent, cx| {
                cx.notify()
            });
        let aging_subscription = cx.subscribe(&aging_table, |this: &mut Self, table, event, cx| {
            let TableEvent::DoubleClickedRow(row) = event else {
                return;
            };
            if let Some(customer) = table.read(cx).delegate().rows.get(*row) {
                this.selected_customer_id = Some(customer.customer_id.clone());
                this.ledger_page_idx = 0;
                this.payment_page_idx = 0;
                this.current_view = CurrentView::CustomerDetail;
                this.status_msg = format!(
                    "Membuka buku piutang pelanggan '{}'.",
                    customer.customer_name
                );
                cx.notify();
            }
        });
        let table_subscription =
            cx.subscribe(&customer_table, |this: &mut Self, table, event, cx| {
                let (row, open) = match event {
                    TableEvent::SelectRow(row) => (*row, false),
                    TableEvent::DoubleClickedRow(row) => (*row, true),
                    _ => return,
                };
                if let Some(customer) = table.read(cx).delegate().rows.get(row) {
                    if this.selected_customer_id.as_ref() != Some(&customer.id) {
                        this.ledger_page_idx = 0;
                        this.payment_page_idx = 0;
                    }
                    this.selected_customer_id = Some(customer.id.clone());
                    if open {
                        this.current_view = CurrentView::CustomerDetail;
                    }
                    cx.notify();
                }
            });
        let search_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Cari nama atau kode pelanggan"));
        let login_inputs = vec![
            cx.new(|cx| InputState::new(window, cx).placeholder("Nama pengguna")),
            cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Kata sandi")
                    .masked(true)
            }),
        ];
        let login_subscriptions = login_inputs
            .iter()
            .enumerate()
            .map(|(index, input)| {
                cx.subscribe_in(
                    input,
                    window,
                    move |this: &mut Self, input, event, _, cx| {
                        if this.active_modal != Some(ActiveModal::Login) {
                            return;
                        }
                        match event {
                            InputEvent::Change => {
                                if let Some(field) = this.form_fields.get_mut(index) {
                                    field.1 = input.read(cx).value().to_string();
                                }
                                this.form_error.clear();
                                cx.notify();
                            }
                            InputEvent::PressEnter { .. } => this.submit_modal(cx),
                            InputEvent::Focus => {
                                this.form_active = index;
                            }
                            _ => {}
                        }
                    },
                )
            })
            .collect();
        let customer_form_inputs: Vec<_> = [
            "Kode pelanggan",
            "Nama pelanggan",
            "Alamat pelanggan",
            "aktif atau nonaktif",
            "Catatan",
        ]
        .into_iter()
        .map(|placeholder| cx.new(|cx| InputState::new(window, cx).placeholder(placeholder)))
        .collect();
        let customer_form_subscriptions = customer_form_inputs
            .iter()
            .enumerate()
            .map(|(index, input)| {
                cx.subscribe_in(
                    input,
                    window,
                    move |this: &mut Self, input, event, _, cx| {
                        if !this.customer_form_initialized || !this.uses_generic_inputs() {
                            return;
                        }
                        match event {
                            InputEvent::Change => {
                                if let Some(field) = this.form_fields.get_mut(index) {
                                    field.1 = input.read(cx).value().to_string();
                                }
                                this.form_error.clear();
                                cx.notify();
                            }
                            InputEvent::Focus => {
                                if this.active_modal == Some(ActiveModal::RecordSale) {
                                    this.sale_active_field = (this.sale_items.len(), index);
                                } else {
                                    this.form_active = index;
                                }
                            }
                            InputEvent::PressEnter { .. } => this.submit_modal(cx),
                            _ => {}
                        }
                    },
                )
            })
            .collect();
        let search_subscription = cx.subscribe_in(
            &search_input,
            window,
            |this: &mut Self, input, event, _, cx| {
                if matches!(event, InputEvent::Change) {
                    this.query = input.read(cx).value().to_string();
                    this.customer_page = 0;
                    this.selected_customer_id = None;
                    cx.notify();
                }
            },
        );
        let store = build_store_from_env();
        // A fresh HTTP adapter has no persisted cookies or session to restore.
        let session = store.current_session();
        let today = Local::now().date_naive();
        let mut this = Self {
            store,
            session,
            current_view: CurrentView::CustomerList,
            selected_customer_id: None,
            open_customers: Vec::new(),
            detail_result: None,
            detail_pending: None,
            detail_generation: 0,
            customer_tab: CustomerTab::Ledger,
            query: String::new(),
            search_input,
            _search_subscription: search_subscription,
            login_inputs,
            _login_subscriptions: login_subscriptions,
            login_initialized: false,
            customer_form_inputs,
            _customer_form_subscriptions: customer_form_subscriptions,
            customer_form_initialized: false,
            sale_inputs: Vec::new(),
            sale_input_subscriptions: Vec::new(),
            sale_inputs_initialized: false,
            date_pickers,
            _date_subscriptions: date_subscriptions,
            picker_modal: None,
            customer_table,
            _customer_table_subscription: table_subscription,
            aging_table,
            _aging_table_subscription: aging_subscription,
            ledger_table,
            payment_table,
            _payment_table_subscription: payment_subscription,
            customer_active_only: false,
            customers_result: None,
            customers_pending: None,
            customers_generation: 0,
            users_result: None,
            users_pending: None,
            users_generation: 0,
            ledger_sort_desc: true, // Default sesuai Web: Terbaru di Atas
            ledger_from: None,
            ledger_to: None,
            status_msg: "Sistem siap. Gunakan panel navigasi kiri, tombol toolbar kontekstual, atau pintasan keyboard.".into(),
            active_modal: None,
            reviewing_transaction: false,
            transaction_uncertain: false,
            form_fields: Vec::new(),
            form_active: 0,
            form_error: String::new(),
            sale_items: vec![("".into(), "1".into(), "".into())],
            sale_active_field: (0, 0),
            aging_as_of: today,
            aging_result: None,
            aging_pending: None,
            aging_generation: 0,
            aging_selected_bucket: None,
            customer_page: 0,
            ledger_page_idx: 0,
            payment_page_idx: 0,
            page_size: 50,
            is_loading: false,
            loading_label: String::new(),
            focus: cx.focus_handle(),
            dialog_focus: cx.focus_handle(),
            previous_reviewing: false,
            dialog_view: None,
            kit_dialog_open: false,
        };
        let owner = cx.entity();
        this.dialog_view = Some(cx.new(|cx| {
            let subscription = cx.observe(&owner, |_, _, cx| cx.notify());
            DesktopDialogView {
                desktop: owner.downgrade(),
                _subscription: subscription,
            }
        }));
        if this.session.is_none() {
            this.open_login_modal();
            window.focus(&this.focus, cx);
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
        self.customer_form_initialized = false;
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
        let Some(detail) = self.current_detail().map(|data| data.customer.clone()) else {
            self.status_msg = "Tunggu detail pelanggan selesai dimuat.".into();
            return;
        };
        self.customer_form_initialized = false;
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
        self.form_fields = vec![
            (
                "Kode atau Nama Pelanggan Tujuan (Target Aktif)",
                String::new(),
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
        self.reviewing_transaction = false;
        self.transaction_uncertain = false;
        if !self.can_write() {
            self.status_msg =
                "Akses ditolak: Hanya Pemilik (Owner) dan Operator yang dapat mencatat penjualan kredit."
                    .into();
            return;
        }
        let Some(cust) = self.current_detail().map(|data| &data.customer) else {
            self.status_msg = "Tunggu detail pelanggan selesai dimuat.".into();
            return;
        };
        if !cust.is_active {
            self.status_msg = "Pelanggan ini berstatus Nonaktif. Aktifkan terlebih dahulu untuk mencatat penjualan.".into();
            return;
        }
        self.form_fields = vec![
            ("Tanggal Transaksi", self.today_jakarta().to_string()),
            ("No. Referensi / Nota (Opsional)", String::new()),
            (
                "Alasan Tanggal Lampau / Backdate (Wajib & khusus Owner jika tanggal < hari ini)",
                String::new(),
            ),
        ];
        self.sale_items = vec![("".into(), "1".into(), "".into())];
        self.sale_inputs_initialized = false;
        self.sale_active_field = (0, 0);
        self.form_active = 0;
        self.form_error.clear();
        self.active_modal = Some(ActiveModal::RecordSale);
    }

    pub fn open_record_payment_modal(&mut self) {
        self.reviewing_transaction = false;
        self.transaction_uncertain = false;
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
            ("Tanggal Diterima", self.today_jakarta().to_string()),
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

    pub fn open_apply_deposit_modal_for_payment(
        &mut self,
        preferred_payment_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.transaction_uncertain = false;
        if !self.can_write() {
            self.status_msg =
                "Akses ditolak: peran ini tidak dapat menerapkan saldo kredit.".into();
            return;
        }
        let Some(cid) = self.selected_customer_id.clone() else {
            self.status_msg = "Pilih pelanggan terlebih dahulu.".into();
            return;
        };
        if self.is_loading {
            return;
        }
        self.is_loading = true;
        self.loading_label = "Menyiapkan saldo kredit...".into();
        cx.notify();
        let reader = self.store.background_reader();
        let request = cx.background_spawn(async move {
            let payments = reader.list_payments(&cid)?;
            let ledger = reader.get_ledger(
                &cid,
                &LedgerQuery {
                    from: None,
                    to: None,
                    order_desc: false,
                },
            )?;
            let credit = payments.iter().find(|payment| {
                payment.unallocated_credit_idr > 0
                    && preferred_payment_id
                        .as_ref()
                        .is_none_or(|id| payment.id == *id)
            });
            let debit = ledger
                .entries
                .iter()
                .find(|entry| entry.remaining_idr > 0 && !entry.is_reversed);
            let payment_id = credit.map(|payment| payment.id.clone()).unwrap_or_default();
            let debit_id = debit.map(|entry| entry.id.clone()).unwrap_or_default();
            let amount = match (credit, debit) {
                (Some(payment), Some(entry)) => payment
                    .unallocated_credit_idr
                    .min(entry.remaining_idr)
                    .to_string(),
                _ => String::new(),
            };
            Ok::<_, AppError>((payment_id, debit_id, amount))
        });
        cx.spawn(async move |this, cx| {
            let result = request.await;
            let _ = this.update(cx, |this, cx| {
                this.is_loading = false;
                match result {
                    Ok((payment_id, debit_id, amount)) => {
                        this.form_fields = vec![
                            ("ID Pembayaran Sumber Saldo Kredit", payment_id),
                            ("ID Transaksi Debet (Piutang Terbuka) Tujuan", debit_id),
                            ("Nominal Kredit Diterapkan (Rp)", amount),
                            (
                                "Alasan Penerapan Saldo Kredit (Wajib untuk Audit)",
                                "Penerapan saldo lebih pembayaran ke nota terbuka".into(),
                            ),
                        ];
                        this.form_active = 2;
                        this.form_error.clear();
                        this.active_modal = Some(ActiveModal::ApplyDeposit);
                    }
                    Err(error) => {
                        this.status_msg = format!("Gagal memuat saldo kredit: {}", error.0)
                    }
                }
                cx.notify();
            });
        })
        .detach();
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
                "Dari Tanggal (kosongkan untuk dari awal)",
                self.ledger_from.map(|d| d.to_string()).unwrap_or_default(),
            ),
            (
                "Sampai Tanggal (kosongkan untuk sampai akhir)",
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
        self.login_initialized = false;
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

    fn logout(&mut self, cx: &mut Context<Self>) {
        if self.is_loading {
            return;
        }
        self.invalidate_aging();
        self.is_loading = true;
        self.loading_label = "Mengakhiri sesi...".into();
        cx.notify();
        let mut reader = self.store.background_reader();
        let request = cx.background_spawn(async move {
            reader.logout();
            reader
        });
        cx.spawn(async move |this, cx| {
            let reader = request.await;
            let _ = this.update(cx, |this, cx| {
                this.store = reader;
                this.is_loading = false;
                this.session = None;
                this.selected_customer_id = None;
                this.open_customers.clear();
                this.current_view = CurrentView::CustomerList;
                this.open_login_modal();
                cx.notify();
            });
        })
        .detach();
    }

    fn refresh_session(&mut self, cx: &mut Context<Self>) {
        if self.is_loading {
            return;
        }
        self.invalidate_aging();
        self.store.invalidate_cache();
        self.is_loading = true;
        self.loading_label = "Memuat ulang data...".into();
        cx.notify();
        let mut reader = self.store.background_reader();
        let request = cx.background_spawn(async move {
            let session = reader.restore_session();
            (reader, session)
        });
        cx.spawn(async move |this, cx| {
            let (reader, session) = request.await;
            let _ = this.update(cx, |this, cx| {
                this.store = reader;
                this.session = session;
                this.is_loading = false;
                if this.session.is_some() {
                    this.status_msg = "Data dimuat ulang dari sumber.".into();
                } else {
                    this.status_msg = "Sesi berakhir. Silakan masuk kembali.".into();
                    this.open_login_modal();
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn toggle_user_active(&mut self, user: UserItem, cx: &mut Context<Self>) {
        if self.is_loading {
            return;
        }
        self.is_loading = true;
        self.loading_label = "Memperbarui status akun...".into();
        cx.notify();
        let mut reader = self.store.background_reader();
        let request = cx.background_spawn(async move {
            let input = UpdateUserInput {
                username: user.username,
                role: user.role,
                is_active: !user.is_active,
            };
            let result = reader.update_user(&user.id, &input);
            (reader, result)
        });
        cx.spawn(async move |this, cx| {
            let (reader, result) = request.await;
            let _ = this.update(cx, |this, cx| {
                this.is_loading = false;
                match result {
                    Ok(updated) => {
                        this.store = reader;
                        this.invalidate_aging();
                        this.status_msg = format!(
                            "Status akun '{}' diubah menjadi {}.",
                            updated.username,
                            if updated.is_active {
                                "Aktif"
                            } else {
                                "Nonaktif"
                            }
                        );
                    }
                    Err(error) => this.status_msg = error.0,
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn invalidate_aging(&mut self) {
        self.aging_generation = self.aging_generation.wrapping_add(1);
        self.aging_result = None;
        self.aging_pending = None;
        self.customers_generation = self.customers_generation.wrapping_add(1);
        self.customers_result = None;
        self.customers_pending = None;
        self.detail_generation = self.detail_generation.wrapping_add(1);
        self.detail_result = None;
        self.detail_pending = None;
        self.users_generation = self.users_generation.wrapping_add(1);
        self.users_result = None;
        self.users_pending = None;
    }

    fn ensure_users_loaded(&mut self, cx: &mut Context<Self>) {
        if self.users_result.is_some() || self.users_pending.is_some() {
            return;
        }
        self.users_generation = self.users_generation.wrapping_add(1);
        let generation = self.users_generation;
        self.users_pending = Some(generation);
        let reader = self.store.background_reader();
        let request = cx.background_spawn(async move { reader.list_users() });
        cx.spawn(async move |this, cx| {
            let result = request.await;
            let _ = this.update(cx, |this, cx| {
                if this.users_pending == Some(generation) {
                    this.users_pending = None;
                    this.users_result = Some(result);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn detail_key(&self) -> Option<DetailKey> {
        Some(DetailKey {
            customer_id: self.selected_customer_id.clone()?,
            from: self.ledger_from,
            to: self.ledger_to,
            order_desc: self.ledger_sort_desc,
            tab: self.customer_tab,
        })
    }

    fn current_detail(&self) -> Option<&DetailData> {
        let key = self.detail_key()?;
        self.detail_result
            .as_ref()
            .filter(|(loaded, _)| loaded == &key)
            .and_then(|(_, result)| result.as_ref().ok())
    }

    fn ensure_detail_loaded(&mut self, cx: &mut Context<Self>) {
        let Some(key) = self.detail_key() else {
            return;
        };
        if self
            .detail_result
            .as_ref()
            .is_some_and(|(loaded, _)| loaded == &key)
            || self
                .detail_pending
                .as_ref()
                .is_some_and(|(_, pending)| pending == &key)
        {
            return;
        }
        self.detail_generation = self.detail_generation.wrapping_add(1);
        let generation = self.detail_generation;
        self.detail_pending = Some((generation, key.clone()));
        let reader = self.store.background_reader();
        let request_key = key.clone();
        let request = cx.background_spawn(async move {
            let customer = reader.get_customer(&request_key.customer_id)?;
            let ledger = reader.get_ledger(
                &request_key.customer_id,
                &LedgerQuery {
                    from: request_key.from,
                    to: request_key.to,
                    order_desc: request_key.order_desc,
                },
            )?;
            let payments = if request_key.tab == CustomerTab::Payments {
                reader.list_payments(&request_key.customer_id)?
            } else {
                Vec::new()
            };
            Ok::<_, AppError>(DetailData {
                customer,
                ledger,
                payments,
            })
        });
        cx.spawn(async move |this, cx| {
            let result = request.await;
            let _ = this.update(cx, |this, cx| {
                if this
                    .detail_pending
                    .as_ref()
                    .is_some_and(|(current, pending)| *current == generation && pending == &key)
                {
                    this.detail_pending = None;
                    this.detail_result = Some((key, result));
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn ensure_customers_loaded(&mut self, cx: &mut Context<Self>) {
        if self.customers_result.is_some() || self.customers_pending.is_some() {
            return;
        }
        self.customers_generation = self.customers_generation.wrapping_add(1);
        let generation = self.customers_generation;
        self.customers_pending = Some(generation);
        let reader = self.store.background_reader();
        let request = cx.background_spawn(async move {
            reader.list_customers_result(&CustomerQuery {
                search: None,
                active_only: false,
            })
        });
        cx.spawn(async move |this, cx| {
            let result = request.await;
            let _ = this.update(cx, |this, cx| {
                if this.customers_pending == Some(generation) {
                    this.customers_pending = None;
                    this.customers_result = Some(result);
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn ensure_aging_loaded(&mut self, cx: &mut Context<Self>) {
        let date = self.aging_as_of;
        if self
            .aging_result
            .as_ref()
            .is_some_and(|(loaded, _)| *loaded == date)
            || self
                .aging_pending
                .is_some_and(|(_, pending)| pending == date)
        {
            return;
        }
        self.aging_generation = self.aging_generation.wrapping_add(1);
        let generation = self.aging_generation;
        self.aging_pending = Some((generation, date));
        let reader = self.store.background_reader();
        let request = cx.background_spawn(async move { reader.get_aging_report(date) });
        cx.spawn(async move |this, cx| {
            let result = request.await;
            let _ = this.update(cx, |this, cx| {
                if this.aging_pending == Some((generation, date)) {
                    this.aging_pending = None;
                    this.aging_result = Some((date, result));
                    cx.notify();
                }
            });
        })
        .detach();
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
            "Tanggal Acuan Laporan Umur Piutang (maksimal hari ini)",
            self.aging_as_of.to_string(),
        )];
        self.form_active = 0;
        self.form_error.clear();
        self.active_modal = Some(ActiveModal::ChangeAsOfDate);
    }

    fn submit_modal(&mut self, cx: &mut Context<Self>) {
        if self.is_loading {
            return;
        }
        let Some(modal) = self.active_modal.clone() else {
            return;
        };

        match modal {
            ActiveModal::Login => {
                let username = self
                    .form_fields
                    .get(0)
                    .map(|f| f.1.clone())
                    .unwrap_or_default();
                let password = self
                    .form_fields
                    .get(1)
                    .map(|f| f.1.clone())
                    .unwrap_or_default();
                self.is_loading = true;
                self.form_error.clear();
                cx.notify();
                let mut reader = self.store.background_reader();
                let request = cx.background_spawn(async move {
                    let result = reader.login(&username, &password);
                    (reader, result)
                });
                cx.spawn(async move |this, cx| {
                    let (reader, result) = request.await;
                    let _ = this.update(cx, |this, cx| {
                        this.is_loading = false;
                        match result {
                            Ok(sess) => {
                                let must_change = sess.must_change_password;
                                this.status_msg = format!("Berhasil masuk sebagai {} ({}).", sess.username, sess.role.label());
                                if sess.role != UserRole::Owner && this.current_view == CurrentView::UserManagement {
                                    this.current_view = CurrentView::CustomerList;
                                }
                                this.store = reader;
                                this.session = Some(sess);
                                this.active_modal = None;
                                this.form_fields.clear();
                                if must_change {
                                    this.open_change_password_modal();
                                    this.form_error = "Akun menggunakan kata sandi sementara. Anda wajib menggantinya sebelum melanjutkan (AC-019).".into();
                                }
                            }
                            Err(error) => this.form_error = error.0,
                        }
                        cx.notify();
                    });
                }).detach();
                return;
            }
            ActiveModal::ChangePassword => {
                let old_pw = self
                    .form_fields
                    .get(0)
                    .map(|f| f.1.clone())
                    .unwrap_or_default();
                let new_pw = self
                    .form_fields
                    .get(1)
                    .map(|f| f.1.clone())
                    .unwrap_or_default();
                self.run_operation("Memperbarui kata sandi...", false, cx, move |store| {
                    store.change_password(&old_pw, &new_pw)?;
                    Ok(OperationOutcome::PasswordChanged)
                });
                return;
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

                self.run_operation("Menyimpan pelanggan...", false, cx, move |store| {
                    store
                        .create_customer(&input)
                        .map(OperationOutcome::CustomerCreated)
                });
                return;
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

                self.run_operation(
                    "Menyimpan perubahan pelanggan...",
                    false,
                    cx,
                    move |store| {
                        store
                            .update_customer(&customer_id, &input)
                            .map(OperationOutcome::CustomerUpdated)
                    },
                );
                return;
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

                self.run_operation("Menggabungkan pelanggan...", false, cx, move |store| {
                    let customers = store.list_customers_result(&CustomerQuery { search: None, active_only: false })?;
                    let target = customers.iter().find(|c| c.id != source_id && (c.name.to_lowercase() == target_key
                        || c.code.as_deref().is_some_and(|code| code.to_lowercase() == target_key)
                        || c.id.to_lowercase() == target_key))
                        .ok_or_else(|| AppError("Pelanggan tujuan tidak ditemukan. Periksa kode atau nama pelanggan.".into()))?;
                    let id = target.id.clone();
                    let name = target.name.clone();
                    store.merge_customers(&source_id, &id, &reason)?;
                    Ok(OperationOutcome::CustomerMerged { id, name })
                });
                return;
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
                    let Some(unit_price_idr) = parse_money_input(price_str) else {
                        self.form_error =
                            "Harga satuan harus berupa rupiah bulat yang valid.".into();
                        cx.notify();
                        return;
                    };
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

                if !self.reviewing_transaction {
                    let total = input.items.iter().try_fold(0_i64, |sum, item| {
                        if item.description.is_empty() {
                            return None;
                        }
                        let line =
                            compute_line_total_half_up(&item.quantity, item.unit_price_idr).ok()?;
                        sum.checked_add(line)
                    });
                    if input.items.is_empty() || total.is_none() {
                        self.form_error =
                            "Periksa nama barang, jumlah, dan harga sebelum meninjau.".into();
                        cx.notify();
                        return;
                    }
                    self.form_error.clear();
                    self.reviewing_transaction = true;
                    cx.notify();
                    return;
                }

                if self.transaction_uncertain {
                    return;
                }
                self.is_loading = true;
                self.loading_label = "Menyimpan belanja".into();
                cx.notify();
                let mut reader = self.store.background_reader();
                let request = cx.background_spawn(async move {
                    let result = reader.create_instant_sale(&cid, &input);
                    (reader, result)
                });
                cx.spawn(async move |this, cx| {
                    let (reader, result) = request.await;
                    let _ = this.update(cx, |this, cx| {
                        this.is_loading = false;
                        match result {
                            Ok(total) => {
                                this.store = reader;
                                this.invalidate_aging();
                                this.status_msg = format!("Penjualan kredit sebesar {} berhasil diposting ke Buku Piutang.", format_rupiah(total));
                                this.customer_tab = CustomerTab::Ledger;
                                this.active_modal = None;
                            }
                            Err(error) => {
                                this.transaction_uncertain = true;
                                this.form_error = format!("{} Periksa buku piutang sebelum mencoba transaksi lagi.", error.0);
                            }
                        }
                        cx.notify();
                    });
                }).detach();
                return;
            }
            ActiveModal::RecordPayment => {
                let Some(cid) = self.selected_customer_id.clone() else {
                    return;
                };
                let amt_raw = self.form_fields.get(0).map(|f| f.1.trim()).unwrap_or("");
                let Some(amount_idr) = parse_money_input(amt_raw) else {
                    self.form_error =
                        "Nominal pembayaran harus berupa rupiah bulat yang valid.".into();
                    cx.notify();
                    return;
                };

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

                if !self.reviewing_transaction {
                    if input.amount_idr <= 0 {
                        self.form_error = "Nominal pembayaran harus lebih dari nol.".into();
                        cx.notify();
                        return;
                    }
                    self.form_error.clear();
                    self.reviewing_transaction = true;
                    cx.notify();
                    return;
                }

                if self.transaction_uncertain {
                    return;
                }
                self.is_loading = true;
                self.loading_label = "Menyimpan pembayaran".into();
                cx.notify();
                let mut reader = self.store.background_reader();
                let request = cx.background_spawn(async move {
                    let result = reader.record_payment(&cid, &input);
                    (reader, result)
                });
                cx.spawn(async move |this, cx| {
                    let (reader, result) = request.await;
                    let _ = this.update(cx, |this, cx| {
                        this.is_loading = false;
                        match result {
                            Ok(pay) => {
                                this.store = reader;
                                this.invalidate_aging();
                                this.status_msg = format!("Pembayaran {} dicatat (Teralokasi FIFO: {}, Saldo Lebih/Kredit: {}).", format_rupiah(pay.amount_idr), format_rupiah(pay.allocated_idr), format_rupiah(pay.unallocated_credit_idr));
                                this.active_modal = None;
                            }
                            Err(error) => {
                                this.transaction_uncertain = true;
                                this.form_error = format!("{} Periksa riwayat pembayaran sebelum mencoba lagi.", error.0);
                            }
                        }
                        cx.notify();
                    });
                }).detach();
                return;
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
                let Some(amount_idr) = parse_money_input(amt_raw) else {
                    self.form_error = "Nominal harus berupa rupiah bulat yang valid.".into();
                    cx.notify();
                    return;
                };
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

                if self.transaction_uncertain {
                    return;
                }
                self.run_operation("Menerapkan saldo kredit...", true, cx, move |store| {
                    store.apply_credit(&cid, &input)?;
                    Ok(OperationOutcome::CreditApplied(amount_idr))
                });
                return;
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
                self.ledger_page_idx = 0;
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

                self.run_operation("Membuat akun...", false, cx, move |store| {
                    store.create_user(&input).map(OperationOutcome::UserCreated)
                });
                return;
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

                let input = UpdateUserInput {
                    username,
                    role,
                    is_active,
                };
                self.run_operation("Memperbarui akun...", false, cx, move |store| {
                    store
                        .update_user(&user_id, &input)
                        .map(OperationOutcome::UserUpdated)
                });
                return;
            }
            ActiveModal::ResetUserPassword { user_id, username } => {
                let temp_pw = self
                    .form_fields
                    .get(0)
                    .map(|f| f.1.clone())
                    .unwrap_or_default();
                self.run_operation("Mereset kata sandi...", false, cx, move |store| {
                    store.reset_password(&user_id, &temp_pw)?;
                    Ok(OperationOutcome::PasswordReset(username))
                });
                return;
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
        self.invalidate_aging();
        self.session = self.store.current_session();
        cx.notify();
    }

    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let key = &event.keystroke;

        // 1. Penanganan input saat Modal Dialog terbuka
        if let Some(modal) = &self.active_modal {
            if *modal == ActiveModal::Login {
                return;
            }
            if self.uses_generic_inputs() && !self.reviewing_transaction {
                if key.key == "escape" && !self.is_loading {
                    self.active_modal = None;
                    window.focus(&self.focus, cx);
                    cx.notify();
                    cx.stop_propagation();
                }
                return;
            }
            if self.date_pickers.iter().enumerate().any(|(slot, picker)| {
                self.date_field_index(slot).is_some()
                    && picker.read(cx).focus_handle(cx).is_focused(window)
            }) {
                return;
            }
            cx.stop_propagation();
            if self.is_loading {
                return;
            }
            if self.reviewing_transaction
                && matches!(modal, ActiveModal::RecordSale | ActiveModal::RecordPayment)
            {
                match key.key.as_str() {
                    "escape" => {
                        self.reviewing_transaction = false;
                        self.form_error.clear();
                        self.focus_active_modal_field(window, cx);
                        cx.notify();
                    }
                    "enter" => self.submit_modal(cx),
                    _ => {}
                }
                return;
            }
            match key.key.as_str() {
                "escape" => {
                    if !matches!(modal, ActiveModal::Login) {
                        self.active_modal = None;
                    }
                    cx.notify();
                    return;
                }
                "enter" => {
                    self.submit_modal(cx);
                    return;
                }
                "tab" => {
                    self.next_modal_field();
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
            self.logout(cx);
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

        // The component owns text editing, selection, clipboard, and IME.
        if self.current_view == CurrentView::CustomerList
            && self
                .search_input
                .read(cx)
                .focus_handle(cx)
                .is_focused(window)
            && !key.modifiers.control
            && (key.key_char.is_some() || key.key == "backspace" || key.key == "space")
        {
            return;
        }

        match key.key.as_str() {
            "f2" => {
                self.current_view = CurrentView::CustomerList;
                self.status_msg = "Modul: Buku Ringkasan Saldo Pelanggan.".into();
                window.focus(&self.focus, cx);
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
                    self.open_apply_deposit_modal_for_payment(None, cx);
                    cx.notify();
                }
            }
            "f6" => {
                self.current_view = CurrentView::AgingReport;
                self.status_msg = "Modul: Laporan Analisis Umur Piutang (Aging Report).".into();
                window.focus(&self.focus, cx);
                cx.notify();
            }
            "f9" => {
                self.refresh_session(cx);
            }
            "f8" => {
                if self.is_owner() {
                    self.current_view = CurrentView::UserManagement;
                    self.status_msg = "Modul: Manajemen Akun & Otorisasi Pengguna.".into();
                    window.focus(&self.focus, cx);
                } else {
                    self.status_msg =
                        "Akses ditolak: Manajemen Pengguna hanya untuk Pemilik (Owner).".into();
                }
                cx.notify();
            }
            "enter" => {
                if self.current_view == CurrentView::CustomerList
                    && self
                        .customer_table
                        .read(cx)
                        .selected_row()
                        .and_then(|row| self.customer_table.read(cx).delegate().rows.get(row))
                        .is_some_and(|customer| {
                            Some(&customer.id) == self.selected_customer_id.as_ref()
                        })
                {
                    self.current_view = CurrentView::CustomerDetail;
                    self.status_msg = "Membuka detail buku piutang pelanggan terpilih.".into();
                    cx.notify();
                }
            }
            "escape" => {
                if self.current_view == CurrentView::CustomerDetail {
                    self.current_view = CurrentView::CustomerList;
                    self.status_msg = "Kembali ke Buku Ringkasan Saldo Pelanggan.".into();
                } else if self.current_view == CurrentView::CustomerList && !self.query.is_empty() {
                    self.query.clear();
                    self.search_input
                        .update(cx, |input, cx| input.set_value("", window, cx));
                    self.status_msg = "Pencarian pelanggan dibersihkan.".into();
                }
                cx.notify();
            }
            _ => {}
        }
    }
}

impl Render for Desktop {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_date_pickers(window, cx);
        if self.reviewing_transaction && !self.previous_reviewing {
            cx.on_next_frame(window, |this, window, cx| {
                if this.reviewing_transaction {
                    window.focus(&this.dialog_focus, cx);
                }
            });
        }
        self.previous_reviewing = self.reviewing_transaction;
        let should_open_dialog = self.session.is_some()
            && self
                .active_modal
                .as_ref()
                .is_some_and(|modal| *modal != ActiveModal::Login);
        if should_open_dialog != self.kit_dialog_open {
            self.kit_dialog_open = should_open_dialog;
            cx.defer_in(window, move |this, window, cx| {
                let still_needed = this.session.is_some()
                    && this
                        .active_modal
                        .as_ref()
                        .is_some_and(|modal| *modal != ActiveModal::Login);
                if still_needed && should_open_dialog {
                    let Some(view) = this.dialog_view.clone() else {
                        return;
                    };
                    let desktop = cx.weak_entity();
                    window.open_dialog(cx, move |dialog, _, cx| {
                        let width = desktop
                            .upgrade()
                            .map(|entity| {
                                let state = entity.read(cx);
                                if state.reviewing_transaction {
                                    560.0
                                } else if state.active_modal == Some(ActiveModal::RecordSale) {
                                    800.0
                                } else {
                                    520.0
                                }
                            })
                            .unwrap_or(520.0);
                        dialog
                            .centered()
                            .w(px(width))
                            .p_0()
                            .close_button(false)
                            .overlay_closable(false)
                            .keyboard(false)
                            .child(view.clone())
                    });
                    cx.on_next_frame(window, |this, window, cx| {
                        if this.active_modal.is_some() {
                            if matches!(this.active_modal, Some(ActiveModal::Notice { .. })) {
                                window.focus(&this.dialog_focus, cx);
                            } else {
                                this.focus_active_modal_field(window, cx);
                            }
                        }
                    });
                } else if !still_needed && !should_open_dialog {
                    window.close_dialog(cx);
                }
            });
        }
        if self.active_modal == Some(ActiveModal::RecordSale) {
            if !self.sale_inputs_initialized {
                self.sale_inputs.clear();
                self.sale_input_subscriptions.clear();
                for (row, (description, qty, price)) in self.sale_items.iter().enumerate() {
                    let mut fields = Vec::new();
                    for (col, (placeholder, value)) in [
                        ("Nama barang", description),
                        ("1", qty),
                        ("Contoh: 150.000", price),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        let input =
                            cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
                        input.update(cx, |state, cx| state.set_value(value.clone(), window, cx));
                        let subscription = cx.subscribe_in(
                            &input,
                            window,
                            move |this: &mut Self, input, event, _, cx| {
                                if this.active_modal != Some(ActiveModal::RecordSale)
                                    || !this.sale_inputs_initialized
                                {
                                    return;
                                }
                                match event {
                                    InputEvent::Change => {
                                        if let Some(item) = this.sale_items.get_mut(row) {
                                            let value = input.read(cx).value().to_string();
                                            match col {
                                                0 => item.0 = value,
                                                1 => item.1 = value,
                                                _ => item.2 = value,
                                            }
                                        }
                                        this.form_error.clear();
                                        cx.notify();
                                    }
                                    InputEvent::Focus => this.sale_active_field = (row, col),
                                    InputEvent::PressEnter { .. } => this.submit_modal(cx),
                                    _ => {}
                                }
                            },
                        );
                        self.sale_input_subscriptions.push(subscription);
                        fields.push(input);
                    }
                    self.sale_inputs.push(fields);
                }
                self.sale_inputs_initialized = true;
                self.focus_active_modal_field(window, cx);
            }
        } else {
            self.sale_inputs_initialized = false;
            self.sale_inputs.clear();
            self.sale_input_subscriptions.clear();
        }
        if self.uses_generic_inputs() {
            if !self.customer_form_initialized {
                for (index, input) in self.customer_form_inputs.iter().enumerate() {
                    let value = self
                        .form_fields
                        .get(index)
                        .map(|field| field.1.clone())
                        .unwrap_or_default();
                    let label = self
                        .form_fields
                        .get(index)
                        .map(|field| field.0)
                        .unwrap_or("");
                    let lower = label.to_lowercase();
                    let placeholder = if lower.contains("nominal")
                        || lower.contains("harga")
                        || lower.contains("jumlah (rp)")
                    {
                        "Contoh: 150.000"
                    } else {
                        label
                    };
                    input.update(cx, |state, cx| {
                        state.set_masked(lower.contains("sandi"), window, cx);
                        state.set_placeholder(placeholder, window, cx);
                        state.set_value(value, window, cx);
                    });
                }
                self.customer_form_initialized = true;
                self.focus_active_modal_field(window, cx);
            }
        } else if self.customer_form_initialized {
            self.customer_form_initialized = false;
            for input in &self.customer_form_inputs {
                input.update(cx, |state, cx| {
                    state.set_value("", window, cx);
                    state.set_masked(false, window, cx);
                });
            }
        }
        if !self.is_loading && self.session.is_some() && self.store.current_session().is_none() {
            self.invalidate_aging();
            self.session = None;
            self.selected_customer_id = None;
            self.open_customers.clear();
            self.current_view = CurrentView::CustomerList;
            self.open_login_modal();
            self.form_error = if self.transaction_uncertain {
                "Sesi berakhir saat transaksi diproses. Masuk kembali dan periksa buku piutang atau riwayat sebelum mencoba lagi.".into()
            } else {
                "Sesi telah berakhir. Silakan masuk kembali.".into()
            };
            window.focus(&self.focus, cx);
        }
        if self.session.is_none() {
            if !self.login_initialized {
                self.login_initialized = true;
                for (index, input) in self.login_inputs.iter().enumerate() {
                    let value = self
                        .form_fields
                        .get(index)
                        .map(|field| field.1.clone())
                        .unwrap_or_default();
                    input.update(cx, |state, cx| state.set_value(value, window, cx));
                }
                window.focus(&self.login_inputs[0].read(cx).focus_handle(cx), cx);
            }
            return div()
                .size_full()
                .key_context("DesktopModal")
                .on_action(cx.listener(|this, _: &ModalNextField, window, cx| {
                    this.next_modal_field();
                    window.focus(
                        &this.login_inputs[this.form_active]
                            .read(cx)
                            .focus_handle(cx),
                        cx,
                    );
                    cx.notify();
                    cx.stop_propagation();
                }))
                .on_action(cx.listener(|this, _: &ModalPreviousField, window, cx| {
                    this.previous_modal_field();
                    window.focus(
                        &this.login_inputs[this.form_active]
                            .read(cx)
                            .focus_handle(cx),
                        cx,
                    );
                    cx.notify();
                    cx.stop_propagation();
                }))
                .track_focus(&self.focus)
                .on_key_down(cx.listener(Self::key_down))
                .child(self.render_login_page(cx))
                .into_any_element();
        }
        if self.login_initialized {
            self.login_initialized = false;
            for input in &self.login_inputs {
                input.update(cx, |state, cx| state.set_value("", window, cx));
            }
        }
        // Do not render data-backed views while a write is in flight: those
        // views still use synchronous readers and may otherwise block the UI.
        if self.is_loading {
            return div()
                .size_full()
                .track_focus(&self.focus)
                .bg(rgb(Theme::BG_APP))
                .flex()
                .items_center()
                .justify_center()
                .child(render_loading_state(&self.loading_label))
                .into_any_element();
        }
        if self.current_view == CurrentView::AgingReport {
            self.ensure_aging_loaded(cx);
        }
        if self.current_view == CurrentView::CustomerList {
            self.ensure_customers_loaded(cx);
        }
        if self.current_view == CurrentView::CustomerDetail {
            self.ensure_detail_loaded(cx);
        }
        if self.current_view == CurrentView::UserManagement && self.is_owner() {
            self.ensure_users_loaded(cx);
        }
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
            .detail_result
            .as_ref()
            .filter(|(key, _)| Some(key) == self.detail_key().as_ref())
            .and_then(|(_, result)| result.as_ref().ok())
            .map(|data| data.customer.clone());

        if self.current_view == CurrentView::CustomerDetail {
            if let Some(customer) = &selected_cust_summary {
                if !self.open_customers.iter().any(|(id, _)| id == &customer.id) {
                    self.open_customers
                        .push((customer.id.clone(), customer.name.clone()));
                }
            }
        }

        div()
            .when(self.is_loading, |d| {
                d.capture_any_mouse_down(|_, _, cx| cx.stop_propagation())
            })
            .when(self.active_modal.is_some(), |d| {
                d.key_context("DesktopModal")
            })
            .on_action(cx.listener(|this, _: &ModalNextField, window, cx| {
                if this.active_modal.is_some() && !this.reviewing_transaction {
                    this.next_modal_field();
                    this.focus_active_modal_field(window, cx);
                    cx.notify();
                }
                cx.stop_propagation();
            }))
            .on_action(cx.listener(|this, _: &ModalPreviousField, window, cx| {
                if this.active_modal.is_some() && !this.reviewing_transaction {
                    this.previous_modal_field();
                    this.focus_active_modal_field(window, cx);
                    cx.notify();
                }
                cx.stop_propagation();
            }))
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key_down))
            .size_full()
            .bg(rgb(Theme::BG_APP))
            .flex()
            .flex_col()
            .overflow_hidden()
            .when(is_sim, |d| {
                d.child(
                    div()
                        .bg(rgb(0xFEF3C7))
                        .text_color(rgb(0x78350F))
                        .px(px(14.))
                        .py(px(5.))
                        .text_size(px(12.))
                        .child(
                            "DATA CONTOH - Perubahan hanya simulasi dan tidak tersimpan di server.",
                        ),
                )
            })
            // 1. NATIVE WINDOWS SYSTEM MENU STRIP (26px)
            .child(
                render_system_menu_bar()
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
                                    // Pelanggan yang dibuka selama sesi ini.
                                    .children(self.open_customers.iter().map(|(id, name)| {
                                        let customer_id = id.clone();
                                        let close_id = id.clone();
                                        let active = self.current_view
                                            == CurrentView::CustomerDetail
                                            && self.selected_customer_id.as_ref() == Some(id);
                                        div()
                                            .id(format!("open-customer-{}", id))
                                            .h(px(30.0))
                                            .mx(px(6.0))
                                            .pl(px(14.0))
                                            .pr(px(4.0))
                                            .rounded(px(4.0))
                                            .bg(rgb(if active {
                                                Theme::BG_SIDEBAR_ACTIVE
                                            } else {
                                                Theme::BG_SIDEBAR
                                            }))
                                            .flex()
                                            .items_center()
                                            .gap(px(4.0))
                                            .child(
                                                div()
                                                    .id(format!("open-customer-select-{}", id))
                                                    .flex_1()
                                                    .min_w_0()
                                                    .overflow_hidden()
                                                    .whitespace_nowrap()
                                                    .cursor_pointer()
                                                    .text_size(px(11.0))
                                                    .text_color(rgb(Theme::TEXT_PRIMARY))
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        this.selected_customer_id =
                                                            Some(customer_id.clone());
                                                        this.current_view =
                                                            CurrentView::CustomerDetail;
                                                        this.ledger_page_idx = 0;
                                                        this.payment_page_idx = 0;
                                                        cx.notify();
                                                    }))
                                                    .child(name.clone()),
                                            )
                                            .child(
                                                div()
                                                    .id(format!("open-customer-close-{}", id))
                                                    .px(px(5.0))
                                                    .cursor_pointer()
                                                    .child("x")
                                                    .on_click(cx.listener(
                                                        move |this, _, _, cx| {
                                                            this.open_customers
                                                                .retain(|(id, _)| id != &close_id);
                                                            if this.current_view
                                                                == CurrentView::CustomerDetail
                                                                && this
                                                                    .selected_customer_id
                                                                    .as_ref()
                                                                    == Some(&close_id)
                                                            {
                                                                this.selected_customer_id = None;
                                                                this.current_view =
                                                                    CurrentView::CustomerList;
                                                            }
                                                            cx.stop_propagation();
                                                            cx.notify();
                                                        },
                                                    )),
                                            )
                                    }))
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
                                        sidebar_nav_item("⇄", "Logout", "Ctrl+L", false)
                                            .id("side-nav-login")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.logout(cx);
                                                cx.notify();
                                            })),
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
            .child({
                let status_display = if self.is_loading {
                    format!("⏳ {} | {}", self.loading_label, self.status_msg)
                } else {
                    self.status_msg.clone()
                };
                StatusBar::new(
                    &status_display,
                    self.active_module_name(),
                    &date_label,
                    &user_display,
                )
                .adapter(self.store.adapter_label())
                .render()
            })
            // 4. MODAL DIALOGS OVERLAY
            .into_any_element()
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
            CurrentView::CustomerList => {
                "Cari nama/kode • Pilih baris • Enter: Detail • Ctrl+N: Baru"
            }
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
            .h(px(74.0))
            .min_w_0()
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
                    .flex_col()
                    .gap(px(3.0))
                    .child(
                        div()
                            .text_size(px(10.5))
                            .font_weight(FontWeight::BOLD)
                            .text_color(rgb(Theme::TEXT_MUTED))
                            .child(title),
                    )
                    .child(
                        div()
                            .text_size(px(15.0))
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
            .w(px(160.0))
            .flex_shrink_0()
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
}

fn main() {
    app::run();
}

#[cfg(test)]
mod input_tests {
    use super::{compute_line_total_half_up, parse_money_input};

    #[test]
    fn money_input_rejects_signs_malformed_groups_and_overflow() {
        assert_eq!(parse_money_input("1.234.567"), Some(1_234_567));
        assert_eq!(parse_money_input("1234567"), Some(1_234_567));
        for invalid in [
            "",
            "-100",
            "1,5",
            "1.23",
            ".123",
            "1..000",
            "9999999999999999999999999",
        ] {
            assert_eq!(parse_money_input(invalid), None, "{invalid}");
        }
    }

    #[test]
    fn line_total_rejects_amount_outside_idr_range() {
        assert!(compute_line_total_half_up("2", i64::MAX).is_err());
        assert!(compute_line_total_half_up("-0.1", 1000).is_err());
        assert!(compute_line_total_half_up("170141183460469231731687303715884105727", 1).is_err());
    }
}
