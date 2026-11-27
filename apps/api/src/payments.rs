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
    EmptyMeta, PageMeta,
    customers::{
        ApplyCredit, CreatePayment, PaymentAllocation, PaymentDetail, PaymentListResponse,
        PaymentMethod, PaymentResponse, PaymentsQuery,
    },
};
use toko_domain::payment::{OpenDebit, allocate_fifo};
use uuid::Uuid;

type DebitRow = (
    Uuid,
    Option<NaiveDate>,
    Option<i64>,
    DateTime<Utc>,
    i64,
    i64,
);
type PaymentRow = (
    Uuid,
    i64,
    NaiveDate,
    String,
    Option<String>,
    Option<String>,
    String,
    DateTime<Utc>,
    i64,
    i32,
);

use crate::{
    auth::Session,
    customers,
    error::{Body, Failure, Params, RequestId, RouteId},
    ledger,
};

pub async fn create(
    State(pool): State<PgPool>,
    Extension(session): Extension<Session>,
    request: RequestId,
    headers: HeaderMap,
    Body(body): Body<Value>,
) -> Result<(StatusCode, Json<Value>), Failure> {
    let key = RetryKey::new(request, &headers, "payments:create".to_owned(), &body)?;
    let input: CreatePayment = decode(request, body)?;
    let mut tx = pool.begin().await.map_err(|_| request.internal())?;
    if let Some(saved) = key.replay(&mut tx, session.user_id, request).await? {
        return Ok(saved);
    }
    let amount = parse_money(request, "amount_idr", &input.amount_idr)?;
    if amount <= 0 {
        return Err(request.invalid("amount_idr", "Nominal pembayaran harus lebih dari nol"));
    }
    validate_date(&mut tx, request, input.received_date).await?;
    lock_customer(&mut tx, input.customer_id, request).await?;
    let now: DateTime<Utc> = sqlx::query_scalar("SELECT statement_timestamp()")
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| request.internal())?;
    let debits: Vec<DebitRow> = sqlx::query_as(
        "SELECT e.id, e.effective_date, e.source_order, coalesce(s.posted_at,e.created_at), e.debit_idr,
                coalesce(a.allocated, 0)::bigint
         FROM receivable_entries e
         LEFT JOIN credit_sales s ON s.id = e.source_id AND e.entry_type = 'credit_sale'
         LEFT JOIN LATERAL (
             SELECT (SELECT coalesce(sum(pa.amount_idr),0) FROM payment_allocations pa
             JOIN payments p ON p.id = pa.payment_id
             WHERE pa.debit_entry_id = e.id AND pa.status = 'active' AND p.status = 'posted')
             + (SELECT coalesce(sum(la.amount_idr),0) FROM legacy_credit_allocations la WHERE la.debit_entry_id=e.id AND la.status='active') AS allocated
         ) a ON true
         WHERE e.customer_id = $1 AND e.debit_idr > 0
           AND e.entry_type IN ('legacy_debit', 'credit_sale')
           AND (e.entry_type <> 'legacy_debit' OR NOT EXISTS(SELECT 1 FROM transaction_reversals tr WHERE tr.original_entry_id=e.id))
           AND (e.entry_type = 'legacy_debit' OR s.status = 'posted')
         ORDER BY 2 NULLS LAST, 3 NULLS LAST, 4, 1",
    )
    .bind(input.customer_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| request.internal())?;
    let mut debits: Vec<OpenDebit> = debits
        .into_iter()
        .map(
            |(entry_id, effective_date, source_order, posted_at, amount_idr, allocated_idr)| {
                OpenDebit {
                    entry_id,
                    effective_date,
                    source_order,
                    posted_at,
                    amount_idr,
                    allocated_idr,
                }
            },
        )
        .collect();
    let allocations = allocate_fifo(amount, &mut debits).ok_or_else(|| request.internal())?;
    let payment_id = Uuid::new_v4();
    let method = match input.method {
        PaymentMethod::Cash => "cash",
        PaymentMethod::BankTransfer => "bank_transfer",
    };
    let reference = clean_optional(request, "reference", input.reference, 256)?;
    let note = clean_optional(request, "note", input.note, 2000)?;
    let allocation_count = i32::try_from(allocations.len()).unwrap_or(i32::MAX);
    sqlx::query("INSERT INTO payments (id, customer_id, amount_idr, received_date, method, reference, note, status, created_by, posted_at) VALUES ($1,$2,$3,$4,$5,$6,$7,'posted',$8,$9)")
        .bind(payment_id).bind(input.customer_id).bind(amount).bind(input.received_date).bind(method).bind(reference).bind(note).bind(session.user_id).bind(now)
        .execute(&mut *tx).await.map_err(|_| request.internal())?;
    for allocation in allocations {
        sqlx::query("INSERT INTO payment_allocations (id,payment_id,debit_entry_id,amount_idr,status,created_by) VALUES ($1,$2,$3,$4,'active',$5)")
            .bind(Uuid::new_v4()).bind(payment_id).bind(allocation.debit_entry_id).bind(allocation.amount_idr).bind(session.user_id)
            .execute(&mut *tx).await.map_err(|_| request.internal())?;
    }
    sqlx::query("INSERT INTO receivable_entries (id,customer_id,entry_type,source_id,effective_date,debit_idr,credit_idr,created_by,created_at) VALUES ($1,$2,'payment',$3,$4,0,$5,$6,$7)")
        .bind(Uuid::new_v4()).bind(input.customer_id).bind(payment_id).bind(input.received_date).bind(amount).bind(session.user_id).bind(now)
        .execute(&mut *tx).await.map_err(|_| request.internal())?;
    sqlx::query("INSERT INTO audit_events (id,actor_id,action,entity_type,entity_id,request_id,metadata) VALUES ($1,$2,'payment.post','payment',$3,$4,jsonb_build_object('amount_idr',$5::text,'allocation_count',$6))")
        .bind(Uuid::new_v4()).bind(session.user_id).bind(payment_id).bind(request.0).bind(amount).bind(allocation_count)
        .execute(&mut *tx).await.map_err(|_| request.internal())?;
    let payment = detail(&mut tx, payment_id, request).await?;
    let body = serde_json::to_value(PaymentResponse {
        data: payment,
        meta: EmptyMeta::default(),
    })
    .map_err(|_| request.internal())?;
    key.commit(tx, session.user_id, StatusCode::CREATED, body, request)
        .await
}

