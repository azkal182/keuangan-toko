use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::error::{Body, Failure, RequestId};
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, PasswordVerifier, phc::PasswordHash},
};
use axum::{
    Extension, Json,
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use subtle::ConstantTimeEq;
use tokio::sync::Semaphore;
use toko_application::{Permission, Role};
use toko_contracts::{
    EmptyMeta,
    auth::{
        CurrentUser, LoginRequest, LogoutData, LogoutResponse, SessionDetails, SessionResponse,
        UserRole,
    },
};
use uuid::Uuid;

#[derive(Clone)]
pub struct AccessPolicy {
    pool: PgPool,
    permission: Permission,
}

impl AccessPolicy {
    pub fn new(pool: PgPool, permission: Permission) -> Self {
        Self { pool, permission }
    }
}

#[derive(sqlx::FromRow)]
struct SessionRow {
    id: Uuid,
    session_id: Uuid,
    username: String,
    role: String,
    expires_at: DateTime<Utc>,
    needs_refresh: bool,
}

#[derive(Clone)]
pub struct Session {
    pub user_id: Uuid,
    session_id: Uuid,
    pub role: Role,
    username: String,
    expires_at: DateTime<Utc>,
    csrf_token: String,
}

pub async fn request_context(mut request: Request, next: Next) -> Response {
    let request_id = RequestId(Uuid::new_v4());
    request.extensions_mut().insert(request_id);
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(&request_id.0.to_string()).expect("UUID is a valid header value"),
    );
    response
}

pub async fn require_access(
    State(policy): State<AccessPolicy>,
    request_id: RequestId,
    mut request: Request,
    next: Next,
) -> Response {
    match authenticate(&policy, request.headers(), request.method(), request_id).await {
        Ok(session) => {
            request.extensions_mut().insert(session);
            next.run(request).await
        }
        Err(error) => error.into_response(),
    }
}

async fn authenticate(
    policy: &AccessPolicy,
    headers: &HeaderMap,
    method: &Method,
    request: RequestId,
) -> Result<Session, Failure> {
    let token = session_cookie(headers).ok_or_else(|| unauthenticated(request))?;
    let token_hash = Sha256::digest(token.as_bytes());
    let row: SessionRow = sqlx::query_as(
        "SELECT u.id, u.username, u.role, s.expires_at, s.id AS session_id,
                s.last_seen_at <= statement_timestamp() - interval '1 minute' AS needs_refresh
         FROM sessions s JOIN users u ON u.id = s.user_id
         WHERE s.token_hash = $1 AND s.revoked_at IS NULL AND u.is_active
           AND s.expires_at > statement_timestamp()
           AND s.last_seen_at > statement_timestamp() - interval '30 minutes'",
    )
    .bind(&token_hash[..])
    .fetch_optional(&policy.pool)
    .await
    .map_err(|_| request.internal())?
    .ok_or_else(|| unauthenticated(request))?;

    let role = Role::try_from(row.role.as_str()).map_err(|_| forbidden(request))?;
    if !policy.permission.allows(role) {
        return Err(forbidden(request));
    }

    let csrf_token = csrf_token(token);
    if !matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
        && !valid_csrf(headers, &csrf_token)
    {
        return Err(request.error(
            StatusCode::FORBIDDEN,
            "CSRF_FAILED",
            "Token CSRF tidak valid",
        ));
    }

    // Session activity is security metadata. Refresh only after permission and CSRF checks.
    // Skip writes on closely spaced requests and recheck revocation on each lookup.
    if row.needs_refresh {
        sqlx::query(
            "UPDATE sessions SET last_seen_at = statement_timestamp()
                 WHERE id = $1 AND revoked_at IS NULL
                   AND last_seen_at <= statement_timestamp() - interval '1 minute'",
        )
        .bind(row.session_id)
        .execute(&policy.pool)
        .await
        .map_err(|_| request.internal())?;
    }
    Ok(Session {
        user_id: row.id,
        session_id: row.session_id,
        username: row.username,
        role,
        expires_at: row.expires_at,
        csrf_token,
    })
}

