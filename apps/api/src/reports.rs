use axum::{Extension, Json, extract::State};
use chrono::NaiveDate;
use sqlx::PgPool;
use toko_contracts::{EmptyMeta, PageMeta, reports::*};
use uuid::Uuid;

use crate::{
    auth::Session,
    customers,
    error::{Failure, Params, RequestId},
};

#[derive(sqlx::FromRow)]
pub(crate) struct DebitRow {
    pub(crate) customer_id: Uuid,
    pub(crate) customer_name: String,
    pub(crate) effective_date: Option<NaiveDate>,
    pub(crate) open_idr: i64,
}

pub async fn receivables_aging(
    State(pool): State<PgPool>,
    Extension(_session): Extension<Session>,
    request: RequestId,
    Params(query): Params<AgingQuery>,
) -> Result<Json<AgingResponse>, Failure> {
    let as_of = report_date(&pool, query.as_of, request).await?;
    let rows = open_debits(&pool, None, request).await?;
    let mut amounts = [0_i64; 7];
    let mut total = 0_i64;
    for row in rows {
        let amount = bucket(row.effective_date, as_of);
        amounts[amount] = amounts[amount]
            .checked_add(row.open_idr)
            .ok_or_else(|| request.internal())?;
        total = total
            .checked_add(row.open_idr)
            .ok_or_else(|| request.internal())?;
    }
    Ok(Json(AgingResponse {
        data: AgingSummary {
            as_of,
            days_0_15: amounts[0].to_string(),
            days_16_30: amounts[1].to_string(),
            days_31_45: amounts[2].to_string(),
            days_46_60: amounts[3].to_string(),
            days_over_60: amounts[4].to_string(),
            unknown_date: amounts[5].to_string(),
            future_date: amounts[6].to_string(),
            total_open_idr: total.to_string(),
        },
        meta: EmptyMeta::default(),
    }))
}

pub async fn receivables_aging_customers(
    State(pool): State<PgPool>,
    Extension(_session): Extension<Session>,
    request: RequestId,
    Params(query): Params<AgingCustomersQuery>,
) -> Result<Json<AgingCustomersResponse>, Failure> {
    let as_of = report_date(&pool, query.as_of, request).await?;
    let (limit, cursor) = customers::pagination(request, query.limit, query.cursor.as_deref())?;
    let rows = open_debits(&pool, None, request).await?;
    let mut grouped = std::collections::BTreeMap::<Uuid, (String, i64)>::new();
    for row in rows
        .into_iter()
        .filter(|row| bucket(row.effective_date, as_of) == bucket_index(query.bucket))
    {
        let entry = grouped
            .entry(row.customer_id)
            .or_insert((row.customer_name, 0));
        entry.1 = entry
            .1
            .checked_add(row.open_idr)
            .ok_or_else(|| request.internal())?;
    }
    let mut items: Vec<AgingCustomer> = grouped
        .into_iter()
        .filter(|(id, _)| cursor.is_none_or(|cursor| *id > cursor))
        .map(|(customer_id, (customer_name, amount))| AgingCustomer {
            customer_id,
            customer_name,
            amount_idr: amount.to_string(),
        })
        .collect();
    let next_cursor = customers::trim_page(&mut items, limit, |item| item.customer_id);
    Ok(Json(AgingCustomersResponse {
        data: items,
        meta: PageMeta { next_cursor },
    }))
}

async fn report_date(
    pool: &PgPool,
    requested: Option<NaiveDate>,
    request: RequestId,
) -> Result<NaiveDate, Failure> {
    let today: NaiveDate = sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Asia/Jakarta')::date")
        .fetch_one(pool)
        .await
        .map_err(|_| request.internal())?;
    let as_of = requested.unwrap_or(today);
    if as_of > today {
        return Err(request.invalid("as_of", "Tanggal acuan tidak boleh di masa depan"));
    }
    Ok(as_of)
}

pub(crate) async fn open_debits(
    pool: &PgPool,
    customer_ids: Option<&[Uuid]>,
    request: RequestId,
) -> Result<Vec<DebitRow>, Failure> {
    let mut tx = pool.begin().await.map_err(|_| request.internal())?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(|_| request.internal())?;
    let rows = sqlx::query_as::<_, DebitRow>(
        "SELECT e.customer_id, c.name AS customer_name, e.effective_date,
                (e.debit_idr::numeric - coalesce(a.allocated,0))::bigint AS open_idr
         FROM receivable_entries e JOIN customers c ON c.id=e.customer_id
         LEFT JOIN credit_sales s ON s.id=e.source_id AND e.entry_type='credit_sale'
         LEFT JOIN LATERAL (
           SELECT coalesce((SELECT sum(pa.amount_idr::numeric)
           FROM payment_allocations pa JOIN payments p ON p.id=pa.payment_id
           WHERE pa.debit_entry_id=e.id AND pa.status='active' AND p.status='posted'),0)
           + coalesce((SELECT sum(la.amount_idr::numeric) FROM legacy_credit_allocations la WHERE la.debit_entry_id=e.id AND la.status='active'),0) AS allocated
         ) a ON true
         WHERE ($1::uuid[] IS NULL OR e.customer_id=ANY($1))
           AND e.debit_idr > 0 AND (e.entry_type='legacy_debit' OR (e.entry_type='credit_sale' AND s.status='posted'))
           AND (e.entry_type<>'legacy_debit' OR NOT EXISTS(SELECT 1 FROM transaction_reversals tr WHERE tr.original_entry_id=e.id))
           AND e.debit_idr::numeric > coalesce(a.allocated,0)
         ORDER BY c.id, e.effective_date NULLS LAST, e.created_at, e.id"
    ).bind(customer_ids).fetch_all(&mut *tx).await.map_err(|_| request.internal())?;
    tx.commit().await.map_err(|_| request.internal())?;
    Ok(rows)
}

pub(crate) fn bucket(date: Option<NaiveDate>, as_of: NaiveDate) -> usize {
    let Some(date) = date else { return 5 };
    let days = as_of.signed_duration_since(date).num_days();
    if days < 0 {
        6
    } else if days <= 15 {
        0
    } else if days <= 30 {
        1
    } else if days <= 45 {
        2
    } else if days <= 60 {
        3
    } else {
        4
    }
}
fn bucket_index(bucket: AgingBucket) -> usize {
    match bucket {
        AgingBucket::Days0_15 => 0,
        AgingBucket::Days16_30 => 1,
        AgingBucket::Days31_45 => 2,
        AgingBucket::Days46_60 => 3,
        AgingBucket::DaysOver60 => 4,
        AgingBucket::UnknownDate => 5,
        AgingBucket::FutureDate => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn assigns_calendar_day_boundaries_and_exceptions() {
        let as_of = NaiveDate::from_ymd_opt(2026, 11, 23).unwrap();
        for (days, expected) in [
            (0, 0),
            (15, 0),
            (16, 1),
            (30, 1),
            (31, 2),
            (45, 2),
            (46, 3),
            (60, 3),
            (61, 4),
        ] {
            assert_eq!(
                bucket(Some(as_of - chrono::Days::new(days)), as_of),
                expected
            );
        }
        assert_eq!(bucket(None, as_of), 5);
        assert_eq!(bucket(Some(as_of + chrono::Days::new(1)), as_of), 6);
    }
}