pub async fn apply(
    State(pool): State<PgPool>,
    Extension(session): Extension<Session>,
    request: RequestId,
    RouteId(customer_id): RouteId,
    headers: HeaderMap,
    Body(body): Body<Value>,
) -> Result<(StatusCode, Json<Value>), Failure> {
    let key = RetryKey::new(
        request,
        &headers,
        format!("customers:{customer_id}:credit-application"),
        &body,
    )?;
    let input: ApplyCredit = decode(request, body)?;
    let amount = parse_money(request, "amount_idr", &input.amount_idr)?;
    let reason = input.reason.trim();
    if amount <= 0 {
        return Err(request.invalid("amount_idr", "Nominal harus lebih dari nol"));
    }
    if reason.is_empty() || reason.len() > 1000 || reason.contains('\0') {
        return Err(request.invalid("reason", "Alasan wajib diisi, maksimal 1.000 karakter"));
    }
    let mut tx = pool.begin().await.map_err(|_| request.internal())?;
    if let Some(saved) = key.replay(&mut tx, session.user_id, request).await? {
        return Ok(saved);
    }
    lock_customer(&mut tx, customer_id, request).await?;
    let normal_payment =
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM payments WHERE id=$1 FOR UPDATE")
            .bind(input.payment_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| request.internal())?;
    let (source_type, source_entry, source_customer, source_amount, source_allocated) =
        if normal_payment.is_some() {
            let rows: (Uuid, Uuid, i64, i64) = sqlx::query_as(
                "SELECT p.customer_id,p.id,p.amount_idr,coalesce(sum(a.amount_idr),0)::bigint
         FROM payments p LEFT JOIN payment_allocations a ON a.payment_id=p.id AND a.status='active'
         WHERE p.id=$1 AND p.status='posted' GROUP BY p.customer_id,p.id",
            )
            .bind(input.payment_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| request.internal())?
            .ok_or_else(|| {
                request.error(
                    StatusCode::NOT_FOUND,
                    "NOT_FOUND",
                    "Pembayaran tidak ditemukan atau sudah dibalik",
                )
            })?;
            ("payment", rows.1, rows.0, rows.2, rows.3)
        } else {
            let legacy_lock:Option<Uuid>=sqlx::query_scalar("SELECT id FROM receivable_entries WHERE entry_type='legacy_payment' AND source_id=$1 AND NOT EXISTS(SELECT 1 FROM transaction_reversals tr WHERE tr.original_entry_id=receivable_entries.id) FOR UPDATE")
            .bind(input.payment_id).fetch_optional(&mut *tx).await.map_err(|_|request.internal())?;
            if legacy_lock.is_none() {
                return Err(request.error(
                    StatusCode::NOT_FOUND,
                    "NOT_FOUND",
                    "Pembayaran tidak ditemukan",
                ));
            }
            let row:Option<(Uuid,Uuid,i64,i64)>=sqlx::query_as("SELECT e.id,e.customer_id,e.credit_idr,coalesce(sum(a.amount_idr),0)::bigint FROM receivable_entries e LEFT JOIN legacy_credit_allocations a ON a.payment_entry_id=e.id AND a.status='active' WHERE e.entry_type='legacy_payment' AND e.source_id=$1 GROUP BY e.id")
            .bind(input.payment_id).fetch_optional(&mut *tx).await.map_err(|_|request.internal())?;
            let row = row.ok_or_else(|| {
                request.error(
                    StatusCode::NOT_FOUND,
                    "NOT_FOUND",
                    "Pembayaran tidak ditemukan",
                )
            })?;
            ("legacy_payment", row.0, row.1, row.2, row.3)
        };
    if source_customer != customer_id {
        return Err(request.error(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "Pembayaran tidak ditemukan",
        ));
    }
    let debit: (Uuid,i64,i64) = sqlx::query_as(
        "SELECT e.customer_id,e.debit_idr,((SELECT coalesce(sum(a.amount_idr),0) FROM payment_allocations a JOIN payments p ON p.id=a.payment_id WHERE a.debit_entry_id=e.id AND a.status='active' AND p.status='posted')+(SELECT coalesce(sum(la.amount_idr),0) FROM legacy_credit_allocations la WHERE la.debit_entry_id=e.id AND la.status='active'))::bigint
         FROM receivable_entries e
         LEFT JOIN credit_sales s ON s.id=e.source_id AND e.entry_type='credit_sale'
         WHERE e.id=$1 AND e.debit_idr>0
           AND (e.entry_type<>'legacy_debit' OR NOT EXISTS(SELECT 1 FROM transaction_reversals tr WHERE tr.original_entry_id=e.id))
           AND (e.entry_type='legacy_debit' OR (e.entry_type='credit_sale' AND s.status='posted'))",
    ).bind(input.debit_entry_id).fetch_optional(&mut *tx).await.map_err(|_| request.internal())?
        .ok_or_else(|| request.error(StatusCode::NOT_FOUND,"NOT_FOUND","Piutang tidak ditemukan"))?;
    if debit.0 != customer_id {
        return Err(request.error(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "Piutang tidak ditemukan",
        ));
    }
    if source_amount - source_allocated < amount || debit.1 <= debit.2 || debit.1 - debit.2 < amount
    {
        return Err(request.invalid(
            "amount_idr",
            "Nominal melebihi saldo kredit atau piutang yang tersedia",
        ));
    }
    if source_type == "payment" {
        sqlx::query("INSERT INTO payment_allocations (id,payment_id,debit_entry_id,amount_idr,status,created_by,reason) VALUES ($1,$2,$3,$4,'active',$5,$6)")
            .bind(Uuid::new_v4()).bind(input.payment_id).bind(input.debit_entry_id).bind(amount).bind(session.user_id).bind(reason)
            .execute(&mut *tx).await.map_err(|_| request.internal())?;
    } else {
        sqlx::query("INSERT INTO legacy_credit_allocations(id,payment_entry_id,debit_entry_id,amount_idr,status,created_by) VALUES($1,$2,$3,$4,'active',$5)")
            .bind(Uuid::new_v4()).bind(source_entry).bind(input.debit_entry_id).bind(amount).bind(session.user_id)
            .execute(&mut *tx).await.map_err(|_|request.internal())?;
    }
    sqlx::query("INSERT INTO audit_events (id,actor_id,action,entity_type,entity_id,reason,request_id,metadata) VALUES ($1,$2,'credit.apply',$3,$4,$5,$6,jsonb_build_object('debit_entry_id',$7::uuid,'amount_idr',$8::text))")
        .bind(Uuid::new_v4()).bind(session.user_id).bind(source_type).bind(input.payment_id).bind(reason).bind(request.0).bind(input.debit_entry_id).bind(amount)
        .execute(&mut *tx).await.map_err(|_| request.internal())?;
    let value = serde_json::json!({"data":{"debit_entry_id":input.debit_entry_id,"amount_idr":amount.to_string()},"meta":{}});
    key.commit(tx, session.user_id, StatusCode::OK, value, request)
        .await
}

