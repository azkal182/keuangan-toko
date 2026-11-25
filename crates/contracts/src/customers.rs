use chrono::NaiveDate;
use serde::{Deserialize, Deserializer, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{ApiError, EmptyMeta, PageMeta};

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Customer {
    pub id: Uuid,
    #[schema(required = true)]
    pub code: Option<String>,
    pub name: String,
    #[schema(required = true)]
    pub address: Option<String>,
    pub is_active: bool,
    pub version: i32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CustomerDetail {
    pub customer: Customer,
    #[schema(pattern = "^[0-9]+$", example = "60000")]
    pub outstanding_idr: String,
    #[schema(pattern = "^[0-9]+$", example = "0")]
    pub customer_credit_idr: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CustomerList {
    pub items: Vec<CustomerSummary>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CustomerSummary {
    pub customer: Customer,
    #[schema(pattern = "^[0-9]+$")]
    pub outstanding_idr: String,
    #[schema(pattern = "^[0-9]+$")]
    pub customer_credit_idr: String,
    pub days_0_15_idr: String,
    pub days_16_30_idr: String,
    pub days_31_45_idr: String,
    pub days_46_60_idr: String,
    pub days_over_60_idr: String,
    pub unknown_date_idr: String,
    pub future_date_idr: String,
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListCustomersQuery {
    pub search: Option<String>,
    pub cursor: Option<String>,
    #[schema(minimum = 1, maximum = 100, default = 50)]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateCustomer {
    pub code: Option<String>,
    pub name: String,
    pub address: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateCustomer {
    pub expected_version: i32,
    #[serde(
        default,
        deserialize_with = "present_value",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(value_type = String, required = false)]
    pub name: Option<String>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(value_type = Option<String>)]
    pub code: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(value_type = Option<String>)]
    pub address: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_value",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(value_type = bool, required = false)]
    pub is_active: Option<bool>,
}

fn present_value<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}

pub(crate) fn present_nullable<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Option<String>>, D::Error> {
    Option::deserialize(deserializer).map(Some)
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MergeCustomer {
    pub target_customer_id: Uuid,
    pub reason: String,
    pub expected_version: i32,
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct LedgerQuery {
    pub cursor: Option<String>,
    #[schema(minimum = 1, maximum = 100, default = 50)]
    pub limit: Option<u32>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    #[schema(pattern = "^(asc|desc)$", default = "desc")]
    pub order: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LedgerEntry {
    pub id: Uuid,
    pub entry_type: String,
    pub source_id: Uuid,
    pub is_reversed: bool,
    #[schema(required = true)]
    pub effective_date: Option<NaiveDate>,
    #[schema(required = true)]
    pub reference: Option<String>,
    #[schema(required = true)]
    pub description: Option<String>,
    #[schema(required = true)]
    pub quantity: Option<String>,
    #[schema(required = true)]
    pub unit_price_idr: Option<String>,
    #[schema(pattern = "^[0-9]+$")]
    pub debit_idr: String,
    #[schema(pattern = "^[0-9]+$")]
    pub credit_idr: String,
    #[schema(pattern = "^[0-9]+$")]
    pub allocated_idr: String,
    #[schema(pattern = "^[0-9]+$")]
    pub remaining_idr: String,
    #[schema(required = true)]
    pub age_days: Option<i32>,
    #[schema(required = true)]
    pub allocation_status: Option<String>,
    #[schema(pattern = "^-?[0-9]+$")]
    pub running_balance_idr: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LedgerSummary {
    #[schema(pattern = "^[0-9]+$")]
    pub outstanding_idr: String,
    #[schema(pattern = "^[0-9]+$")]
    pub customer_credit_idr: String,
    #[schema(pattern = "^-?[0-9]+$")]
    pub ledger_balance_idr: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LedgerPage {
    pub entries: Vec<LedgerEntry>,
    pub summary: LedgerSummary,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CustomerListResponse {
    pub data: CustomerList,
    pub meta: PageMeta,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CustomerResponse {
    pub data: Customer,
    pub meta: EmptyMeta,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CustomerDetailResponse {
    pub data: CustomerDetail,
    pub meta: EmptyMeta,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LedgerResponse {
    pub data: LedgerPage,
    pub meta: PageMeta,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_distinguishes_absent_nullable_fields_from_explicit_null() {
        let patch: UpdateCustomer =
            serde_json::from_str(r#"{"expected_version":1,"code":null}"#).unwrap();
        assert_eq!(patch.code, Some(None));
        assert_eq!(patch.address, None);
        assert!(
            serde_json::from_str::<UpdateCustomer>(r#"{"expected_version":1,"name":null}"#)
                .is_err()
        );
        assert!(
            serde_json::from_str::<UpdateCustomer>(r#"{"expected_version":1,"is_active":null}"#)
                .is_err()
        );
        assert_eq!(
            serde_json::to_value(patch).unwrap(),
            serde_json::json!({"expected_version":1,"code":null})
        );
    }
}

#[utoipa::path(
    get, path = "/api/v1/customers", security(("session" = [])),
    params(ListCustomersQuery),
    responses(
        (status = 200, description = "Berhasil", body = CustomerListResponse),
        (status = 400, description = "Permintaan tidak valid", body = ApiError),
        (status = 401, description = "Sesi diperlukan", body = ApiError),
        (status = 422, description = "Validasi gagal", body = ApiError),
        (status = 500, description = "Kegagalan internal", body = ApiError)
    )
)]
pub fn list_customers() {}

#[utoipa::path(
    post, path = "/api/v1/customers", security(("session" = [])),
    params(("X-CSRF-Token" = String, Header)),
    request_body = CreateCustomer,
    responses(
        (status = 201, description = "Berhasil", body = CustomerResponse),
        (status = 400, description = "Permintaan tidak valid", body = ApiError),
        (status = 401, description = "Sesi diperlukan", body = ApiError),
        (status = 403, description = "Akses ditolak", body = ApiError),
        (status = 409, description = "Konflik data", body = ApiError),
        (status = 422, description = "Validasi gagal", body = ApiError),
        (status = 500, description = "Kegagalan internal", body = ApiError)
    )
)]
pub fn create_customer() {}

#[utoipa::path(
    get, path = "/api/v1/customers/{id}", security(("session" = [])),
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "Berhasil", body = CustomerDetailResponse),
        (status = 400, description = "Permintaan tidak valid", body = ApiError),
        (status = 401, description = "Sesi diperlukan", body = ApiError),
        (status = 404, description = "Pelanggan tidak ditemukan", body = ApiError),
        (status = 500, description = "Kegagalan internal", body = ApiError)
    )
)]
pub fn get_customer() {}

#[utoipa::path(
    patch, path = "/api/v1/customers/{id}", security(("session" = [])),
    params(("id" = Uuid, Path), ("X-CSRF-Token" = String, Header)),
    request_body = UpdateCustomer,
    responses(
        (status = 200, description = "Berhasil", body = CustomerResponse),
        (status = 400, description = "Permintaan tidak valid", body = ApiError),
        (status = 401, description = "Sesi diperlukan", body = ApiError),
        (status = 403, description = "Akses ditolak", body = ApiError),
        (status = 404, description = "Pelanggan tidak ditemukan", body = ApiError),
        (status = 409, description = "Konflik data", body = ApiError),
        (status = 422, description = "Validasi gagal", body = ApiError),
        (status = 500, description = "Kegagalan internal", body = ApiError)
    )
)]
pub fn update_customer() {}

#[utoipa::path(
    post, path = "/api/v1/customers/{id}/merge", security(("session" = [])),
    params(("id" = Uuid, Path), ("X-CSRF-Token" = String, Header)),
    request_body = MergeCustomer,
    responses(
        (status = 200, description = "Berhasil", body = CustomerResponse),
        (status = 400, description = "Permintaan tidak valid", body = ApiError),
        (status = 401, description = "Sesi diperlukan", body = ApiError),
        (status = 403, description = "Akses ditolak", body = ApiError),
        (status = 404, description = "Pelanggan tidak ditemukan", body = ApiError),
        (status = 409, description = "Konflik data", body = ApiError),
        (status = 422, description = "Validasi gagal", body = ApiError),
        (status = 500, description = "Kegagalan internal", body = ApiError)
    )
)]
pub fn merge_customer() {}

#[utoipa::path(
    get, path = "/api/v1/customers/{id}/ledger", security(("session" = [])),
    params(("id" = Uuid, Path), LedgerQuery),
    responses(
        (status = 200, description = "Berhasil", body = LedgerResponse),
        (status = 400, description = "Permintaan tidak valid", body = ApiError),
        (status = 401, description = "Sesi diperlukan", body = ApiError),
        (status = 404, description = "Pelanggan tidak ditemukan", body = ApiError),
        (status = 422, description = "Validasi gagal", body = ApiError),
        (status = 500, description = "Kegagalan internal", body = ApiError)
    )
)]
pub fn list_ledger() {}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct PaymentsQuery {
    pub cursor: Option<String>,
    #[schema(minimum = 1, maximum = 100, default = 50)]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaymentAllocation {
    pub debit_entry_id: Uuid,
    pub amount_idr: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Payment {
    pub id: Uuid,
    pub customer_id: Uuid,
    pub amount_idr: String,
    pub received_date: NaiveDate,
    pub method: String,
    pub reference: Option<String>,
    pub note: Option<String>,
    pub status: String,
    pub posted_at: chrono::DateTime<chrono::Utc>,
    pub allocations: Vec<PaymentAllocation>,
    pub unallocated_idr: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreatePayment {
    pub customer_id: Uuid,
    pub amount_idr: String,
    pub received_date: NaiveDate,
    pub method: PaymentMethod,
    pub reference: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethod {
    Cash,
    BankTransfer,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplyCredit {
    pub payment_id: Uuid,
    pub debit_entry_id: Uuid,
    pub amount_idr: String,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaymentDetail {
    pub id: Uuid,
    pub customer_id: Uuid,
    pub amount_idr: String,
    pub received_date: NaiveDate,
    pub method: PaymentMethod,
    pub reference: Option<String>,
    pub note: Option<String>,
    pub status: String,
    pub version: i32,
    pub posted_at: chrono::DateTime<chrono::Utc>,
    pub allocations: Vec<PaymentAllocation>,
    pub unallocated_idr: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaymentResponse {
    pub data: PaymentDetail,
    pub meta: EmptyMeta,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PaymentListResponse {
    pub data: Vec<PaymentDetail>,
    pub meta: PageMeta,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreditApplicationResponse {
    pub data: PaymentAllocation,
    pub meta: EmptyMeta,
}

#[utoipa::path(
    post, path = "/api/v1/payments", security(("session" = [])),
    params(("X-CSRF-Token" = String, Header), ("Idempotency-Key" = String, Header)),
    request_body = CreatePayment,
    responses(
        (status = 201, description = "Pembayaran diterima", body = PaymentResponse),
        (status = 400, description = "Permintaan tidak valid", body = ApiError),
        (status = 401, description = "Sesi diperlukan", body = ApiError),
        (status = 403, description = "Akses ditolak", body = ApiError),
        (status = 404, description = "Pelanggan tidak ditemukan", body = ApiError),
        (status = 409, description = "Konflik idempotensi", body = ApiError),
        (status = 422, description = "Validasi gagal", body = ApiError),
        (status = 500, description = "Kegagalan internal", body = ApiError)
    )
)]
pub fn create_payment() {}

#[utoipa::path(
    get, path = "/api/v1/customers/{id}/payments", security(("session" = [])),
    params(("id" = Uuid, Path), PaymentsQuery),
    responses((status = 200, description = "Riwayat pembayaran", body = PaymentListResponse),
      (status = 400, description = "Cursor tidak valid", body = ApiError),
      (status = 401, description = "Sesi diperlukan", body = ApiError),
      (status = 404, description = "Pelanggan tidak ditemukan", body = ApiError),
      (status = 500, description = "Kegagalan internal", body = ApiError))
)]
pub fn customer_payments() {}

#[utoipa::path(
    get, path = "/api/v1/payments/{id}", security(("session" = [])),
    params(("id" = Uuid, Path)),
    responses((status = 200, description = "Rincian pembayaran", body = PaymentResponse),
      (status = 401, description = "Sesi diperlukan", body = ApiError),
      (status = 404, description = "Pembayaran tidak ditemukan", body = ApiError),
      (status = 500, description = "Kegagalan internal", body = ApiError))
)]
pub fn payment_detail() {}

#[utoipa::path(
    post, path = "/api/v1/customers/{id}/credit-applications", security(("session" = [])),
    params(("id" = Uuid, Path), ("X-CSRF-Token" = String, Header), ("Idempotency-Key" = String, Header)),
    request_body = ApplyCredit,
    responses((status = 200, description = "Kredit diterapkan", body = CreditApplicationResponse),
      (status = 400, description = "Permintaan tidak valid", body = ApiError),
      (status = 401, description = "Sesi diperlukan", body = ApiError),
      (status = 403, description = "Akses ditolak", body = ApiError),
      (status = 404, description = "Pembayaran/piutang tidak ditemukan", body = ApiError),
      (status = 409, description = "Konflik idempotensi", body = ApiError),
      (status = 422, description = "Validasi gagal", body = ApiError),
      (status = 500, description = "Kegagalan internal", body = ApiError))
)]
pub fn apply_credit() {}
