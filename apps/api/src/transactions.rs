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
use toko_contracts::{EmptyMeta, transactions::*};
use uuid::Uuid;

use crate::{
    auth::Session,
    error::{Body, Failure, RequestId, RouteId},
};

pub async fn reverse(
    State(pool): State<PgPool>,
    Extension(session): Extension<Session>,
    request: RequestId,
    RouteId(id): RouteId,
    headers: HeaderMap,
    Body(body): Body<Value>,
) -> Result<(StatusCode, Json<Value>), Failure> {
    let key = RetryKey::new(
        request,
        &headers,
        format!("transactions:{id}:reversal"),
        &body,
    )?;
    let input: ReverseTransaction = decode(request, body.clone())?;
    let reason = clean_reason(request, &input.reason)?;
    let mut tx = pool.begin().await.map_err(|_| request.internal())?;
    if let Some(replay) = key.replay(&mut tx, session.user_id, request).await? {
        return Ok(replay);
    }
    let sources: Vec<(String, Uuid)> = sqlx::query_as(
        "SELECT 'credit_sale',customer_id FROM credit_sales WHERE id=$1
         UNION ALL SELECT 'payment',customer_id FROM payments WHERE id=$1
         UNION ALL SELECT entry_type,customer_id FROM receivable_entries WHERE id=$1 AND entry_type IN ('legacy_debit','legacy_payment')",
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| request.internal())?;
    if sources.len() != 1 {
        return Err(request.error(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "Transaksi tidak ditemukan",
        ));
    }
    let (source_type, customer_id) = sources[0].clone();
    sqlx::query("SELECT id FROM customers WHERE id=$1 FOR UPDATE")
        .bind(customer_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|_| request.internal())?
        .ok_or_else(|| request.missing_customer())?;
    let current_status: String = if source_type == "credit_sale" {
        sqlx::query_scalar("SELECT status FROM credit_sales WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| request.internal())?
    } else if source_type == "payment" {
        sqlx::query_scalar("SELECT status FROM payments WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| request.internal())?
    } else {
        let reversed: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM transaction_reversals WHERE original_entry_id=$1)",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| request.internal())?;
        if reversed {
            "reversed".into()
        } else {
            "posted".into()
        }
    };
    if current_status != "posted" {
        return Err(request.error(
            StatusCode::CONFLICT,
            "TRANSACTION_NOT_POSTED",
            "Transaksi sudah dibalik atau belum diposting",
        ));
    }
    let original: (Uuid, i64, i64, Option<NaiveDate>) = if source_type.starts_with("legacy_") {
        sqlx::query_as("SELECT id,debit_idr,credit_idr,effective_date FROM receivable_entries WHERE id=$1 AND entry_type=$2 AND customer_id=$3 FOR UPDATE")
            .bind(id).bind(&source_type).bind(customer_id).fetch_optional(&mut *tx).await.map_err(|_|request.internal())?.ok_or_else(||request.error(StatusCode::NOT_FOUND,"NOT_FOUND","Transaksi ledger tidak ditemukan"))?
    } else {
        sqlx::query_as("SELECT id,debit_idr,credit_idr,effective_date FROM receivable_entries WHERE entry_type=$1 AND source_id=$2 AND customer_id=$3 FOR UPDATE")
            .bind(&source_type).bind(id).bind(customer_id).fetch_optional(&mut *tx).await.map_err(|_|request.internal())?.ok_or_else(||request.error(StatusCode::NOT_FOUND,"NOT_FOUND","Transaksi ledger tidak ditemukan"))?
    };
    if source_type == "credit_sale" || source_type == "legacy_debit" {
        let allocated: i64=sqlx::query_scalar("SELECT ((SELECT coalesce(sum(amount_idr),0) FROM payment_allocations WHERE debit_entry_id=$1 AND status='active')+(SELECT coalesce(sum(amount_idr),0) FROM legacy_credit_allocations WHERE debit_entry_id=$1 AND status='active'))::bigint")
            .bind(original.0).fetch_one(&mut *tx).await.map_err(|_|request.internal())?;
        if allocated > 0 {
            return Err(request.error(
                StatusCode::CONFLICT,
                "ACTIVE_ALLOCATIONS_EXIST",
                "Alihkan alokasi aktif sebelum membalik penjualan",
            ));
        }
    } else if source_type == "payment" {
        sqlx::query("UPDATE payment_allocations SET status='reversed',reversed_by=$2,reversal_reason=$3 WHERE payment_id=$1 AND status='active'")
            .bind(id).bind(session.user_id).bind(reason).execute(&mut *tx).await.map_err(|_|request.internal())?;
    } else {
        sqlx::query("UPDATE legacy_credit_allocations SET status='reversed',reversed_by=$2,reversal_reason=$3 WHERE payment_entry_id=$1 AND status='active'")
            .bind(id).bind(session.user_id).bind(reason).execute(&mut *tx).await.map_err(|_|request.internal())?;
    }
    let reversed_at: DateTime<Utc> = sqlx::query_scalar("SELECT statement_timestamp()")
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| request.internal())?;
    let reversal_entry = Uuid::new_v4();
    sqlx::query("INSERT INTO receivable_entries(id,customer_id,entry_type,source_id,effective_date,debit_idr,credit_idr,created_by,created_at) VALUES($1,$2,'reversal',$3,(now() AT TIME ZONE 'Asia/Jakarta')::date,$4,$5,$6,$7)")
        .bind(reversal_entry).bind(customer_id).bind(original.0).bind(original.2).bind(original.1).bind(session.user_id).bind(reversed_at)
        .execute(&mut *tx).await.map_err(|_|request.internal())?;
    if source_type == "credit_sale" {
        sqlx::query("UPDATE credit_sales SET status='reversed',version=version+1,reversed_at=$2,reversed_by=$3,reversal_reason=$4 WHERE id=$1")
            .bind(id).bind(reversed_at).bind(session.user_id).bind(reason).execute(&mut *tx).await.map_err(|_|request.internal())?;
    } else if source_type == "payment" {
        sqlx::query("UPDATE payments SET status='reversed',reversed_at=$2,reversed_by=$3,reversal_reason=$4 WHERE id=$1")
            .bind(id).bind(reversed_at).bind(session.user_id).bind(reason).execute(&mut *tx).await.map_err(|_|request.internal())?;
    }
    let reversal_id = Uuid::new_v4();
    sqlx::query("INSERT INTO transaction_reversals(id,original_entry_id,reversal_entry_id,source_type,source_id,reversed_by,reason,request_id,reversed_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)")
        .bind(reversal_id).bind(original.0).bind(reversal_entry).bind(&source_type).bind(id).bind(session.user_id).bind(reason).bind(request.0).bind(reversed_at)
        .execute(&mut *tx).await.map_err(|_|request.internal())?;
    sqlx::query("INSERT INTO audit_events(id,actor_id,action,entity_type,entity_id,reason,request_id,metadata) VALUES($1,$2,'transaction.reverse',$3,$4,$5,$6,jsonb_build_object('original_entry_id',$7::uuid,'reversal_entry_id',$8::uuid))")
        .bind(Uuid::new_v4()).bind(session.user_id).bind(&source_type).bind(id).bind(reason).bind(request.0).bind(original.0).bind(reversal_entry).execute(&mut *tx).await.map_err(|_|request.internal())?;
    let response = serde_json::to_value(ReversalResponse {
        data: ReversalData {
            original_transaction_id: id,
            original_entry_id: original.0,
            reversal_entry_id: reversal_entry,
            status: "reversed".into(),
            reversed_at,
        },
        meta: EmptyMeta::default(),
    })
    .map_err(|_| request.internal())?;
    key.commit(tx, session.user_id, StatusCode::OK, response, request)
        .await
}

