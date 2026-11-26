//! Abstraksi adapter dan tipe data untuk aplikasi desktop Keuangan Toko.
//! Memisahkan secara tegas antarmuka pengguna (UI) dari implementasi data store (Mock sekarang, API / SQLite kelak).
//! Seluruh tipe dan aturan bisnis diselaraskan dengan DATA_DICTIONARY.md, FEATURE_SPECIFICATIONS.md, dan crates/contracts.

use chrono::NaiveDate;
use std::fmt;

pub mod mock_store;
pub use mock_store::MockDataAdapter;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserRole {
    Owner,
    Operator,
    Viewer,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserRole::Owner => "owner",
            UserRole::Operator => "operator",
            UserRole::Viewer => "viewer",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            UserRole::Owner => "Pemilik (Owner)",
            UserRole::Operator => "Operator Kasir",
            UserRole::Viewer => "Pembaca (Viewer)",
        }
    }
}

impl fmt::Display for UserRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone)]
pub struct SessionUser {
    pub id: String,
    pub username: String,
    pub role: UserRole,
    pub must_change_password: bool,
}

#[derive(Debug, Clone, Default)]
pub struct AgingBuckets {
    pub days_0_15: i64,
    pub days_16_30: i64,
    pub days_31_45: i64,
    pub days_46_60: i64,
    pub days_over_60: i64,
    pub unknown_date: i64,
    pub future_date: i64,
}

impl AgingBuckets {
    pub fn exception_total(&self) -> i64 {
        self.unknown_date + self.future_date
    }
}

#[derive(Debug, Clone)]
pub struct CustomerSummary {
    pub id: String,
    pub code: Option<String>,
    pub name: String,
    pub address: Option<String>,
    pub is_active: bool,
    pub version: i32,
    pub outstanding_idr: i64,
    pub customer_credit_idr: i64,
    pub aging: AgingBuckets,
}

