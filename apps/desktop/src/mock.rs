//! Data sintetis dalam memori. Tidak ada koneksi ke server atau penyimpanan.
#![allow(dead_code)]

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ENTRY_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug)]
pub struct Allocation {
    pub debit_id: u64,
    pub amount: i64,
    pub reason: String,
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub id: u64,
    pub date: String,
    pub description: String,
    pub debit: i64,
    pub credit: i64,
    pub remaining: i64,
    pub unallocated: i64,
    pub allocations: Vec<Allocation>,
    pub reference: String,
    pub note: String,
    pub quantity: Option<String>,
    pub unit_price: Option<i64>,
    pub reversed: bool,
    pub reversal_of: Option<u64>,
    pub replacement_id: Option<u64>,
    pub audit: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct SaleItem {
    pub description: String,
    pub quantity: String,
    pub unit_price: i64,
    pub total: i64,
}

pub struct Draft {
    pub id: u64,
    pub customer_id: usize,
    pub date: String,
    pub reason: String,
    pub items: Vec<SaleItem>,
}

impl Draft {
    pub fn post(&self, customer: &mut Customer) -> Result<(), &'static str> {
        if customer.id != self.customer_id {
            return Err("Draft bukan milik pelanggan ini.");
        }
        customer.sale_items(&self.date, &self.items)
    }
}

#[derive(Clone, Debug)]
pub struct Customer {
    pub id: usize,
    pub code: String,
    pub name: String,
    pub address: String,
    pub active: bool,
    pub entries: Vec<Entry>,
    pub outstanding: i64,
    pub customer_credit: i64,
}