pub async fn link_replacement(
    State(pool): State<PgPool>,
    Extension(session): Extension<Session>,
    request: RequestId,
    RouteId(id): RouteId,
    headers: HeaderMap,
    Body(body): Body<Value>,
) -> Result<(StatusCode, Json<Value>), Failure> {
    let key = RetryKey::new(
        request,
        &headers,
        format!("transactions:{id}:replacement"),
        &body,
    )?;
    let input: LinkReplacement = decode(request, body.clone())?;
    let reason = clean_reason(request, &input.reason)?;
    let mut tx = pool.begin().await.map_err(|_| request.internal())?;
    if let Some(saved) = key.replay(&mut tx, session.user_id, request).await? {
        return Ok(saved);
    }
    let original:Option<(Uuid,Uuid,Uuid,String)>=sqlx::query_as("SELECT tr.original_entry_id,tr.reversal_entry_id,oe.customer_id,tr.source_type FROM transaction_reversals tr JOIN receivable_entries oe ON oe.id=tr.original_entry_id WHERE tr.source_id=$1 FOR UPDATE OF tr")
        .bind(id).fetch_optional(&mut *tx).await.map_err(|_|request.internal())?;
    let (original_entry, reversal_entry, customer_id, source_type) = original.ok_or_else(|| {
        request.error(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "Koreksi transaksi tidak ditemukan",
        )
    })?;
    let replacement_type = if matches!(source_type.as_str(), "credit_sale" | "legacy_debit") {
        "credit_sale"
    } else {
        "payment"
    };
    let replacement:Option<(Uuid,String)>=sqlx::query_as("SELECT e.id,CASE WHEN e.entry_type='credit_sale' THEN s.status ELSE p.status END FROM receivable_entries e LEFT JOIN credit_sales s ON s.id=e.source_id AND e.entry_type='credit_sale' LEFT JOIN payments p ON p.id=e.source_id AND e.entry_type='payment' WHERE e.entry_type=$1 AND e.source_id=$2 AND e.customer_id=$3 FOR UPDATE OF e")
        .bind(replacement_type).bind(input.replacement_transaction_id).bind(customer_id).fetch_optional(&mut *tx).await.map_err(|_|request.internal())?;
    let (replacement_entry, replacement_status) = replacement.ok_or_else(|| {
        request.error(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "Transaksi pengganti tidak ditemukan untuk pelanggan ini",
        )
    })?;
    if replacement_status != "posted" {
        return Err(request.invalid(
            "replacement_transaction_id",
            "Transaksi pengganti harus sudah diposting",
        ));
    }
    let updated=sqlx::query("UPDATE transaction_reversals SET replacement_entry_id=$2 WHERE source_id=$1 AND replacement_entry_id IS NULL")
        .bind(id).bind(replacement_entry).execute(&mut *tx).await.map_err(|_|request.internal())?;
    if updated.rows_affected() != 1 {
        return Err(request.error(
            StatusCode::CONFLICT,
            "REPLACEMENT_ALREADY_LINKED",
            "Transaksi pengganti sudah tertaut",
        ));
    }
    sqlx::query("INSERT INTO audit_events(id,actor_id,action,entity_type,entity_id,reason,request_id,metadata) VALUES($1,$2,'transaction.replacement_link','transaction',$3,$4,$5,jsonb_build_object('original_entry_id',$6::uuid,'reversal_entry_id',$7::uuid,'replacement_entry_id',$8::uuid))")
        .bind(Uuid::new_v4()).bind(session.user_id).bind(id).bind(reason).bind(request.0).bind(original_entry).bind(reversal_entry).bind(replacement_entry).execute(&mut *tx).await.map_err(|_|request.internal())?;
    let response = serde_json::to_value(CorrectionLinkResponse {
        data: CorrectionLinkData {
            original_transaction_id: id,
            original_entry_id: original_entry,
            reversal_entry_id: reversal_entry,
            replacement_transaction_id: input.replacement_transaction_id,
            replacement_entry_id: replacement_entry,
        },
        meta: EmptyMeta::default(),
    })
    .map_err(|_| request.internal())?;
    key.commit(tx, session.user_id, StatusCode::OK, response, request)
        .await
}

