use chrono::NaiveDate;
use toko_money::{line_total, quantity_milli, rupiah};

use crate::identity::Role;

pub struct SaleItem {
    pub description: String,
    pub quantity_milli: i64,
    pub unit_price_idr: i64,
    pub line_total_idr: i64,
}

impl SaleItem {
    pub fn new(
        description: &str,
        quantity: &str,
        price: &str,
    ) -> Result<Self, (&'static str, &'static str)> {
        let description = description.trim();
        if description.is_empty()
            || description.contains('\0')
            || description.chars().count() > 1000
        {
            return Err((
                "description",
                "Uraian wajib diisi dan maksimal 1.000 karakter",
            ));
        }
        let quantity_milli = quantity_milli(quantity).ok_or((
            "quantity",
            "Kuantitas harus positif dan maksimal tiga angka desimal",
        ))?;
        let unit_price_idr = rupiah(price).ok_or((
            "unit_price_idr",
            "Harga satuan harus berupa rupiah bulat dalam batas yang didukung",
        ))?;
        let line_total_idr = line_total(quantity_milli, unit_price_idr)
            .filter(|amount| *amount > 0)
            .ok_or((
                "unit_price_idr",
                "Total item setelah pembulatan harus positif dan dalam batas rupiah",
            ))?;
        Ok(Self {
            description: description.to_owned(),
            quantity_milli,
            unit_price_idr,
            line_total_idr,
        })
    }
}

pub fn sale_total(items: &[SaleItem]) -> Option<i64> {
    if items.is_empty() {
        return None;
    }
    items
        .iter()
        .try_fold(0_i64, |sum, item| sum.checked_add(item.line_total_idr))
}

pub enum SaleDateError {
    Future,
    OwnerRequired,
    ReasonRequired,
}

pub fn validate_date(
    effective: NaiveDate,
    today: NaiveDate,
    _role: Role,
    _reason: Option<&str>,
) -> Result<(), SaleDateError> {
    if effective > today {
        return Err(SaleDateError::Future);
    }
    // Tanggal lampau diperbolehkan secara bebas untuk semua pengguna toko tanpa wajib alasan
    Ok(())
}
