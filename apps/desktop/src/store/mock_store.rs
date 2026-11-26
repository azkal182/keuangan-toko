//! Implementasi MockDataAdapter in-memory yang mengimplementasikan TokoStoreAdapter.
//! Menggunakan data sintetis toko lokal Indonesia yang realistis dengan logika FIFO dan saldo akurat.
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
    effective_date: NaiveDate,
    description: String,
    quantity: Option<String>,
    unit_price_idr: Option<i64>,
    debit_idr: i64,
    credit_idr: i64,
    is_reversed: bool,
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
}

#[derive(Debug, Clone)]
struct InternalCustomer {
    id: String,
    code: Option<String>,
    name: String,
    address: Option<String>,
    phone: Option<String>,
    active: bool,
    entries: Vec<InternalLedgerEntry>,
    payments: Vec<InternalPayment>,
    allocations: Vec<InternalAllocation>,
}

#[derive(Debug, Clone)]
struct InternalUser {
    id: String,
    username: String,
    full_name: String,
    role: UserRole,
    password: String,
    active: bool,
    must_change_password: bool,
}

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
            next_source_order: 100,
        };
        store.populate_initial_data();
        store
    }

    fn populate_initial_data(&mut self) {
        // Pengguna awal
        self.users = vec![
            InternalUser {
                id: Uuid::new_v4().to_string(),
                username: "owner".into(),
                full_name: "Budi Santoso".into(),
                role: UserRole::Owner,
                password: "password123".into(),
                active: true,
                must_change_password: false,
            },
            InternalUser {
                id: Uuid::new_v4().to_string(),
                username: "kasir".into(),
                full_name: "Siti Aminah".into(),
                role: UserRole::Operator,
                password: "password123".into(),
                active: true,
                must_change_password: false,
            },
            InternalUser {
                id: Uuid::new_v4().to_string(),
                username: "pemeriksa".into(),
                full_name: "Ahmad Fauzi".into(),
                role: UserRole::Viewer,
                password: "password123".into(),
                active: true,
                must_change_password: false,
            },
        ];
        self.current_user_id = Some(self.users[0].id.clone());

        // 1. DESA SUMBARANG (P001) - Pelanggan dengan anomali yang sudah dibenahi (Sep & Des 2025)
        let cust1_id = "c1111111-1111-1111-1111-111111111111".to_string();
        let mut cust1 = InternalCustomer {
            id: cust1_id.clone(),
            code: Some("P001".into()),
            name: "DESA SUMBARANG".into(),
            address: Some("Kantor Kepala Desa Sumbarang, RT 02/01".into()),
            phone: Some("0812-3456-7890".into()),
            active: true,
            entries: Vec::new(),
            payments: Vec::new(),
            allocations: Vec::new(),
        };

        // Entri historis Desa Sumbarang
        let d_sep = NaiveDate::from_ymd_opt(2025, 9, 23).unwrap();
        let d_des = NaiveDate::from_ymd_opt(2025, 12, 8).unwrap();

        // Nota-nota sebelum pembayaran 23 Sep (total 7.051.600)
        let mut order = 1;
        let e_amplop_id = Uuid::new_v4().to_string();
        cust1.entries.push(InternalLedgerEntry {
            id: e_amplop_id.clone(),
            effective_date: d_sep,
            description: "AMPLOP COKLAT".into(),
            quantity: Some("1".into()),
            unit_price_idr: Some(2_000),
            debit_idr: 2_000,
            credit_idr: 0,
            is_reversed: false,
            source_order: order,
        });
        order += 1;

        let e_kater_id = Uuid::new_v4().to_string();
        cust1.entries.push(InternalLedgerEntry {
            id: e_kater_id.clone(),
            effective_date: d_sep,
            description: "KATER".into(),
            quantity: Some("1".into()),
            unit_price_idr: Some(10_000),
            debit_idr: 10_000,
            credit_idr: 0,
            is_reversed: false,
            source_order: order,
        });
        order += 1;

        let e_saldo_awal_id = Uuid::new_v4().to_string();
        cust1.entries.push(InternalLedgerEntry {
            id: e_saldo_awal_id.clone(),
            effective_date: d_sep,
            description: "SALDO NOTA SEBELUMNYA".into(),
            quantity: None,
            unit_price_idr: None,
            debit_idr: 7_039_600,
            credit_idr: 0,
            is_reversed: false,
            source_order: order,
        });
        order += 1;

        // Pembayaran 23 Sep 2025 sebesar 7.070.500 (surplus/deposit 18.900)
        let pay1_id = Uuid::new_v4().to_string();
        let e_pay1_id = Uuid::new_v4().to_string();
        cust1.payments.push(InternalPayment {
            id: pay1_id.clone(),
            customer_id: cust1_id.clone(),
            amount_idr: 7_070_500,
            received_date: d_sep,
            method: "Transfer Bank".into(),
            reference: Some("TRF-20250923-01".into()),
            note: Some("Pelunasan nota toko".into()),
        });
        cust1.entries.push(InternalLedgerEntry {
            id: e_pay1_id.clone(),
            effective_date: d_sep,
            description: "PEMBAYARAN LUNAS".into(),
            quantity: None,
            unit_price_idr: None,
            debit_idr: 0,
            credit_idr: 7_070_500,
            is_reversed: false,
            source_order: order,
        });
        order += 1;

        // Alokasi pelunasan penuh nota sebelum bayar
        cust1.allocations.push(InternalAllocation {
            payment_id: pay1_id.clone(),
            debit_id: e_saldo_awal_id,
            amount_idr: 7_039_600,
            reason: "FIFO Auto".into(),
        });
        cust1.allocations.push(InternalAllocation {
            payment_id: pay1_id.clone(),
            debit_id: e_amplop_id,
            amount_idr: 2_000,
            reason: "FIFO Auto".into(),
        });
        cust1.allocations.push(InternalAllocation {
            payment_id: pay1_id.clone(),
            debit_id: e_kater_id,
            amount_idr: 10_000,
            reason: "FIFO Auto".into(),
        });

        // Belanja berikutnya di 23 Sep yang langsung menyerap deposit 18.900
        let e_fc_id = Uuid::new_v4().to_string();
        cust1.entries.push(InternalLedgerEntry {
            id: e_fc_id.clone(),
            effective_date: d_sep,
            description: "FC (FOTOCOPY)".into(),
            quantity: Some("51".into()),
            unit_price_idr: Some(300),
            debit_idr: 15_300,
            credit_idr: 0,
            is_reversed: false,
            source_order: order,
        });
        order += 1;
        // Alokasikan 15.300 dari sisa 18.900 (sisa deposit 3.600)
        cust1.allocations.push(InternalAllocation {
            payment_id: pay1_id.clone(),
            debit_id: e_fc_id,
            amount_idr: 15_300,
            reason: "FIFO Kronologis".into(),
        });

        let e_klip_id = Uuid::new_v4().to_string();
        cust1.entries.push(InternalLedgerEntry {
            id: e_klip_id.clone(),
            effective_date: d_sep,
            description: "PAPER KLIP".into(),
            quantity: Some("1".into()),
            unit_price_idr: Some(3_000),
            debit_idr: 3_000,
            credit_idr: 0,
            is_reversed: false,
            source_order: order,
        });
        order += 1;
        // Alokasikan 3.000 dari sisa 3.600 (sisa deposit 600)
        cust1.allocations.push(InternalAllocation {
            payment_id: pay1_id.clone(),
            debit_id: e_klip_id,
            amount_idr: 3_000,
            reason: "FIFO Kronologis".into(),
        });

        let e_kertas_id = Uuid::new_v4().to_string();
        cust1.entries.push(InternalLedgerEntry {
            id: e_kertas_id.clone(),
            effective_date: d_sep,
            description: "KERTAS F4 (2 DUS)".into(),
            quantity: Some("2".into()),
            unit_price_idr: Some(55_000),
            debit_idr: 110_000,
            credit_idr: 0,
            is_reversed: false,
            source_order: order,
        });
        order += 1;
        // Alokasikan sisa 600 ke kertas F4 (sebagian lunas, sisa 109.400)
        cust1.allocations.push(InternalAllocation {
            payment_id: pay1_id,
            debit_id: e_kertas_id,
            amount_idr: 600,
            reason: "FIFO Kronologis".into(),
        });

        // Belanja 8 Des 2025 (piutang terbuka/belum lunas)
        cust1.entries.push(InternalLedgerEntry {
            id: Uuid::new_v4().to_string(),
            effective_date: d_des,
            description: "BATU BATERAI".into(),
            quantity: Some("2".into()),
            unit_price_idr: Some(9_000),
            debit_idr: 18_000,
            credit_idr: 0,
            is_reversed: false,
            source_order: order,
        });
        order += 1;

        cust1.entries.push(InternalLedgerEntry {
            id: Uuid::new_v4().to_string(),
            effective_date: d_des,
            description: "FC JILID LAPORAN".into(),
            quantity: Some("3".into()),
            unit_price_idr: Some(300),
            debit_idr: 900,
            credit_idr: 0,
            is_reversed: false,
            source_order: order,
        });
        self.customers.insert(cust1_id, cust1);

        // 2. KECAMATAN (P002) - Pelanggan Aktif
        let cust2_id = "c2222222-2222-2222-2222-222222222222".to_string();
        let mut cust2 = InternalCustomer {
            id: cust2_id.clone(),
            code: Some("P002".into()),
            name: "KECAMATAN".into(),
            address: Some("Gedung Kantor Kecamatan, Jl. Sudirman No. 1".into()),
            phone: Some("0812-9876-5432".into()),
            active: true,
            entries: Vec::new(),
            payments: Vec::new(),
            allocations: Vec::new(),
        };
        let d_jan = NaiveDate::from_ymd_opt(2026, 1, 15).unwrap();
        cust2.entries.push(InternalLedgerEntry {
            id: Uuid::new_v4().to_string(),
            effective_date: d_jan,
            description: "BAK STEMPEL JOYKO".into(),
            quantity: Some("1".into()),
            unit_price_idr: Some(12_000),
            debit_idr: 12_000,
            credit_idr: 0,
            is_reversed: false,
            source_order: 1,
        });
        cust2.entries.push(InternalLedgerEntry {
            id: Uuid::new_v4().to_string(),
            effective_date: d_jan,
            description: "BINDER KLIP NO. 200".into(),
            quantity: Some("1".into()),
            unit_price_idr: Some(6_000),
            debit_idr: 6_000,
            credit_idr: 0,
            is_reversed: false,
            source_order: 2,
        });
        self.customers.insert(cust2_id, cust2);

        // 3. TOKO BERKAH ABADI (P003) - Pelanggan dengan Deposit / Saldo Kredit
        let cust3_id = "c3333333-3333-3333-3333-333333333333".to_string();
        let mut cust3 = InternalCustomer {
            id: cust3_id.clone(),
            code: Some("P003".into()),
            name: "TOKO BERKAH ABADI".into(),
            address: Some("Pasar Baru Blok A No. 3".into()),
            phone: Some("0856-1234-5678".into()),
            active: true,
            entries: Vec::new(),
            payments: Vec::new(),
            allocations: Vec::new(),
        };
        let d_nov = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
        let pay3_id = Uuid::new_v4().to_string();
        cust3.payments.push(InternalPayment {
            id: pay3_id,
            customer_id: cust3_id.clone(),
            amount_idr: 500_000,
            received_date: d_nov,
            method: "Tunai".into(),
            reference: Some("DP-001".into()),
            note: Some("Uang Muka Belanja Kertas".into()),
        });
        cust3.entries.push(InternalLedgerEntry {
            id: Uuid::new_v4().to_string(),
            effective_date: d_nov,
            description: "PEMBAYARAN DEPOSIT UANG MUKA".into(),
            quantity: None,
            unit_price_idr: None,
            debit_idr: 0,
            credit_idr: 500_000,
            is_reversed: false,
            source_order: 1,
        });
        self.customers.insert(cust3_id, cust3);

        // 4. CV MANDIRI JAYA (P004)
        let cust4_id = "c4444444-4444-4444-4444-444444444444".to_string();
        let mut cust4 = InternalCustomer {
            id: cust4_id.clone(),
            code: Some("P004".into()),
            name: "CV MANDIRI JAYA".into(),
            address: Some("Kawasan Industri Cilegon Blok C".into()),
            phone: Some("0877-9988-1122".into()),
            active: true,
            entries: Vec::new(),
            payments: Vec::new(),
            allocations: Vec::new(),
        };
        cust4.entries.push(InternalLedgerEntry {
            id: Uuid::new_v4().to_string(),
            effective_date: NaiveDate::from_ymd_opt(2026, 10, 10).unwrap(),
            description: "BANNER OUTDOOR 3X1 METER".into(),
            quantity: Some("2".into()),
            unit_price_idr: Some(85_000),
            debit_idr: 170_000,
            credit_idr: 0,
            is_reversed: false,
            source_order: 1,
        });
        self.customers.insert(cust4_id, cust4);

        // 5. WARUNG BU ENI (P005)
        let cust5_id = "c5555555-5555-5555-5555-555555555555".to_string();
        let mut cust5 = InternalCustomer {
            id: cust5_id.clone(),
            code: Some("P005".into()),
            name: "WARUNG BU ENI".into(),
            address: Some("Jl. Melati No. 8".into()),
            phone: Some("0813-2233-4455".into()),
            active: true,
            entries: Vec::new(),
            payments: Vec::new(),
            allocations: Vec::new(),
        };
        cust5.entries.push(InternalLedgerEntry {
            id: Uuid::new_v4().to_string(),
            effective_date: NaiveDate::from_ymd_opt(2026, 11, 20).unwrap(),
            description: "BUKU NOTA KONTAN 2 PLY".into(),
            quantity: Some("5".into()),
            unit_price_idr: Some(4_500),
            debit_idr: 22_500,
            credit_idr: 0,
            is_reversed: false,
            source_order: 1,
        });
        self.customers.insert(cust5_id, cust5);

        // 6. SDN 01 MERDEKA (P006)
        let cust6_id = "c6666666-6666-6666-6666-666666666666".to_string();
        let mut cust6 = InternalCustomer {
            id: cust6_id.clone(),
            code: Some("P006".into()),
            name: "SDN 01 MERDEKA".into(),
            address: Some("Jl. Pendidikan No. 45".into()),
            phone: Some("0821-4455-6677".into()),
            active: true,
            entries: Vec::new(),
            payments: Vec::new(),
            allocations: Vec::new(),
        };
        cust6.entries.push(InternalLedgerEntry {
            id: Uuid::new_v4().to_string(),
            effective_date: NaiveDate::from_ymd_opt(2026, 11, 18).unwrap(),
            description: "KERTAS F4 75GR (5 DUS)".into(),
            quantity: Some("5".into()),
            unit_price_idr: Some(240_000),
            debit_idr: 1_200_000,
            credit_idr: 0,
            is_reversed: false,
            source_order: 1,
        });
        self.customers.insert(cust6_id, cust6);

        // 7. KLINIK SEHAT FARMA (P007)
        let cust7_id = "c7777777-7777-7777-7777-777777777777".to_string();
        let mut cust7 = InternalCustomer {
            id: cust7_id.clone(),
            code: Some("P007".into()),
            name: "KLINIK SEHAT FARMA".into(),
            address: Some("Jl. Kesehatan Raya No. 12".into()),
            phone: Some("0818-3344-5566".into()),
            active: true,
            entries: Vec::new(),
            payments: Vec::new(),
            allocations: Vec::new(),
        };
        cust7.entries.push(InternalLedgerEntry {
            id: Uuid::new_v4().to_string(),
            effective_date: NaiveDate::from_ymd_opt(2026, 11, 24).unwrap(),
            description: "STEMPEL FLESH DOKTER".into(),
            quantity: Some("2".into()),
            unit_price_idr: Some(75_000),
            debit_idr: 150_000,
            credit_idr: 0,
            is_reversed: false,
            source_order: 1,
        });
        self.customers.insert(cust7_id, cust7);

        // 8. KOPERASI WARGA MAKMUR (P008) - Nonaktif
        let cust8_id = "c8888888-8888-8888-8888-888888888888".to_string();
        let cust8 = InternalCustomer {
            id: cust8_id.clone(),
            code: Some("P008".into()),
            name: "KOPERASI WARGA MAKMUR".into(),
            address: Some("Komplek Ruko Permata No. 5".into()),
            phone: Some("0819-7766-5544".into()),
            active: false,
            entries: Vec::new(),
            payments: Vec::new(),
            allocations: Vec::new(),
        };
        self.customers.insert(cust8_id, cust8);
    }

    fn calculate_customer_balances(&self, customer: &InternalCustomer) -> (i64, i64, AgingBuckets) {
        let today = Local::now().date_naive();
        let mut total_debit = 0_i64;
        let mut total_credit = 0_i64;
        let mut allocated_per_debit: BTreeMap<String, i64> = BTreeMap::new();

        for alloc in &customer.allocations {
            *allocated_per_debit
                .entry(alloc.debit_id.clone())
                .or_insert(0) += alloc.amount_idr;
        }

        let mut aging = AgingBuckets::default();

        for entry in &customer.entries {
            if entry.is_reversed {
                continue;
            }
            total_debit += entry.debit_idr;
            total_credit += entry.credit_idr;

            if entry.debit_idr > 0 {
                let allocated = *allocated_per_debit.get(&entry.id).unwrap_or(&0);
                let remaining = entry.debit_idr - allocated;
                if remaining > 0 {
                    let days = (today - entry.effective_date).num_days();
                    if days < 0 {
                        aging.exception += remaining;
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

        let total_allocated: i64 = customer.allocations.iter().map(|a| a.amount_idr).sum();
        let outstanding = total_debit.saturating_sub(total_allocated);
        let customer_credit = total_credit.saturating_sub(total_allocated);

        (outstanding, customer_credit, aging)
    }
}

impl TokoStoreAdapter for MockDataAdapter {
    fn current_session(&self) -> Option<SessionUser> {
        let user_id = self.current_user_id.as_ref()?;
        let u = self.users.iter().find(|user| &user.id == user_id)?;
        Some(SessionUser {
            id: u.id.clone(),
            username: u.username.clone(),
            full_name: u.full_name.clone(),
            role: u.role,
            must_change_password: u.must_change_password,
        })
    }

    fn login(&mut self, username: &str, password: &str) -> Result<SessionUser, AppError> {
        let user = self
            .users
            .iter()
            .find(|u| u.username.eq_ignore_ascii_case(username) && u.password == password)
            .ok_or_else(|| AppError("Username atau kata sandi tidak valid.".into()))?;

        if !user.active {
            return Err(AppError("Akun pengguna sedang dinonaktifkan.".into()));
        }

        self.current_user_id = Some(user.id.clone());
        Ok(self.current_session().unwrap())
    }

    fn logout(&mut self) {
        self.current_user_id = None;
    }

    fn switch_session_role(&mut self, role: UserRole) {
        if let Some(user_id) = &self.current_user_id {
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
            return Err(AppError("Kata sandi lama salah.".into()));
        }
        if new_pw.len() < 8 {
            return Err(AppError("Kata sandi baru minimal 8 karakter.".into()));
        }

        u.password = new_pw.to_string();
        u.must_change_password = false;
        Ok(())
    }

    fn list_customers(&self, query: &CustomerQuery) -> Vec<CustomerSummary> {
        let mut list = Vec::new();
        let search_term = query.search.as_deref().unwrap_or("").trim().to_lowercase();

        for cust in self.customers.values() {
            if query.active_only && !cust.active {
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

            let (outstanding, credit, aging) = self.calculate_customer_balances(cust);
            list.push(CustomerSummary {
                id: cust.id.clone(),
                code: cust.code.clone(),
                name: cust.name.clone(),
                address: cust.address.clone(),
                phone: cust.phone.clone(),
                active: cust.active,
                total_outstanding_idr: outstanding,
                customer_credit_idr: credit,
                aging,
            });
        }

        // Urutkan berdasarkan nama
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    fn get_customer(&self, id: &str) -> Result<CustomerDetail, AppError> {
        let cust = self
            .customers
            .get(id)
            .ok_or_else(|| AppError("Pelanggan tidak ditemukan.".into()))?;
        let (outstanding, credit, _) = self.calculate_customer_balances(cust);
        Ok(CustomerDetail {
            id: cust.id.clone(),
            code: cust.code.clone(),
            name: cust.name.clone(),
            address: cust.address.clone(),
            phone: cust.phone.clone(),
            active: cust.active,
            total_outstanding_idr: outstanding,
            customer_credit_idr: credit,
        })
    }

    fn create_customer(&mut self, input: &CreateCustomerInput) -> Result<CustomerDetail, AppError> {
        let name_clean = input.name.trim();
        if name_clean.is_empty() {
            return Err(AppError("Nama pelanggan tidak boleh kosong.".into()));
        }

        // Cek duplikasi kode
        if let Some(code) = &input.code {
            let code_clean = code.trim().to_uppercase();
            if !code_clean.is_empty()
                && self.customers.values().any(|c| {
                    c.code
                        .as_deref()
                        .unwrap_or("")
                        .eq_ignore_ascii_case(&code_clean)
                })
            {
                return Err(AppError(format!(
                    "Kode pelanggan '{}' sudah dipakai.",
                    code_clean
                )));
            }
        }

        let id = Uuid::new_v4().to_string();
        let cust = InternalCustomer {
            id: id.clone(),
            code: input.code.as_ref().map(|c| c.trim().to_uppercase()),
            name: name_clean.to_string(),
            address: input.address.as_ref().map(|a| a.trim().to_string()),
            phone: input.phone.as_ref().map(|p| p.trim().to_string()),
            active: true,
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
        let cust = self
            .customers
            .get_mut(id)
            .ok_or_else(|| AppError("Pelanggan tidak ditemukan.".into()))?;

        let name_clean = input.name.trim();
        if name_clean.is_empty() {
            return Err(AppError("Nama pelanggan tidak boleh kosong.".into()));
        }

        cust.name = name_clean.to_string();
        cust.code = input.code.as_ref().map(|c| c.trim().to_uppercase());
        cust.address = input.address.as_ref().map(|a| a.trim().to_string());
        cust.phone = input.phone.as_ref().map(|p| p.trim().to_string());
        cust.active = input.active;

        self.get_customer(id)
    }

    fn toggle_customer_active(&mut self, id: &str) -> Result<bool, AppError> {
        let cust = self
            .customers
            .get_mut(id)
            .ok_or_else(|| AppError("Pelanggan tidak ditemukan.".into()))?;
        cust.active = !cust.active;
        Ok(cust.active)
    }

    fn merge_customers(
        &mut self,
        source_id: &str,
        target_id: &str,
        _reason: &str,
    ) -> Result<(), AppError> {
        if source_id == target_id {
            return Err(AppError(
                "Pelanggan sumber dan tujuan tidak boleh sama.".into(),
            ));
        }

        let mut source = self
            .customers
            .remove(source_id)
            .ok_or_else(|| AppError("Pelanggan sumber tidak ditemukan.".into()))?;
        let target = self
            .customers
            .get_mut(target_id)
            .ok_or_else(|| AppError("Pelanggan tujuan tidak ditemukan.".into()))?;

        // Pindahkan entri buku piutang dan pembayaran ke target
        target.entries.append(&mut source.entries);
        target.payments.append(&mut source.payments);
        target.allocations.append(&mut source.allocations);

        // Sumber dinonaktifkan dan disimpan kembali
        source.active = false;
        self.customers.insert(source_id.to_string(), source);

        Ok(())
    }

    fn get_ledger(
        &self,
        customer_id: &str,
        query: &LedgerQuery,
    ) -> Result<Vec<LedgerEntry>, AppError> {
        let cust = self
            .customers
            .get(customer_id)
            .ok_or_else(|| AppError("Pelanggan tidak ditemukan.".into()))?;

        let today = Local::now().date_naive();
        let mut allocated_per_debit: BTreeMap<String, i64> = BTreeMap::new();
        for alloc in &cust.allocations {
            *allocated_per_debit
                .entry(alloc.debit_id.clone())
                .or_insert(0) += alloc.amount_idr;
        }

        // Urutkan entri secara kronologis dahulu untuk menghitung saldo berjalan akurat
        let mut chronological_entries = cust.entries.clone();
        chronological_entries.sort_by_key(|e| (e.effective_date, e.source_order));

        let mut running_balance = 0_i64;
        let mut calculated_list = Vec::new();

        for e in chronological_entries {
            let debit = if e.is_reversed { 0 } else { e.debit_idr };
            let credit = if e.is_reversed { 0 } else { e.credit_idr };
            running_balance += debit - credit;

            // Filter tanggal
            if let Some(from) = query.from {
                if e.effective_date < from {
                    continue;
                }
            }
            if let Some(to) = query.to {
                if e.effective_date > to {
                    continue;
                }
            }

            let (allocated, remaining, status, age) = if e.debit_idr > 0 && !e.is_reversed {
                let alloc = *allocated_per_debit.get(&e.id).unwrap_or(&0);
                let rem = e.debit_idr.saturating_sub(alloc);
                let st = if alloc == 0 {
                    AllocationStatus::Unallocated
                } else if alloc >= e.debit_idr {
                    AllocationStatus::Allocated
                } else {
                    AllocationStatus::PartiallyAllocated
                };
                let age_days = if rem > 0 {
                    Some((today - e.effective_date).num_days().max(0) as i32)
                } else {
                    None
                };
                (alloc, rem, Some(st), age_days)
            } else {
                (0, 0, None, None)
            };

            calculated_list.push(LedgerEntry {
                id: e.id,
                effective_date: e.effective_date,
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
                source_order: e.source_order,
            });
        }

        // Terapkan urutan akhir (Terbaru di Atas / Terlama di Atas)
        if query.order_desc {
            calculated_list.sort_by(|a, b| {
                b.effective_date
                    .cmp(&a.effective_date)
                    .then_with(|| b.source_order.cmp(&a.source_order))
            });
        } else {
            calculated_list.sort_by(|a, b| {
                a.effective_date
                    .cmp(&b.effective_date)
                    .then_with(|| a.source_order.cmp(&b.source_order))
            });
        }

        Ok(calculated_list)
    }

    fn create_instant_sale(
        &mut self,
        customer_id: &str,
        input: &InstantSaleInput,
    ) -> Result<i64, AppError> {
        let next_order = self.next_source_order;
        let cust = self
            .customers
            .get_mut(customer_id)
            .ok_or_else(|| AppError("Pelanggan tidak ditemukan.".into()))?;

        if !cust.active {
            return Err(AppError(
                "Pelanggan sedang nonaktif. Aktifkan terlebih dahulu.".into(),
            ));
        }
        if input.items.is_empty() {
            return Err(AppError("Minimal harus ada 1 barang belanja.".into()));
        }

        let mut total_sale = 0_i64;
        let mut created_debits = Vec::new();

        // 1. Masukkan setiap baris item belanja ke buku piutang
        let mut current_order = next_order;
        for item in &input.items {
            let desc_clean = item.description.trim();
            if desc_clean.is_empty() || item.unit_price_idr <= 0 {
                continue;
            }
            let qty_num: f64 = item.quantity.trim().parse().unwrap_or(1.0);
            let line_total = ((qty_num * (item.unit_price_idr as f64)).round()) as i64;
            if line_total <= 0 {
                continue;
            }

            let entry_id = Uuid::new_v4().to_string();
            cust.entries.push(InternalLedgerEntry {
                id: entry_id.clone(),
                effective_date: input.effective_date,
                description: desc_clean.to_string(),
                quantity: Some(item.quantity.clone()),
                unit_price_idr: Some(item.unit_price_idr),
                debit_idr: line_total,
                credit_idr: 0,
                is_reversed: false,
                source_order: current_order,
            });
            current_order += 1;
            total_sale += line_total;
            created_debits.push((entry_id, line_total));
        }
        self.next_source_order = current_order;

        if total_sale <= 0 {
            return Err(AppError("Total belanja harus lebih dari nol.".into()));
        }

        // 2. Auto-apply deposit (jika pelanggan memiliki kelebihan saldo kredit sebelumnya)
        let total_credit_posted: i64 = cust
            .entries
            .iter()
            .filter(|e| !e.is_reversed)
            .map(|e| e.credit_idr)
            .sum();
        let total_allocated: i64 = cust.allocations.iter().map(|a| a.amount_idr).sum();
        let mut available_credit = total_credit_posted.saturating_sub(total_allocated);

        if available_credit > 0 {
            // Cari pembayaran yang memiliki sisa unallocated credit
            for pay in &cust.payments {
                let pay_allocated: i64 = cust
                    .allocations
                    .iter()
                    .filter(|a| a.payment_id == pay.id)
                    .map(|a| a.amount_idr)
                    .sum();
                let mut pay_unallocated = pay.amount_idr.saturating_sub(pay_allocated);

                for (debit_id, debit_amount) in &mut created_debits {
                    if *debit_amount <= 0 || pay_unallocated <= 0 || available_credit <= 0 {
                        continue;
                    }
                    let allocate_amount =
                        (*debit_amount).min(pay_unallocated).min(available_credit);
                    cust.allocations.push(InternalAllocation {
                        payment_id: pay.id.clone(),
                        debit_id: debit_id.clone(),
                        amount_idr: allocate_amount,
                        reason: "Auto-apply Saldo Deposit Pelanggan".into(),
                    });
                    *debit_amount -= allocate_amount;
                    pay_unallocated -= allocate_amount;
                    available_credit -= allocate_amount;
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
        let reason_clean = reason.trim();
        if reason_clean.is_empty() {
            return Err(AppError("Alasan pembatalan/koreksi harus diisi.".into()));
        }

        let next_order = self.next_source_order;
        let cust = self
            .customers
            .get_mut(customer_id)
            .ok_or_else(|| AppError("Pelanggan tidak ditemukan.".into()))?;

        let entry = cust
            .entries
            .iter_mut()
            .find(|e| e.id == entry_id)
            .ok_or_else(|| AppError("Transaksi tidak ditemukan.".into()))?;

        if entry.is_reversed {
            return Err(AppError(
                "Transaksi ini sudah dibatalkan sebelumnya.".into(),
            ));
        }

        entry.is_reversed = true;
        let contra_desc = format!("PEMBATALAN: {} ({})", entry.description, reason_clean);
        let contra_debit = entry.credit_idr;
        let contra_credit = entry.debit_idr;

        // Buat kontra-entry
        cust.entries.push(InternalLedgerEntry {
            id: Uuid::new_v4().to_string(),
            effective_date: Local::now().date_naive(),
            description: contra_desc,
            quantity: None,
            unit_price_idr: None,
            debit_idr: contra_debit,
            credit_idr: contra_credit,
            is_reversed: false,
            source_order: next_order,
        });
        self.next_source_order += 1;

        // Pulihkan alokasi terkait
        cust.allocations
            .retain(|a| a.debit_id != entry_id && a.payment_id != entry_id);

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
            let unallocated = p.amount_idr.saturating_sub(allocated);

            list.push(PaymentRow {
                id: p.id.clone(),
                customer_id: p.customer_id.clone(),
                amount_idr: p.amount_idr,
                received_date: p.received_date,
                method: p.method.clone(),
                reference: p.reference.clone(),
                note: p.note.clone(),
                allocated_idr: allocated,
                unallocated_credit_idr: unallocated,
                allocations,
            });
        }

        // Urutkan pembayaran terbaru di atas
        list.sort_by(|a, b| b.received_date.cmp(&a.received_date));
        Ok(list)
    }

    fn record_payment(
        &mut self,
        customer_id: &str,
        input: &RecordPaymentInput,
    ) -> Result<PaymentRow, AppError> {
        if input.amount_idr <= 0 {
            return Err(AppError("Jumlah pembayaran harus lebih dari nol.".into()));
        }

        let next_order = self.next_source_order;
        let cust = self
            .customers
            .get_mut(customer_id)
            .ok_or_else(|| AppError("Pelanggan tidak ditemukan.".into()))?;

        let payment_id = Uuid::new_v4().to_string();
        let payment = InternalPayment {
            id: payment_id.clone(),
            customer_id: customer_id.to_string(),
            amount_idr: input.amount_idr,
            received_date: input.received_date,
            method: input.method.clone(),
            reference: input.reference.clone(),
            note: input.note.clone(),
        };

        // Catat entri kredit ke buku piutang
        let desc = format!("PEMBAYARAN ({})", input.method);
        cust.entries.push(InternalLedgerEntry {
            id: payment_id.clone(),
            effective_date: input.received_date,
            description: desc,
            quantity: None,
            unit_price_idr: None,
            debit_idr: 0,
            credit_idr: input.amount_idr,
            is_reversed: false,
            source_order: next_order,
        });
        self.next_source_order += 1;
        cust.payments.push(payment);

        // Alokasi FIFO terhadap nota-nota debet terbuka terlama
        let mut allocated_per_debit: BTreeMap<String, i64> = BTreeMap::new();
        for alloc in &cust.allocations {
            *allocated_per_debit
                .entry(alloc.debit_id.clone())
                .or_insert(0) += alloc.amount_idr;
        }

        let mut remaining_payment = input.amount_idr;
        let mut open_debits: Vec<(String, i64, NaiveDate, i64)> = cust
            .entries
            .iter()
            .filter(|e| e.debit_idr > 0 && !e.is_reversed)
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

        // Urutkan debet berdasarkan tanggal terlama dahulu (FIFO)
        open_debits.sort_by_key(|(_, _, d, o)| (*d, *o));

        for (debit_id, open_amount, _, _) in open_debits {
            if remaining_payment <= 0 {
                break;
            }
            let alloc = open_amount.min(remaining_payment);
            cust.allocations.push(InternalAllocation {
                payment_id: payment_id.clone(),
                debit_id,
                amount_idr: alloc,
                reason: "Pelunasan Otomatis FIFO".into(),
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
        if input.amount_idr <= 0 {
            return Err(AppError(
                "Jumlah kredit yang diterapkan harus lebih dari nol.".into(),
            ));
        }
        let reason_clean = input.reason.trim();
        if reason_clean.is_empty() {
            return Err(AppError(
                "Alasan penerapan saldo kredit harus diisi.".into(),
            ));
        }

        let cust = self
            .customers
            .get_mut(customer_id)
            .ok_or_else(|| AppError("Pelanggan tidak ditemukan.".into()))?;

        // Validasi ketersediaan saldo kredit pada payment bersangkutan
        let payment = cust
            .payments
            .iter()
            .find(|p| p.id == input.payment_id)
            .ok_or_else(|| AppError("Pembayaran sumber tidak ditemukan.".into()))?;

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

        // Validasi nota debet tujuan
        let debit_entry = cust
            .entries
            .iter()
            .find(|e| e.id == input.debit_entry_id && e.debit_idr > 0 && !e.is_reversed)
            .ok_or_else(|| AppError("Nota debet tujuan tidak ditemukan.".into()))?;

        let debit_allocated: i64 = cust
            .allocations
            .iter()
            .filter(|a| a.debit_id == input.debit_entry_id)
            .map(|a| a.amount_idr)
            .sum();
        let debit_remaining = debit_entry.debit_idr.saturating_sub(debit_allocated);

        if input.amount_idr > debit_remaining {
            return Err(AppError(format!(
                "Jumlah penerapan melebihi sisa tagihan nota (Sisa: Rp {}, Diminta: Rp {}).",
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

    fn get_aging_report(&self, as_of: NaiveDate) -> Result<AgingReport, AppError> {
        let mut report = AgingReport {
            as_of,
            total_open_idr: 0,
            days_0_15: 0,
            days_16_30: 0,
            days_31_45: 0,
            days_46_60: 0,
            days_over_60: 0,
            exception: 0,
            customers: Vec::new(),
        };

        for cust in self.customers.values() {
            let mut allocated_per_debit: BTreeMap<String, i64> = BTreeMap::new();
            for alloc in &cust.allocations {
                *allocated_per_debit
                    .entry(alloc.debit_id.clone())
                    .or_insert(0) += alloc.amount_idr;
            }

            let mut c_aging = AgingBuckets::default();
            let mut c_total = 0_i64;

            for e in &cust.entries {
                if e.debit_idr > 0 && !e.is_reversed && e.effective_date <= as_of {
                    let alloc = *allocated_per_debit.get(&e.id).unwrap_or(&0);
                    let rem = e.debit_idr.saturating_sub(alloc);
                    if rem > 0 {
                        c_total += rem;
                        let days = (as_of - e.effective_date).num_days();
                        if days < 0 {
                            c_aging.exception += rem;
                        } else if days <= 15 {
                            c_aging.days_0_15 += rem;
                        } else if days <= 30 {
                            c_aging.days_16_30 += rem;
                        } else if days <= 45 {
                            c_aging.days_31_45 += rem;
                        } else if days <= 60 {
                            c_aging.days_46_60 += rem;
                        } else {
                            c_aging.days_over_60 += rem;
                        }
                    }
                }
            }

            if c_total > 0 {
                report.total_open_idr += c_total;
                report.days_0_15 += c_aging.days_0_15;
                report.days_16_30 += c_aging.days_16_30;
                report.days_31_45 += c_aging.days_31_45;
                report.days_46_60 += c_aging.days_46_60;
                report.days_over_60 += c_aging.days_over_60;
                report.exception += c_aging.exception;

                report.customers.push(AgingReportCustomerItem {
                    customer_id: cust.id.clone(),
                    customer_code: cust.code.clone(),
                    customer_name: cust.name.clone(),
                    total_outstanding_idr: c_total,
                    days_0_15: c_aging.days_0_15,
                    days_16_30: c_aging.days_16_30,
                    days_31_45: c_aging.days_31_45,
                    days_46_60: c_aging.days_46_60,
                    days_over_60: c_aging.days_over_60,
                    exception: c_aging.exception,
                });
            }
        }

        report
            .customers
            .sort_by(|a, b| b.total_outstanding_idr.cmp(&a.total_outstanding_idr));
        Ok(report)
    }

    fn list_users(&self) -> Result<Vec<UserItem>, AppError> {
        let list = self
            .users
            .iter()
            .map(|u| UserItem {
                id: u.id.clone(),
                username: u.username.clone(),
                full_name: u.full_name.clone(),
                role: u.role,
                active: u.active,
                must_change_password: u.must_change_password,
            })
            .collect();
        Ok(list)
    }

    fn create_user(&mut self, input: &CreateUserInput) -> Result<UserItem, AppError> {
        let username_clean = input.username.trim().to_lowercase();
        if username_clean.is_empty() {
            return Err(AppError("Username tidak boleh kosong.".into()));
        }
        if self
            .users
            .iter()
            .any(|u| u.username.eq_ignore_ascii_case(&username_clean))
        {
            return Err(AppError("Username sudah dipakai.".into()));
        }

        let user = InternalUser {
            id: Uuid::new_v4().to_string(),
            username: username_clean,
            full_name: input.full_name.trim().to_string(),
            role: input.role,
            password: input.temporary_password.clone(),
            active: true,
            must_change_password: true,
        };

        let item = UserItem {
            id: user.id.clone(),
            username: user.username.clone(),
            full_name: user.full_name.clone(),
            role: user.role,
            active: user.active,
            must_change_password: user.must_change_password,
        };
        self.users.push(user);
        Ok(item)
    }

    fn update_user(&mut self, id: &str, input: &UpdateUserInput) -> Result<UserItem, AppError> {
        let active_owners = self
            .users
            .iter()
            .filter(|user| user.active && user.role == UserRole::Owner)
            .count();

        let u = self
            .users
            .iter_mut()
            .find(|u| u.id == id)
            .ok_or_else(|| AppError("Pengguna tidak ditemukan.".into()))?;

        // Jangan izinkan menonaktifkan akun owner jika tinggal satu
        if !input.active && u.role == UserRole::Owner && active_owners <= 1 {
            return Err(AppError("Minimal harus ada satu Owner aktif.".into()));
        }

        u.full_name = input.full_name.trim().to_string();
        u.role = input.role;
        u.active = input.active;

        Ok(UserItem {
            id: u.id.clone(),
            username: u.username.clone(),
            full_name: u.full_name.clone(),
            role: u.role,
            active: u.active,
            must_change_password: u.must_change_password,
        })
    }

    fn reset_password(&mut self, id: &str) -> Result<String, AppError> {
        let u = self
            .users
            .iter_mut()
            .find(|u| u.id == id)
            .ok_or_else(|| AppError("Pengguna tidak ditemukan.".into()))?;

        let temp_pw = "reset123".to_string();
        u.password = temp_pw.clone();
        u.must_change_password = true;
        Ok(temp_pw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_store_initial_customers_and_balances() {
        let store = MockDataAdapter::new();
        let query = CustomerQuery {
            search: None,
            active_only: false,
        };
        let customers = store.list_customers(&query);
        assert!(customers.len() >= 8);

        // Cari Desa Sumbarang
        let sumbarang = customers
            .iter()
            .find(|c| c.name == "DESA SUMBARANG")
            .unwrap();
        assert_eq!(sumbarang.code.as_deref(), Some("P001"));
        // Transaksi 8 Des 2025: BATU (18.000) + FC (900) + KERTAS F4 sisa (109.400) = 128.300
        assert_eq!(sumbarang.total_outstanding_idr, 128_300);
        assert_eq!(sumbarang.customer_credit_idr, 0);
    }

    #[test]
    fn test_instant_sale_and_deposit_auto_application() {
        let mut store = MockDataAdapter::new();
        // Toko Berkah Abadi punya deposit 500.000
        let berkah = store
            .list_customers(&CustomerQuery {
                search: Some("Berkah".into()),
                active_only: true,
            })
            .into_iter()
            .next()
            .unwrap();
        assert_eq!(berkah.customer_credit_idr, 500_000);
        assert_eq!(berkah.total_outstanding_idr, 0);

        // Belanja instan senilai 150.000
        let sale_input = InstantSaleInput {
            effective_date: NaiveDate::from_ymd_opt(2026, 11, 26).unwrap(),
            reference_no: Some("NOTA-001".into()),
            items: vec![SaleItemInput {
                description: "KERTAS F4 1 DUS".into(),
                quantity: "1".into(),
                unit_price_idr: 150_000,
            }],
        };
        let total = store.create_instant_sale(&berkah.id, &sale_input).unwrap();
        assert_eq!(total, 150_000);

        // Karena deposit 500.000 > 150.000, belanja otomatis lunas dipotong deposit!
        let updated = store.get_customer(&berkah.id).unwrap();
        assert_eq!(updated.total_outstanding_idr, 0);
        assert_eq!(updated.customer_credit_idr, 350_000);
    }

    #[test]
    fn test_record_payment_fifo_allocation() {
        let mut store = MockDataAdapter::new();
        // Kecamatan (P002) punya tagihan 12.000 + 6.000 = 18.000
        let kec = store
            .list_customers(&CustomerQuery {
                search: Some("Kecamatan".into()),
                active_only: true,
            })
            .into_iter()
            .next()
            .unwrap();
        assert_eq!(kec.total_outstanding_idr, 18_000);

        // Bayar 25.000 (melunasi 18.000 dan sisa 7.000 jadi saldo kredit deposit)
        let pay_input = RecordPaymentInput {
            received_date: NaiveDate::from_ymd_opt(2026, 11, 26).unwrap(),
            amount_idr: 25_000,
            method: "Tunai".into(),
            reference: None,
            note: None,
        };
        let pay_row = store.record_payment(&kec.id, &pay_input).unwrap();
        assert_eq!(pay_row.allocated_idr, 18_000);
        assert_eq!(pay_row.unallocated_credit_idr, 7_000);

        let updated = store.get_customer(&kec.id).unwrap();
        assert_eq!(updated.total_outstanding_idr, 0);
        assert_eq!(updated.customer_credit_idr, 7_000);
    }
}
