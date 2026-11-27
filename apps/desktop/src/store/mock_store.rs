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
    let milli_qty = whole * 1000 + frac;
    if milli_qty <= 0 {
        return Err(AppError("Kuantitas harus lebih besar dari nol.".into()));
    }
    if unit_price_idr <= 0 {
        return Err(AppError(
            "Harga satuan harus berupa bilangan rupiah bulat lebih dari nol.".into(),
        ));
    }
    // Half-up rounding: (milli_qty * unit_price + 500) / 1000
    let product = milli_qty * (unit_price_idr as i128);
    let rounded = (product + 500) / 1000;
    Ok(rounded as i64)
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

        // =========================================================================
        // 1. DESA SUMBARANG (P017) - Dataset lengkap 48 transaksi sesuai buku riil
        //    Total Piutang Terutang = Rp 1.938.200 | Saldo Kredit = Rp 0
        // =========================================================================
        let cust1_id = "c1111111-1111-1111-1111-111111111111".to_string();
        let mut cust1 = InternalCustomer {
            id: cust1_id.clone(),
            code: Some("P017".into()),
            name: "DESA SUMBARANG".into(),
            address: None, // Sesuai tampilan web: "Alamat belum tercatat"
            is_active: true,
            version: 1,
            entries: Vec::new(),
            payments: Vec::new(),
            allocations: Vec::new(),
        };

        let d_sep_date = NaiveDate::from_ymd_opt(2025, 9, 23).unwrap();
        let d_sep = Some(d_sep_date);
        let d_des = Some(NaiveDate::from_ymd_opt(2025, 12, 8).unwrap());
        let mut order = 1_i64;

        // Nota sebelum pembayaran 23 Sep 2025: total 7.051.600
        // (1) Saldo awal sebelum 23 Sep: 7.039.600, (2) AMPLOP COKLAT 2.000 -> 7.041.600, (3) KATER 10.000 -> 7.051.600
        let pre_payment_items: Vec<(&str, Option<&str>, Option<i64>, i64)> = vec![
            ("SALDO AWAL PERIode SEBELUMNYA", None, None, 7_039_600),
            ("AMPLOP COKLAT", Some("1"), Some(2_000), 2_000),
            ("KATER", Some("1"), Some(10_000), 10_000),
        ];
        let mut pre_debit_ids = Vec::new();
        for (desc, qty, price, debit) in pre_payment_items {
            let eid = Uuid::new_v4().to_string();
            pre_debit_ids.push((eid.clone(), debit));
            cust1.entries.push(InternalLedgerEntry {
                id: eid.clone(),
                entry_type: "legacy_debit".into(),
                source_id: eid,
                effective_date: d_sep,
                reference: None,
                description: desc.into(),
                quantity: qty.map(String::from),
                unit_price_idr: price,
                debit_idr: debit,
                credit_idr: 0,
                is_reversed: false,
                reversal_reason: None,
                replacement_entry_id: None,
                source_order: order,
            });
            order += 1;
        }

        // Pembayaran Lunas 23 Sep 2025: Rp 7.070.500 -> menjadikan saldo berjalan (Rp 18.900)
        let pay1_id = Uuid::new_v4().to_string();
        cust1.payments.push(InternalPayment {
            id: pay1_id.clone(),
            customer_id: cust1_id.clone(),
            amount_idr: 7_070_500,
            received_date: d_sep_date,
            method: "legacy".into(),
            reference: Some("HIST-P017-01".into()),
            note: Some("PEMBAYARAN LUNAS (Workbook Historis)".into()),
            status: "posted".into(),
            version: 1,
        });
        cust1.entries.push(InternalLedgerEntry {
            id: pay1_id.clone(),
            entry_type: "legacy_payment".into(),
            source_id: pay1_id.clone(),
            effective_date: d_sep,
            reference: None,
            description: "PEMBAYARAN LUNAS".into(),
            quantity: None,
            unit_price_idr: None,
            debit_idr: 0,
            credit_idr: 7_070_500,
            is_reversed: false,
            reversal_reason: None,
            replacement_entry_id: None,
            source_order: order,
        });
        order += 1;

        for (did, amt) in pre_debit_ids {
            cust1.allocations.push(InternalAllocation {
                payment_id: pay1_id.clone(),
                debit_id: did,
                amount_idr: amt,
                reason: "Alokasi FIFO Historis".into(),
            });
        }

        // Daftar 45 item belanja setelah pembayaran 23 Sep 2025 + 2 item 8 Des 2025
        // Surplus 18.900 dari PEMBAYARAN LUNAS menyerap:
        // - FC (15.300) -> Lunas, sisa surplus 3.600, running balance (Rp 3.600)
        // - PAPER KLIP (3.000) -> Lunas, sisa surplus 600, running balance (Rp 600)
        // - KERTAS F4 (110.000) -> Terserap 600, Belum Lunas (Sisa Rp 109.400), running balance Rp 109.400
        let post_sep_items: Vec<(&str, &str, i64, i64, i64)> = vec![
            ("FC", "51", 300, 15_300, 15_300),
            ("PAPER KLIP", "1", 3_000, 3_000, 3_000),
            ("KERTAS F4", "2", 55_000, 110_000, 600),
            ("TINTA PSEN", "1", 110_000, 110_000, 0),
            ("PEN", "1", 5_000, 5_000, 0),
            ("BANER 1X75", "4", 18_750, 75_000, 0),
            ("DESAIN", "1", 10_000, 10_000, 0),
            ("BANER PPAN KARYA 1X75", "4", 18_750, 75_000, 0),
            ("JILID", "5", 2_000, 10_000, 0),
            ("ABNER 3X1", "1", 85_000, 85_000, 0),
            ("BUKU BIGBOS 4 PAJK", "4", 32_000, 128_000, 0),
            ("KERTAS F4", "1", 55_000, 55_000, 0),
            ("MAP", "1", 35_000, 35_000, 0),
            ("KUTANSI", "2", 4_000, 8_000, 0),
            ("PEN WARNA", "1", 7_000, 7_000, 0),
            ("PRINT", "4", 2_000, 8_000, 0),
            ("FC", "13", 300, 3_900, 0),
            ("PRINT", "1", 2_000, 2_000, 0),
            ("MATRAI", "4", 12_000, 48_000, 0),
            ("STEMPEL", "1", 70_000, 70_000, 0),
            ("BENER PKK 120X90", "1", 37_000, 37_000, 0),
            ("STEMPEL PKK", "1", 70_000, 70_000, 0),
            ("ASTRO", "4", 4_000, 16_000, 0),
            ("MANILA", "1", 4_000, 4_000, 0),
            ("STEMPEL", "1", 70_000, 70_000, 0),
            ("KERTAS THE", "10", 4_000, 40_000, 0),
            ("BENER BPD 2,5X1", "1", 72_500, 72_500, 0),
            ("KERTAS MANILA", "5", 4_000, 20_000, 0),
            ("BOLPOIN STANDAR", "1", 23_000, 23_000, 0),
            ("SPIDOL BESAR", "1", 10_000, 10_000, 0),
            ("SPIDOL KECIL", "2", 2_000, 4_000, 0),
            ("LEM BESAR", "1", 5_000, 5_000, 0),
            ("BENER RKP DES 3X1", "1", 85_000, 85_000, 0),
            ("BENER BPD PEMILIHAN 3X1", "1", 85_000, 85_000, 0),
            ("BENER BPD 1,5X1,2", "4", 45_000, 180_000, 0),
            ("SNAL KETER 2 BKS", "2", 35_000, 70_000, 0),
            ("PRINT", "33", 1_000, 33_000, 0),
            ("MIKA", "18", 750, 13_500, 0),
            ("JLD", "9", 5_000, 45_000, 0),
            ("PULPEN JOYKO", "1", 5_000, 5_000, 0),
            ("JLD", "9", 5_000, 45_000, 0),
            ("KERTAS F4", "1", 55_000, 55_000, 0),
            ("JLD", "6", 5_000, 30_000, 0),
            ("FC", "90", 300, 27_000, 0),
            ("JLD", "6", 5_000, 30_000, 0),
        ];

        for (desc, qty, price, debit, allocated) in post_sep_items {
            let eid = Uuid::new_v4().to_string();
            cust1.entries.push(InternalLedgerEntry {
                id: eid.clone(),
                entry_type: "legacy_debit".into(),
                source_id: eid.clone(),
                effective_date: d_sep,
                reference: None,
                description: desc.into(),
                quantity: Some(qty.into()),
                unit_price_idr: Some(price),
                debit_idr: debit,
                credit_idr: 0,
                is_reversed: false,
                reversal_reason: None,
                replacement_entry_id: None,
                source_order: order,
            });
            order += 1;
            if allocated > 0 {
                cust1.allocations.push(InternalAllocation {
                    payment_id: pay1_id.clone(),
                    debit_id: eid,
                    amount_idr: allocated,
                    reason: "Alokasi FIFO Historis".into(),
                });
            }
        }

        // Transaksi 8 Des 2025: BATU (18.000) dan FC (900) -> Total Outstanding = 1.938.200
        let des_items: Vec<(&str, &str, i64, i64)> =
            vec![("BATU", "2", 9_000, 18_000), ("FC", "3", 300, 900)];
        for (desc, qty, price, debit) in des_items {
            let eid = Uuid::new_v4().to_string();
            cust1.entries.push(InternalLedgerEntry {
                id: eid.clone(),
                entry_type: "legacy_debit".into(),
                source_id: eid,
                effective_date: d_des,
                reference: None,
                description: desc.into(),
                quantity: Some(qty.into()),
                unit_price_idr: Some(price),
                debit_idr: debit,
                credit_idr: 0,
                is_reversed: false,
                reversal_reason: None,
                replacement_entry_id: None,
                source_order: order,
            });
            order += 1;
        }
        self.customers.insert(cust1_id, cust1);

        // =========================================================================
        // 2. KECAMATAN (P002) - Piutang 0-15 Hari & 16-30 Hari
        // =========================================================================
        let cust2_id = "c2222222-2222-2222-2222-222222222222".to_string();
        let mut cust2 = InternalCustomer {
            id: cust2_id.clone(),
            code: Some("P002".into()),
            name: "KECAMATAN".into(),
            address: Some("Gedung Kantor Kecamatan, Jl. Sudirman No. 1".into()),
            is_active: true,
            version: 1,
            entries: Vec::new(),
            payments: Vec::new(),
            allocations: Vec::new(),
        };
        let d_kec = Some(NaiveDate::from_ymd_opt(2026, 11, 5).unwrap());
        let e_k1 = Uuid::new_v4().to_string();
        cust2.entries.push(InternalLedgerEntry {
            id: e_k1.clone(),
            entry_type: "credit_sale".into(),
            source_id: e_k1,
            effective_date: d_kec,
            reference: Some("INV-2026-011".into()),
            description: "BAK STEMPEL JOYKO".into(),
            quantity: Some("1".into()),
            unit_price_idr: Some(12_000),
            debit_idr: 12_000,
            credit_idr: 0,
            is_reversed: false,
            reversal_reason: None,
            replacement_entry_id: None,
            source_order: 1,
        });
        let e_k2 = Uuid::new_v4().to_string();
        cust2.entries.push(InternalLedgerEntry {
            id: e_k2.clone(),
            entry_type: "credit_sale".into(),
            source_id: e_k2,
            effective_date: d_kec,
            reference: Some("INV-2026-011".into()),
            description: "BINDER KLIP NO. 200".into(),
            quantity: Some("1".into()),
            unit_price_idr: Some(6_000),
            debit_idr: 6_000,
            credit_idr: 0,
            is_reversed: false,
            reversal_reason: None,
            replacement_entry_id: None,
            source_order: 2,
        });
        self.customers.insert(cust2_id, cust2);

        // =========================================================================
        // 3. TOKO BERKAH ABADI (P003) - Memiliki Saldo Kredit (Deposit) Rp 500.000
        //    serta tagihan baru yang dapat diuji dengan fitur "Terapkan Saldo Kredit"
        // =========================================================================
        let cust3_id = "c3333333-3333-3333-3333-333333333333".to_string();
        let mut cust3 = InternalCustomer {
            id: cust3_id.clone(),
            code: Some("P003".into()),
            name: "TOKO BERKAH ABADI".into(),
            address: Some("Pasar Baru Blok A No. 3".into()),
            is_active: true,
            version: 1,
            entries: Vec::new(),
            payments: Vec::new(),
            allocations: Vec::new(),
        };
        let d_nov = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
        let pay3_id = Uuid::new_v4().to_string();
        cust3.payments.push(InternalPayment {
            id: pay3_id.clone(),
            customer_id: cust3_id.clone(),
            amount_idr: 500_000,
            received_date: d_nov,
            method: "cash".into(),
            reference: Some("DP-001".into()),
            note: Some("Setoran deposit belanja bulanan".into()),
            status: "posted".into(),
            version: 1,
        });
        cust3.entries.push(InternalLedgerEntry {
            id: pay3_id.clone(),
            entry_type: "payment".into(),
            source_id: pay3_id,
            effective_date: Some(d_nov),
            reference: Some("DP-001".into()),
            description: "PEMBAYARAN (Tunai)".into(),
            quantity: None,
            unit_price_idr: None,
            debit_idr: 0,
            credit_idr: 500_000,
            is_reversed: false,
            reversal_reason: None,
            replacement_entry_id: None,
            source_order: 1,
        });
        self.customers.insert(cust3_id, cust3);

        // =========================================================================
        // 4. CV MANDIRI JAYA (P004) - Bucket 31-45 Hari
        // =========================================================================
        let cust4_id = "c4444444-4444-4444-4444-444444444444".to_string();
        let mut cust4 = InternalCustomer {
            id: cust4_id.clone(),
            code: Some("P004".into()),
            name: "CV MANDIRI JAYA".into(),
            address: Some("Kawasan Industri Cilegon Blok C".into()),
            is_active: true,
            version: 1,
            entries: Vec::new(),
            payments: Vec::new(),
            allocations: Vec::new(),
        };
        let e_c4 = Uuid::new_v4().to_string();
        cust4.entries.push(InternalLedgerEntry {
            id: e_c4.clone(),
            entry_type: "credit_sale".into(),
            source_id: e_c4,
            effective_date: Some(NaiveDate::from_ymd_opt(2026, 10, 15).unwrap()),
            reference: Some("INV-2026-004".into()),
            description: "BANNER OUTDOOR 3X1 METER".into(),
            quantity: Some("2".into()),
            unit_price_idr: Some(85_000),
            debit_idr: 170_000,
            credit_idr: 0,
            is_reversed: false,
            reversal_reason: None,
            replacement_entry_id: None,
            source_order: 1,
        });
        self.customers.insert(cust4_id, cust4);

        // =========================================================================
        // 5. WARUNG BU ENI (P005) - Bucket 0-15 Hari
        // =========================================================================
        let cust5_id = "c5555555-5555-5555-5555-555555555555".to_string();
        let mut cust5 = InternalCustomer {
            id: cust5_id.clone(),
            code: Some("P005".into()),
            name: "WARUNG BU ENI".into(),
            address: Some("Jl. Melati No. 8".into()),
            is_active: true,
            version: 1,
            entries: Vec::new(),
            payments: Vec::new(),
            allocations: Vec::new(),
        };
        let e_c5 = Uuid::new_v4().to_string();
        cust5.entries.push(InternalLedgerEntry {
            id: e_c5.clone(),
            entry_type: "credit_sale".into(),
            source_id: e_c5,
            effective_date: Some(NaiveDate::from_ymd_opt(2026, 11, 20).unwrap()),
            reference: None,
            description: "BUKU NOTA KONTAN 2 PLY".into(),
            quantity: Some("5".into()),
            unit_price_idr: Some(4_500),
            debit_idr: 22_500,
            credit_idr: 0,
            is_reversed: false,
            reversal_reason: None,
            replacement_entry_id: None,
            source_order: 1,
        });
        self.customers.insert(cust5_id, cust5);

        // =========================================================================
        // 6. SDN 01 MERDEKA (P006) - Bucket 46-60 Hari
        // =========================================================================
        let cust6_id = "c6666666-6666-6666-6666-666666666666".to_string();
        let mut cust6 = InternalCustomer {
            id: cust6_id.clone(),
            code: Some("P006".into()),
            name: "SDN 01 MERDEKA".into(),
            address: Some("Jl. Pendidikan No. 45".into()),
            is_active: true,
            version: 1,
            entries: Vec::new(),
            payments: Vec::new(),
            allocations: Vec::new(),
        };
        let e_c6 = Uuid::new_v4().to_string();
        cust6.entries.push(InternalLedgerEntry {
            id: e_c6.clone(),
            entry_type: "credit_sale".into(),
            source_id: e_c6,
            effective_date: Some(NaiveDate::from_ymd_opt(2026, 10, 1).unwrap()),
            reference: Some("INV-SDN-09".into()),
            description: "KERTAS F4 75GR (5 DUS)".into(),
            quantity: Some("5".into()),
            unit_price_idr: Some(240_000),
            debit_idr: 1_200_000,
            credit_idr: 0,
            is_reversed: false,
            reversal_reason: None,
            replacement_entry_id: None,
            source_order: 1,
        });
        self.customers.insert(cust6_id, cust6);

        // =========================================================================
        // 7. KLINIK SEHAT FARMA (P007) - Bucket 0-15 Hari + 1 Unknown Date Exception
        // =========================================================================
        let cust7_id = "c7777777-7777-7777-7777-777777777777".to_string();
        let mut cust7 = InternalCustomer {
            id: cust7_id.clone(),
            code: Some("P007".into()),
            name: "KLINIK SEHAT FARMA".into(),
            address: Some("Jl. Kesehatan Raya No. 12".into()),
            is_active: true,
            version: 1,
            entries: Vec::new(),
            payments: Vec::new(),
            allocations: Vec::new(),
        };
        let e_c7a = Uuid::new_v4().to_string();
        cust7.entries.push(InternalLedgerEntry {
            id: e_c7a.clone(),
            entry_type: "credit_sale".into(),
            source_id: e_c7a,
            effective_date: Some(NaiveDate::from_ymd_opt(2026, 11, 24).unwrap()),
            reference: None,
            description: "STEMPEL FLASH DOKTER".into(),
            quantity: Some("2".into()),
            unit_price_idr: Some(75_000),
            debit_idr: 150_000,
            credit_idr: 0,
            is_reversed: false,
            reversal_reason: None,
            replacement_entry_id: None,
            source_order: 1,
        });
        let e_c7b = Uuid::new_v4().to_string();
        cust7.entries.push(InternalLedgerEntry {
            id: e_c7b.clone(),
            entry_type: "legacy_debit".into(),
            source_id: e_c7b,
            effective_date: None, // Tanggal tidak diketahui (exception bucket sesuai FR-005)
            reference: None,
            description: "NOTA LAMA TANPA TANGGAL (ARSIP)".into(),
            quantity: Some("1".into()),
            unit_price_idr: Some(45_000),
            debit_idr: 45_000,
            credit_idr: 0,
            is_reversed: false,
            reversal_reason: None,
            replacement_entry_id: None,
            source_order: 2,
        });
        self.customers.insert(cust7_id, cust7);

        // =========================================================================
        // 8. KOPERASI WARGA MAKMUR (P008) - Nonaktif
        // =========================================================================
        let cust8_id = "c8888888-8888-8888-8888-888888888888".to_string();
        let cust8 = InternalCustomer {
            id: cust8_id.clone(),
            code: Some("P008".into()),
            name: "KOPERASI WARGA MAKMUR".into(),
            address: Some("Komplek Ruko Permata No. 5".into()),
            is_active: false,
            version: 1,
            entries: Vec::new(),
            payments: Vec::new(),
            allocations: Vec::new(),
        };
        self.customers.insert(cust8_id, cust8);
    }

    fn calculate_customer_balances(
        &self,
        customer: &InternalCustomer,
        as_of: NaiveDate,
    ) -> (i64, i64, i64, AgingBuckets) {
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
        let ledger_balance = total_debit - total_credit;

        (outstanding, customer_credit, ledger_balance, aging)
    }
}