impl Customer {
    pub fn sale_items(&mut self, date: &str, items: &[SaleItem]) -> Result<(), &'static str> {
        if items.is_empty() {
            return Err("Minimal satu barang harus diisi.");
        }
        let mut updated = self.clone();
        for item in items {
            updated.sale(date, &item.description, item.total)?;
            let entry = updated.entries.last_mut().unwrap();
            entry.quantity = Some(item.quantity.clone());
            entry.unit_price = Some(item.unit_price);
        }
        *self = updated;
        Ok(())
    }
    pub fn sale(&mut self, date: &str, description: &str, amount: i64) -> Result<(), &'static str> {
        if !self.active
            || amount <= 0
            || description.trim().is_empty()
            || chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err()
        {
            return Err("Pelanggan harus aktif; tanggal, uraian dan nominal harus valid.");
        }
        self.outstanding = self
            .outstanding
            .checked_add(amount)
            .ok_or("Nominal terlalu besar.")?;
        self.entries.push(Entry {
            id: NEXT_ENTRY_ID.fetch_add(1, Ordering::Relaxed),
            date: date.into(),
            description: description.trim().into(),
            debit: amount,
            credit: 0,
            remaining: amount,
            unallocated: 0,
            allocations: vec![],
            reference: String::new(),
            note: String::new(),
            quantity: None,
            unit_price: None,
            reversed: false,
            reversal_of: None,
            replacement_id: None,
            audit: vec![],
        });
        Ok(())
    }

    pub fn payment(&mut self, date: &str, amount: i64, method: &str) -> Result<(), &'static str> {
        self.payment_with_details(date, amount, method, "", "")
    }

    pub fn payment_with_details(
        &mut self,
        date: &str,
        amount: i64,
        method: &str,
        reference: &str,
        note: &str,
    ) -> Result<(), &'static str> {
        if !self.active
            || chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err()
            || amount <= 0
            || !matches!(method, "Tunai" | "Transfer")
        {
            return Err("Pelanggan, tanggal, metode dan nominal pembayaran harus valid.");
        }
        let payment_date = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();
        let mut eligible: Vec<usize> = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(i, entry)| {
                let entry_date = chrono::NaiveDate::parse_from_str(&entry.date, "%Y-%m-%d").ok()?;
                (entry.remaining > 0 && entry_date <= payment_date).then_some(i)
            })
            .collect();
        eligible.sort_by_key(|&i| (self.entries[i].date.clone(), i));
        let eligible_total = eligible
            .iter()
            .try_fold(0_i64, |total, &i| {
                total.checked_add(self.entries[i].remaining)
            })
            .ok_or("Nominal terlalu besar.")?;
        let available = (amount - eligible_total).max(0);
        let new_credit = self
            .customer_credit
            .checked_add(available)
            .ok_or("Nominal terlalu besar.")?;
        let mut unallocated = amount;
        let mut allocations = Vec::new();
        for index in eligible {
            let entry = &mut self.entries[index];
            let applied = unallocated.min(entry.remaining);
            entry.remaining -= applied;
            unallocated -= applied;
            if applied > 0 {
                allocations.push(Allocation {
                    debit_id: entry.id,
                    amount: applied,
                    reason: "FIFO".into(),
                });
            }
        }
        self.outstanding -= amount - unallocated;
        self.customer_credit = new_credit;
        self.entries.push(Entry {
            id: NEXT_ENTRY_ID.fetch_add(1, Ordering::Relaxed),
            date: date.into(),
            description: format!("Pembayaran contoh · {method}"),
            debit: 0,
            credit: amount,
            remaining: 0,
            unallocated,
            allocations,
            reference: reference.trim().into(),
            note: note.trim().into(),
            quantity: None,
            unit_price: None,
            reversed: false,
            reversal_of: None,
            replacement_id: None,
            audit: vec![],
        });
        Ok(())
    }

    pub fn apply_credit(
        &mut self,
        payment_id: u64,
        debit_id: u64,
        amount: i64,
        reason: &str,
    ) -> Result<(), &'static str> {
        let payment = self
            .entries
            .iter()
            .position(|entry| {
                entry.id == payment_id
                    && entry.credit > 0
                    && !entry.reversed
                    && entry.reversal_of.is_none()
            })
            .ok_or("Pembayaran tidak ditemukan.")?;
        let debit = self
            .entries
            .iter()
            .position(|entry| {
                entry.id == debit_id
                    && entry.debit > 0
                    && !entry.reversed
                    && entry.reversal_of.is_none()
            })
            .ok_or("Debit tidak ditemukan.")?;
        if amount <= 0
            || amount > self.entries[payment].unallocated
            || amount > self.entries[debit].remaining
            || reason.trim().is_empty()
        {
            return Err(
                "Alasan wajib; nominal tidak boleh melebihi kredit pembayaran atau sisa debit.",
            );
        }
        self.entries[debit].remaining -= amount;
        self.entries[payment].unallocated -= amount;
        self.entries[payment].allocations.push(Allocation {
            debit_id,
            amount,
            reason: reason.trim().into(),
        });
        self.entries[payment].audit.push(format!(
            "Kredit {} ke debit #{} · {}",
            rupiah(amount),
            debit_id,
            reason.trim()
        ));
        self.customer_credit -= amount;
        self.outstanding -= amount;
        Ok(())
    }

    pub fn reallocate(
        &mut self,
        payment_id: u64,
        target: Option<(u64, i64)>,
        reason: &str,
    ) -> Result<(), &'static str> {
        if reason.trim().is_empty() {
            return Err("Alasan realokasi wajib diisi.");
        }
        let mut updated = self.clone();
        let index = updated
            .entries
            .iter()
            .position(|entry| {
                entry.id == payment_id
                    && entry.credit > 0
                    && !entry.reversed
                    && entry.reversal_of.is_none()
            })
            .ok_or("Pembayaran aktif tidak ditemukan.")?;
        let old_allocations = std::mem::take(&mut updated.entries[index].allocations);
        for allocation in old_allocations {
            updated.entries[index].audit.push(format!(
                "Alokasi lama ke #{} sebesar {} dibalik · {}",
                allocation.debit_id,
                rupiah(allocation.amount),
                reason.trim()
            ));
            let debit = updated
                .entries
                .iter_mut()
                .find(|entry| entry.id == allocation.debit_id && !entry.reversed)
                .ok_or("Debit alokasi tidak ditemukan.")?;
            debit.remaining = debit
                .remaining
                .checked_add(allocation.amount)
                .ok_or("Nominal terlalu besar.")?;
            updated.outstanding = updated
                .outstanding
                .checked_add(allocation.amount)
                .ok_or("Nominal terlalu besar.")?;
            updated.customer_credit = updated
                .customer_credit
                .checked_add(allocation.amount)
                .ok_or("Nominal terlalu besar.")?;
            updated.entries[index].unallocated = updated.entries[index]
                .unallocated
                .checked_add(allocation.amount)
                .ok_or("Nominal terlalu besar.")?;
        }
        if let Some((debit_id, amount)) = target {
            updated.apply_credit(payment_id, debit_id, amount, reason)?;
        }
        updated.entries[index]
            .audit
            .push(format!("Realokasi · {}", reason.trim()));
        *self = updated;
        Ok(())
    }

    pub fn link_replacement(
        &mut self,
        original_id: u64,
        replacement_id: u64,
        reason: &str,
    ) -> Result<(), &'static str> {
        if reason.trim().is_empty() || original_id == replacement_id {
            return Err("Alasan dan transaksi pengganti yang berbeda wajib diisi.");
        }
        let original = self
            .entries
            .iter()
            .position(|entry| {
                entry.id == original_id && entry.reversed && entry.reversal_of.is_none()
            })
            .ok_or("Transaksi asal yang dibalik tidak ditemukan.")?;
        let replacement = self
            .entries
            .iter()
            .position(|entry| {
                entry.id == replacement_id && !entry.reversed && entry.reversal_of.is_none()
            })
            .ok_or("Transaksi pengganti aktif tidak ditemukan.")?;
        if (self.entries[original].debit > 0) != (self.entries[replacement].debit > 0) {
            return Err("Jenis transaksi pengganti harus sama dengan transaksi asal.");
        }
        if self.entries[original].replacement_id.is_some() {
            return Err("Transaksi asal sudah memiliki pengganti.");
        }
        self.entries[original].replacement_id = Some(replacement_id);
        self.entries[original].audit.push(format!(
            "Pengganti #{} · {}",
            replacement_id,
            reason.trim()
        ));
        Ok(())
    }

    pub fn reverse(&mut self, id: u64, date: &str, reason: &str) -> Result<(), &'static str> {
        if reason.trim().is_empty() || chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err()
        {
            return Err("Tanggal valid dan alasan pembalikan wajib diisi.");
        }
        let index = self
            .entries
            .iter()
            .position(|entry| entry.id == id && !entry.reversed && entry.reversal_of.is_none())
            .ok_or("Transaksi tidak ditemukan atau sudah dibalik.")?;
        let original = self.entries[index].clone();
        if original.debit > 0 && original.remaining != original.debit {
            return Err("Debit yang sudah dialokasikan tidak dapat dibalik.");
        }
        let mut updated = self.clone();
        if original.debit > 0 {
            updated.outstanding -= original.debit;
            updated.entries[index].remaining = 0;
        } else {
            for allocation in &original.allocations {
                let debit = updated
                    .entries
                    .iter_mut()
                    .find(|entry| entry.id == allocation.debit_id)
                    .ok_or("Debit alokasi tidak ditemukan.")?;
                debit.remaining = debit
                    .remaining
                    .checked_add(allocation.amount)
                    .ok_or("Nominal terlalu besar.")?;
                updated.outstanding = updated
                    .outstanding
                    .checked_add(allocation.amount)
                    .ok_or("Nominal terlalu besar.")?;
            }
            updated.customer_credit -= original.unallocated;
            updated.entries[index].unallocated = 0;
        }
        updated.entries[index].reversed = true;
        updated.entries[index]
            .audit
            .push(format!("Dibalik · {}", reason.trim()));
        updated.entries.push(Entry {
            id: NEXT_ENTRY_ID.fetch_add(1, Ordering::Relaxed),
            date: date.into(),
            description: format!("Pembalikan #{} · {}", id, reason.trim()),
            debit: original.credit,
            credit: original.debit,
            remaining: 0,
            unallocated: 0,
            allocations: vec![],
            reference: String::new(),
            note: reason.trim().into(),
            quantity: None,
            unit_price: None,
            reversed: false,
            reversal_of: Some(id),
            replacement_id: None,
            audit: vec![format!("Pembalikan · {}", reason.trim())],
        });
        *self = updated;
        Ok(())
    }
}

