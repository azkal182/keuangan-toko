use axum::{
    Extension, Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::de::DeserializeOwned;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction};
use toko_contracts::{
    EmptyMeta,
    sales::{
        CreateSale, PostSale, PostSaleResponse, PostedSale, Sale, SaleItem, SaleItemInput,
        SaleResponse, SaleStatus, UpdateSale,
    },
};
use toko_domain::{
    identity::Role,
    sale::{self, SaleDateError, SaleItem as DomainItem},
};
use uuid::Uuid;

use crate::{
    auth::Session,
    error::{Body, Failure, RequestId, RouteId},
    ledger, payments,
};

#[derive(sqlx::FromRow)]
struct SaleRow {
    id: Uuid,
    customer_id: Uuid,
    effective_date: NaiveDate,
    reference_no: Option<String>,
    backdate_reason: Option<String>,
    status: String,
    total_idr: i64,
    version: i32,
    posted_at: Option<DateTime<Utc>>,
}

#[derive(sqlx::FromRow)]
struct ItemRow {
    id: Uuid,
    description: String,
    quantity: String,
    unit_price_idr: String,
    line_total_idr: String,
}

pub async fn get(
    State(pool): State<PgPool>,
    request: RequestId,
    RouteId(id): RouteId,
) -> Result<Json<SaleResponse>, Failure> {
    let mut tx = ledger::snapshot(&pool, request).await?;
    let data = detail(&mut tx, request, id).await?;
    tx.commit().await.map_err(|_| request.internal())?;
    Ok(Json(SaleResponse {
        data,
        meta: EmptyMeta::default(),
    }))
}

pub async fn create(
    State(pool): State<PgPool>,
    Extension(session): Extension<Session>,
    request: RequestId,
    headers: HeaderMap,
    Body(body): Body<Value>,
) -> Result<(StatusCode, Json<Value>), Failure> {
    let key = ReplayKey::new(request, &headers, "credit-sales:create".into(), &body)?;
    let mut tx = pool.begin().await.map_err(|_| request.internal())?;
    if let Some(response) = key.replay(&mut tx, request, session.user_id).await? {
        return Ok(response);
    }
    let input: CreateSale = decode(request, body)?;
    lock_customers(&mut tx, request, input.customer_id, input.customer_id).await?;
    let (reference, reason) = sale_text(request, input.reference_no, input.backdate_reason)?;
    enforce_date(
        &mut tx,
        request,
        input.effective_date,
        session.role,
        reason.as_deref(),
    )
    .await?;
    let (items, total) = validate_items(request, input.items)?;
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO credit_sales (id, customer_id, effective_date, reference_no, backdate_reason, total_idr, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    ).bind(id).bind(input.customer_id).bind(input.effective_date).bind(reference).bind(reason)
        .bind(total).bind(session.user_id).execute(&mut *tx).await.map_err(|_| request.internal())?;
    replace_items(&mut tx, request, id, items).await?;
    let data = detail(&mut tx, request, id).await?;
    let body = serde_json::to_value(SaleResponse {
        data,
        meta: EmptyMeta::default(),
    })
    .map_err(|_| request.internal())?;
    key.commit(tx, request, session.user_id, StatusCode::CREATED, body)
        .await
}

pub async fn update(
    State(pool): State<PgPool>,
    Extension(session): Extension<Session>,
    request: RequestId,
    RouteId(id): RouteId,
    Body(input): Body<UpdateSale>,
) -> Result<Json<SaleResponse>, Failure> {
    let mut tx = pool.begin().await.map_err(|_| request.internal())?;
    let before = row(&mut tx, request, id, false).await?;
    require_draft(request, &before, input.expected_version, before.customer_id)?;
    let customer = input.customer_id.unwrap_or(before.customer_id);
    lock_customers(&mut tx, request, before.customer_id, customer).await?;
    let current = row(&mut tx, request, id, true).await?;
    require_draft(
        request,
        &current,
        input.expected_version,
        before.customer_id,
    )?;
    let effective = input.effective_date.unwrap_or(current.effective_date);
    let (reference, reason) = sale_text(
        request,
        input.reference_no.unwrap_or(current.reference_no),
        input.backdate_reason.unwrap_or(current.backdate_reason),
    )?;
    enforce_date(&mut tx, request, effective, session.role, reason.as_deref()).await?;
    let mut total = current.total_idr;
    if let Some(items) = input.items {
        let (items, replacement_total) = validate_items(request, items)?;
        replace_items(&mut tx, request, id, items).await?;
        total = replacement_total;
    }
    sqlx::query(
        "UPDATE credit_sales SET customer_id = $2, effective_date = $3, reference_no = $4, backdate_reason = $5,
         total_idr = $6, version = version + 1 WHERE id = $1",
    ).bind(id).bind(customer).bind(effective).bind(reference).bind(reason).bind(total)
        .execute(&mut *tx).await.map_err(|_| request.internal())?;
    let data = detail(&mut tx, request, id).await?;
    tx.commit().await.map_err(|_| request.internal())?;
    Ok(Json(SaleResponse {
        data,
        meta: EmptyMeta::default(),
    }))
}

