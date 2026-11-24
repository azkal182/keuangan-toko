use axum::{Extension, Json, extract::State, http::StatusCode};
use sqlx::{PgPool, Postgres, Transaction};
use toko_contracts::{
    EmptyMeta, PageMeta,
    customers::{
        CreateCustomer, Customer, CustomerDetail, CustomerDetailResponse, CustomerList,
        CustomerListResponse, CustomerResponse, CustomerSummary, ListCustomersQuery, MergeCustomer,
        UpdateCustomer,
    },
};
use toko_domain::customer::{CustomerIdentity, fold_customer_code};
use uuid::Uuid;

use crate::{
    auth::Session,
    error::{Body, Failure, Params, RequestId, RouteId},
    ledger, reports,
};

#[derive(sqlx::FromRow)]
struct CustomerRow {
    id: Uuid,
    code: Option<String>,
    name: String,
    address: Option<String>,
    is_active: bool,
    version: i32,
}

impl From<CustomerRow> for Customer {
    fn from(row: CustomerRow) -> Self {
        Self {
            id: row.id,
            code: row.code,
            name: row.name,
            address: row.address,
            is_active: row.is_active,
            version: row.version,
        }
    }
}

pub async fn list(
    State(pool): State<PgPool>,
    request: RequestId,
    Params(query): Params<ListCustomersQuery>,
) -> Result<Json<CustomerListResponse>, Failure> {
    let (limit, cursor) = pagination(request, query.limit, query.cursor.as_deref())?;
    let search = query.search.as_deref().unwrap_or_default().trim();
    if search.chars().count() > 256 {
        return Err(request.invalid("search", "Pencarian maksimal 256 karakter"));
    }
    let folded_code = fold_customer_code(search);
    #[derive(sqlx::FromRow)]
    struct SummaryRow {
        id: Uuid,
        code: Option<String>,
        name: String,
        address: Option<String>,
        is_active: bool,
        version: i32,
        outstanding_idr: String,
        customer_credit_idr: String,
    }
    let mut customers: Vec<SummaryRow> = sqlx::query_as(
        "WITH eligible AS (
           SELECT e.customer_id, sum(e.debit_idr::numeric) amount FROM receivable_entries e
           LEFT JOIN credit_sales s ON s.id=e.source_id AND e.entry_type='credit_sale'
           WHERE e.debit_idr>0 AND ((e.entry_type='legacy_debit' AND NOT EXISTS(SELECT 1 FROM transaction_reversals tr WHERE tr.original_entry_id=e.id)) OR (e.entry_type='credit_sale' AND s.status='posted')) GROUP BY e.customer_id
         ), received AS (
           SELECT p.customer_id, sum(p.amount_idr::numeric) amount FROM payments p WHERE p.status='posted' GROUP BY p.customer_id
           UNION ALL
           SELECT e.customer_id, sum(e.credit_idr::numeric) FROM receivable_entries e WHERE e.entry_type='legacy_payment' AND NOT EXISTS(SELECT 1 FROM payments p WHERE p.id=e.source_id) AND NOT EXISTS(SELECT 1 FROM transaction_reversals tr WHERE tr.original_entry_id=e.id) GROUP BY e.customer_id
         ), allocated AS (
           SELECT p.customer_id, sum(a.amount_idr::numeric) amount FROM payment_allocations a JOIN payments p ON p.id=a.payment_id AND p.status='posted' JOIN receivable_entries d ON d.id=a.debit_entry_id AND d.debit_idr>0 WHERE a.status='active' GROUP BY p.customer_id
           UNION ALL
           SELECT p.customer_id, sum(a.amount_idr::numeric) FROM legacy_credit_allocations a JOIN receivable_entries p ON p.id=a.payment_entry_id JOIN receivable_entries d ON d.id=a.debit_entry_id AND d.debit_idr>0 WHERE a.status='active' AND NOT EXISTS(SELECT 1 FROM transaction_reversals tr WHERE tr.original_entry_id=p.id) GROUP BY p.customer_id
         ), totals AS (
           SELECT c.id, coalesce(e.amount,0)-coalesce(a.amount,0) outstanding,
             coalesce((SELECT sum(r.amount) FROM received r WHERE r.customer_id=c.id),0)-coalesce(a.amount,0) credit
           FROM customers c LEFT JOIN eligible e ON e.customer_id=c.id LEFT JOIN (SELECT customer_id,sum(amount) amount FROM allocated GROUP BY customer_id) a ON a.customer_id=c.id
         )
         SELECT c.id, c.code, c.name, c.address, c.is_active, c.version, t.outstanding::text outstanding_idr, t.credit::text customer_credit_idr
         FROM customers c JOIN totals t ON t.id=c.id
         WHERE ($1::uuid IS NULL OR c.id > $1)
           AND ($2 = '' OR strpos(lower(c.name), lower($2)) > 0 OR strpos(c.normalized_code, $3) > 0)
         ORDER BY c.id LIMIT $4",
    )
    .bind(cursor)
    .bind(search)
    .bind(folded_code)
    .bind(i64::from(limit) + 1)
    .fetch_all(&pool)
    .await
    .map_err(|_| request.internal())?;
    let next_cursor = trim_page(&mut customers, limit, |customer| customer.id);
    let customer_ids: Vec<Uuid> = customers.iter().map(|customer| customer.id).collect();
    let as_of: chrono::NaiveDate =
        sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Asia/Jakarta')::date")
            .fetch_one(&pool)
            .await
            .map_err(|_| request.internal())?;
    let debits = reports::open_debits(&pool, Some(&customer_ids), request).await?;
    let mut aging = std::collections::HashMap::<Uuid, [i64; 7]>::new();
    for debit in debits {
        let buckets = aging.entry(debit.customer_id).or_insert([0; 7]);
        let index = reports::bucket(debit.effective_date, as_of);
        buckets[index] = buckets[index]
            .checked_add(debit.open_idr)
            .ok_or_else(|| request.internal())?;
    }
    Ok(Json(CustomerListResponse {
        data: CustomerList {
            items: customers
                .into_iter()
                .map(|row| {
                    let amounts = aging.get(&row.id).copied().unwrap_or([0; 7]);
                    CustomerSummary {
                        customer: Customer {
                            id: row.id,
                            code: row.code,
                            name: row.name,
                            address: row.address,
                            is_active: row.is_active,
                            version: row.version,
                        },
                        outstanding_idr: row.outstanding_idr,
                        customer_credit_idr: row.customer_credit_idr,
                        days_0_15_idr: amounts[0].to_string(),
                        days_16_30_idr: amounts[1].to_string(),
                        days_31_45_idr: amounts[2].to_string(),
                        days_46_60_idr: amounts[3].to_string(),
                        days_over_60_idr: amounts[4].to_string(),
                        unknown_date_idr: amounts[5].to_string(),
                        future_date_idr: amounts[6].to_string(),
                    }
                })
                .collect(),
        },
        meta: PageMeta { next_cursor },
    }))
}