pub async fn get(
    State(pool): State<PgPool>,
    request: RequestId,
    RouteId(id): RouteId,
) -> Result<Json<PaymentResponse>, Failure> {
    let mut tx = ledger::snapshot(&pool, request).await?;
    let data = detail(&mut tx, id, request).await?;
    tx.commit().await.map_err(|_| request.internal())?;
    Ok(Json(PaymentResponse {
        data,
        meta: EmptyMeta::default(),
    }))
}

pub async fn customer_list(
    State(pool): State<PgPool>,
    request: RequestId,
    RouteId(customer): RouteId,
    Params(query): Params<PaymentsQuery>,
) -> Result<Json<PaymentListResponse>, Failure> {
    let (limit, cursor) = customers::pagination(request, query.limit, query.cursor.as_deref())?;
    if let Some(cursor) = cursor {
        let found: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM payments WHERE id=$1 AND customer_id=$2)",
        )
        .bind(cursor)
        .bind(customer)
        .fetch_one(&pool)
        .await
        .map_err(|_| request.internal())?;
        if !found {
            return Err(request.invalid("cursor", "Cursor tidak valid untuk pelanggan ini"));
        }
    }
    let mut tx = ledger::snapshot(&pool, request).await?;
    customers::exists(&mut tx, customer, request).await?;
    let mut ids:Vec<Uuid>=sqlx::query_scalar("SELECT id FROM payments WHERE customer_id=$1 AND ($2::uuid IS NULL OR id>$2) ORDER BY id LIMIT $3").bind(customer).bind(cursor).bind(i64::from(limit)+1).fetch_all(&mut *tx).await.map_err(|_|request.internal())?;
    let next = customers::trim_page(&mut ids, limit, |id| *id);
    let mut payments = Vec::with_capacity(ids.len());
    for id in ids {
        payments.push(detail(&mut tx, id, request).await?);
    }
    tx.commit().await.map_err(|_| request.internal())?;
    Ok(Json(PaymentListResponse {
        data: payments,
        meta: PageMeta { next_cursor: next },
    }))
}