pub async fn reallocate(
    State(pool): State<PgPool>,
    Extension(session): Extension<Session>,
    request: RequestId,
    RouteId(id): RouteId,
    headers: HeaderMap,
    Body(body): Body<Value>,
) -> Result<(StatusCode, Json<Value>), Failure> {
    let key = RetryKey::new(
        request,
        &headers,
        format!("payments:{id}:reallocation"),
        &body,
    )?;
    let input: ReallocatePayment = decode(request, body.clone())?;
    let reason = clean_reason(request, &input.reason)?;
    if input.expected_version <= 0 || input.allocations.len() > 100 {
        return Err(request.invalid("allocations", "Versi atau jumlah alokasi tidak valid"));
    }
    let mut targets = std::collections::BTreeMap::<Uuid, i64>::new();
    let mut total = 0_i64;
    for item in &input.allocations {
        let amount = parse_money(request, &item.amount_idr)?;
        if amount <= 0 || targets.insert(item.debit_entry_id, amount).is_some() {
            return Err(request.invalid(
                "allocations",
                "Alokasi harus positif dan setiap piutang hanya boleh muncul sekali",
            ));
        }
        total = total
            .checked_add(amount)
            .ok_or_else(|| request.invalid("allocations", "Total alokasi melampaui batas"))?;
    }
    let mut tx = pool.begin().await.map_err(|_| request.internal())?;
    if let Some(saved) = key.replay(&mut tx, session.user_id, request).await? {
        return Ok(saved);
    }
    let payment: Option<(Uuid, i64, String, i32)> = sqlx::query_as(
        "SELECT customer_id,amount_idr,status,version FROM payments WHERE id=$1 FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|_| request.internal())?;
    let (customer, amount, status, version) = payment.ok_or_else(|| {
        request.error(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            "Pembayaran tidak ditemukan",
        )
    })?;
    if status != "posted" {
        return Err(request.error(
            StatusCode::CONFLICT,
            "PAYMENT_NOT_POSTED",
            "Pembayaran sudah dibalik",
        ));
    }
    if version != input.expected_version {
        return Err(request.error(
            StatusCode::CONFLICT,
            "VERSION_CONFLICT",
            "Versi pembayaran sudah berubah",
        ));
    }
    if total > amount {
        return Err(request.invalid("allocations", "Total alokasi melebihi pembayaran"));
    }
    sqlx::query("SELECT id FROM customers WHERE id=$1 FOR UPDATE")
        .bind(customer)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| request.internal())?;
    let targets_ids: Vec<Uuid> = targets.keys().copied().collect();
    if !targets_ids.is_empty() {
        let found:Vec<Uuid>=sqlx::query_scalar("SELECT e.id FROM receivable_entries e LEFT JOIN credit_sales s ON s.id=e.source_id AND e.entry_type='credit_sale' WHERE e.id=ANY($1) AND e.customer_id=$2 AND e.debit_idr>0 AND (e.entry_type='legacy_debit' OR (e.entry_type='credit_sale' AND s.status='posted')) AND (e.entry_type<>'legacy_debit' OR NOT EXISTS(SELECT 1 FROM transaction_reversals tr WHERE tr.original_entry_id=e.id)) ORDER BY e.id FOR UPDATE OF e")
            .bind(&targets_ids).bind(customer).fetch_all(&mut *tx).await.map_err(|_|request.internal())?;
        if found.len() != targets.len() {
            return Err(request.error(
                StatusCode::NOT_FOUND,
                "NOT_FOUND",
                "Satu atau lebih piutang tidak tersedia untuk pelanggan ini",
            ));
        }
    }
    sqlx::query("UPDATE payment_allocations SET status='reversed',reversed_by=$2,reversal_reason=$3 WHERE payment_id=$1 AND status='active'")
        .bind(id).bind(session.user_id).bind(reason).execute(&mut *tx).await.map_err(|_|request.internal())?;
    for (debit, allocation) in &targets {
        sqlx::query("INSERT INTO payment_allocations(id,payment_id,debit_entry_id,amount_idr,status,created_by,reason) VALUES($1,$2,$3,$4,'active',$5,$6)")
            .bind(Uuid::new_v4()).bind(id).bind(debit).bind(allocation).bind(session.user_id).bind(reason).execute(&mut *tx).await.map_err(|error|{
                if error.as_database_error().and_then(|e|e.code()).as_deref()==Some("23514") {request.invalid("allocations","Alokasi melampaui saldo piutang yang tersedia")} else {request.internal()}
            })?;
    }
    let new_version = version + 1;
    sqlx::query("UPDATE payments SET version=$2 WHERE id=$1")
        .bind(id)
        .bind(new_version)
        .execute(&mut *tx)
        .await
        .map_err(|_| request.internal())?;
    sqlx::query("INSERT INTO audit_events(id,actor_id,action,entity_type,entity_id,reason,request_id,metadata) VALUES($1,$2,'payment.reallocate','payment',$3,$4,$5,jsonb_build_object('allocation_count',$6))")
        .bind(Uuid::new_v4()).bind(session.user_id).bind(id).bind(reason).bind(request.0).bind(i64::try_from(targets.len()).unwrap_or(i64::MAX)).execute(&mut *tx).await.map_err(|_|request.internal())?;
    let allocations: Vec<ReplacementAllocation> = targets
        .into_iter()
        .map(|(debit_entry_id, amount)| ReplacementAllocation {
            debit_entry_id,
            amount_idr: amount.to_string(),
        })
        .collect();
    let response = serde_json::to_value(ReallocationResponse {
        data: ReallocationData {
            payment_id: id,
            version: new_version,
            allocations,
        },
        meta: EmptyMeta::default(),
    })
    .map_err(|_| request.internal())?;
    key.commit(tx, session.user_id, StatusCode::OK, response, request)
        .await
}

