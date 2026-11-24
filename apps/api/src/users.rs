use axum::{Extension, Json, extract::State, http::StatusCode};
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use toko_contracts::{EmptyMeta, auth::UserRole, users as contract};
use uuid::Uuid;

use crate::{
    auth::{Session, hash_password},
    error::{Body, Failure, RequestId, RouteId},
};

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    username: String,
    role: String,
    is_active: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<UserRow> for contract::User {
    type Error = ();

    fn try_from(row: UserRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            username: row.username,
            role: role_from_db(&row.role).ok_or(())?,
            is_active: row.is_active,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

fn role_from_db(role: &str) -> Option<UserRole> {
    match role {
        "owner" => Some(UserRole::Owner),
        "operator" => Some(UserRole::Operator),
        "viewer" => Some(UserRole::Viewer),
        _ => None,
    }
}

fn role_to_db(role: UserRole) -> &'static str {
    match role {
        UserRole::Owner => "owner",
        UserRole::Operator => "operator",
        UserRole::Viewer => "viewer",
    }
}

fn map_user(row: UserRow, request: RequestId) -> Result<contract::User, Failure> {
    contract::User::try_from(row).map_err(|_| request.internal())
}

fn validate_username(username: String, request: RequestId) -> Result<String, Failure> {
    let username = username.trim().to_owned();
    if username.is_empty() || username.len() > 128 || username.contains('\0') {
        return Err(request.invalid("username", "Nama pengguna harus 1–128 byte"));
    }
    Ok(username)
}

fn validate_password(password: &str, field: &str, request: RequestId) -> Result<(), Failure> {
    if password.chars().count() < 15 || password.len() > 1024 {
        return Err(request.invalid(
            field,
            "Kata sandi harus minimal 15 karakter dan maksimal 1024 byte",
        ));
    }
    Ok(())
}

async fn hash(password: String, request: RequestId) -> Result<String, Failure> {
    tokio::task::spawn_blocking(move || hash_password(&password))
        .await
        .map_err(|_| request.internal())?
        .map_err(|_| request.internal())
}

fn username_conflict(request: RequestId) -> Failure {
    request.error(
        StatusCode::CONFLICT,
        "USERNAME_TAKEN",
        "Nama pengguna sudah digunakan",
    )
}

fn user_not_found(request: RequestId) -> Failure {
    request.error(
        StatusCode::NOT_FOUND,
        "USER_NOT_FOUND",
        "Pengguna tidak ditemukan",
    )
}

fn owner_required(request: RequestId) -> Failure {
    request.error(
        StatusCode::CONFLICT,
        "LAST_ACTIVE_OWNER",
        "Minimal harus ada satu pemilik aktif",
    )
}

async fn audit(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    action: &str,
    entity: Uuid,
    request: RequestId,
) -> Result<(), Failure> {
    sqlx::query(
        "INSERT INTO audit_events (id, actor_id, action, entity_type, entity_id, request_id)
         VALUES ($1, $2, $3, 'user', $4, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(actor)
    .bind(action)
    .bind(entity)
    .bind(request.0)
    .execute(&mut **tx)
    .await
    .map_err(|_| request.internal())?;
    Ok(())
}

fn user_response(
    row: UserRow,
    request: RequestId,
) -> Result<Json<contract::UserResponse>, Failure> {
    Ok(Json(contract::UserResponse {
        data: map_user(row, request)?,
        meta: EmptyMeta::default(),
    }))
}

pub async fn list(
    State(pool): State<PgPool>,
    request: RequestId,
) -> Result<Json<contract::UserListResponse>, Failure> {
    let rows: Vec<UserRow> = sqlx::query_as(
        "SELECT id, username, role, is_active, created_at, updated_at FROM users ORDER BY username, id",
    )
    .fetch_all(&pool)
    .await
    .map_err(|_| request.internal())?;
    let data = rows
        .into_iter()
        .map(|row| map_user(row, request))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Json(contract::UserListResponse {
        data,
        meta: EmptyMeta::default(),
    }))
}

pub async fn create(
    State(pool): State<PgPool>,
    Extension(session): Extension<Session>,
    request: RequestId,
    Body(input): Body<contract::CreateUser>,
) -> Result<(StatusCode, Json<contract::UserResponse>), Failure> {
    let username = validate_username(input.username, request)?;
    validate_password(&input.password, "password", request)?;
    let password_hash = hash(input.password, request).await?;
    let id = Uuid::new_v4();
    let mut tx = pool.begin().await.map_err(|_| request.internal())?;
    let row: UserRow = match sqlx::query_as(
        "INSERT INTO users (id, username, password_hash, role, must_change_password)
         VALUES ($1, $2, $3, $4, true)
         RETURNING id, username, role, is_active, created_at, updated_at",
    )
    .bind(id)
    .bind(username)
    .bind(password_hash)
    .bind(role_to_db(input.role))
    .fetch_one(&mut *tx)
    .await
    {
        Ok(row) => row,
        Err(error)
            if error
                .as_database_error()
                .is_some_and(|db| db.is_unique_violation()) =>
        {
            return Err(username_conflict(request));
        }
        Err(_) => return Err(request.internal()),
    };
    audit(&mut tx, session.user_id, "user.create", id, request).await?;
    tx.commit().await.map_err(|_| request.internal())?;
    Ok((StatusCode::CREATED, user_response(row, request)?))
}

pub async fn update(
    State(pool): State<PgPool>,
    Extension(session): Extension<Session>,
    RouteId(id): RouteId,
    request: RequestId,
    Body(input): Body<contract::UpdateUser>,
) -> Result<Json<contract::UserResponse>, Failure> {
    let username = validate_username(input.username, request)?;
    if id == session.user_id && !input.is_active {
        return Err(request.error(
            StatusCode::CONFLICT,
            "CANNOT_DEACTIVATE_SELF",
            "Akun yang sedang digunakan tidak dapat dinonaktifkan",
        ));
    }

    let mut tx = pool.begin().await.map_err(|_| request.internal())?;
    sqlx::query("LOCK TABLE users IN SHARE ROW EXCLUSIVE MODE")
        .execute(&mut *tx)
        .await
        .map_err(|_| request.internal())?;
    let current: Option<UserRow> = sqlx::query_as(
        "SELECT id, username, role, is_active, created_at, updated_at FROM users WHERE id = $1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| request.internal())?;
    let current = current.ok_or_else(|| user_not_found(request))?;
    if id == session.user_id && role_to_db(input.role) != current.role {
        return Err(request.error(
            StatusCode::CONFLICT,
            "CANNOT_EDIT_OWN_ACCESS",
            "Role sendiri tidak dapat diubah; ganti kata sandi melalui pengaturan akun",
        ));
    }

    let row: UserRow = match sqlx::query_as(
        "UPDATE users SET username = $2, role = $3, is_active = $4, updated_at = now()
         WHERE id = $1
         RETURNING id, username, role, is_active, created_at, updated_at",
    )
    .bind(id)
    .bind(username)
    .bind(role_to_db(input.role))
    .bind(input.is_active)
    .fetch_one(&mut *tx)
    .await
    {
        Ok(row) => row,
        Err(error)
            if error
                .as_database_error()
                .is_some_and(|db| db.is_unique_violation()) =>
        {
            return Err(username_conflict(request));
        }
        Err(_) => return Err(request.internal()),
    };
    let active_owners: i64 =
        sqlx::query_scalar("SELECT count(*) FROM users WHERE role = 'owner' AND is_active")
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| request.internal())?;
    if active_owners == 0 {
        return Err(owner_required(request));
    }
    if !row.is_active || row.is_active != current.is_active || row.role != current.role {
        sqlx::query(
            "UPDATE sessions SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL",
        )
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| request.internal())?;
    }
    audit(&mut tx, session.user_id, "user.update", id, request).await?;
    tx.commit().await.map_err(|_| request.internal())?;
    user_response(row, request)
}

pub async fn deactivate(
    State(pool): State<PgPool>,
    Extension(session): Extension<Session>,
    RouteId(id): RouteId,
    request: RequestId,
) -> Result<Json<contract::UserResponse>, Failure> {
    if id == session.user_id {
        return Err(request.error(
            StatusCode::CONFLICT,
            "CANNOT_DEACTIVATE_SELF",
            "Akun yang sedang digunakan tidak dapat dinonaktifkan",
        ));
    }
    let mut tx = pool.begin().await.map_err(|_| request.internal())?;
    sqlx::query("LOCK TABLE users IN SHARE ROW EXCLUSIVE MODE")
        .execute(&mut *tx)
        .await
        .map_err(|_| request.internal())?;
    let row: Option<UserRow> = sqlx::query_as(
        "UPDATE users SET is_active = false, updated_at = now() WHERE id = $1
         RETURNING id, username, role, is_active, created_at, updated_at",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| request.internal())?;
    let row = row.ok_or_else(|| user_not_found(request))?;
    let active_owners: i64 =
        sqlx::query_scalar("SELECT count(*) FROM users WHERE role = 'owner' AND is_active")
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| request.internal())?;
    if active_owners == 0 {
        return Err(owner_required(request));
    }
    sqlx::query("UPDATE sessions SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| request.internal())?;
    audit(&mut tx, session.user_id, "user.deactivate", id, request).await?;
    tx.commit().await.map_err(|_| request.internal())?;
    user_response(row, request)
}

pub async fn reset_password(
    State(pool): State<PgPool>,
    Extension(session): Extension<Session>,
    RouteId(id): RouteId,
    request: RequestId,
    Body(input): Body<contract::ResetUserPassword>,
) -> Result<Json<contract::UserResponse>, Failure> {
    validate_password(&input.temporary_password, "temporary_password", request)?;
    if id == session.user_id {
        return Err(request.error(
            StatusCode::CONFLICT,
            "CANNOT_RESET_OWN_PASSWORD",
            "Gunakan halaman pengaturan akun untuk mengganti kata sandi sendiri",
        ));
    }
    let password_hash = hash(input.temporary_password, request).await?;
    let mut tx = pool.begin().await.map_err(|_| request.internal())?;
    let row: Option<UserRow> = sqlx::query_as(
        "UPDATE users SET password_hash = $2, must_change_password = true, updated_at = now()
         WHERE id = $1 RETURNING id, username, role, is_active, created_at, updated_at",
    )
    .bind(id)
    .bind(password_hash)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| request.internal())?;
    let row = row.ok_or_else(|| user_not_found(request))?;
    sqlx::query("UPDATE sessions SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL")
        .bind(id)
        .execute(&mut *tx)
        .await
        .map_err(|_| request.internal())?;
    audit(&mut tx, session.user_id, "user.password_reset", id, request).await?;
    tx.commit().await.map_err(|_| request.internal())?;
    user_response(row, request)
}

pub async fn change_password(
    State(pool): State<PgPool>,
    Extension(session): Extension<Session>,
    request: RequestId,
    Body(input): Body<contract::ChangePassword>,
) -> Result<Json<contract::PasswordChangedResponse>, Failure> {
    if input.current_password.is_empty() || input.current_password.len() > 1024 {
        return Err(request.invalid("current_password", "Kata sandi saat ini tidak valid"));
    }
    validate_password(&input.new_password, "new_password", request)?;
    if input.current_password == input.new_password {
        return Err(request.invalid(
            "new_password",
            "Kata sandi baru harus berbeda dari kata sandi saat ini",
        ));
    }
    let old_hash: Option<String> =
        sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1 AND is_active")
            .bind(session.user_id)
            .fetch_optional(&pool)
            .await
            .map_err(|_| request.internal())?;
    let old_hash = old_hash.ok_or_else(|| user_not_found(request))?;
    let current_password = input.current_password;
    let verified = tokio::task::spawn_blocking({
        let old_hash = old_hash.clone();
        move || {
            use argon2::{
                Argon2,
                password_hash::{PasswordVerifier, phc::PasswordHash},
            };
            PasswordHash::new(&old_hash).is_ok_and(|parsed| {
                Argon2::default()
                    .verify_password(current_password.as_bytes(), &parsed)
                    .is_ok()
            })
        }
    })
    .await
    .map_err(|_| request.internal())?;
    if !verified {
        return Err(request.error(
            StatusCode::UNAUTHORIZED,
            "CURRENT_PASSWORD_INVALID",
            "Kata sandi saat ini salah",
        ));
    }
    let new_hash = hash(input.new_password, request).await?;
    let mut tx = pool.begin().await.map_err(|_| request.internal())?;
    let changed: Option<Uuid> = sqlx::query_scalar(
        "UPDATE users SET password_hash = $3, must_change_password = false, updated_at = now()
         WHERE id = $1 AND password_hash = $2 AND is_active RETURNING id",
    )
    .bind(session.user_id)
    .bind(old_hash)
    .bind(new_hash)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| request.internal())?;
    if changed.is_none() {
        return Err(request.error(
            StatusCode::CONFLICT,
            "PASSWORD_CHANGED_CONCURRENTLY",
            "Kata sandi berubah pada sesi lain. Silakan masuk kembali.",
        ));
    }
    sqlx::query(
        "UPDATE sessions SET revoked_at = now()
         WHERE user_id = $1 AND id <> $2 AND revoked_at IS NULL",
    )
    .bind(session.user_id)
    .bind(session.session_id)
    .execute(&mut *tx)
    .await
    .map_err(|_| request.internal())?;
    audit(
        &mut tx,
        session.user_id,
        "auth.password_change",
        session.user_id,
        request,
    )
    .await?;
    tx.commit().await.map_err(|_| request.internal())?;
    Ok(Json(contract::PasswordChangedResponse {
        data: contract::PasswordChanged { changed: true },
        meta: EmptyMeta::default(),
    }))
}