async fn detail(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    request: RequestId,
) -> Result<PaymentDetail, Failure> {
    let (customer_id,amount,received_date,method,reference,note,status,posted_at,allocated,version):PaymentRow=sqlx::query_as(
        "SELECT p.customer_id,p.amount_idr,p.received_date,p.method,p.reference,p.note,p.status,p.posted_at,
         coalesce((SELECT sum(a.amount_idr) FROM payment_allocations a WHERE a.payment_id=p.id AND a.status='active'),0)::bigint,p.version
         FROM payments p WHERE p.id=$1"
    ).bind(id).fetch_optional(&mut **tx).await.map_err(|_|request.internal())?.ok_or_else(||request.error(StatusCode::NOT_FOUND,"NOT_FOUND","Pembayaran tidak ditemukan"))?;
    let rows: Vec<(Uuid, i64)> = sqlx::query_as(
        "SELECT a.debit_entry_id,a.amount_idr
         FROM payment_allocations a
         JOIN receivable_entries e ON e.id=a.debit_entry_id
         LEFT JOIN credit_sales s ON s.id=e.source_id AND e.entry_type='credit_sale'
         WHERE a.payment_id=$1 AND a.status='active'
         ORDER BY e.effective_date NULLS LAST,coalesce(s.posted_at,e.created_at),e.id",
    )
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(|_| request.internal())?;
    let allocations = rows
        .into_iter()
        .map(|(debit_entry_id, amount_idr)| PaymentAllocation {
            debit_entry_id,
            amount_idr: amount_idr.to_string(),
        })
        .collect();
    let method = match method.as_str() {
        "cash" => PaymentMethod::Cash,
        "bank_transfer" => PaymentMethod::BankTransfer,
        _ => return Err(request.internal()),
    };
    Ok(PaymentDetail {
        id,
        customer_id,
        amount_idr: amount.to_string(),
        received_date,
        method,
        reference,
        note,
        status,
        version,
        posted_at,
        allocations,
        unallocated_idr: (amount - allocated).to_string(),
    })
}