fn clean_reason(request: RequestId, reason: &str) -> Result<&str, Failure> {
    let reason = reason.trim();
    if reason.is_empty() || reason.len() > 1000 || reason.contains('\0') {
        Err(request.invalid("reason", "Alasan wajib diisi, maksimal 1.000 karakter"))
    } else {
        Ok(reason)
    }
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
fn parse_money(request: RequestId, value: &str) -> Result<i64, Failure> {
    if value.is_empty() || value.len() > 18 || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(request.invalid("amount_idr", "Nominal harus berupa digit IDR"));
    }
    value
        .parse()
        .map_err(|_| request.invalid("amount_idr", "Nominal melampaui batas"))
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
        let mut values = headers.get_all("idempotency-key").iter();
        let key = values
            .next()
            .and_then(|v| v.to_str().ok())
            .filter(|v| !v.is_empty() && v.len() <= 128 && v.bytes().all(|b| b.is_ascii_graphic()))
            .filter(|_| values.next().is_none())
            .ok_or_else(|| {
                request.invalid("Idempotency-Key", "Kunci idempotensi wajib dan valid")
            })?;
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
                "Kunci sudah dipakai dengan isi berbeda",
            ));
        }
        let status = u16::try_from(status)
            .ok()
            .and_then(|v| StatusCode::from_u16(v).ok())
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