pub async fn post(
    State(pool): State<PgPool>,
    Extension(session): Extension<Session>,
    request: RequestId,
    RouteId(id): RouteId,
    headers: HeaderMap,
    Body(body): Body<Value>,
) -> Result<(StatusCode, Json<Value>), Failure> {
    let key = ReplayKey::new(request, &headers, format!("credit-sales:{id}:post"), &body)?;
    let mut tx = pool.begin().await.map_err(|_| request.internal())?;
    if let Some(response) = key.replay(&mut tx, request, session.user_id).await? {
        return Ok(response);
    }
    let input: PostSale = decode(request, body)?;
    let before = row(&mut tx, request, id, false).await?;
    require_draft(request, &before, input.expected_version, before.customer_id)?;
    lock_customers(&mut tx, request, before.customer_id, before.customer_id).await?;
    let current = row(&mut tx, request, id, true).await?;
    require_draft(
        request,
        &current,
        input.expected_version,
        before.customer_id,
    )?;
    let backdated = enforce_date(
        &mut tx,
        request,
        current.effective_date,
        session.role,
        current.backdate_reason.as_deref(),
    )
    .await?;
    let posted_at: DateTime<Utc> = sqlx::query_scalar(
        "UPDATE credit_sales SET status = 'posted', posted_by = $2, posted_at = statement_timestamp(),
         version = version + 1 WHERE id = $1 RETURNING posted_at",
    ).bind(id).bind(session.user_id).fetch_one(&mut *tx).await.map_err(|_| request.internal())?;
    let entry = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO receivable_entries (id, customer_id, entry_type, source_id, effective_date, debit_idr, created_by, created_at)
         VALUES ($1, $2, 'credit_sale', $3, $4, $5, $6, $7)",
    ).bind(entry).bind(current.customer_id).bind(id).bind(current.effective_date).bind(current.total_idr)
        .bind(session.user_id).bind(posted_at).execute(&mut *tx).await.map_err(|_| request.internal())?;
    let audit_reason = current.backdate_reason.filter(|_| backdated);
    sqlx::query(
        "INSERT INTO audit_events (id, actor_id, action, entity_type, entity_id, reason, request_id, metadata)
         VALUES ($1, $2, 'credit_sale.post', 'credit_sale', $3, $4, $5, jsonb_build_object('ledger_entry_id', $6::uuid))",
    ).bind(Uuid::new_v4()).bind(session.user_id).bind(id).bind(audit_reason).bind(request.0).bind(entry)
        .execute(&mut *tx).await.map_err(|_| request.internal())?;
    payments::auto_apply_credits(&mut tx, current.customer_id, session.user_id, request).await?;
    let sale = detail(&mut tx, request, id).await?;
    let balance = ledger::summary(&mut tx, current.customer_id, request).await?;
    let body = serde_json::to_value(PostSaleResponse {
        data: PostedSale { sale, balance },
        meta: EmptyMeta::default(),
    })
    .map_err(|_| request.internal())?;
    key.commit(tx, request, session.user_id, StatusCode::OK, body)
        .await
}

async fn row(
    tx: &mut Transaction<'_, Postgres>,
    request: RequestId,
    id: Uuid,
    lock: bool,
) -> Result<SaleRow, Failure> {
    let sql = if lock {
        "SELECT id, customer_id, effective_date, reference_no, backdate_reason, status, total_idr, version, posted_at
         FROM credit_sales WHERE id = $1 FOR UPDATE"
    } else {
        "SELECT id, customer_id, effective_date, reference_no, backdate_reason, status, total_idr, version, posted_at
         FROM credit_sales WHERE id = $1"
    };
    sqlx::query_as(sql)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|_| request.internal())?
        .ok_or_else(|| {
            request.error(
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                "Penjualan kredit tidak ditemukan",
            )
        })
}