async fn lock_customer(
    tx: &mut Transaction<'_, Postgres>,
    customer: Uuid,
    request: RequestId,
) -> Result<(), Failure> {
    let row: Option<bool> =
        sqlx::query_scalar("SELECT is_active FROM customers WHERE id=$1 FOR UPDATE")
            .bind(customer)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|_| request.internal())?;
    match row {
        None => Err(request.missing_customer()),
        Some(_) => Ok(()),
    }
}

async fn validate_date(
    tx: &mut Transaction<'_, Postgres>,
    request: RequestId,
    date: NaiveDate,
) -> Result<(), Failure> {
    let today: NaiveDate =
        sqlx::query_scalar("SELECT (clock_timestamp() AT TIME ZONE 'Asia/Jakarta')::date")
            .fetch_one(&mut **tx)
            .await
            .map_err(|_| request.internal())?;
    if date > today {
        return Err(request.invalid(
            "received_date",
            "Tanggal penerimaan tidak boleh di masa depan",
        ));
    }
    Ok(())
}

fn parse_money(request: RequestId, field: &str, value: &str) -> Result<i64, Failure> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(request.invalid(field, "Nominal harus berupa string digit rupiah"));
    }
    value
        .parse()
        .map_err(|_| request.invalid(field, "Nominal berada di luar batas"))
}
fn clean_optional(
    request: RequestId,
    field: &str,
    value: Option<String>,
    limit: usize,
) -> Result<Option<String>, Failure> {
    let v = value.as_deref().map(str::trim).filter(|v| !v.is_empty());
    if v.is_some_and(|v| v.len() > limit || v.contains('\0')) {
        return Err(request.invalid(field, "Teks terlalu panjang atau tidak valid"));
    }
    Ok(v.map(str::to_owned))
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

struct RetryKey {
    route: String,
    key: String,
    hash: String,
}
impl RetryKey {
    fn new(
        request: RequestId,
        headers: &HeaderMap,
        route: String,
        body: &Value,
    ) -> Result<Self, Failure> {
        let mut it = headers.get_all("idempotency-key").iter();
        let key = it
            .next()
            .and_then(|v| v.to_str().ok())
            .filter(|v| !v.is_empty() && v.len() <= 128 && v.bytes().all(|b| b.is_ascii_graphic()))
            .filter(|_| it.next().is_none())
            .ok_or_else(|| request.invalid("Idempotency-Key", "Idempotency-Key wajib dan valid"))?;
        let hash = hex::encode(Sha256::digest(
            serde_json::to_vec(body).map_err(|_| request.internal())?,
        ));
        Ok(Self {
            route,
            key: key.into(),
            hash,
        })
    }
    async fn replay(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        actor: Uuid,
        request: RequestId,
    ) -> Result<Option<(StatusCode, Json<Value>)>, Failure> {
        let lock = format!("{actor}:{}:{}", self.route, self.key);
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
            .bind(lock)
            .execute(&mut **tx)
            .await
            .map_err(|_| request.internal())?;
        let saved:Option<(String,i32,Value)>=sqlx::query_as("SELECT request_hash,response_status,response_body FROM idempotency_records WHERE actor_id=$1 AND route_key=$2 AND idempotency_key=$3").bind(actor).bind(&self.route).bind(&self.key).fetch_optional(&mut **tx).await.map_err(|_|request.internal())?;
        let Some((hash, status, body)) = saved else {
            return Ok(None);
        };
        if hash != self.hash {
            return Err(request.error(
                StatusCode::CONFLICT,
                "IDEMPOTENCY_CONFLICT",
                "Kunci digunakan dengan isi berbeda",
            ));
        }
        let status = u16::try_from(status)
            .ok()
            .and_then(|s| StatusCode::from_u16(s).ok())
            .ok_or_else(|| request.internal())?;
        Ok(Some((status, Json(body))))
    }
    async fn commit(
        self,
        mut tx: Transaction<'_, Postgres>,
        actor: Uuid,
        status: StatusCode,
        body: Value,
        request: RequestId,
    ) -> Result<(StatusCode, Json<Value>), Failure> {
        sqlx::query("INSERT INTO idempotency_records(id,actor_id,route_key,idempotency_key,request_hash,response_status,response_body) VALUES($1,$2,$3,$4,$5,$6,$7)").bind(Uuid::new_v4()).bind(actor).bind(self.route).bind(self.key).bind(self.hash).bind(i32::from(status.as_u16())).bind(&body).execute(&mut *tx).await.map_err(|_|request.internal())?;
        tx.commit().await.map_err(|_| request.internal())?;
        Ok((status, Json(body)))
    }
}
