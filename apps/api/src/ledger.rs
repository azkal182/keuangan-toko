use axum::{
    Json,
    extract::State,
    http::{HeaderMap, header},
};
use chrono::{NaiveDate, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use toko_contracts::{
    PageMeta,
    customers::{LedgerEntry, LedgerPage, LedgerQuery, LedgerResponse, LedgerSummary},
};
use uuid::Uuid;

use crate::{
    customers,
    error::{Failure, Params, RequestId, RouteId},
};

pub async fn snapshot(
    pool: &PgPool,
    request: RequestId,
) -> Result<Transaction<'_, Postgres>, Failure> {
    let mut tx = pool.begin().await.map_err(|_| request.internal())?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(|_| request.internal())?;
    Ok(tx)
}

pub async fn summary(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    request: RequestId,
) -> Result<LedgerSummary, Failure> {
    let (outstanding_idr, customer_credit_idr, ledger_balance_idr) = sqlx::query_as(
        "WITH ledger AS (
             SELECT coalesce(sum(debit_idr::numeric), 0) AS ledger_debit,
                    coalesce(sum(credit_idr::numeric), 0) AS credit
             FROM receivable_entries WHERE customer_id = $1
         ), eligible_debits AS (
             SELECT coalesce(sum(e.debit_idr::numeric),0) AS amount
             FROM receivable_entries e LEFT JOIN credit_sales s ON s.id=e.source_id AND e.entry_type='credit_sale'
             WHERE e.customer_id=$1 AND e.debit_idr>0
               AND ((e.entry_type='legacy_debit' AND NOT EXISTS(SELECT 1 FROM transaction_reversals tr WHERE tr.original_entry_id=e.id))
                 OR (e.entry_type='credit_sale' AND s.status='posted'))
         ), received AS (
             SELECT coalesce(sum(amount_idr::numeric), 0) AS amount
             FROM payments WHERE customer_id = $1 AND status = 'posted'
             UNION ALL
             SELECT coalesce(sum(e.credit_idr::numeric),0) AS amount FROM receivable_entries e
             WHERE e.customer_id=$1 AND e.entry_type='legacy_payment'
               AND NOT EXISTS(SELECT 1 FROM payments p WHERE p.id=e.source_id)
               AND NOT EXISTS(SELECT 1 FROM transaction_reversals tr WHERE tr.original_entry_id=e.id)
         ), allocated AS (
             SELECT coalesce(sum(a.amount_idr::numeric), 0) AS amount
             FROM payment_allocations a
             JOIN payments p ON p.id = a.payment_id AND p.customer_id = $1 AND p.status = 'posted'
             JOIN receivable_entries d ON d.id = a.debit_entry_id AND d.customer_id = $1 AND d.debit_idr > 0
             WHERE a.status = 'active'
             UNION ALL
             SELECT coalesce(sum(a.amount_idr::numeric),0) FROM legacy_credit_allocations a
             JOIN receivable_entries p ON p.id=a.payment_entry_id AND p.customer_id=$1
             JOIN receivable_entries d ON d.id=a.debit_entry_id AND d.customer_id=$1 AND d.debit_idr>0
             WHERE a.status='active' AND NOT EXISTS(SELECT 1 FROM transaction_reversals tr WHERE tr.original_entry_id=p.id)
         )
         SELECT (eligible_debits.amount - allocated.amount)::text,
                (received.amount - allocated.amount)::text,
                (ledger.ledger_debit - ledger.credit)::text
         FROM ledger, eligible_debits,
              (SELECT coalesce(sum(amount),0) AS amount FROM received) received,
              (SELECT coalesce(sum(amount),0) AS amount FROM allocated) allocated",
    ).bind(id).fetch_one(&mut **tx).await.map_err(|_| request.internal())?;
    Ok(LedgerSummary {
        outstanding_idr,
        customer_credit_idr,
        ledger_balance_idr,
    })
}

#[derive(sqlx::FromRow)]
struct LedgerRow {
    id: Uuid,
    entry_type: String,
    source_id: Uuid,
    is_reversed: bool,
    effective_date: Option<NaiveDate>,
    reference: Option<String>,
    description: Option<String>,
    quantity: Option<String>,
    unit_price_idr: Option<String>,
    debit_idr: String,
    credit_idr: String,
    allocated_idr: String,
    remaining_idr: String,
    age_days: Option<i32>,
    allocation_status: Option<String>,
    running_balance_idr: String,
}