fn session_cookie(headers: &HeaderMap) -> Option<&str> {
    let mut token = None;
    for header in headers.get_all(header::COOKIE) {
        for cookie in header.to_str().ok()?.split(';') {
            let Some((name, value)) = cookie.trim().split_once('=') else {
                continue;
            };
            if name == "__Host-session" {
                if token.is_some()
                    || value.len() != 64
                    || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
                {
                    return None;
                }
                token = Some(value);
            }
        }
    }
    token
}

fn csrf_token(session_token: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"keuangan-toko:csrf:v1\0");
    digest.update(session_token.as_bytes());
    hex::encode(digest.finalize())
}

fn valid_csrf(headers: &HeaderMap, expected: &str) -> bool {
    let mut tokens = headers.get_all("x-csrf-token").iter();
    let Some(token) = tokens.next() else {
        return false;
    };
    tokens.next().is_none() && bool::from(token.as_bytes().ct_eq(expected.as_bytes()))
}

pub async fn get_me(Extension(session): Extension<Session>) -> Json<SessionResponse> {
    Json(session_response(session))
}

fn session_response(session: Session) -> SessionResponse {
    let role = match session.role {
        Role::Owner => UserRole::Owner,
        Role::Operator => UserRole::Operator,
        Role::Viewer => UserRole::Viewer,
    };
    SessionResponse {
        data: SessionDetails {
            user: CurrentUser {
                id: session.user_id,
                username: session.username,
                role,
            },
            expires_at: session.expires_at,
            csrf_token: session.csrf_token,
        },
        meta: EmptyMeta::default(),
    }
}

fn unauthenticated(request: RequestId) -> Failure {
    request.error(
        StatusCode::UNAUTHORIZED,
        "UNAUTHENTICATED",
        "Sesi tidak aktif",
    )
}

fn forbidden(request: RequestId) -> Failure {
    request.error(StatusCode::FORBIDDEN, "FORBIDDEN", "Akses ditolak")
}

#[derive(Clone)]
pub struct LoginState {
    pool: PgPool,
    rate: Arc<Mutex<LoginWindow>>,
    jobs: Arc<Semaphore>,
}

struct LoginWindow {
    started: Instant,
    total: usize,
    accounts: HashMap<String, usize>,
}

impl LoginState {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            rate: Arc::new(Mutex::new(LoginWindow::new())),
            jobs: Arc::new(Semaphore::new(4)),
        }
    }

    fn allow_attempt(&self, username: &str, request: RequestId) -> Result<(), Failure> {
        let mut rate = self.rate.lock().map_err(|_| request.internal())?;
        if rate.started.elapsed() >= Duration::from_secs(60) {
            *rate = LoginWindow::new();
        }
        if rate.total >= 60 {
            return Err(rate_limited(request));
        }
        let attempts = rate.accounts.entry(username.to_owned()).or_default();
        if *attempts >= 8 {
            return Err(rate_limited(request));
        }
        *attempts += 1;
        rate.total += 1;
        Ok(())
    }
}

impl LoginWindow {
    fn new() -> Self {
        Self {
            started: Instant::now(),
            total: 0,
            accounts: HashMap::new(),
        }
    }
}

fn rate_limited(request: RequestId) -> Failure {
    request.error(
        StatusCode::TOO_MANY_REQUESTS,
        "RATE_LIMITED",
        "Terlalu banyak percobaan. Coba lagi sebentar.",
    )
}

#[derive(sqlx::FromRow)]
struct Account {
    id: Uuid,
    username: String,
    password_hash: String,
    role: String,
    is_active: bool,
}

pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|hash| hash.to_string())
}

// A syntactically valid Argon2id digest for the same work on unknown accounts.
const DUMMY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$c29tZS1yYW5kb20tc2FsdA$AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