#[derive(Debug, Clone)]
pub struct CustomerDetail {
    pub id: String,
    pub code: Option<String>,
    pub name: String,
    pub address: Option<String>,
    pub is_active: bool,
    pub version: i32,
    pub outstanding_idr: i64,
    pub customer_credit_idr: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationStatus {
    Allocated,
    PartiallyAllocated,
    Unallocated,
}

impl AllocationStatus {
    pub fn label(&self) -> &'static str {
        match self {
            AllocationStatus::Allocated => "Lunas",
            AllocationStatus::PartiallyAllocated => "Belum",
            AllocationStatus::Unallocated => "Belum",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LedgerEntry {
    pub id: String,
    pub entry_type: String, // "credit_sale" | "payment" | "legacy_debit" | "legacy_payment" | "reversal"
    pub source_id: String,
    pub effective_date: Option<NaiveDate>,
    pub reference: Option<String>,
    pub description: String,
    pub quantity: Option<String>,
    pub unit_price_idr: Option<i64>,
    pub debit_idr: i64,
    pub credit_idr: i64,
    pub running_balance_idr: i64,
    pub allocated_idr: i64,
    pub remaining_idr: i64,
    pub age_days: Option<i32>,
    pub allocation_status: Option<AllocationStatus>,
    pub is_reversed: bool,
    pub reversal_reason: Option<String>,
    pub replacement_entry_id: Option<String>,
    pub source_order: i64,
}

#[derive(Debug, Clone)]
pub struct LedgerSummary {
    pub outstanding_idr: i64,
    pub customer_credit_idr: i64,
    pub ledger_balance_idr: i64,
}

#[derive(Debug, Clone)]
pub struct LedgerPage {
    pub entries: Vec<LedgerEntry>,
    pub summary: LedgerSummary,
}

#[derive(Debug, Clone)]
pub struct PaymentAllocationItem {
    pub debit_entry_id: String,
    pub amount_idr: i64,
    pub debit_description: String,
}

#[derive(Debug, Clone)]
pub struct PaymentRow {
    pub id: String,
    pub customer_id: String,
    pub amount_idr: i64,
    pub received_date: NaiveDate,
    pub method: String, // "cash" | "bank_transfer" | "legacy"
    pub reference: Option<String>,
    pub note: Option<String>,
    pub status: String, // "posted" | "reversed"
    pub version: i32,
    pub allocated_idr: i64,
    pub unallocated_credit_idr: i64,
    pub allocations: Vec<PaymentAllocationItem>,
}

impl PaymentRow {
    pub fn method_label(&self) -> &'static str {
        match self.method.as_str() {
            "cash" | "Tunai" => "Tunai",
            "bank_transfer" | "Transfer Bank" => "Transfer Bank",
            "legacy" => "Historis",
            _ => "Tunai",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SaleItemInput {
    pub description: String,
    pub quantity: String,
    pub unit_price_idr: i64,
}

#[derive(Debug, Clone)]
pub struct InstantSaleInput {
    pub effective_date: NaiveDate,
    pub reference_no: Option<String>,
    pub backdate_reason: Option<String>,
    pub items: Vec<SaleItemInput>,
}

#[derive(Debug, Clone)]
pub struct RecordPaymentInput {
    pub received_date: NaiveDate,
    pub amount_idr: i64,
    pub method: String, // "cash" | "bank_transfer"
    pub reference: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ApplyCreditInput {
    pub payment_id: String,
    pub debit_entry_id: String,
    pub amount_idr: i64,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct ReallocatePaymentInput {
    pub payment_id: String,
    pub debit_entry_id: Option<String>,
    pub amount_idr: i64,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct AgingReportCustomerItem {
    pub customer_id: String,
    pub customer_code: Option<String>,
    pub customer_name: String,
    pub is_active: bool,
    pub total_outstanding_idr: i64,
    pub days_0_15: i64,
    pub days_16_30: i64,
    pub days_31_45: i64,
    pub days_46_60: i64,
    pub days_over_60: i64,
    pub unknown_date: i64,
    pub future_date: i64,
}

#[derive(Debug, Clone)]
pub struct AgingReport {
    pub as_of: NaiveDate,
    pub total_open_idr: i64,
    pub days_0_15: i64,
    pub days_16_30: i64,
    pub days_31_45: i64,
    pub days_46_60: i64,
    pub days_over_60: i64,
    pub unknown_date: i64,
    pub future_date: i64,
    pub customers: Vec<AgingReportCustomerItem>,
}

#[derive(Debug, Clone)]
pub struct UserItem {
    pub id: String,
    pub username: String,
    pub role: UserRole,
    pub is_active: bool,
    pub must_change_password: bool,
    pub created_at: NaiveDate,
}

#[derive(Debug, Clone)]
pub struct CreateCustomerInput {
    pub code: Option<String>,
    pub name: String,
    pub address: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UpdateCustomerInput {
    pub code: Option<String>,
    pub name: String,
    pub address: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct CreateUserInput {
    pub username: String,
    pub role: UserRole,
    pub temporary_password: String,
}

#[derive(Debug, Clone)]
pub struct UpdateUserInput {
    pub username: String,
    pub role: UserRole,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct CustomerQuery {
    pub search: Option<String>,
    pub active_only: bool,
}

#[derive(Debug, Clone)]
pub struct LedgerQuery {
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    pub order_desc: bool,
}

#[derive(Debug, Clone)]
pub struct AppError(pub String);

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for AppError {}

/// Trait Adapter utama yang memisahkan presentasi UI dari sumber data.
/// Diimplementasikan oleh `MockDataAdapter` sekarang, dan `HttpApiAdapter` / `SqliteAdapter` kelak.
pub trait TokoStoreAdapter: Send + Sync {
    // Sesi & Pengguna
    fn current_session(&self) -> Option<SessionUser>;
    fn login(&mut self, username: &str, password: &str) -> Result<SessionUser, AppError>;
    fn logout(&mut self);
    fn switch_session_role(&mut self, role: UserRole);
    fn change_password(&mut self, old_pw: &str, new_pw: &str) -> Result<(), AppError>;

    // Pelanggan (API-101..104, API-107)
    fn list_customers(&self, query: &CustomerQuery) -> Vec<CustomerSummary>;
    fn get_customer(&self, id: &str) -> Result<CustomerDetail, AppError>;
    fn create_customer(&mut self, input: &CreateCustomerInput) -> Result<CustomerDetail, AppError>;
    fn update_customer(
        &mut self,
        id: &str,
        input: &UpdateCustomerInput,
    ) -> Result<CustomerDetail, AppError>;
    fn toggle_customer_active(&mut self, id: &str) -> Result<bool, AppError>;
    fn merge_customers(
        &mut self,
        source_id: &str,
        target_id: &str,
        reason: &str,
    ) -> Result<(), AppError>;

    // Buku Piutang (Ledger) & Penjualan (API-105, API-201..203, API-304, API-306)
    fn get_ledger(&self, customer_id: &str, query: &LedgerQuery) -> Result<LedgerPage, AppError>;
    fn create_instant_sale(
        &mut self,
        customer_id: &str,
        input: &InstantSaleInput,
    ) -> Result<i64, AppError>;
    fn reverse_transaction(
        &mut self,
        customer_id: &str,
        entry_id: &str,
        reason: &str,
    ) -> Result<(), AppError>;

    // Pembayaran & Alokasi Saldo Kredit (API-106, API-301..303, API-305)
    fn list_payments(&self, customer_id: &str) -> Result<Vec<PaymentRow>, AppError>;
    fn record_payment(
        &mut self,
        customer_id: &str,
        input: &RecordPaymentInput,
    ) -> Result<PaymentRow, AppError>;
    fn apply_credit(&mut self, customer_id: &str, input: &ApplyCreditInput)
    -> Result<(), AppError>;
    fn reallocate_payment(
        &mut self,
        customer_id: &str,
        input: &ReallocatePaymentInput,
    ) -> Result<(), AppError>;

    // Laporan Umur Piutang (API-501..502)
    fn get_aging_report(&self, as_of: NaiveDate) -> Result<AgingReport, AppError>;

    // Manajemen Pengguna (API-005..009 - Owner Only)
    fn list_users(&self) -> Result<Vec<UserItem>, AppError>;
    fn create_user(&mut self, input: &CreateUserInput) -> Result<UserItem, AppError>;
    fn update_user(&mut self, id: &str, input: &UpdateUserInput) -> Result<UserItem, AppError>;
    fn reset_password(&mut self, id: &str, temporary_password: &str) -> Result<(), AppError>;
}