impl From<LedgerRow> for LedgerEntry {
    fn from(row: LedgerRow) -> Self {
        Self {
            id: row.id,
            entry_type: row.entry_type,
            source_id: row.source_id,
            is_reversed: row.is_reversed,
            effective_date: row.effective_date,
            reference: row.reference,
            description: row.description,
            quantity: row.quantity,
            unit_price_idr: row.unit_price_idr,
            debit_idr: row.debit_idr,
            credit_idr: row.credit_idr,
            allocated_idr: row.allocated_idr,
            remaining_idr: row.remaining_idr,
            age_days: row.age_days,
            allocation_status: row.allocation_status,
            running_balance_idr: row.running_balance_idr,
        }
    }
}

pub async fn list(
    State(pool): State<PgPool>,
    request: RequestId,
    RouteId(id): RouteId,
    Params(query): Params<LedgerQuery>,
) -> Result<Json<LedgerResponse>, Failure> {
    let (limit, cursor) = customers::pagination(request, query.limit, query.cursor.as_deref())?;
    let is_desc = match query.order.as_deref() {
        Some(s) if s.eq_ignore_ascii_case("desc") => true,
        Some(s) if s.eq_ignore_ascii_case("asc") => false,
        None => false,
        Some(_) => {
            return Err(request.invalid("order", "Urutan harus 'asc' atau 'desc'"));
        }
    };
    if let (Some(from), Some(to)) = (query.from, query.to)
        && from > to
    {
        return Err(request.invalid(
            "to",
            "Tanggal akhir harus setelah atau sama dengan tanggal awal",
        ));
    }
    let mut tx = snapshot(&pool, request).await?;
    customers::exists(&mut tx, id, request).await?;
    if let Some(cursor) = cursor {
        let owned: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM receivable_entries WHERE id = $1 AND customer_id = $2)",
        )
        .bind(cursor)
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| request.internal())?;
        if !owned {
            return Err(request.invalid("cursor", "Cursor tidak valid untuk buku piutang ini"));
        }
    }
    let mut entries: Vec<LedgerRow> = sqlx::query_as(
        "WITH ordered AS (
             SELECT e.*, row_number() OVER history AS position,
                    sum(e.debit_idr::numeric - e.credit_idr::numeric) OVER history AS balance
             FROM receivable_entries e WHERE e.customer_id = $1
             WINDOW history AS (ORDER BY e.effective_date NULLS LAST, e.source_order NULLS LAST, e.created_at, e.id)
         ), page AS (
             SELECT * FROM ordered
             WHERE ($2::uuid IS NULL OR (
                    CASE WHEN $6 = true THEN position < (SELECT position FROM ordered WHERE id = $2)
                         ELSE position > (SELECT position FROM ordered WHERE id = $2) END
                   ))
               AND ($3::date IS NULL OR effective_date >= $3)
               AND ($4::date IS NULL OR effective_date <= $4)
             ORDER BY CASE WHEN $6 = true THEN -position ELSE position END LIMIT $5
         )
         SELECT e.id, e.entry_type, e.source_id, EXISTS(SELECT 1 FROM transaction_reversals tr WHERE tr.original_entry_id=e.id) AS is_reversed, e.effective_date, coalesce(s.reference_no, p.reference) AS reference, coalesce(e.description,item.description,p.note) AS description,
                coalesce(e.quantity,item.quantity::text) AS quantity, coalesce(e.unit_price_idr,item.unit_price_idr)::text AS unit_price_idr,
                e.debit_idr::text, e.credit_idr::text,
                CASE WHEN e.debit_idr > 0 THEN coalesce(debit_alloc.amount,0)::text
                     WHEN e.entry_type='legacy_payment' THEN coalesce(legacy_payment_alloc.amount,0)::text
                     ELSE coalesce(payment_alloc.amount,0)::text END AS allocated_idr,
                e.balance::text AS running_balance_idr,
                CASE WHEN e.debit_idr > 0 THEN greatest(e.debit_idr::numeric-coalesce(debit_alloc.amount,0),0)::text ELSE '0' END AS remaining_idr,
                CASE WHEN e.debit_idr > 0 AND (e.debit_idr::numeric - coalesce(debit_alloc.amount,0)) > 0 AND e.effective_date IS NOT NULL THEN ((now() AT TIME ZONE 'Asia/Jakarta')::date-e.effective_date)::int END AS age_days,
                CASE WHEN e.debit_idr > 0 THEN
                    CASE WHEN debit_alloc.amount = 0 THEN 'unallocated'
                          WHEN debit_alloc.amount = e.debit_idr THEN 'allocated' ELSE 'partially_allocated' END
                     WHEN p.status = 'posted' THEN
                     CASE WHEN payment_alloc.amount = 0 THEN 'unallocated'
                          WHEN payment_alloc.amount = p.amount_idr THEN 'allocated' ELSE 'partially_allocated' END
                     WHEN e.entry_type='legacy_payment' THEN
                     CASE WHEN legacy_payment_alloc.amount = 0 THEN 'unallocated'
                          WHEN legacy_payment_alloc.amount = e.credit_idr THEN 'allocated' ELSE 'partially_allocated' END
                END AS allocation_status
         FROM page e
         LEFT JOIN payments p ON p.id = e.source_id AND p.customer_id = $1 AND e.entry_type IN ('payment','legacy_payment')
         LEFT JOIN credit_sales s ON s.id = e.source_id AND s.customer_id = $1 AND e.entry_type = 'credit_sale'
         LEFT JOIN LATERAL (
             SELECT description, quantity, unit_price_idr FROM credit_sale_items WHERE sale_id = s.id ORDER BY ordinal LIMIT 1
         ) item ON s.id IS NOT NULL
         LEFT JOIN LATERAL (
             SELECT (SELECT coalesce(sum(a.amount_idr), 0) FROM payment_allocations a JOIN payments ap ON ap.id = a.payment_id
                     WHERE a.debit_entry_id = e.id AND a.status = 'active' AND ap.customer_id = $1 AND ap.status = 'posted')
                  + (SELECT coalesce(sum(la.amount_idr),0) FROM legacy_credit_allocations la WHERE la.debit_entry_id=e.id AND la.status='active') AS amount
         ) debit_alloc ON e.debit_idr > 0
         LEFT JOIN LATERAL (
             SELECT coalesce(sum(a.amount_idr), 0) AS amount
             FROM payment_allocations a JOIN receivable_entries d ON d.id = a.debit_entry_id
             WHERE a.payment_id = p.id AND a.status = 'active' AND d.customer_id = $1 AND d.debit_idr > 0
         ) payment_alloc ON p.status = 'posted'
         LEFT JOIN LATERAL (
             SELECT coalesce(sum(a.amount_idr),0) AS amount FROM legacy_credit_allocations a
             WHERE a.payment_entry_id=e.id AND a.status='active'
         ) legacy_payment_alloc ON e.entry_type='legacy_payment'
         ORDER BY CASE WHEN $6 = true THEN -e.position ELSE e.position END",
    ).bind(id).bind(cursor).bind(query.from).bind(query.to).bind(i64::from(limit) + 1).bind(is_desc)
        .fetch_all(&mut *tx).await.map_err(|_| request.internal())?;
    let next_cursor = customers::trim_page(&mut entries, limit, |entry| entry.id);
    let summary = summary(&mut tx, id, request).await?;
    tx.commit().await.map_err(|_| request.internal())?;
    Ok(Json(LedgerResponse {
        data: LedgerPage {
            entries: entries.into_iter().map(LedgerEntry::from).collect(),
            summary,
        },
        meta: PageMeta { next_cursor },
    }))
}