pub async fn login(
    State(state): State<LoginState>,
    request: RequestId,
    headers: HeaderMap,
    Body(input): Body<LoginRequest>,
) -> Result<(HeaderMap, Json<SessionResponse>), Failure> {
    if headers
        .get("sec-fetch-site")
        .is_some_and(|value| matches!(value.as_bytes(), b"cross-site" | b"same-site"))
    {
        return Err(forbidden(request));
    }
    let username = input.username.trim();
    if username.is_empty() || username.len() > 128 || username.contains(' ') {
        return Err(request.invalid("username", "Nama pengguna tidak valid"));
    }
    if input.password.is_empty() || input.password.len() > 1024 {
        return Err(request.invalid("password", "Kata sandi tidak valid"));
    }
    state.allow_attempt(username, request)?;
    let permit = state
        .jobs
        .clone()
        .try_acquire_owned()
        .map_err(|_| rate_limited(request))?;
    let account: Option<Account> = sqlx::query_as(
        "SELECT id, username, password_hash, role, is_active FROM users WHERE username = $1",
    )
    .bind(username)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| request.internal())?;
    let password = input.password;
    let account = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let hash = account
            .as_ref()
            .map_or(DUMMY_HASH, |user| &user.password_hash);
        let hash = PasswordHash::new(hash)
            .or_else(|_| PasswordHash::new(DUMMY_HASH))
            .expect("valid dummy hash");
        let valid = Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_ok();
        account.filter(|user| valid && user.is_active)
    })
    .await
    .map_err(|_| request.internal())?
    .ok_or_else(|| {
        request.error(
            StatusCode::UNAUTHORIZED,
            "INVALID_CREDENTIALS",
            "Nama pengguna atau kata sandi salah",
        )
    })?;
    let role = Role::try_from(account.role.as_str()).map_err(|_| request.internal())?;
    let mut random = [0u8; 32];
    getrandom::fill(&mut random).map_err(|_| request.internal())?;
    let token = hex::encode(random);
    let token_hash = Sha256::digest(token.as_bytes());
    let session_id = Uuid::new_v4();
    let mut tx = state.pool.begin().await.map_err(|_| request.internal())?;
    let expires_at: DateTime<Utc> = sqlx::query_scalar(
        "INSERT INTO sessions (id, user_id, token_hash, expires_at)
         SELECT $1, id, $3, now() + interval '12 hours' FROM users
         WHERE id = $2 AND is_active AND password_hash = $4 AND role = $5
         FOR SHARE RETURNING expires_at",
    )
    .bind(session_id)
    .bind(account.id)
    .bind(&token_hash[..])
    .bind(&account.password_hash)
    .bind(&account.role)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| request.internal())?
    .ok_or_else(|| unauthenticated(request))?;
    if let Some(old_token) = session_cookie(&headers) {
        let old_hash = Sha256::digest(old_token.as_bytes());
        sqlx::query(
            "UPDATE sessions SET revoked_at = now() WHERE token_hash = $1 AND revoked_at IS NULL",
        )
        .bind(&old_hash[..])
        .execute(&mut *tx)
        .await
        .map_err(|_| request.internal())?;
    }
    sqlx::query(
        "INSERT INTO audit_events (id, actor_id, action, entity_type, entity_id, request_id)
         VALUES ($1, $2, 'auth.login', 'session', $3, $4)",
    )
    .bind(Uuid::new_v4())
    .bind(account.id)
    .bind(session_id)
    .bind(request.0)
    .execute(&mut *tx)
    .await
    .map_err(|_| request.internal())?;
    tx.commit().await.map_err(|_| request.internal())?;

    let cookie =
        format!("__Host-session={token}; Secure; HttpOnly; SameSite=Strict; Path=/; Max-Age=43200");
    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie).map_err(|_| request.internal())?,
    );
    Ok((
        headers,
        Json(session_response(Session {
            user_id: account.id,
            session_id,
            role,
            username: account.username,
            expires_at,
            csrf_token: csrf_token(&token),
        })),
    ))
}

pub async fn logout(
    State(pool): State<PgPool>,
    Extension(session): Extension<Session>,
    request: RequestId,
) -> Result<(HeaderMap, Json<LogoutResponse>), Failure> {
    sqlx::query("UPDATE sessions SET revoked_at = now() WHERE id = $1 AND revoked_at IS NULL")
        .bind(session.session_id)
        .execute(&pool)
        .await
        .map_err(|_| request.internal())?;
    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_static(
            "__Host-session=; Secure; HttpOnly; SameSite=Strict; Path=/; Max-Age=0",
        ),
    );
    Ok((
        headers,
        Json(LogoutResponse {
            data: LogoutData { logged_out: true },
            meta: EmptyMeta::default(),
        }),
    ))
}