async fn detail(
    tx: &mut Transaction<'_, Postgres>,
    request: RequestId,
    id: Uuid,
) -> Result<Sale, Failure> {
    let row = row(tx, request, id, false).await?;
    let items: Vec<ItemRow> = sqlx::query_as(
        "SELECT id, description, quantity::text, unit_price_idr::text, line_total_idr::text
         FROM credit_sale_items WHERE sale_id = $1 ORDER BY ordinal",
    )
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(|_| request.internal())?;
    let ledger_entry_id = sqlx::query_scalar(
        "SELECT id FROM receivable_entries WHERE entry_type = 'credit_sale' AND source_id = $1 AND customer_id = $2",
    ).bind(id).bind(row.customer_id).fetch_optional(&mut **tx).await.map_err(|_| request.internal())?;
    let status = match row.status.as_str() {
        "draft" => SaleStatus::Draft,
        "posted" => SaleStatus::Posted,
        "reversed" => SaleStatus::Reversed,
        _ => return Err(request.internal()),
    };
    Ok(Sale {
        id: row.id,
        customer_id: row.customer_id,
        effective_date: row.effective_date,
        reference_no: row.reference_no,
        backdate_reason: row.backdate_reason,
        status,
        total_idr: row.total_idr.to_string(),
        version: row.version,
        posted_at: row.posted_at,
        ledger_entry_id,
        items: items
            .into_iter()
            .map(|item| SaleItem {
                id: item.id,
                description: item.description,
                quantity: item.quantity,
                unit_price_idr: item.unit_price_idr,
                line_total_idr: item.line_total_idr,
            })
            .collect(),
    })
}

async fn lock_customers(
    tx: &mut Transaction<'_, Postgres>,
    request: RequestId,
    current: Uuid,
    target: Uuid,
) -> Result<(), Failure> {
    // Merge acquires the same customer locks before relinking sales and ledger entries.
    let rows: Vec<(Uuid, bool)> = sqlx::query_as(
        "SELECT id, is_active FROM customers WHERE id IN ($1, $2) ORDER BY id FOR UPDATE",
    )
    .bind(current)
    .bind(target)
    .fetch_all(&mut **tx)
    .await
    .map_err(|_| request.internal())?;
    match rows.iter().find(|(id, _)| *id == target) {
        Some((_, true)) => Ok(()),
        Some((_, false)) => Err(request.invalid("customer_id", "Pelanggan harus aktif")),
        None => Err(request.missing_customer()),
    }
}

fn require_draft(
    request: RequestId,
    current: &SaleRow,
    expected: i32,
    original_customer: Uuid,
) -> Result<(), Failure> {
    if current.status != "draft" {
        return Err(request.error(
            StatusCode::CONFLICT,
            "SALE_NOT_DRAFT",
            "Penjualan sudah diposting atau dibalik",
        ));
    }
    if expected <= 0 {
        return Err(request.invalid("expected_version", "Versi harus positif"));
    }
    if current.version != expected || current.customer_id != original_customer {
        return Err(request.field(
            StatusCode::CONFLICT,
            "VERSION_CONFLICT",
            "expected_version",
            "Draft telah berubah. Muat ulang penjualan.",
        ));
    }
    Ok(())
}

async fn enforce_date(
    tx: &mut Transaction<'_, Postgres>,
    request: RequestId,
    effective: NaiveDate,
    role: Role,
    reason: Option<&str>,
) -> Result<bool, Failure> {
    let today: NaiveDate =
        sqlx::query_scalar("SELECT (clock_timestamp() AT TIME ZONE 'Asia/Jakarta')::date")
            .fetch_one(&mut **tx)
            .await
            .map_err(|_| request.internal())?;
    sale::validate_date(effective, today, role, reason).map_err(|error| match error {
        SaleDateError::Future => {
            request.invalid("effective_date", "Tanggal usaha tidak boleh di masa depan")
        }
        SaleDateError::OwnerRequired => request.field(
            StatusCode::FORBIDDEN,
            "BACKDATE_FORBIDDEN",
            "effective_date",
            "Hanya Owner yang dapat memakai tanggal lampau",
        ),
        SaleDateError::ReasonRequired => {
            request.invalid("backdate_reason", "Alasan wajib diisi untuk tanggal lampau")
        }
    })?;
    Ok(effective < today)
}

fn sale_text(
    request: RequestId,
    reference: Option<String>,
    reason: Option<String>,
) -> Result<(Option<String>, Option<String>), Failure> {
    fn trim(
        request: RequestId,
        field: &str,
        value: Option<String>,
        max: usize,
    ) -> Result<Option<String>, Failure> {
        let value = value
            .as_deref()
            .map(str::trim)
            .filter(|text| !text.is_empty());
        if value.is_some_and(|text| text.contains('\0') || text.chars().count() > max) {
            return Err(request.invalid(
                field,
                "Teks terlalu panjang atau mengandung karakter yang tidak valid",
            ));
        }
        Ok(value.map(str::to_owned))
    }
    Ok((
        trim(request, "reference_no", reference, 256)?,
        trim(request, "backdate_reason", reason, 1000)?,
    ))
}