pub fn aging(customers: &[Customer], as_of: chrono::NaiveDate) -> [i64; 7] {
    let mut buckets = [0; 7];
    for entry in customers
        .iter()
        .flat_map(|customer| &customer.entries)
        .filter(|entry| entry.remaining > 0)
    {
        let bucket = match chrono::NaiveDate::parse_from_str(&entry.date, "%Y-%m-%d") {
            Err(_) => 5,
            Ok(date) => match (as_of - date).num_days() {
                i64::MIN..=-1 => 6,
                0..=15 => 0,
                16..=30 => 1,
                31..=45 => 2,
                46..=60 => 3,
                _ => 4,
            },
        };
        buckets[bucket] += entry.remaining;
    }
    buckets
}

pub fn merge_customers(
    customers: &mut [Customer],
    source_id: usize,
    target_code: &str,
) -> Result<usize, &'static str> {
    let source = customers
        .iter()
        .position(|customer| customer.id == source_id)
        .ok_or("Pelanggan asal tidak ditemukan.")?;
    let target = customers
        .iter()
        .position(|customer| {
            customer.active
                && !customer.code.is_empty()
                && customer.code.eq_ignore_ascii_case(target_code.trim())
        })
        .ok_or("Pelanggan tujuan aktif tidak ditemukan.")?;
    if source == target || !customers[source].active {
        return Err("Pilih pelanggan asal dan tujuan aktif yang berbeda.");
    }
    let outstanding = customers[target]
        .outstanding
        .checked_add(customers[source].outstanding)
        .ok_or("Nominal terlalu besar.")?;
    let credit = customers[target]
        .customer_credit
        .checked_add(customers[source].customer_credit)
        .ok_or("Nominal terlalu besar.")?;
    let entries = std::mem::take(&mut customers[source].entries);
    customers[source].active = false;
    customers[source].outstanding = 0;
    customers[source].customer_credit = 0;
    customers[target].entries.extend(entries);
    customers[target]
        .entries
        .sort_by(|a, b| a.date.cmp(&b.date));
    customers[target].outstanding = outstanding;
    customers[target].customer_credit = credit;
    Ok(customers[target].id)
}