pub async fn detail(
    State(pool): State<PgPool>,
    request: RequestId,
    RouteId(id): RouteId,
) -> Result<Json<CustomerDetailResponse>, Failure> {
    let mut tx = ledger::snapshot(&pool, request).await?;
    let customer = lookup(&mut tx, id, request).await?;
    let summary = ledger::summary(&mut tx, id, request).await?;
    tx.commit().await.map_err(|_| request.internal())?;
    Ok(Json(CustomerDetailResponse {
        data: CustomerDetail {
            customer: customer.into(),
            outstanding_idr: summary.outstanding_idr,
            customer_credit_idr: summary.customer_credit_idr,
        },
        meta: EmptyMeta::default(),
    }))
}

pub async fn create(
    State(pool): State<PgPool>,
    Extension(session): Extension<Session>,
    request: RequestId,
    Body(input): Body<CreateCustomer>,
) -> Result<(StatusCode, Json<CustomerResponse>), Failure> {
    let identity = identity(
        request,
        &input.name,
        input.code.as_deref(),
        input.address.as_deref(),
    )?;
    let row: CustomerRow = sqlx::query_as(
        "INSERT INTO customers (id, code, normalized_code, name, address, created_by)
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING id, code, name, address, is_active, version",
    )
    .bind(Uuid::new_v4())
    .bind(identity.code)
    .bind(identity.normalized_code)
    .bind(identity.name)
    .bind(identity.address)
    .bind(session.user_id)
    .fetch_one(&pool)
    .await
    .map_err(|error| write_error(request, error))?;
    Ok((StatusCode::CREATED, response(row)))
}

