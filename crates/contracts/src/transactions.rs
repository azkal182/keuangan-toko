use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{ApiError, EmptyMeta};

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReverseTransaction {
    pub reason: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ReversalData {
    pub original_transaction_id: Uuid,
    pub original_entry_id: Uuid,
    pub reversal_entry_id: Uuid,
    pub status: String,
    pub reversed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ReversalResponse {
    pub data: ReversalData,
    pub meta: EmptyMeta,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LinkReplacement {
    pub replacement_transaction_id: Uuid,
    pub reason: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CorrectionLinkData {
    pub original_transaction_id: Uuid,
    pub original_entry_id: Uuid,
    pub reversal_entry_id: Uuid,
    pub replacement_transaction_id: Uuid,
    pub replacement_entry_id: Uuid,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CorrectionLinkResponse {
    pub data: CorrectionLinkData,
    pub meta: EmptyMeta,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReallocatePayment {
    pub expected_version: i32,
    pub allocations: Vec<ReplacementAllocation>,
    pub reason: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReplacementAllocation {
    pub debit_entry_id: Uuid,
    pub amount_idr: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ReallocationData {
    pub payment_id: Uuid,
    pub version: i32,
    pub allocations: Vec<ReplacementAllocation>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ReallocationResponse {
    pub data: ReallocationData,
    pub meta: EmptyMeta,
}

#[utoipa::path(post, path = "/api/v1/transactions/{id}/reversal", security(("session" = [])), params(("id" = Uuid, Path), ("X-CSRF-Token" = String, Header), ("Idempotency-Key" = String, Header)), request_body = ReverseTransaction, responses((status = 200, body = ReversalResponse), (status = 401, body = ApiError), (status = 403, body = ApiError), (status = 404, body = ApiError), (status = 409, body = ApiError), (status = 422, body = ApiError), (status = 500, body = ApiError)))]
pub fn reverse_transaction() {}

#[utoipa::path(post, path = "/api/v1/transactions/{id}/replacement", security(("session" = [])), params(("id" = Uuid, Path), ("X-CSRF-Token" = String, Header)), request_body = LinkReplacement, responses((status = 200, body = CorrectionLinkResponse), (status = 401, body = ApiError), (status = 403, body = ApiError), (status = 404, body = ApiError), (status = 409, body = ApiError), (status = 422, body = ApiError), (status = 500, body = ApiError)))]
pub fn link_replacement() {}

#[utoipa::path(post, path = "/api/v1/payments/{id}/reallocation", security(("session" = [])), params(("id" = Uuid, Path), ("X-CSRF-Token" = String, Header), ("Idempotency-Key" = String, Header)), request_body = ReallocatePayment, responses((status = 200, body = ReallocationResponse), (status = 401, body = ApiError), (status = 403, body = ApiError), (status = 404, body = ApiError), (status = 409, body = ApiError), (status = 422, body = ApiError), (status = 500, body = ApiError)))]
pub fn reallocate_payment() {}