#[derive(sqlx::FromRow)]
struct UnpaidDbRow {
    effective_date: Option<NaiveDate>,
    description: Option<String>,
    quantity: Option<String>,
    unit_price_idr: Option<i64>,
    debit_idr: i64,
    allocated_idr: i64,
    remaining_idr: i64,
}

pub async fn export_bill_pdf(
    State(pool): State<PgPool>,
    request: RequestId,
    RouteId(id): RouteId,
    Params(query): Params<LedgerQuery>,
) -> Result<(HeaderMap, Vec<u8>), Failure> {
    if let (Some(from), Some(to)) = (query.from, query.to)
        && from > to
    {
        return Err(request.invalid(
            "to",
            "Tanggal akhir harus setelah atau sama dengan tanggal awal",
        ));
    }
    let mut tx = snapshot(&pool, request).await?;
    let customer: Option<(String, Option<String>, Option<String>)> =
        sqlx::query_as("SELECT name, code, address FROM customers WHERE id = $1")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| request.internal())?;

    let (customer_name, customer_code, customer_address) = match customer {
        Some(c) => c,
        None => return Err(request.missing_customer()),
    };

    let rows: Vec<UnpaidDbRow> = sqlx::query_as(
        "WITH debits AS (
             SELECT e.id, e.effective_date, e.source_order, e.created_at,
                    e.debit_idr,
                    coalesce(e.description, item.description, s.reference_no, 'Penjualan') AS description,
                    coalesce(e.quantity, item.quantity::text) AS quantity,
                    coalesce(e.unit_price_idr, item.unit_price_idr) AS unit_price_idr,
                    coalesce(debit_alloc.amount, 0) AS allocated_idr,
                    (e.debit_idr - coalesce(debit_alloc.amount, 0)) AS remaining_idr
             FROM receivable_entries e
             LEFT JOIN credit_sales s ON s.id = e.source_id AND s.customer_id = $1 AND e.entry_type = 'credit_sale'
             LEFT JOIN LATERAL (
                 SELECT description, quantity, unit_price_idr FROM credit_sale_items WHERE sale_id = s.id ORDER BY ordinal LIMIT 1
             ) item ON s.id IS NOT NULL
             LEFT JOIN LATERAL (
                 SELECT (SELECT coalesce(sum(a.amount_idr), 0) FROM payment_allocations a JOIN payments ap ON ap.id = a.payment_id
                         WHERE a.debit_entry_id = e.id AND a.status = 'active' AND ap.customer_id = $1 AND ap.status = 'posted')
                      + (SELECT coalesce(sum(la.amount_idr), 0) FROM legacy_credit_allocations la WHERE la.debit_entry_id = e.id AND la.status = 'active') AS amount
             ) debit_alloc ON true
             WHERE e.customer_id = $1
               AND e.debit_idr > 0
               AND (
                   (e.entry_type = 'legacy_debit' AND NOT EXISTS(SELECT 1 FROM transaction_reversals tr WHERE tr.original_entry_id = e.id))
                   OR (e.entry_type = 'credit_sale' AND s.status = 'posted')
               )
         )
         SELECT effective_date, description, quantity, unit_price_idr, debit_idr, allocated_idr, remaining_idr
         FROM debits
         WHERE remaining_idr > 0
           AND ($2::date IS NULL OR effective_date >= $2)
           AND ($3::date IS NULL OR effective_date <= $3)
         ORDER BY effective_date NULLS LAST, source_order NULLS LAST, created_at, id",
    )
    .bind(id)
    .bind(query.from)
    .bind(query.to)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| request.internal())?;

    tx.commit().await.map_err(|_| request.internal())?;

    let items: Vec<crate::pdf::UnpaidBillItem> = rows
        .into_iter()
        .map(|r| crate::pdf::UnpaidBillItem {
            effective_date: r.effective_date,
            description: r.description.unwrap_or_else(|| "Penjualan".into()),
            quantity: r.quantity,
            unit_price_idr: r.unit_price_idr,
            debit_idr: r.debit_idr,
            allocated_idr: r.allocated_idr,
            remaining_idr: r.remaining_idr,
        })
        .collect();

    let bill_data = crate::pdf::CustomerBillData {
        customer_id: id,
        customer_code,
        customer_name: customer_name.clone(),
        customer_address,
        printed_at: Utc::now(),
        items,
    };

    let pdf_bytes = crate::pdf::generate_customer_bill_pdf(&bill_data);

    let safe_name: String = customer_name
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    let filename = format!("Tagihan-{}-{}.pdf", safe_name, Utc::now().format("%Y%m%d"));

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, "application/pdf".parse().unwrap());
    headers.insert(
        header::CONTENT_DISPOSITION,
        format!("inline; filename=\"{filename}\"").parse().unwrap(),
    );

    Ok((headers, pdf_bytes))
}