pub async fn update(
    State(pool): State<PgPool>,
    request: RequestId,
    RouteId(id): RouteId,
    Body(input): Body<UpdateCustomer>,
) -> Result<Json<CustomerResponse>, Failure> {
    let mut tx = pool.begin().await.map_err(|_| request.internal())?;
    let current: CustomerRow = sqlx::query_as(
        "SELECT id, code, name, address, is_active, version FROM customers WHERE id = $1 FOR UPDATE",
    ).bind(id).fetch_optional(&mut *tx).await.map_err(|_| request.internal())?
        .ok_or_else(|| request.missing_customer())?;
    require_version(request, input.expected_version, current.version)?;
    let identity = identity(
        request,
        input.name.as_deref().unwrap_or(&current.name),
        input.code.unwrap_or(current.code).as_deref(),
        input.address.unwrap_or(current.address).as_deref(),
    )?;
    let row: CustomerRow = sqlx::query_as(
        "UPDATE customers SET code = $2, normalized_code = $3, name = $4, address = $5,
         is_active = $6, version = version + 1, updated_at = now()
         WHERE id = $1 RETURNING id, code, name, address, is_active, version",
    )
    .bind(id)
    .bind(identity.code)
    .bind(identity.normalized_code)
    .bind(identity.name)
    .bind(identity.address)
    .bind(input.is_active.unwrap_or(current.is_active))
    .fetch_one(&mut *tx)
    .await
    .map_err(|error| write_error(request, error))?;
    tx.commit().await.map_err(|_| request.internal())?;
    Ok(response(row))
}

pub async fn merge(
    State(pool): State<PgPool>,
    Extension(session): Extension<Session>,
    request: RequestId,
    RouteId(source): RouteId,
    Body(input): Body<MergeCustomer>,
) -> Result<Json<CustomerResponse>, Failure> {
    let target = input.target_customer_id;
    if source == target {
        return Err(request.invalid("target_customer_id", "Pelanggan tujuan harus berbeda"));
    }
    let reason = input.reason.trim();
    if reason.is_empty() || reason.chars().count() > 1000 {
        return Err(request.invalid("reason", "Alasan wajib diisi, maksimal 1.000 karakter"));
    }
    let mut tx = pool.begin().await.map_err(|_| request.internal())?;
    let locked: Vec<CustomerRow> = sqlx::query_as(
        "SELECT id, code, name, address, is_active, version FROM customers
         WHERE id IN ($1, $2) ORDER BY id FOR UPDATE",
    )
    .bind(source)
    .bind(target)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| request.internal())?;
    let original = locked
        .iter()
        .find(|row| row.id == source)
        .ok_or_else(|| request.missing_customer())?;
    if !locked.iter().any(|row| row.id == target) {
        return Err(request.missing_customer());
    }
    require_version(request, input.expected_version, original.version)?;
    sqlx::query("UPDATE receivable_entries SET customer_id = $2 WHERE customer_id = $1")
        .bind(source)
        .bind(target)
        .execute(&mut *tx)
        .await
        .map_err(|_| request.internal())?;
    sqlx::query("UPDATE payments SET customer_id = $2 WHERE customer_id = $1")
        .bind(source)
        .bind(target)
        .execute(&mut *tx)
        .await
        .map_err(|_| request.internal())?;
    sqlx::query("UPDATE credit_sales SET customer_id = $2,
        version = CASE WHEN status = 'draft' THEN version + 1 ELSE version END WHERE customer_id = $1")
        .bind(source)
        .bind(target)
        .execute(&mut *tx)
        .await
        .map_err(|_| request.internal())?;
    let original: CustomerRow = sqlx::query_as(
        "UPDATE customers SET is_active = false, version = version + 1, updated_at = now()
         WHERE id = $1 RETURNING id, code, name, address, is_active, version",
    )
    .bind(source)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| request.internal())?;
    sqlx::query("UPDATE customers SET version = version + 1, updated_at = now() WHERE id = $1")
        .bind(target)
        .execute(&mut *tx)
        .await
        .map_err(|_| request.internal())?;
    sqlx::query(
        "INSERT INTO audit_events (id, actor_id, action, entity_type, entity_id, reason, request_id, metadata)
         VALUES ($1, $2, 'customer.merge', 'customer', $3, $4, $5, jsonb_build_object('target_customer_id', $6::uuid))",
    ).bind(Uuid::new_v4()).bind(session.user_id).bind(source).bind(reason).bind(request.0).bind(target)
        .execute(&mut *tx).await.map_err(|_| request.internal())?;
    tx.commit().await.map_err(|_| request.internal())?;
    Ok(response(original))
}

