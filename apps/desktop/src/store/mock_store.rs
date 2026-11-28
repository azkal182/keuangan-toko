//! Implementasi MockDataAdapter in-memory yang mengimplementasikan TokoStoreAdapter.
//! Seluruh perilaku mengikuti spesifikasi final (ADR-007, PRODUCT_SPEC.md, FEATURE_SPECIFICATIONS.md, DATA_DICTIONARY.md):
//! - BR-005: Nilai uang integer IDR (i64), quantity decimal skala maks 3, pembulatan half-up per item.
//! - BR-006 / FR-003: Pembayaran dialokasikan FIFO ke debit terbuka terlama; surplus menjadi Saldo Kredit Pelanggan
//!   dan TIDAK auto-apply pada penjualan baru (hanya diterapkan melalui aksi manual eksplisit dengan alasan audit).
//! - BR-012 / FR-002: Tanggal masa depan ditolak; backdate hanya Owner dengan alasan wajib.
//! - FR-004: Saldo berjalan dihitung kronologis `(effective_date, source_order)` dari awal histori meskipun difilter tanggal.
//! - FR-005: 5 bucket umur piutang (0-15, 16-30, 31-45, 46-60, >60) + unknown_date + future_date = total_open_idr.
//! - FR-007: Reversal, Reallocation, Merge Customer, dan User Management khusus Owner; kata sandi minimal 15 karakter.
#![allow(
    clippy::collapsible_if,
    clippy::manual_is_multiple_of,
    clippy::unnecessary_sort_by,
    clippy::new_without_default
)]

use super::*;
use chrono::{Local, NaiveDate};
use std::collections::BTreeMap;
use uuid::Uuid;

fn format_idr(amount: i64) -> String {
    let s = amount.abs().to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    for (i, c) in chars.into_iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push('.');
        }
        result.push(c);
    }
    if amount < 0 {
        format!("-{}", result)
    } else {
        result
    }
}

/// Pembulatan half-up untuk quantity string (maks 3 desimal) dikali harga satuan IDR integer.
pub fn compute_line_total_half_up(
    quantity_str: &str,
    unit_price_idr: i64,
) -> Result<i64, AppError> {
    let clean = quantity_str.trim().replace(',', ".");
    if clean.is_empty() {
        return Err(AppError("Kuantitas barang wajib diisi.".into()));
    }
    if !clean.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return Err(AppError("Kuantitas harus berupa angka positif.".into()));
    }
    let parts: Vec<&str> = clean.split('.').collect();
    if parts.len() > 2 {
        return Err(AppError("Format kuantitas tidak valid.".into()));
    }
    let whole: i128 = parts[0]
        .parse()
        .map_err(|_| AppError("Kuantitas harus berupa angka positif.".into()))?;
    let frac_str = if parts.len() == 2 { parts[1] } else { "" };
    if frac_str.len() > 3 {
        return Err(AppError(
            "Kuantitas maksimal memiliki 3 angka di belakang koma.".into(),
        ));
    }
    let mut frac_padded = frac_str.to_string();
    while frac_padded.len() < 3 {
        frac_padded.push('0');
    }
    let frac: i128 = if frac_padded.is_empty() {
        0
    } else {
        frac_padded
            .parse()
            .map_err(|_| AppError("Desimal kuantitas tidak valid.".into()))?
    };
    let milli_qty = whole
        .checked_mul(1000)
        .and_then(|value| value.checked_add(frac))
        .ok_or_else(|| AppError("Kuantitas terlalu besar.".into()))?;
    if milli_qty <= 0 {
        return Err(AppError("Kuantitas harus lebih besar dari nol.".into()));
    }
    if unit_price_idr <= 0 {
        return Err(AppError(
            "Harga satuan harus berupa bilangan rupiah bulat lebih dari nol.".into(),
        ));
    }
    // Half-up rounding: (milli_qty * unit_price + 500) / 1000
    let product = milli_qty
        .checked_mul(unit_price_idr as i128)
        .and_then(|value| value.checked_add(500))
        .ok_or_else(|| AppError("Total barang terlalu besar.".into()))?;
    let rounded = product / 1000;
    i64::try_from(rounded).map_err(|_| AppError("Total barang terlalu besar.".into()))
}

#[derive(Debug, Clone)]
struct InternalAllocation {
    payment_id: String,
    debit_id: String,
    amount_idr: i64,
    #[allow(dead_code)]
    reason: String,
}

#[derive(Debug, Clone)]
struct InternalLedgerEntry {
    id: String,
    entry_type: String,
    source_id: String,
    effective_date: Option<NaiveDate>,
    reference: Option<String>,
    description: String,
    quantity: Option<String>,
    unit_price_idr: Option<i64>,
    debit_idr: i64,
    credit_idr: i64,
    is_reversed: bool,
    reversal_reason: Option<String>,
    replacement_entry_id: Option<String>,
    source_order: i64,
}

#[derive(Debug, Clone)]
struct InternalPayment {
    id: String,
    customer_id: String,
    amount_idr: i64,
    received_date: NaiveDate,
    method: String,
    reference: Option<String>,
    note: Option<String>,
    status: String,
    version: i32,
}

#[derive(Debug, Clone)]
struct InternalCustomer {
    id: String,
    code: Option<String>,
    name: String,
    address: Option<String>,
    is_active: bool,
    version: i32,
    entries: Vec<InternalLedgerEntry>,
    payments: Vec<InternalPayment>,
    allocations: Vec<InternalAllocation>,
}

#[derive(Debug, Clone)]
struct InternalUser {
    id: String,
    username: String,
    role: UserRole,
    password: String,
    is_active: bool,
    must_change_password: bool,
    created_at: NaiveDate,
}

#[derive(Clone)]
pub struct MockDataAdapter {
    users: Vec<InternalUser>,
    customers: BTreeMap<String, InternalCustomer>,
    current_user_id: Option<String>,
    next_source_order: i64,
}

impl MockDataAdapter {
    pub fn new() -> Self {
        let mut store = Self {
            users: Vec::new(),
            customers: BTreeMap::new(),
            current_user_id: None,
            next_source_order: 200,
        };
        store.populate_initial_data();
        store
    }

    fn current_role(&self) -> Option<UserRole> {
        let uid = self.current_user_id.as_ref()?;
        self.users.iter().find(|u| &u.id == uid).map(|u| u.role)
    }

    fn ensure_can_write(&self) -> Result<UserRole, AppError> {
        match self.current_role() {
            Some(role @ (UserRole::Owner | UserRole::Operator)) => Ok(role),
            Some(UserRole::Viewer) => Err(AppError(
                "Akses ditolak: Peran Pembaca (Viewer) hanya memiliki izin baca.".into(),
            )),
            None => Err(AppError(
                "Sesi belum aktif. Silakan masuk terlebih dahulu.".into(),
            )),
        }
    }

    fn ensure_owner(&self) -> Result<(), AppError> {
        match self.current_role() {
            Some(UserRole::Owner) => Ok(()),
            Some(_) => Err(AppError(
                "Akses ditolak: Operasi ini hanya diizinkan untuk Pemilik (Owner).".into(),
            )),
            None => Err(AppError(
                "Sesi belum aktif. Silakan masuk terlebih dahulu.".into(),
            )),
        }
    }