pub trait CustomerSource {
    fn customers(&self) -> Vec<Customer>;
}
pub struct MockSource;

impl CustomerSource for MockSource {
    fn customers(&self) -> Vec<Customer> {
        let mut a = Customer {
            id: 1,
            code: "CTH-001".into(),
            name: "Pelanggan Contoh A".into(),
            address: "Alamat contoh".into(),
            active: true,
            entries: vec![],
            outstanding: 0,
            customer_credit: 0,
        };
        a.sale("2026-11-01", "Penjualan contoh", 100_000).unwrap();
        a.payment("2026-11-05", 40_000, "Tunai").unwrap();
        let mut b = Customer {
            id: 2,
            code: "CTH-002".into(),
            name: "Pelanggan Contoh B".into(),
            address: String::new(),
            active: true,
            entries: vec![],
            outstanding: 0,
            customer_credit: 0,
        };
        b.sale("2026-11-03", "Penjualan contoh", 250_000).unwrap();
        let mut c = Customer {
            id: 3,
            code: "CTH-003".into(),
            name: "Pelanggan Contoh C".into(),
            address: String::new(),
            active: true,
            entries: vec![],
            outstanding: 0,
            customer_credit: 0,
        };
        c.payment("2026-11-07", 50_000, "Tunai").unwrap();
        vec![a, b, c]
    }
}

