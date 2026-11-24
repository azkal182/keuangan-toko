use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{ApiError, EmptyMeta, PageMeta};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
pub enum AgingBucket {
    #[serde(rename = "days_0_15")]
    Days0_15,
    #[serde(rename = "days_16_30")]
    Days16_30,
    #[serde(rename = "days_31_45")]
    Days31_45,
    #[serde(rename = "days_46_60")]
    Days46_60,
    #[serde(rename = "days_over_60")]
    DaysOver60,
    #[serde(rename = "unknown_date")]
    UnknownDate,
    #[serde(rename = "future_date")]
    FutureDate,
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct AgingQuery {
    pub as_of: Option<NaiveDate>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AgingSummary {
    pub as_of: NaiveDate,
    pub days_0_15: String,
    pub days_16_30: String,
    pub days_31_45: String,
    pub days_46_60: String,
    pub days_over_60: String,
    pub unknown_date: String,
    pub future_date: String,
    pub total_open_idr: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AgingResponse {
    pub data: AgingSummary,
    pub meta: EmptyMeta,
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct AgingCustomersQuery {
    pub as_of: Option<NaiveDate>,
    pub bucket: AgingBucket,
    pub cursor: Option<String>,
    #[schema(minimum = 1, maximum = 100, default = 50)]
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AgingCustomer {
    pub customer_id: Uuid,
    pub customer_name: String,
    pub amount_idr: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AgingCustomersResponse {
    pub data: Vec<AgingCustomer>,
    pub meta: PageMeta,
}

#[utoipa::path(get, path = "/api/v1/reports/receivables-aging", security(("session" = [])), params(AgingQuery), responses((status = 200, body = AgingResponse), (status = 400, body = ApiError), (status = 401, body = ApiError), (status = 403, body = ApiError), (status = 422, body = ApiError), (status = 500, body = ApiError)))]
pub fn receivables_aging() {}

#[utoipa::path(get, path = "/api/v1/reports/receivables-aging/customers", security(("session" = [])), params(AgingCustomersQuery), responses((status = 200, body = AgingCustomersResponse), (status = 400, body = ApiError), (status = 401, body = ApiError), (status = 403, body = ApiError), (status = 422, body = ApiError), (status = 500, body = ApiError)))]
pub fn receivables_aging_customers() {}
