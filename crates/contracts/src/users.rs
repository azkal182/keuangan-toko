use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{ApiError, EmptyMeta, auth::UserRole};

#[derive(Debug, Serialize, ToSchema)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub role: UserRole,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserListResponse {
    pub data: Vec<User>,
    pub meta: EmptyMeta,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserResponse {
    pub data: User,
    pub meta: EmptyMeta,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateUser {
    pub username: String,
    pub password: String,
    pub role: UserRole,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateUser {
    pub username: String,
    pub role: UserRole,
    pub is_active: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ResetUserPassword {
    pub temporary_password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangePassword {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PasswordChangedResponse {
    pub data: PasswordChanged,
    pub meta: EmptyMeta,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PasswordChanged {
    pub changed: bool,
}

#[utoipa::path(
    get, path = "/api/v1/users", security(("session" = [])),
    responses((status = 200, description = "Daftar pengguna", body = UserListResponse),
        (status = 401, description = "Sesi diperlukan", body = ApiError),
        (status = 403, description = "Akses ditolak", body = ApiError),
        (status = 500, description = "Kegagalan internal", body = ApiError))
)]
pub fn list_users() {}

#[utoipa::path(
    post, path = "/api/v1/users", request_body = CreateUser, security(("session" = [])),
    params(("X-CSRF-Token" = String, Header)),
    responses((status = 201, description = "Pengguna dibuat", body = UserResponse),
        (status = 400, description = "Permintaan tidak valid", body = ApiError),
        (status = 401, description = "Sesi diperlukan", body = ApiError),
        (status = 403, description = "Akses ditolak", body = ApiError),
        (status = 409, description = "Nama pengguna sudah digunakan", body = ApiError),
        (status = 422, description = "Validasi gagal", body = ApiError),
        (status = 500, description = "Kegagalan internal", body = ApiError))
)]
pub fn create_user() {}

#[utoipa::path(
    patch, path = "/api/v1/users/{id}", request_body = UpdateUser, security(("session" = [])),
    params(("id" = Uuid, Path, description = "ID pengguna"), ("X-CSRF-Token" = String, Header)),
    responses((status = 200, description = "Pengguna diperbarui", body = UserResponse),
        (status = 400, description = "Permintaan tidak valid", body = ApiError),
        (status = 401, description = "Sesi diperlukan", body = ApiError),
        (status = 403, description = "Akses ditolak", body = ApiError),
        (status = 404, description = "Pengguna tidak ditemukan", body = ApiError),
        (status = 409, description = "Nama pengguna sudah digunakan atau pemilik terakhir", body = ApiError),
        (status = 422, description = "Validasi gagal", body = ApiError),
        (status = 500, description = "Kegagalan internal", body = ApiError))
)]
pub fn update_user() {}

#[utoipa::path(
    delete, path = "/api/v1/users/{id}", security(("session" = [])),
    params(("id" = Uuid, Path, description = "ID pengguna"), ("X-CSRF-Token" = String, Header)),
    responses((status = 200, description = "Pengguna dinonaktifkan", body = UserResponse),
        (status = 400, description = "Permintaan tidak valid", body = ApiError),
        (status = 401, description = "Sesi diperlukan", body = ApiError),
        (status = 403, description = "Akses ditolak", body = ApiError),
        (status = 404, description = "Pengguna tidak ditemukan", body = ApiError),
        (status = 409, description = "Tidak dapat menonaktifkan pemilik terakhir", body = ApiError),
        (status = 500, description = "Kegagalan internal", body = ApiError))
)]
pub fn deactivate_user() {}

#[utoipa::path(
    post, path = "/api/v1/users/{id}/password-reset", request_body = ResetUserPassword,
    security(("session" = [])), params(("id" = Uuid, Path, description = "ID pengguna"), ("X-CSRF-Token" = String, Header)),
    responses((status = 200, description = "Kata sandi sementara ditetapkan", body = UserResponse),
        (status = 400, description = "Permintaan tidak valid", body = ApiError),
        (status = 401, description = "Sesi diperlukan", body = ApiError),
        (status = 403, description = "Akses ditolak", body = ApiError),
        (status = 404, description = "Pengguna tidak ditemukan", body = ApiError),
        (status = 409, description = "Tidak dapat mereset akun sendiri", body = ApiError),
        (status = 422, description = "Validasi gagal", body = ApiError),
        (status = 500, description = "Kegagalan internal", body = ApiError))
)]
pub fn reset_user_password() {}

#[utoipa::path(
    post, path = "/api/v1/auth/password", request_body = ChangePassword,
    security(("session" = [])), params(("X-CSRF-Token" = String, Header)),
    responses((status = 200, description = "Kata sandi diubah", body = PasswordChangedResponse),
        (status = 400, description = "Permintaan tidak valid", body = ApiError),
        (status = 401, description = "Sesi diperlukan atau kata sandi saat ini salah", body = ApiError),
        (status = 403, description = "Token CSRF tidak valid", body = ApiError),
        (status = 422, description = "Validasi gagal", body = ApiError),
        (status = 500, description = "Kegagalan internal", body = ApiError))
)]
pub fn change_password() {}
