use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    ApiError, EmptyMeta,
    customers::{LedgerSummary, present_nullable},
};

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaleItemInput {
    pub description: String,
    pub quantity: String,
    pub unit_price_idr: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateSale {
    pub customer_id: Uuid,
    pub effective_date: NaiveDate,
    pub reference_no: Option<String>,
    pub backdate_reason: Option<String>,
    pub items: Vec<SaleItemInput>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateSale {
    pub expected_version: i32,
    pub customer_id: Option<Uuid>,
    pub effective_date: Option<NaiveDate>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(value_type = Option<String>)]
    pub reference_no: Option<Option<String>>,
    #[serde(
        default,
        deserialize_with = "present_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    #[schema(value_type = Option<String>)]
    pub backdate_reason: Option<Option<String>>,
    pub items: Option<Vec<SaleItemInput>>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PostSale {
    pub expected_version: i32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SaleStatus {
    Draft,
    Posted,
    Reversed,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SaleItem {
    pub id: Uuid,
    pub description: String,
    pub quantity: String,
    pub unit_price_idr: String,
    pub line_total_idr: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Sale {
    pub id: Uuid,
    pub customer_id: Uuid,
    pub effective_date: NaiveDate,
    pub reference_no: Option<String>,
    pub backdate_reason: Option<String>,
    pub status: SaleStatus,
    pub total_idr: String,
    pub version: i32,
    pub posted_at: Option<DateTime<Utc>>,
    pub ledger_entry_id: Option<Uuid>,
    pub items: Vec<SaleItem>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SaleResponse {
    pub data: Sale,
    pub meta: EmptyMeta,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PostedSale {
    pub sale: Sale,
    pub balance: LedgerSummary,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PostSaleResponse {
    pub data: PostedSale,
    pub meta: EmptyMeta,
}

#[utoipa::path(
    post, path = "/api/v1/credit-sales", security(("session" = [])),
    params(("X-CSRF-Token" = String, Header), ("Idempotency-Key" = String, Header)),
    request_body = CreateSale,
    responses(
        (status = 201, description = "Berhasil", body = SaleResponse),
        (status = 400, description = "Permintaan gagal", body = ApiError),
        (status = 401, description = "Permintaan gagal", body = ApiError),
        (status = 403, description = "Permintaan gagal", body = ApiError),
        (status = 404, description = "Permintaan gagal", body = ApiError),
        (status = 409, description = "Permintaan gagal", body = ApiError),
        (status = 413, description = "Permintaan gagal", body = ApiError),
        (status = 422, description = "Permintaan gagal", body = ApiError),
        (status = 500, description = "Permintaan gagal", body = ApiError)
    )
)]
pub fn create_sale() {}

#[utoipa::path(
    patch, path = "/api/v1/credit-sales/{id}", security(("session" = [])),
    params(("id" = Uuid, Path), ("X-CSRF-Token" = String, Header)),
    request_body = UpdateSale,
    responses(
        (status = 200, description = "Berhasil", body = SaleResponse),
        (status = 400, description = "Permintaan gagal", body = ApiError),
        (status = 401, description = "Permintaan gagal", body = ApiError),
        (status = 403, description = "Permintaan gagal", body = ApiError),
        (status = 404, description = "Permintaan gagal", body = ApiError),
        (status = 409, description = "Permintaan gagal", body = ApiError),
        (status = 413, description = "Permintaan gagal", body = ApiError),
        (status = 422, description = "Permintaan gagal", body = ApiError),
        (status = 500, description = "Permintaan gagal", body = ApiError)
    )
)]
pub fn update_sale() {}

#[utoipa::path(
    post, path = "/api/v1/credit-sales/{id}/post", security(("session" = [])),
    params(("id" = Uuid, Path), ("X-CSRF-Token" = String, Header), ("Idempotency-Key" = String, Header)),
    request_body = PostSale,
    responses(
        (status = 200, description = "Berhasil", body = PostSaleResponse),
        (status = 400, description = "Permintaan gagal", body = ApiError),
        (status = 401, description = "Permintaan gagal", body = ApiError),
        (status = 403, description = "Permintaan gagal", body = ApiError),
        (status = 404, description = "Permintaan gagal", body = ApiError),
        (status = 409, description = "Permintaan gagal", body = ApiError),
        (status = 413, description = "Permintaan gagal", body = ApiError),
        (status = 422, description = "Permintaan gagal", body = ApiError),
        (status = 500, description = "Permintaan gagal", body = ApiError)
    )
)]
pub fn post_sale() {}

#[utoipa::path(
    get, path = "/api/v1/credit-sales/{id}", security(("session" = [])),
    params(("id" = Uuid, Path)),
    responses(
        (status = 200, description = "Berhasil", body = SaleResponse),
        (status = 400, description = "Permintaan gagal", body = ApiError),
        (status = 401, description = "Permintaan gagal", body = ApiError),
        (status = 404, description = "Permintaan gagal", body = ApiError),
        (status = 500, description = "Permintaan gagal", body = ApiError)
    )
)]
pub fn get_sale() {}