    fn populate_initial_data(&mut self) {
        let created_date = NaiveDate::from_ymd_opt(2025, 9, 1).unwrap();
        self.users = vec![
            InternalUser {
                id: "u1111111-1111-1111-1111-111111111111".into(),
                username: "owner".into(),
                role: UserRole::Owner,
                password: "password1234567".into(),
                is_active: true,
                must_change_password: false,
                created_at: created_date,
            },
            InternalUser {
                id: "u2222222-2222-2222-2222-222222222222".into(),
                username: "operator_kasir".into(),
                role: UserRole::Operator,
                password: "password1234567".into(),
                is_active: true,
                must_change_password: false,
                created_at: NaiveDate::from_ymd_opt(2025, 10, 12).unwrap(),
            },
            InternalUser {
                id: "u3333333-3333-3333-3333-333333333333".into(),
                username: "auditor_viewer".into(),
                role: UserRole::Viewer,
                password: "password1234567".into(),
                is_active: true,
                must_change_password: false,
                created_at: NaiveDate::from_ymd_opt(2025, 11, 5).unwrap(),
            },
        ];
        self.current_user_id = Some(self.users[0].id.clone());

        // Data di bawah dibuat khusus untuk demo; tidak berasal dari workbook pelanggan.
        let today = Local::now().date_naive();
        for (id, name, code, active, debit, dated) in [
            (
                "demo-1",
                "Pelanggan Contoh A",
                "DEMO-01",
                true,
                100_000_i64,
                true,
            ),
            ("demo-2", "Pelanggan Contoh B", "DEMO-02", true, 0_i64, true),
            (
                "demo-3",
                "Pelanggan Contoh C",
                "DEMO-03",
                true,
                18_000_i64,
                true,
            ),
            (
                "demo-4",
                "Pelanggan Contoh D",
                "DEMO-04",
                true,
                45_000_i64,
                false,
            ),
            (
                "demo-5",
                "Pelanggan Contoh Nonaktif",
                "DEMO-05",
                false,
                0_i64,
                true,
            ),
        ] {
            let entries = if debit > 0 {
                vec![InternalLedgerEntry {
                    id: format!("{id}-debit"),
                    entry_type: "credit_sale".into(),
                    source_id: format!("{id}-sale"),
                    effective_date: dated.then_some(today),
                    reference: None,
                    description: "Barang contoh".into(),
                    quantity: Some("1".into()),
                    unit_price_idr: Some(debit),
                    debit_idr: debit,
                    credit_idr: 0,
                    is_reversed: false,
                    reversal_reason: None,
                    replacement_entry_id: None,
                    source_order: 1,
                }]
            } else {
                Vec::new()
            };
            self.customers.insert(
                id.into(),
                InternalCustomer {
                    id: id.into(),
                    code: Some(code.into()),
                    name: name.into(),
                    address: None,
                    is_active: active,
                    version: 1,
                    entries,
                    payments: Vec::new(),
                    allocations: Vec::new(),
                },
            );
        }
        // Saldo kredit contoh tanpa debit terbuka; tidak dipakai otomatis untuk penjualan baru.
        self.record_payment(
            "demo-2",
            &RecordPaymentInput {
                received_date: today,
                amount_idr: 500_000,
                method: "cash".into(),
                reference: None,
                note: Some("Pembayaran contoh".into()),
            },
        )
        .expect("seed pembayaran contoh valid");
    }

    fn calculate_customer_balances(
        &self,
        customer: &InternalCustomer,
        as_of: NaiveDate,
    ) -> (i64, i64, i64, AgingBuckets) {
        let mut total_debit = 0_i64;
        let mut total_credit = 0_i64;
        let mut ledger_balance = 0_i64;
        let mut allocated_per_debit: BTreeMap<String, i64> = BTreeMap::new();

        for alloc in &customer.allocations {
            *allocated_per_debit
                .entry(alloc.debit_id.clone())
                .or_insert(0) += alloc.amount_idr;
        }

        let mut aging = AgingBuckets::default();

        for entry in &customer.entries {
            // Ledger keeps both the original and its contra-entry. Open items and
            // available credit instead use only transactions that remain active.
            ledger_balance += entry.debit_idr - entry.credit_idr;
            if entry.is_reversed || entry.entry_type == "reversal" {
                continue;
            }
            total_debit += entry.debit_idr;
            total_credit += entry.credit_idr;

            if entry.debit_idr > 0 {
                let allocated = *allocated_per_debit.get(&entry.id).unwrap_or(&0);
                let remaining = entry.debit_idr.saturating_sub(allocated);
                if remaining > 0 {
                    match entry.effective_date {
                        None => {
                            aging.unknown_date += remaining;
                        }
                        Some(d) => {
                            let days = (as_of - d).num_days();
                            if days < 0 {
                                aging.future_date += remaining;
                            } else if days <= 15 {
                                aging.days_0_15 += remaining;
                            } else if days <= 30 {
                                aging.days_16_30 += remaining;
                            } else if days <= 45 {
                                aging.days_31_45 += remaining;
                            } else if days <= 60 {
                                aging.days_46_60 += remaining;
                            } else {
                                aging.days_over_60 += remaining;
                            }
                        }
                    }
                }
            }
        }

        let total_allocated: i64 = customer.allocations.iter().map(|a| a.amount_idr).sum();
        let outstanding = total_debit.saturating_sub(total_allocated);
        let customer_credit = total_credit.saturating_sub(total_allocated);
        (outstanding, customer_credit, ledger_balance, aging)
    }
}

impl TokoStoreAdapter for MockDataAdapter {
    fn background_reader(&self) -> Box<dyn TokoStoreAdapter> {
        Box::new(self.clone())
    }
    fn adapter_label(&self) -> String {
        "MockStore (In-Memory)".into()
    }

    fn is_simulation(&self) -> bool {
        true
    }

    fn current_session(&self) -> Option<SessionUser> {
        let user_id = self.current_user_id.as_ref()?;
        let u = self.users.iter().find(|user| &user.id == user_id)?;
        Some(SessionUser {
            id: u.id.clone(),
            username: u.username.clone(),
            role: u.role,
            must_change_password: u.must_change_password,
        })
    }

    fn login(&mut self, username: &str, password: &str) -> Result<SessionUser, AppError> {
        let user = self
            .users
            .iter()
            .find(|u| u.username.eq_ignore_ascii_case(username.trim()) && u.password == password)
            .ok_or_else(|| AppError("Nama pengguna atau kata sandi tidak valid.".into()))?;

        if !user.is_active {
            return Err(AppError(
                "Akun pengguna telah dinonaktifkan. Hubungi Pemilik (Owner).".into(),
            ));
        }

        self.current_user_id = Some(user.id.clone());
        Ok(self.current_session().unwrap())
    }

    fn logout(&mut self) {
        self.current_user_id = None;
    }

    fn switch_session_role(&mut self, role: UserRole) {
        if let Some(target) = self
            .users
            .iter()
            .find(|u| u.role == role && u.is_active)
            .map(|u| u.id.clone())
        {
            self.current_user_id = Some(target);
        } else if let Some(user_id) = &self.current_user_id {
            if let Some(u) = self.users.iter_mut().find(|u| &u.id == user_id) {
                u.role = role;
            }
        }
    }