pub fn filter_customers<'a>(customers: &'a [Customer], query: &str) -> Vec<&'a Customer> {
    let query = query.trim().to_lowercase();
    customers
        .iter()
        .filter(|c| {
            c.name.to_lowercase().contains(&query) || c.code.to_lowercase().contains(&query)
        })
        .collect()
}

pub fn rupiah(value: i64) -> String {
    let digits = value.unsigned_abs().to_string();
    let grouped = digits
        .chars()
        .rev()
        .enumerate()
        .fold(String::new(), |mut out, (i, c)| {
            if i > 0 && i % 3 == 0 {
                out.push('.');
            }
            out.push(c);
            out
        });
    let formatted: String = grouped.chars().rev().collect();
    if value < 0 {
        format!("(Rp {formatted})")
    } else {
        format!("Rp {formatted}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn search_and_money_are_deterministic() {
        let customers = MockSource.customers();
        assert_eq!(filter_customers(&customers, "cth-002")[0].id, 2);
        assert!(filter_customers(&customers, "tidak ada").is_empty());
        assert_eq!(customers[0].outstanding, 60_000);
        assert_eq!(rupiah(-1250000), "(Rp 1.250.000)");
    }
    #[test]
    fn fifo_surplus_and_explicit_credit_application() {
        let mut c = MockSource.customers().remove(0);
        c.sale("2026-11-06", "Barang", 200_000).unwrap();
        c.payment("2026-11-07", 150_000, "Tunai").unwrap();
        assert_eq!(
            (
                c.entries[0].remaining,
                c.entries[2].remaining,
                c.outstanding
            ),
            (0, 110_000, 110_000)
        );
        c.payment("2026-11-08", 160_000, "Transfer").unwrap();
        assert_eq!((c.outstanding, c.customer_credit), (0, 50_000));
        c.sale("2026-11-09", "Barang berikutnya", 30_000).unwrap();
        assert_eq!((c.outstanding, c.customer_credit), (30_000, 50_000));
        let payment_id = c
            .entries
            .iter()
            .find(|e| e.unallocated == 50_000)
            .unwrap()
            .id;
        let debit_id = c.entries.last().unwrap().id;
        c.apply_credit(payment_id, debit_id, 20_000, "Contoh alokasi")
            .unwrap();
        assert_eq!((c.outstanding, c.customer_credit), (10_000, 30_000));
        assert_eq!(
            c.entries
                .iter()
                .find(|e| e.id == payment_id)
                .unwrap()
                .unallocated,
            30_000
        );
        assert!(
            c.apply_credit(payment_id, debit_id, 20_000, "Contoh")
                .is_err()
        );
    }

    #[test]
    fn multi_item_sale_is_atomic() {
        let item = |description: &str, total| SaleItem {
            description: description.into(),
            quantity: "1".into(),
            unit_price: total,
            total,
        };
        let mut customer = MockSource.customers().remove(0);
        let before = customer.outstanding;
        assert!(
            customer
                .sale_items("2026-11-12", &[item("Barang", 100), item("", 100)])
                .is_err()
        );
        assert_eq!(customer.outstanding, before);
        customer
            .sale_items(
                "2026-11-12",
                &[item("Barang A", 100), item("Barang B", 200)],
            )
            .unwrap();
        assert_eq!(customer.outstanding, before + 300);
    }

    #[test]
    fn draft_does_not_change_balance_until_posted_for_its_customer() {
        let mut customers = MockSource.customers();
        let before = customers[0].outstanding;
        let draft = Draft {
            id: 1,
            customer_id: customers[0].id,
            date: "2026-11-26".into(),
            reason: String::new(),
            items: vec![SaleItem {
                description: "Barang".into(),
                quantity: "2".into(),
                unit_price: 500,
                total: 1000,
            }],
        };
        assert!(draft.post(&mut customers[1]).is_err());
        assert_eq!(customers[0].outstanding, before);
        draft.post(&mut customers[0]).unwrap();
        assert_eq!(customers[0].outstanding, before + 1000);
    }

    #[test]
    fn aging_reconciles_and_observes_boundary() {
        let mut customers = MockSource.customers();
        customers[0].entries[0].date = "2026-11-10".into();
        let buckets = aging(
            &customers,
            chrono::NaiveDate::from_ymd_opt(2026, 11, 26).unwrap(),
        );
        assert_eq!(buckets[1], 310_000);
        assert_eq!(
            buckets.iter().sum::<i64>(),
            customers.iter().map(|c| c.outstanding).sum::<i64>()
        );
    }
    #[test]
    fn merge_preserves_balance_and_does_not_auto_apply_credit() {
        let mut customers = MockSource.customers();
        assert_eq!(merge_customers(&mut customers, 3, "cth-001").unwrap(), 1);
        assert_eq!(
            (customers[0].outstanding, customers[0].customer_credit),
            (60_000, 50_000)
        );
        assert!(!customers[2].active);
        assert!(customers[2].entries.is_empty());
    }

    #[test]
    fn reversal_restores_allocations_and_keeps_history() {
        let mut customer = MockSource.customers().remove(0);
        let debit = customer.entries[0].id;
        let payment = customer.entries[1].id;
        assert!(customer.reverse(debit, "2026-11-26", "koreksi").is_err());
        customer.reverse(payment, "2026-11-26", "koreksi").unwrap();
        assert_eq!(
            (
                customer.outstanding,
                customer.customer_credit,
                customer.entries[0].remaining
            ),
            (100_000, 0, 100_000)
        );
        assert!(customer.entries[1].reversed);
        assert_eq!(customer.entries.last().unwrap().reversal_of, Some(payment));
        assert!(customer.reverse(payment, "2026-11-26", "ulang").is_err());
        customer.reverse(debit, "2026-11-26", "koreksi").unwrap();
        assert_eq!(customer.outstanding, 0);
    }

    #[test]
    fn reallocation_is_atomic_and_replacement_is_tracked() {
        let mut customer = MockSource.customers().remove(0);
        let first_debit = customer.entries[0].id;
        let payment = customer.entries[1].id;
        customer.sale("2026-11-06", "Debit kedua", 80_000).unwrap();
        let second_debit = customer.entries.last().unwrap().id;
        assert!(
            customer
                .reallocate(payment, Some((second_debit, 50_000)), "uji gagal")
                .is_err()
        );
        assert_eq!(customer.entries[0].remaining, 60_000);
        customer
            .reallocate(payment, Some((second_debit, 30_000)), "pemindahan")
            .unwrap();
        assert_eq!(
            (
                customer.entries[0].remaining,
                customer.entries[2].remaining,
                customer.customer_credit
            ),
            (100_000, 50_000, 10_000)
        );
        customer
            .reverse(first_debit, "2026-11-26", "koreksi")
            .unwrap();
        customer.sale("2026-11-26", "Pengganti", 90_000).unwrap();
        let replacement = customer.entries.last().unwrap().id;
        customer
            .link_replacement(first_debit, replacement, "koreksi data")
            .unwrap();
        assert_eq!(customer.entries[0].replacement_id, Some(replacement));
        assert!(
            customer
                .link_replacement(first_debit, replacement, "ulang")
                .is_err()
        );
    }
}
