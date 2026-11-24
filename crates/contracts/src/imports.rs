use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{ApiError, EmptyMeta, PageMeta};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ImportReviewAction {
    Accept,
    Exclude,
    Quarantine,
    Override,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ImportBatchSummary {
    pub id: Uuid,
    pub file_sha256: String,
    pub original_filename: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub version: i32,
    pub total_rows: i64,
    pub candidate_rows: i64,
    pub quarantined_rows: i64,
    pub accepted_rows: i64,
    pub excluded_rows: i64,
    pub committed_rows: i64,
    pub source_debit_idr: String,
    pub source_credit_idr: String,
    pub selected_debit_idr: String,
    pub selected_credit_idr: String,
    pub unresolved_count: i64,
    pub uncomputable_amount_rows: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ImportBatchResponse {
    pub data: ImportBatchSummary,
    pub meta: EmptyMeta,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ImportBatchListResponse {
    pub data: Vec<ImportBatchSummary>,
    pub meta: PageMeta,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ImportRow {
    pub id: Uuid,
    pub sheet_name: String,
    pub row_number: i32,
    pub raw_values: Value,
    pub source_customer: ImportCustomerSource,
    pub parsed_values: Option<Value>,
    pub issues: Value,
    pub review_status: String,
    pub mapped_customer_id: Option<Uuid>,
    pub review_note: Option<String>,
    pub version: i32,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, ToSchema)]
pub struct ImportCustomerSource {
    pub code: Option<String>,
    pub name: Option<String>,
    pub address: Option<String>,
    pub formula_fields: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ImportCustomerIdentityPreview {
    pub source_sheet: String,
    pub source_code: Option<String>,
    pub source_name: Option<String>,
    pub source_address: Option<String>,
    pub source_rows: i64,
    pub source_debit_idr: String,
    pub source_credit_idr: String,
    pub pending_debit_idr: String,
    pub pending_credit_idr: String,
    pub valid_debit_idr: String,
    pub valid_credit_idr: String,
    pub candidate_rows: i64,
    pub accepted_rows: i64,
    pub valid_rows: i64,
    pub quarantined_rows: i64,
    pub excluded_rows: i64,
    pub committed_rows: i64,
    pub uncomputable_amount_rows: i64,
    pub mapped_valid_rows: i64,
    pub resolution: String,
    pub customer_id: Option<Uuid>,
    pub customer_name: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ImportCustomerIdentitiesResponse {
    pub data: Vec<ImportCustomerIdentityPreview>,
    pub meta: EmptyMeta,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PrepareImportCustomerIdentitiesData {
    pub updated_sheets: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PrepareImportCustomerIdentitiesResponse {
    pub data: PrepareImportCustomerIdentitiesData,
    pub meta: EmptyMeta,
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ImportRowsQuery {
    pub cursor: Option<String>,
    pub review_status: Option<String>,
    pub sheet_name: Option<String>,
    #[schema(minimum = 1, maximum = 100, default = 50)]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ImportRowsResponse {
    pub data: Vec<ImportRow>,
    pub meta: PageMeta,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewImportRow {
    pub expected_version: i32,
    pub action: ImportReviewAction,
    pub mapped_customer_id: Option<Uuid>,
    pub overrides: Option<Value>,
    pub review_note: Option<String>,
    pub reason: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ImportReviewResponse {
    pub data: ImportRow,
    pub meta: EmptyMeta,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CommitImport {
    pub expected_version: i32,
    pub row_ids: Vec<Uuid>,
    pub variance_idr: Option<String>,
    pub variance_reason: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ImportCommitData {
    pub batch_id: Uuid,
    pub status: String,
    pub committed_rows: i64,
    pub created_customers: i64,
    pub variance_idr: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ImportCommitResponse {
    pub data: ImportCommitData,
    pub meta: EmptyMeta,
}

#[derive(Debug, ToSchema)]
pub struct UploadImport {
    #[schema(value_type = String, format = Binary)]
    pub file: String,
}

#[utoipa::path(post, path = "/api/v1/import-batches", security(("session" = [])), params(("X-CSRF-Token" = String, Header)), request_body(content = UploadImport, content_type = "multipart/form-data"), responses((status = 201, body = ImportBatchResponse), (status = 200, body = ImportBatchResponse), (status = 400, body = ApiError), (status = 401, body = ApiError), (status = 403, body = ApiError), (status = 409, body = ApiError), (status = 413, body = ApiError), (status = 422, body = ApiError), (status = 500, body = ApiError)))]
pub fn upload_import() {}

#[utoipa::path(get, path = "/api/v1/import-batches", security(("session" = [])), responses((status = 200, body = ImportBatchListResponse), (status = 401, body = ApiError), (status = 403, body = ApiError), (status = 500, body = ApiError)))]
pub fn list_imports() {}

#[utoipa::path(get, path = "/api/v1/import-batches/{id}", security(("session" = [])), params(("id" = Uuid, Path)), responses((status = 200, body = ImportBatchResponse), (status = 401, body = ApiError), (status = 403, body = ApiError), (status = 404, body = ApiError), (status = 500, body = ApiError)))]
pub fn get_import() {}

#[utoipa::path(get, path = "/api/v1/import-batches/{id}/rows", security(("session" = [])), params(("id" = Uuid, Path), ImportRowsQuery), responses((status = 200, body = ImportRowsResponse), (status = 401, body = ApiError), (status = 403, body = ApiError), (status = 404, body = ApiError), (status = 500, body = ApiError)))]
pub fn list_import_rows() {}

#[utoipa::path(get, path = "/api/v1/import-batches/{id}/customer-identities", security(("session" = [])), params(("id" = Uuid, Path)), responses((status = 200, body = ImportCustomerIdentitiesResponse), (status = 401, body = ApiError), (status = 403, body = ApiError), (status = 404, body = ApiError), (status = 500, body = ApiError)))]
pub fn list_import_customer_identities() {}

#[utoipa::path(post, path = "/api/v1/import-batches/{id}/customer-identities/prepare", security(("session" = [])), params(("id" = Uuid, Path), ("X-CSRF-Token" = String, Header)), responses((status = 200, body = PrepareImportCustomerIdentitiesResponse), (status = 401, body = ApiError), (status = 403, body = ApiError), (status = 404, body = ApiError), (status = 409, body = ApiError), (status = 500, body = ApiError)))]
pub fn prepare_import_customer_identities() {}

#[utoipa::path(patch, path = "/api/v1/import-batches/{id}/rows/{row_id}", security(("session" = [])), params(("id" = Uuid, Path), ("row_id" = Uuid, Path), ("X-CSRF-Token" = String, Header)), request_body = ReviewImportRow, responses((status = 200, body = ImportReviewResponse), (status = 401, body = ApiError), (status = 403, body = ApiError), (status = 404, body = ApiError), (status = 409, body = ApiError), (status = 422, body = ApiError), (status = 500, body = ApiError)))]
pub fn review_import_row() {}

#[utoipa::path(post, path = "/api/v1/import-batches/{id}/commit", security(("session" = [])), params(("id" = Uuid, Path), ("X-CSRF-Token" = String, Header), ("Idempotency-Key" = String, Header)), request_body = CommitImport, responses((status = 200, body = ImportCommitResponse), (status = 401, body = ApiError), (status = 403, body = ApiError), (status = 404, body = ApiError), (status = 409, body = ApiError), (status = 422, body = ApiError), (status = 500, body = ApiError)))]
pub fn commit_import() {}
