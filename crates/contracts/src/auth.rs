use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{
    Modify, ToSchema,
    openapi::{
        self,
        security::{ApiKey, ApiKeyValue, SecurityScheme},
    },
};
use uuid::Uuid;

use crate::{ApiError, EmptyMeta};

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    Owner,
    Operator,
    Viewer,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CurrentUser {
    pub id: Uuid,
    pub username: String,
    pub role: UserRole,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SessionDetails {
    pub user: CurrentUser,
    pub expires_at: DateTime<Utc>,
    pub csrf_token: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SessionResponse {
    pub data: SessionDetails,
    pub meta: EmptyMeta,
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    operation_id = "getSession",
    security(("session" = [])),
    responses(
        (status = 200, description = "Sesi aktif", body = SessionResponse),
        (status = 401, description = "Sesi diperlukan", body = ApiError),
        (status = 500, description = "Kegagalan internal", body = ApiError)
    )
)]
pub fn get_me() {}

pub(crate) struct SessionSecurity;

impl Modify for SessionSecurity {
    fn modify(&self, api: &mut openapi::OpenApi) {
        if let Some(components) = api.components.as_mut() {
            components.add_security_scheme(
                "session",
                SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new("__Host-session"))),
            );
        }
    }
}

#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LogoutData {
    pub logged_out: bool,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LogoutResponse {
    pub data: LogoutData,
    pub meta: EmptyMeta,
}

#[utoipa::path(
    post, path = "/api/v1/auth/login", request_body = LoginRequest,
    responses(
        (status = 200, description = "Sesi dibuat", body = SessionResponse),
        (status = 400, description = "Isi permintaan tidak valid", body = ApiError),
        (status = 401, description = "Kredensial ditolak", body = ApiError),
        (status = 403, description = "Akses ditolak", body = ApiError),
        (status = 422, description = "Validasi gagal", body = ApiError),
        (status = 429, description = "Percobaan dibatasi", body = ApiError),
        (status = 500, description = "Kegagalan internal", body = ApiError)
    )
)]
pub fn login() {}

#[utoipa::path(
    post, path = "/api/v1/auth/logout", security(("session" = [])),
    params(("X-CSRF-Token" = String, Header)),
    responses(
        (status = 200, description = "Sesi dicabut", body = LogoutResponse),
        (status = 401, description = "Sesi diperlukan", body = ApiError),
        (status = 403, description = "Token CSRF tidak valid", body = ApiError),
        (status = 500, description = "Kegagalan internal", body = ApiError)
    )
)]
pub fn logout() {}