    fn change_password(&mut self, old_pw: &str, new_pw: &str) -> Result<(), AppError> {
        let user_id = self
            .current_user_id
            .as_ref()
            .ok_or_else(|| AppError("Belum masuk sesi.".into()))?;
        let u = self
            .users
            .iter_mut()
            .find(|u| &u.id == user_id)
            .ok_or_else(|| AppError("Pengguna tidak ditemukan.".into()))?;

        if u.password != old_pw {
            return Err(AppError("Kata sandi saat ini tidak sesuai.".into()));
        }
        if new_pw.len() < 15 {
            return Err(AppError(
                "Kata sandi baru wajib minimal 15 karakter sesuai kebijakan keamanan (API-004)."
                    .into(),
            ));
        }
        if new_pw == old_pw {
            return Err(AppError(
                "Kata sandi baru tidak boleh sama dengan kata sandi lama.".into(),
            ));
        }

        u.password = new_pw.to_string();
        u.must_change_password = false;
        Ok(())
    }

    fn list_customers(&self, query: &CustomerQuery) -> Vec<CustomerSummary> {
        let today = Local::now().date_naive();
        let mut list = Vec::new();
        let search_term = query.search.as_deref().unwrap_or("").trim().to_lowercase();

        for cust in self.customers.values() {
            if query.active_only && !cust.is_active {
                continue;
            }
            if !search_term.is_empty() {
                let match_name = cust.name.to_lowercase().contains(&search_term);
                let match_code = cust
                    .code
                    .as_deref()
                    .map(|c| c.to_lowercase().contains(&search_term))
                    .unwrap_or(false);
                let match_addr = cust
                    .address
                    .as_deref()
                    .map(|a| a.to_lowercase().contains(&search_term))
                    .unwrap_or(false);
                if !match_name && !match_code && !match_addr {
                    continue;
                }
            }

            let (outstanding, credit, _, aging) = self.calculate_customer_balances(cust, today);
            list.push(CustomerSummary {
                id: cust.id.clone(),
                code: cust.code.clone(),
                name: cust.name.clone(),
                address: cust.address.clone(),
                is_active: cust.is_active,
                version: cust.version,
                outstanding_idr: outstanding,
                customer_credit_idr: credit,
                aging,
            });
        }

        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    fn get_customer(&self, id: &str) -> Result<CustomerDetail, AppError> {
        let today = Local::now().date_naive();
        let cust = self
            .customers
            .get(id)
            .ok_or_else(|| AppError("Pelanggan tidak ditemukan.".into()))?;
        let (outstanding, credit, _, _) = self.calculate_customer_balances(cust, today);
        Ok(CustomerDetail {
            id: cust.id.clone(),
            code: cust.code.clone(),
            name: cust.name.clone(),
            address: cust.address.clone(),
            is_active: cust.is_active,
            version: cust.version,
            outstanding_idr: outstanding,
            customer_credit_idr: credit,
        })
    }

    fn create_customer(&mut self, input: &CreateCustomerInput) -> Result<CustomerDetail, AppError> {
        self.ensure_can_write()?;
        let name_clean = input.name.trim();
        if name_clean.is_empty() {
            return Err(AppError("Nama pelanggan wajib diisi.".into()));
        }

        let code_normalized = input
            .code
            .as_ref()
            .map(|c| c.trim().to_uppercase())
            .filter(|c| !c.is_empty());

        if let Some(ref code_clean) = code_normalized {
            if self.customers.values().any(|c| {
                c.code
                    .as_deref()
                    .unwrap_or("")
                    .eq_ignore_ascii_case(code_clean)
            }) {
                return Err(AppError(format!(
                    "Kode pelanggan '{}' sudah digunakan pelanggan lain (409 CUSTOMER_CODE_CONFLICT).",
                    code_clean
                )));
            }
        }

        let id = Uuid::new_v4().to_string();
        let cust = InternalCustomer {
            id: id.clone(),
            code: code_normalized,
            name: name_clean.to_string(),
            address: input
                .address
                .as_ref()
                .map(|a| a.trim().to_string())
                .filter(|a| !a.is_empty()),
            is_active: true,
            version: 1,
            entries: Vec::new(),
            payments: Vec::new(),
            allocations: Vec::new(),
        };

        self.customers.insert(id.clone(), cust);
        self.get_customer(&id)
    }

    fn update_customer(
        &mut self,
        id: &str,
        input: &UpdateCustomerInput,
    ) -> Result<CustomerDetail, AppError> {
        self.ensure_can_write()?;
        let name_clean = input.name.trim();
        if name_clean.is_empty() {
            return Err(AppError("Nama pelanggan wajib diisi.".into()));
        }

        let code_normalized = input
            .code
            .as_ref()
            .map(|c| c.trim().to_uppercase())
            .filter(|c| !c.is_empty());

        if let Some(ref code_clean) = code_normalized {
            if self.customers.values().any(|c| {
                c.id != id
                    && c.code
                        .as_deref()
                        .unwrap_or("")
                        .eq_ignore_ascii_case(code_clean)
            }) {
                return Err(AppError(format!(
                    "Kode pelanggan '{}' sudah digunakan pelanggan lain (409 CUSTOMER_CODE_CONFLICT).",
                    code_clean
                )));
            }
        }

        let cust = self
            .customers
            .get_mut(id)
            .ok_or_else(|| AppError("Pelanggan tidak ditemukan.".into()))?;

        cust.name = name_clean.to_string();
        cust.code = code_normalized;
        cust.address = input
            .address
            .as_ref()
            .map(|a| a.trim().to_string())
            .filter(|a| !a.is_empty());
        cust.is_active = input.is_active;
        cust.version += 1;

        self.get_customer(id)
    }

    fn toggle_customer_active(&mut self, id: &str) -> Result<bool, AppError> {
        self.ensure_can_write()?;
        let cust = self
            .customers
            .get_mut(id)
            .ok_or_else(|| AppError("Pelanggan tidak ditemukan.".into()))?;
        cust.is_active = !cust.is_active;
        cust.version += 1;
        Ok(cust.is_active)
    }

    fn merge_customers(
        &mut self,
        source_id: &str,
        target_id: &str,
        reason: &str,
    ) -> Result<(), AppError> {
        self.ensure_owner()?;
        if reason.trim().is_empty() {
            return Err(AppError(
                "Alasan penggabungan (merge) pelanggan wajib diisi untuk audit.".into(),
            ));
        }
        if source_id == target_id {
            return Err(AppError(
                "Pelanggan sumber dan tujuan penggabungan tidak boleh sama.".into(),
            ));
        }
        if !self.customers.contains_key(target_id) {
            return Err(AppError("Pelanggan tujuan tidak ditemukan.".into()));
        }

        let mut source = self
            .customers
            .remove(source_id)
            .ok_or_else(|| AppError("Pelanggan sumber tidak ditemukan.".into()))?;
        let target = self
            .customers
            .get_mut(target_id)
            .ok_or_else(|| AppError("Pelanggan tujuan tidak ditemukan.".into()))?;

        if !target.is_active {
            self.customers.insert(source_id.to_string(), source);
            return Err(AppError("Pelanggan tujuan harus berstatus Aktif.".into()));
        }

        // Pindahkan histori tanpa auto-apply saldo kredit (BR-006 / API-107)
        for p in &mut source.payments {
            p.customer_id = target_id.to_string();
        }
        target.entries.append(&mut source.entries);
        target.payments.append(&mut source.payments);
        target.allocations.append(&mut source.allocations);
        target.version += 1;

        source.is_active = false;
        source.version += 1;
        self.customers.insert(source_id.to_string(), source);

        Ok(())
    }

    fn get_ledger(&self, customer_id: &str, query: &LedgerQuery) -> Result<LedgerPage, AppError> {
        let cust = self
            .customers
            .get(customer_id)
            .ok_or_else(|| AppError("Pelanggan tidak ditemukan.".into()))?;

        if let (Some(from), Some(to)) = (query.from, query.to) {
            if from > to {
                return Err(AppError(
                    "Tanggal awal filter tidak boleh melebihi tanggal akhir.".into(),
                ));
            }
        }

        let today = Local::now().date_naive();
        let (outstanding_idr, customer_credit_idr, ledger_balance_idr, _) =
            self.calculate_customer_balances(cust, today);

        let mut allocated_per_debit: BTreeMap<String, i64> = BTreeMap::new();
        for alloc in &cust.allocations {
            *allocated_per_debit
                .entry(alloc.debit_id.clone())
                .or_insert(0) += alloc.amount_idr;
        }

        // API-105: Urutkan dari tanggal usaha terlama, lalu source_order, dengan tanggal tidak diketahui (None) di akhir
        let mut chronological_entries = cust.entries.clone();
        chronological_entries.sort_by(|a, b| {
            match (a.effective_date, b.effective_date) {
                (Some(da), Some(db)) => da.cmp(&db),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
            .then_with(|| a.source_order.cmp(&b.source_order))
        });

        let mut running_balance = 0_i64;
        let mut calculated_list = Vec::new();

        for e in chronological_entries {
            // Semua entry di ledger (termasuk kontra-entry reversal) ikut mutasi running balance
            running_balance += e.debit_idr - e.credit_idr;

            // Filter rentang tanggal setelah saldo berjalan dihitung (sesuai API-105)
            if let Some(from) = query.from {
                match e.effective_date {
                    Some(d) if d < from => continue,
                    None => continue,
                    _ => {}
                }
            }
            if let Some(to) = query.to {
                match e.effective_date {
                    Some(d) if d > to => continue,
                    None => continue,
                    _ => {}
                }
            }

            let (allocated, remaining, status, age) = if e.debit_idr > 0 && !e.is_reversed {
                let alloc = *allocated_per_debit.get(&e.id).unwrap_or(&0);
                let rem = e.debit_idr.saturating_sub(alloc);
                let st = if rem == 0 {
                    AllocationStatus::Allocated
                } else if alloc > 0 {
                    AllocationStatus::PartiallyAllocated
                } else {
                    AllocationStatus::Unallocated
                };
                let age_days = if rem > 0 {
                    e.effective_date.map(|d| (today - d).num_days() as i32)
                } else {
                    None
                };
                (alloc, rem, Some(st), age_days)
            } else {
                (0, 0, None, None)
            };

            calculated_list.push(LedgerEntry {
                id: e.id,
                entry_type: e.entry_type,
                source_id: e.source_id,
                effective_date: e.effective_date,
                reference: e.reference,
                description: e.description,
                quantity: e.quantity,
                unit_price_idr: e.unit_price_idr,
                debit_idr: e.debit_idr,
                credit_idr: e.credit_idr,
                running_balance_idr: running_balance,
                allocated_idr: allocated,
                remaining_idr: remaining,
                age_days: age,
                allocation_status: status,
                is_reversed: e.is_reversed,
                reversal_reason: e.reversal_reason,
                replacement_entry_id: e.replacement_entry_id,
                source_order: e.source_order,
            });
        }

        if query.order_desc {
            calculated_list.reverse();
        }

        Ok(LedgerPage {
            entries: calculated_list,
            summary: LedgerSummary {
                outstanding_idr,
                customer_credit_idr,
                ledger_balance_idr,
            },
        })
    }

    fn create_instant_sale(
        &mut self,
        customer_id: &str,
        input: &InstantSaleInput,
    ) -> Result<i64, AppError> {
        let _role = self.ensure_can_write()?;
        let today = Local::now().date_naive();

        // Tanggal masa depan ditolak; tanggal lampau diperbolehkan tanpa wajib alasan
        if input.effective_date > today {
            return Err(AppError(
                "Tanggal transaksi penjualan tidak boleh berada di masa depan.".into(),
            ));
        }

        let cust = self
            .customers
            .get(customer_id)
            .ok_or_else(|| AppError("Pelanggan tidak ditemukan.".into()))?;
        if !cust.is_active {
            return Err(AppError(
                "Pelanggan sedang nonaktif. Tidak dapat mencatat penjualan baru.".into(),
            ));
        }

        let mut prepared_items = Vec::new();
        let mut total_sale = 0_i64;
        for item in &input.items {
            let desc_clean = item.description.trim();
            if desc_clean.is_empty() && item.unit_price_idr <= 0 {
                continue;
            }
            if desc_clean.is_empty() {
                return Err(AppError("Nama / uraian barang tidak boleh kosong.".into()));
            }
            let line_total = compute_line_total_half_up(&item.quantity, item.unit_price_idr)?;
            total_sale += line_total;
            prepared_items.push((
                desc_clean.to_string(),
                item.quantity.trim().to_string(),
                item.unit_price_idr,
                line_total,
            ));
        }

        if prepared_items.is_empty() || total_sale <= 0 {
            return Err(AppError(
                "Minimal masukkan 1 barang dengan nama, kuantitas, dan harga satuan valid.".into(),
            ));
        }

        let mut current_order = self.next_source_order;
        let cust = self.customers.get_mut(customer_id).unwrap();
        let ref_clean = input
            .reference_no
            .as_ref()
            .map(|r| r.trim().to_string())
            .filter(|r| !r.is_empty());

        for (desc, qty, price, line_total) in prepared_items {
            let entry_id = Uuid::new_v4().to_string();
            let sale_id = Uuid::new_v4().to_string();
            cust.entries.push(InternalLedgerEntry {
                id: entry_id,
                entry_type: "credit_sale".into(),
                source_id: sale_id,
                effective_date: Some(input.effective_date),
                reference: ref_clean.clone(),
                description: desc,
                quantity: Some(qty),
                unit_price_idr: Some(price),
                debit_idr: line_total,
                credit_idr: 0,
                is_reversed: false,
                reversal_reason: None,
                replacement_entry_id: None,
                source_order: current_order,
            });
            current_order += 1;
        }
        self.next_source_order = current_order;

        // Auto-apply deposit pelanggan FIFO saat penjualan baru (Opsi A)
        let payment_ids: Vec<String> = cust
            .payments
            .iter()
            .filter(|p| p.status == "posted")
            .map(|p| p.id.clone())
            .collect();
        for pay_id in payment_ids {
            let payment_amt = cust
                .payments
                .iter()
                .find(|p| p.id == pay_id)
                .map(|p| p.amount_idr)
                .unwrap_or(0);
            let pay_allocated: i64 = cust
                .allocations
                .iter()
                .filter(|a| a.payment_id == pay_id)
                .map(|a| a.amount_idr)
                .sum();
            let mut unallocated = payment_amt.saturating_sub(pay_allocated);
            if unallocated <= 0 {
                continue;
            }

            let entry_ids: Vec<String> = cust
                .entries
                .iter()
                .filter(|e| e.debit_idr > 0 && !e.is_reversed)
                .map(|e| e.id.clone())
                .collect();
            for ent_id in entry_ids {
                let debit_amt = cust
                    .entries
                    .iter()
                    .find(|e| e.id == ent_id)
                    .map(|e| e.debit_idr)
                    .unwrap_or(0);
                let debit_allocated: i64 = cust
                    .allocations
                    .iter()
                    .filter(|a| a.debit_id == ent_id)
                    .map(|a| a.amount_idr)
                    .sum();
                let debit_remaining = debit_amt.saturating_sub(debit_allocated);
                if debit_remaining <= 0 {
                    continue;
                }

                let to_apply = unallocated.min(debit_remaining);
                if to_apply > 0 {
                    cust.allocations.push(InternalAllocation {
                        payment_id: pay_id.clone(),
                        debit_id: ent_id,
                        amount_idr: to_apply,
                        reason: "Auto-apply dari deposit pelanggan".into(),
                    });
                    unallocated -= to_apply;
                    if unallocated == 0 {
                        break;
                    }
                }
            }
        }

        Ok(total_sale)
    }

    fn reverse_transaction(
        &mut self,
        customer_id: &str,
        entry_id: &str,
        reason: &str,
    ) -> Result<(), AppError> {
        // FR-007 / API-304: Reversal hanya Owner dengan alasan wajib
        self.ensure_owner()?;
        let reason_clean = reason.trim();
        if reason_clean.is_empty() {
            return Err(AppError(
                "Alasan pembalikan (reversal) transaksi wajib diisi untuk audit.".into(),
            ));
        }

        let next_order = self.next_source_order;
        let cust = self
            .customers
            .get_mut(customer_id)
            .ok_or_else(|| AppError("Pelanggan tidak ditemukan.".into()))?;

        let idx = cust
            .entries
            .iter()
            .position(|e| e.id == entry_id)
            .ok_or_else(|| AppError("Transaksi buku piutang tidak ditemukan.".into()))?;

        if cust.entries[idx].is_reversed {
            return Err(AppError(
                "Transaksi ini sudah pernah dibalik (reversed) sebelumnya.".into(),
            ));
        }
        if cust.entries[idx].entry_type == "reversal" {
            return Err(AppError(
                "Entri pembalikan (kontra-entry) tidak dapat dibalik ulang.".into(),
            ));
        }

        let orig_id = cust.entries[idx].id.clone();
        let orig_source_id = cust.entries[idx].source_id.clone();
        let orig_desc = cust.entries[idx].description.clone();
        let orig_debit = cust.entries[idx].debit_idr;
        let orig_credit = cust.entries[idx].credit_idr;

        cust.entries[idx].is_reversed = true;
        cust.entries[idx].reversal_reason = Some(reason_clean.to_string());

        // Jika yang dibalik adalah payment, tandai status payment sebagai reversed
        for p in &mut cust.payments {
            if p.id == orig_id || p.id == orig_source_id {
                p.status = "reversed".into();
                p.version += 1;
            }
        }

        // Pulihkan alokasi aktif yang terkait dengan debit atau payment ini
        cust.allocations.retain(|a| {
            a.debit_id != orig_id && a.payment_id != orig_id && a.payment_id != orig_source_id
        });

        // Buat kontra-entry immutable di buku piutang
        let reversal_id = Uuid::new_v4().to_string();
        cust.entries.push(InternalLedgerEntry {
            id: reversal_id,
            entry_type: "reversal".into(),
            source_id: orig_id,
            effective_date: Some(Local::now().date_naive()),
            reference: None,
            description: format!("PEMBALIKAN: {} — Alasan: {}", orig_desc, reason_clean),
            quantity: None,
            unit_price_idr: None,
            debit_idr: orig_credit,
            credit_idr: orig_debit,
            is_reversed: false,
            reversal_reason: Some(reason_clean.to_string()),
            replacement_entry_id: None,
            source_order: next_order,
        });
        self.next_source_order += 1;

        Ok(())
    }

    fn list_payments(&self, customer_id: &str) -> Result<Vec<PaymentRow>, AppError> {
        let cust = self
            .customers
            .get(customer_id)
            .ok_or_else(|| AppError("Pelanggan tidak ditemukan.".into()))?;

        let debit_descriptions: BTreeMap<String, String> = cust
            .entries
            .iter()
            .map(|e| (e.id.clone(), e.description.clone()))
            .collect();

        let mut list = Vec::new();
        for p in &cust.payments {
            let allocations: Vec<PaymentAllocationItem> = cust
                .allocations
                .iter()
                .filter(|a| a.payment_id == p.id)
                .map(|a| PaymentAllocationItem {
                    debit_entry_id: a.debit_id.clone(),
                    amount_idr: a.amount_idr,
                    debit_description: debit_descriptions
                        .get(&a.debit_id)
                        .cloned()
                        .unwrap_or_else(|| "Nota Debet".into()),
                })
                .collect();

            let allocated: i64 = allocations.iter().map(|a| a.amount_idr).sum();
            let unallocated = if p.status == "reversed" {
                0
            } else {
                p.amount_idr.saturating_sub(allocated)
            };

            list.push(PaymentRow {
                id: p.id.clone(),
                customer_id: p.customer_id.clone(),
                amount_idr: p.amount_idr,
                received_date: p.received_date,
                method: p.method.clone(),
                reference: p.reference.clone(),
                note: p.note.clone(),
                status: p.status.clone(),
                version: p.version,
                allocated_idr: allocated,
                unallocated_credit_idr: unallocated,
                allocations,
            });
        }

        list.sort_by(|a, b| b.received_date.cmp(&a.received_date));
        Ok(list)
    }

    fn record_payment(
        &mut self,
        customer_id: &str,
        input: &RecordPaymentInput,
    ) -> Result<PaymentRow, AppError> {
        self.ensure_can_write()?;
        if input.amount_idr <= 0 {
            return Err(AppError(
                "Nominal pembayaran harus berupa bilangan rupiah bulat lebih dari nol.".into(),
            ));
        }
        let today = Local::now().date_naive();
        if input.received_date > today {
            return Err(AppError(
                "Tanggal penerimaan pembayaran tidak boleh berada di masa depan.".into(),
            ));
        }

        let method_norm = match input.method.trim().to_lowercase().as_str() {
            "bank_transfer" | "transfer bank" | "transfer" => "bank_transfer",
            _ => "cash",
        };
        let method_display = if method_norm == "bank_transfer" {
            "Transfer Bank"
        } else {
            "Tunai"
        };

        let next_order = self.next_source_order;
        let cust = self
            .customers
            .get_mut(customer_id)
            .ok_or_else(|| AppError("Pelanggan tidak ditemukan.".into()))?;

        let payment_id = Uuid::new_v4().to_string();
        let ref_clean = input
            .reference
            .as_ref()
            .map(|r| r.trim().to_string())
            .filter(|r| !r.is_empty());
        let note_clean = input
            .note
            .as_ref()
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty());

        let payment = InternalPayment {
            id: payment_id.clone(),
            customer_id: customer_id.to_string(),
            amount_idr: input.amount_idr,
            received_date: input.received_date,
            method: method_norm.to_string(),
            reference: ref_clean.clone(),
            note: note_clean,
            status: "posted".into(),
            version: 1,
        };

        let desc = match &ref_clean {
            Some(r) => format!("PEMBAYARAN ({}) — Ref: {}", method_display, r),
            None => format!("PEMBAYARAN ({})", method_display),
        };
        cust.entries.push(InternalLedgerEntry {
            id: payment_id.clone(),
            entry_type: "payment".into(),
            source_id: payment_id.clone(),
            effective_date: Some(input.received_date),
            reference: ref_clean,
            description: desc,
            quantity: None,
            unit_price_idr: None,
            debit_idr: 0,
            credit_idr: input.amount_idr,
            is_reversed: false,
            reversal_reason: None,
            replacement_entry_id: None,
            source_order: next_order,
        });
        self.next_source_order += 1;
        cust.payments.push(payment);

        // FR-003 / BR-003: Alokasi FIFO ke debit terbuka saat itu berdasarkan (effective_date, source_order)
        let mut allocated_per_debit: BTreeMap<String, i64> = BTreeMap::new();
        for alloc in &cust.allocations {
            *allocated_per_debit
                .entry(alloc.debit_id.clone())
                .or_insert(0) += alloc.amount_idr;
        }

        let mut remaining_payment = input.amount_idr;
        let mut open_debits: Vec<(String, i64, Option<NaiveDate>, i64)> = cust
            .entries
            .iter()
            .filter(|e| e.debit_idr > 0 && !e.is_reversed && e.entry_type != "reversal")
            .map(|e| {
                let allocated = *allocated_per_debit.get(&e.id).unwrap_or(&0);
                (
                    e.id.clone(),
                    e.debit_idr.saturating_sub(allocated),
                    e.effective_date,
                    e.source_order,
                )
            })
            .filter(|(_, open, _, _)| *open > 0)
            .collect();

        open_debits.sort_by(|a, b| {
            match (a.2, b.2) {
                (Some(da), Some(db)) => da.cmp(&db),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
            .then_with(|| a.3.cmp(&b.3))
        });

        for (debit_id, open_amount, _, _) in open_debits {
            if remaining_payment <= 0 {
                break;
            }
            let alloc = open_amount.min(remaining_payment);
            cust.allocations.push(InternalAllocation {
                payment_id: payment_id.clone(),
                debit_id,
                amount_idr: alloc,
                reason: "Alokasi Otomatis FIFO".into(),
            });
            remaining_payment -= alloc;
        }

        let payments = self.list_payments(customer_id)?;
        payments
            .into_iter()
            .find(|p| p.id == payment_id)
            .ok_or_else(|| AppError("Gagal merekam pembayaran.".into()))
    }

    fn apply_credit(
        &mut self,
        customer_id: &str,
        input: &ApplyCreditInput,
    ) -> Result<(), AppError> {
        self.ensure_can_write()?;
        if input.amount_idr <= 0 {
            return Err(AppError(
                "Nominal saldo kredit yang diterapkan harus lebih dari nol.".into(),
            ));
        }
        let reason_clean = input.reason.trim();
        if reason_clean.is_empty() {
            return Err(AppError(
                "Alasan penerapan saldo kredit wajib diisi untuk audit (API-302).".into(),
            ));
        }

        let cust = self
            .customers
            .get_mut(customer_id)
            .ok_or_else(|| AppError("Pelanggan tidak ditemukan.".into()))?;

        let payment = cust
            .payments
            .iter()
            .find(|p| p.id == input.payment_id && p.status == "posted")
            .ok_or_else(|| AppError("Pembayaran sumber saldo kredit tidak ditemukan.".into()))?;

        let pay_allocated: i64 = cust
            .allocations
            .iter()
            .filter(|a| a.payment_id == input.payment_id)
            .map(|a| a.amount_idr)
            .sum();
        let unallocated = payment.amount_idr.saturating_sub(pay_allocated);

        if input.amount_idr > unallocated {
            return Err(AppError(format!(
                "Saldo kredit tidak mencukupi (Tersedia: Rp {}, Diminta: Rp {}).",
                format_idr(unallocated),
                format_idr(input.amount_idr)
            )));
        }

        let debit_entry = cust
            .entries
            .iter()
            .find(|e| e.id == input.debit_entry_id && e.debit_idr > 0 && !e.is_reversed)
            .ok_or_else(|| AppError("Transaksi piutang (debet) tujuan tidak ditemukan.".into()))?;

        let debit_allocated: i64 = cust
            .allocations
            .iter()
            .filter(|a| a.debit_id == input.debit_entry_id)
            .map(|a| a.amount_idr)
            .sum();
        let debit_remaining = debit_entry.debit_idr.saturating_sub(debit_allocated);

        if input.amount_idr > debit_remaining {
            return Err(AppError(format!(
                "Nominal melebihi sisa piutang nota tujuan (Sisa Nota: Rp {}, Diminta: Rp {}).",
                format_idr(debit_remaining),
                format_idr(input.amount_idr)
            )));
        }

        cust.allocations.push(InternalAllocation {
            payment_id: input.payment_id.clone(),
            debit_id: input.debit_entry_id.clone(),
            amount_idr: input.amount_idr,
            reason: reason_clean.to_string(),
        });

        Ok(())
    }

    fn reallocate_payment(
        &mut self,
        customer_id: &str,
        input: &ReallocatePaymentInput,
    ) -> Result<(), AppError> {
        // API-305: Realokasi pembayaran khusus Owner dengan alasan wajib
        self.ensure_owner()?;
        let reason_clean = input.reason.trim();
        if reason_clean.is_empty() {
            return Err(AppError(
                "Alasan realokasi pembayaran wajib diisi untuk audit (API-305).".into(),
            ));
        }

        let cust = self
            .customers
            .get_mut(customer_id)
            .ok_or_else(|| AppError("Pelanggan tidak ditemukan.".into()))?;

        let pay_amount = cust
            .payments
            .iter()
            .find(|p| p.id == input.payment_id && p.status == "posted")
            .map(|p| p.amount_idr)
            .ok_or_else(|| AppError("Pembayaran tidak ditemukan atau sudah dibalik.".into()))?;

        if input.amount_idr < 0 || input.amount_idr > pay_amount {
            return Err(AppError(format!(
                "Nominal realokasi harus di antara Rp 0 hingga Rp {}.",
                format_idr(pay_amount)
            )));
        }

        // Validasi debit baru jika ada
        if let Some(ref did) = input.debit_entry_id {
            if !did.trim().is_empty() && input.amount_idr > 0 {
                let debit_entry = cust
                    .entries
                    .iter()
                    .find(|e| &e.id == did && e.debit_idr > 0 && !e.is_reversed)
                    .ok_or_else(|| {
                        AppError("Nota debet tujuan realokasi tidak ditemukan.".into())
                    })?;

                let other_allocated: i64 = cust
                    .allocations
                    .iter()
                    .filter(|a| &a.debit_id == did && a.payment_id != input.payment_id)
                    .map(|a| a.amount_idr)
                    .sum();
                let available_room = debit_entry.debit_idr.saturating_sub(other_allocated);
                if input.amount_idr > available_room {
                    return Err(AppError(format!(
                        "Nominal realokasi melebihi sisa tagihan nota tujuan (Maks: Rp {}).",
                        format_idr(available_room)
                    )));
                }
            }
        }

        // Hapus alokasi lama dari payment ini dan tulis alokasi pengganti secara atomik
        cust.allocations
            .retain(|a| a.payment_id != input.payment_id);
        if let Some(ref did) = input.debit_entry_id {
            if !did.trim().is_empty() && input.amount_idr > 0 {
                cust.allocations.push(InternalAllocation {
                    payment_id: input.payment_id.clone(),
                    debit_id: did.clone(),
                    amount_idr: input.amount_idr,
                    reason: reason_clean.to_string(),
                });
            }
        }

        if let Some(p) = cust.payments.iter_mut().find(|p| p.id == input.payment_id) {
            p.version += 1;
        }

        Ok(())
    }

    fn get_aging_report(&self, as_of: NaiveDate) -> Result<AgingReport, AppError> {
        let today = Local::now().date_naive();
        if as_of > today {
            return Err(AppError(
                "Tanggal acuan (as_of) laporan umur piutang tidak boleh di masa depan (API-501)."
                    .into(),
            ));
        }

        let mut report = AgingReport {
            as_of,
            total_open_idr: 0,
            days_0_15: 0,
            days_16_30: 0,
            days_31_45: 0,
            days_46_60: 0,
            days_over_60: 0,
            unknown_date: 0,
            future_date: 0,
            customers: Vec::new(),
        };

        for cust in self.customers.values() {
            let (c_total, _, _, c_aging) = self.calculate_customer_balances(cust, as_of);

            if c_total > 0 {
                report.total_open_idr += c_total;
                report.days_0_15 += c_aging.days_0_15;
                report.days_16_30 += c_aging.days_16_30;
                report.days_31_45 += c_aging.days_31_45;
                report.days_46_60 += c_aging.days_46_60;
                report.days_over_60 += c_aging.days_over_60;
                report.unknown_date += c_aging.unknown_date;
                report.future_date += c_aging.future_date;

                report.customers.push(AgingReportCustomerItem {
                    customer_id: cust.id.clone(),
                    customer_code: cust.code.clone(),
                    customer_name: cust.name.clone(),
                    is_active: cust.is_active,
                    total_outstanding_idr: c_total,
                    days_0_15: c_aging.days_0_15,
                    days_16_30: c_aging.days_16_30,
                    days_31_45: c_aging.days_31_45,
                    days_46_60: c_aging.days_46_60,
                    days_over_60: c_aging.days_over_60,
                    unknown_date: c_aging.unknown_date,
                    future_date: c_aging.future_date,
                });
            }
        }

        report
            .customers
            .sort_by(|a, b| b.total_outstanding_idr.cmp(&a.total_outstanding_idr));
        Ok(report)
    }

    fn list_users(&self) -> Result<Vec<UserItem>, AppError> {
        self.ensure_owner()?;
        let list = self
            .users
            .iter()
            .map(|u| UserItem {
                id: u.id.clone(),
                username: u.username.clone(),
                role: u.role,
                is_active: u.is_active,
                must_change_password: u.must_change_password,
                created_at: u.created_at,
            })
            .collect();
        Ok(list)
    }

    fn create_user(&mut self, input: &CreateUserInput) -> Result<UserItem, AppError> {
        self.ensure_owner()?;
        let username_clean = input.username.trim().to_lowercase();
        if username_clean.is_empty() {
            return Err(AppError("Nama pengguna (username) wajib diisi.".into()));
        }
        if input.temporary_password.len() < 15 {
            return Err(AppError(
                "Kata sandi sementara wajib minimal 15 karakter sesuai kebijakan keamanan (API-006)."
                    .into(),
            ));
        }
        if self
            .users
            .iter()
            .any(|u| u.username.eq_ignore_ascii_case(&username_clean))
        {
            return Err(AppError("Username sudah terdaftar di sistem.".into()));
        }

        let user = InternalUser {
            id: Uuid::new_v4().to_string(),
            username: username_clean,
            role: input.role,
            password: input.temporary_password.clone(),
            is_active: true,
            must_change_password: true,
            created_at: Local::now().date_naive(),
        };

        let item = UserItem {
            id: user.id.clone(),
            username: user.username.clone(),
            role: user.role,
            is_active: user.is_active,
            must_change_password: user.must_change_password,
            created_at: user.created_at,
        };
        self.users.push(user);
        Ok(item)
    }

    fn update_user(&mut self, id: &str, input: &UpdateUserInput) -> Result<UserItem, AppError> {
        self.ensure_owner()?;
        let username_clean = input.username.trim().to_lowercase();
        if username_clean.is_empty() {
            return Err(AppError("Username tidak boleh kosong.".into()));
        }

        if self
            .users
            .iter()
            .any(|u| u.id != id && u.username.eq_ignore_ascii_case(&username_clean))
        {
            return Err(AppError(
                "Username sudah dipakai oleh akun pengguna lain.".into(),
            ));
        }

        let is_self = self.current_user_id.as_deref() == Some(id);
        let active_owners = self
            .users
            .iter()
            .filter(|user| user.is_active && user.role == UserRole::Owner)
            .count();

        let u = self
            .users
            .iter_mut()
            .find(|u| u.id == id)
            .ok_or_else(|| AppError("Pengguna tidak ditemukan.".into()))?;

        if is_self && (!input.is_active || input.role != u.role) {
            return Err(AppError(
                "Tidak dapat menonaktifkan atau mengubah peran akun Anda sendiri (API-007).".into(),
            ));
        }

        if u.role == UserRole::Owner
            && u.is_active
            && (!input.is_active || input.role != UserRole::Owner)
            && active_owners <= 1
        {
            return Err(AppError(
                "Sistem wajib mempertahankan minimal satu akun Pemilik (Owner) aktif (AC-018)."
                    .into(),
            ));
        }

        u.username = username_clean;
        u.role = input.role;
        u.is_active = input.is_active;

        Ok(UserItem {
            id: u.id.clone(),
            username: u.username.clone(),
            role: u.role,
            is_active: u.is_active,
            must_change_password: u.must_change_password,
            created_at: u.created_at,
        })
    }

    fn reset_password(&mut self, id: &str, temporary_password: &str) -> Result<(), AppError> {
        self.ensure_owner()?;
        if self.current_user_id.as_deref() == Some(id) {
            return Err(AppError(
                "Gunakan menu Ganti Kata Sandi untuk mengubah kata sandi akun Anda sendiri.".into(),
            ));
        }
        if temporary_password.len() < 15 {
            return Err(AppError(
                "Kata sandi sementara wajib minimal 15 karakter sesuai kebijakan keamanan (API-008)."
                    .into(),
            ));
        }

        let u = self
            .users
            .iter_mut()
            .find(|u| u.id == id)
            .ok_or_else(|| AppError("Pengguna tidak ditemukan.".into()))?;

        u.password = temporary_password.to_string();
        u.must_change_password = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reversal_of_unpaid_sale_does_not_create_customer_credit() {
        let mut store = MockDataAdapter::new();
        let customer = store
            .list_customers(&CustomerQuery {
                search: Some("DEMO-01".into()),
                active_only: true,
            })
            .pop()
            .unwrap();
        let query = LedgerQuery {
            from: None,
            to: None,
            order_desc: false,
        };
        let entry = store.get_ledger(&customer.id, &query).unwrap().entries[0]
            .id
            .clone();
        store
            .reverse_transaction(&customer.id, &entry, "Uji koreksi")
            .unwrap();
        let ledger = store.get_ledger(&customer.id, &query).unwrap();
        assert_eq!(ledger.summary.outstanding_idr, 0);
        assert_eq!(ledger.summary.customer_credit_idr, 0);
        assert_eq!(ledger.summary.ledger_balance_idr, 0);
        assert_eq!(ledger.entries.len(), 2);
        assert!(ledger.entries.iter().any(|e| e.is_reversed));
        assert!(ledger.entries.iter().any(|e| e.entry_type == "reversal"));
    }

    #[test]
    fn reversal_restores_only_real_unallocated_payment_or_receivable() {
        for reverse_payment in [false, true] {
            let mut store = MockDataAdapter::new();
            let customer = store
                .list_customers(&CustomerQuery {
                    search: Some("DEMO-03".into()),
                    active_only: true,
                })
                .pop()
                .unwrap();
            let payment = store
                .record_payment(
                    &customer.id,
                    &RecordPaymentInput {
                        received_date: Local::now().date_naive(),
                        amount_idr: 10_000,
                        method: "cash".into(),
                        reference: None,
                        note: None,
                    },
                )
                .unwrap();
            let query = LedgerQuery {
                from: None,
                to: None,
                order_desc: false,
            };
            let ledger = store.get_ledger(&customer.id, &query).unwrap();
            let target = ledger
                .entries
                .iter()
                .find(|entry| {
                    if reverse_payment {
                        entry.source_id == payment.id
                    } else {
                        entry.debit_idr > 0
                    }
                })
                .unwrap();
            store
                .reverse_transaction(&customer.id, &target.id, "Uji koreksi")
                .unwrap();
            let result = store.get_ledger(&customer.id, &query).unwrap().summary;
            if reverse_payment {
                assert_eq!(
                    (
                        result.outstanding_idr,
                        result.customer_credit_idr,
                        result.ledger_balance_idr
                    ),
                    (18_000, 0, 18_000)
                );
            } else {
                assert_eq!(
                    (
                        result.outstanding_idr,
                        result.customer_credit_idr,
                        result.ledger_balance_idr
                    ),
                    (0, 10_000, -10_000)
                );
            }
        }
    }

    #[test]
    fn test_mock_store_initial_customers_and_example_reconciliation() {
        let store = MockDataAdapter::new();
        let query = CustomerQuery {
            search: None,
            active_only: false,
        };
        let customers = store.list_customers(&query);
        assert_eq!(customers.len(), 5);

        let example = customers
            .iter()
            .find(|c| c.name == "Pelanggan Contoh A")
            .unwrap();
        assert_eq!(example.code.as_deref(), Some("DEMO-01"));
        assert_eq!(example.outstanding_idr, 100_000);
        assert_eq!(example.customer_credit_idr, 0);

        let ledger = store
            .get_ledger(
                &example.id,
                &LedgerQuery {
                    from: None,
                    to: None,
                    order_desc: true,
                },
            )
            .unwrap();
        assert_eq!(ledger.summary.outstanding_idr, 100_000);
        assert_eq!(ledger.summary.ledger_balance_idr, 100_000);
        assert_eq!(ledger.entries.first().unwrap().running_balance_idr, 100_000);
    }

    #[test]
    fn test_new_sale_auto_applies_deposit_fifo() {
        let mut store = MockDataAdapter::new();
        // Pelanggan contoh memiliki saldo kredit Rp 500.000
        let berkah = store
            .list_customers(&CustomerQuery {
                search: Some("Contoh B".into()),
                active_only: true,
            })
            .into_iter()
            .next()
            .unwrap();
        assert_eq!(berkah.customer_credit_idr, 500_000);
        assert_eq!(berkah.outstanding_idr, 0);

        // Belanja instan senilai Rp 150.000 pada hari ini
        let today = Local::now().date_naive();
        let sale_input = InstantSaleInput {
            effective_date: today,
            reference_no: Some("NOTA-001".into()),
            backdate_reason: None,
            items: vec![SaleItemInput {
                description: "KERTAS F4 1 DUS".into(),
                quantity: "1.5".into(),
                unit_price_idr: 100_000,
            }],
        };
        let total = store.create_instant_sale(&berkah.id, &sale_input).unwrap();
        assert_eq!(total, 150_000);

        // Auto-apply deposit pelanggan FIFO (Opsi A): Piutang tetap 0, deposit berkurang menjadi 350.000
        let after_sale = store.get_customer(&berkah.id).unwrap();
        assert_eq!(after_sale.outstanding_idr, 0);
        assert_eq!(after_sale.customer_credit_idr, 350_000);

        // Transaksi belanja baru langsung berstatus lunas
        let ledger = store
            .get_ledger(
                &berkah.id,
                &LedgerQuery {
                    from: None,
                    to: None,
                    order_desc: true,
                },
            )
            .unwrap();
        let new_entry = ledger
            .entries
            .iter()
            .find(|e| e.description == "KERTAS F4 1 DUS")
            .unwrap();
        assert_eq!(new_entry.remaining_idr, 0);
        assert_eq!(new_entry.allocated_idr, 150_000);
    }

    #[test]
    fn test_record_payment_fifo_allocation_and_aging_reconciliation() {
        let mut store = MockDataAdapter::new();
        let kec = store
            .list_customers(&CustomerQuery {
                search: Some("Contoh C".into()),
                active_only: true,
            })
            .into_iter()
            .next()
            .unwrap();
        assert_eq!(kec.outstanding_idr, 18_000);

        let today = Local::now().date_naive();
        let pay_input = RecordPaymentInput {
            received_date: today,
            amount_idr: 25_000,
            method: "cash".into(),
            reference: None,
            note: None,
        };
        let pay_row = store.record_payment(&kec.id, &pay_input).unwrap();
        assert_eq!(pay_row.allocated_idr, 18_000);
        assert_eq!(pay_row.unallocated_credit_idr, 7_000);

        let updated = store.get_customer(&kec.id).unwrap();
        assert_eq!(updated.outstanding_idr, 0);
        assert_eq!(updated.customer_credit_idr, 7_000);

        // AC-010: Rekonsiliasi 5 bucket + unknown_date + future_date == total_open_idr
        let aging = store.get_aging_report(today).unwrap();
        let sum_buckets = aging.days_0_15
            + aging.days_16_30
            + aging.days_31_45
            + aging.days_46_60
            + aging.days_over_60
            + aging.unknown_date
            + aging.future_date;
        assert_eq!(sum_buckets, aging.total_open_idr);
        assert!(aging.unknown_date >= 45_000);
    }
}