fn validate_items(
    request: RequestId,
    input: Vec<SaleItemInput>,
) -> Result<(Vec<DomainItem>, i64), Failure> {
    if input.is_empty() {
        return Err(request.invalid("items", "Tambahkan minimal satu item"));
    }
    let items = input
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            DomainItem::new(&item.description, &item.quantity, &item.unit_price_idr).map_err(
                |(field, message)| request.invalid(&format!("items.{index}.{field}"), message),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let total = sale::sale_total(&items)
        .ok_or_else(|| request.invalid("items", "Total penjualan melebihi batas rupiah"))?;
    Ok((items, total))
}

async fn replace_items(
    tx: &mut Transaction<'_, Postgres>,
    request: RequestId,
    id: Uuid,
    items: Vec<DomainItem>,
) -> Result<(), Failure> {
    sqlx::query("DELETE FROM credit_sale_items WHERE sale_id = $1")
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(|_| request.internal())?;
    for (index, item) in items.into_iter().enumerate() {
        let ordinal = i32::try_from(index + 1)
            .map_err(|_| request.invalid("items", "Jumlah item melebihi batas"))?;
        sqlx::query(
            "INSERT INTO credit_sale_items (id, sale_id, ordinal, description, quantity, unit_price_idr, line_total_idr)
             VALUES ($1, $2, $3, $4, $5::bigint / 1000.0, $6, $7)",
        ).bind(Uuid::new_v4()).bind(id).bind(ordinal).bind(item.description).bind(item.quantity_milli)
            .bind(item.unit_price_idr).bind(item.line_total_idr).execute(&mut **tx).await.map_err(|_| request.internal())?;
    }
    Ok(())
}

fn decode<T: DeserializeOwned>(request: RequestId, body: Value) -> Result<T, Failure> {
    serde_json::from_value(body).map_err(|_| {
        request.error(
            StatusCode::BAD_REQUEST,
            "INVALID_BODY",
            "Isi permintaan tidak valid",
        )
    })
}

struct ReplayKey {
    route: String,
    key: String,
    hash: String,
}

impl ReplayKey {
    fn new(
        request: RequestId,
        headers: &HeaderMap,
        route: String,
        body: &Value,
    ) -> Result<Self, Failure> {
        let mut values = headers.get_all("idempotency-key").iter();
        let key = values
            .next()
            .and_then(|value| value.to_str().ok())
            .filter(|value| {
                !value.is_empty()
                    && value.len() <= 128
                    && value.bytes().all(|byte| byte.is_ascii_graphic())
            })
            .filter(|_| values.next().is_none())
            .ok_or_else(|| {
                request.field(
                    StatusCode::BAD_REQUEST,
                    "INVALID_IDEMPOTENCY_KEY",
                    "Idempotency-Key",
                    "Kunci idempotensi wajib dan harus valid",
                )
            })?;
        let hash = hex::encode(Sha256::digest(
            serde_json::to_vec(body).map_err(|_| request.internal())?,
        ));
        Ok(Self {
            route,
            key: key.to_owned(),
            hash,
        })
    }

    async fn replay(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        request: RequestId,
        actor: Uuid,
    ) -> Result<Option<(StatusCode, Json<Value>)>, Failure> {
        // Lock this actor/route/key before reading or mutating. A collision only serializes unrelated requests.
        let lock = format!("{actor}:{}:{}", self.route, self.key);
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(lock)
            .execute(&mut **tx)
            .await
            .map_err(|_| request.internal())?;
        let saved: Option<(String, i32, Value)> = sqlx::query_as(
            "SELECT request_hash, response_status, response_body FROM idempotency_records
             WHERE actor_id = $1 AND route_key = $2 AND idempotency_key = $3",
        )
        .bind(actor)
        .bind(&self.route)
        .bind(&self.key)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|_| request.internal())?;
        let Some((hash, status, body)) = saved else {
            return Ok(None);
        };
        if hash != self.hash {
            return Err(request.error(
                StatusCode::CONFLICT,
                "IDEMPOTENCY_CONFLICT",
                "Kunci sudah digunakan dengan isi permintaan berbeda",
            ));
        }
        let status = u16::try_from(status)
            .ok()
            .and_then(|value| StatusCode::from_u16(value).ok())
            .ok_or_else(|| request.internal())?;
        Ok(Some((status, Json(body))))
    }

    async fn commit(
        self,
        mut tx: Transaction<'_, Postgres>,
        request: RequestId,
        actor: Uuid,
        status: StatusCode,
        body: Value,
    ) -> Result<(StatusCode, Json<Value>), Failure> {
        sqlx::query(
            "INSERT INTO idempotency_records (id, actor_id, route_key, idempotency_key, request_hash, response_status, response_body)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        ).bind(Uuid::new_v4()).bind(actor).bind(self.route).bind(self.key).bind(self.hash)
            .bind(i32::from(status.as_u16())).bind(&body).execute(&mut *tx).await.map_err(|_| request.internal())?;
        tx.commit().await.map_err(|_| request.internal())?;
        Ok((status, Json(body)))
    }
}