pub async fn exists(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    request: RequestId,
) -> Result<(), Failure> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM customers WHERE id = $1)")
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .map_err(|_| request.internal())?;
    if exists {
        Ok(())
    } else {
        Err(request.missing_customer())
    }
}

async fn lookup(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    request: RequestId,
) -> Result<CustomerRow, Failure> {
    sqlx::query_as(
        "SELECT id, code, name, address, is_active, version FROM customers WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| request.internal())?
    .ok_or_else(|| request.missing_customer())
}

fn identity(
    request: RequestId,
    name: &str,
    code: Option<&str>,
    address: Option<&str>,
) -> Result<CustomerIdentity, Failure> {
    for (field, value, max) in [
        ("name", Some(name), 256),
        ("code", code, 128),
        ("address", address, 2000),
    ] {
        if value.is_some_and(|value| value.contains('\0') || value.trim().chars().count() > max) {
            return Err(request.invalid(
                field,
                "Teks terlalu panjang atau mengandung karakter yang tidak valid",
            ));
        }
    }
    CustomerIdentity::new(name, code, address)
        .map_err(|_| request.invalid("name", "Nama pelanggan wajib diisi"))
}

fn require_version(request: RequestId, expected: i32, actual: i32) -> Result<(), Failure> {
    if expected <= 0 {
        return Err(request.invalid("expected_version", "Versi harus positif"));
    }
    if expected != actual {
        return Err(request.field(
            StatusCode::CONFLICT,
            "VERSION_CONFLICT",
            "expected_version",
            "Data telah berubah. Muat ulang detail pelanggan.",
        ));
    }
    Ok(())
}

fn write_error(request: RequestId, error: sqlx::Error) -> Failure {
    if error
        .as_database_error()
        .and_then(|error| error.constraint())
        == Some("customers_normalized_code")
    {
        request.field(
            StatusCode::CONFLICT,
            "CUSTOMER_CODE_CONFLICT",
            "code",
            "Kode pelanggan sudah digunakan",
        )
    } else {
        request.internal()
    }
}

fn response(customer: CustomerRow) -> Json<CustomerResponse> {
    Json(CustomerResponse {
        data: customer.into(),
        meta: EmptyMeta::default(),
    })
}

pub fn pagination(
    request: RequestId,
    limit: Option<u32>,
    cursor: Option<&str>,
) -> Result<(u32, Option<Uuid>), Failure> {
    let limit = limit.unwrap_or(50);
    if !(1..=100).contains(&limit) {
        return Err(request.invalid("limit", "Batas hasil harus 1–100"));
    }
    let cursor = cursor
        .map(|cursor| {
            if cursor.len() != 32 {
                return Err(request.invalid("cursor", "Cursor tidak valid"));
            }
            Uuid::try_parse(cursor).map_err(|_| request.invalid("cursor", "Cursor tidak valid"))
        })
        .transpose()?;
    Ok((limit, cursor))
}

pub fn trim_page<T>(rows: &mut Vec<T>, limit: u32, id: impl Fn(&T) -> Uuid) -> Option<String> {
    if rows.len() <= limit as usize {
        return None;
    }
    rows.truncate(limit as usize);
    rows.last().map(|row| id(row).simple().to_string())
}