impl TokoStoreAdapter for MockDataAdapter {
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
        let role = self.ensure_can_write()?;
        let today = Local::now().date_naive();

        // BR-012 / FR-002: Tanggal masa depan ditolak; backdate hanya Owner dengan alasan
        if input.effective_date > today {
            return Err(AppError(
                "Tanggal transaksi penjualan tidak boleh berada di masa depan (FR-002).".into(),
            ));
        }
        if input.effective_date < today {
            if role != UserRole::Owner {
                return Err(AppError(
                    "Pencatatan tanggal lampau (backdate) hanya diizinkan untuk Pemilik (Owner)."
                        .into(),
                ));
            }
            let has_reason = input
                .backdate_reason
                .as_ref()
                .map(|r| !r.trim().is_empty())
                .unwrap_or(false);
            if !has_reason {
                return Err(AppError(
                    "Alasan tanggal lampau (backdate reason) wajib diisi untuk transaksi masa lalu."
                        .into(),
                ));
            }
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

        // Sesuai BR-006 & FR-003: Surplus / Saldo Kredit Pelanggan TIDAK auto-apply pada penjualan baru.
        // Operator/Owner menerapkannya secara eksplisit melalui menu "Terapkan Saldo Kredit".
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
    fn test_mock_store_initial_customers_and_desa_sumbarang_reconciliation() {
        let store = MockDataAdapter::new();
        let query = CustomerQuery {
            search: None,
            active_only: false,
        };
        let customers = store.list_customers(&query);
        assert_eq!(customers.len(), 8);

        // Pastikan DESA SUMBARANG (P017) cocok 100% dengan rekonsiliasi buku riil (Rp 1.938.200)
        let sumbarang = customers
            .iter()
            .find(|c| c.name == "DESA SUMBARANG")
            .unwrap();
        assert_eq!(sumbarang.code.as_deref(), Some("P017"));
        assert_eq!(sumbarang.outstanding_idr, 1_938_200);
        assert_eq!(sumbarang.customer_credit_idr, 0);

        let ledger = store
            .get_ledger(
                &sumbarang.id,
                &LedgerQuery {
                    from: None,
                    to: None,
                    order_desc: true,
                },
            )
            .unwrap();
        assert_eq!(ledger.summary.outstanding_idr, 1_938_200);
        assert_eq!(ledger.summary.ledger_balance_idr, 1_938_200);
        assert_eq!(
            ledger.entries.first().unwrap().running_balance_idr,
            1_938_200
        );
    }

    #[test]
    fn test_new_sale_does_not_auto_apply_deposit_and_explicit_credit_application_works() {
        let mut store = MockDataAdapter::new();
        // Toko Berkah Abadi punya deposit Rp 500.000
        let berkah = store
            .list_customers(&CustomerQuery {
                search: Some("Berkah".into()),
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

        // Sesuai BR-006 / FR-003: Kredit TIDAK auto-apply! Piutang tetap 150.000 dan kredit tetap 500.000
        let after_sale = store.get_customer(&berkah.id).unwrap();
        assert_eq!(after_sale.outstanding_idr, 150_000);
        assert_eq!(after_sale.customer_credit_idr, 500_000);

        // Sekarang terapkan saldo kredit secara eksplisit dengan alasan audit (API-302)
        let payments = store.list_payments(&berkah.id).unwrap();
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
        let open_debit = ledger.entries.iter().find(|e| e.remaining_idr > 0).unwrap();

        store
            .apply_credit(
                &berkah.id,
                &ApplyCreditInput {
                    payment_id: payments[0].id.clone(),
                    debit_entry_id: open_debit.id.clone(),
                    amount_idr: 150_000,
                    reason: "Potong deposit uang muka pelanggan".into(),
                },
            )
            .unwrap();

        let after_apply = store.get_customer(&berkah.id).unwrap();
        assert_eq!(after_apply.outstanding_idr, 0);
        assert_eq!(after_apply.customer_credit_idr, 350_000);
    }

    #[test]
    fn test_record_payment_fifo_allocation_and_aging_reconciliation() {
        let mut store = MockDataAdapter::new();
        let kec = store
            .list_customers(&CustomerQuery {
                search: Some("Kecamatan".into()),
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
