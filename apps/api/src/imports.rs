use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    io::Cursor,
};

use axum::{
    Extension, Json,
    extract::{Multipart, Path, State},
    http::{HeaderMap, StatusCode},
};
use calamine::{Data, Range, Reader, open_workbook_auto_from_rs};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};
use toko_contracts::{EmptyMeta, PageMeta, imports::*};
use toko_domain::customer::{CustomerIdentity, fold_customer_code, fold_customer_name};
use uuid::Uuid;

type ImportCustomerMatchRow = (Uuid, Option<String>, Option<String>, String, bool);
type ImportReviewRow = (i32, Value, Option<Value>, String, String, String, Value);
type ImportCommitRow = (
    Uuid,
    Option<Uuid>,
    Value,
    String,
    String,
    i64,
    String,
    Value,
);
use zip::ZipArchive;

use crate::{
    auth::Session,
    customers,
    error::{Body, Failure, Params, RequestId, RouteId},
};

const MAX_FILE_BYTES: usize = 25 * 1024 * 1024;
const MAX_UNCOMPRESSED_BYTES: u64 = 128 * 1024 * 1024;
const MAX_ZIP_ENTRIES: usize = 2048;
const MAX_SHEETS: usize = 250;
const MAX_SOURCE_ROWS: usize = 25_000;
const MAX_COLUMNS: usize = 256;

#[derive(Debug)]
struct StagedRow {
    sheet: String,
    row: i32,
    source_customer: ImportCustomerSource,
    raw: Value,
    parsed: Option<Value>,
    issues: Value,
    fingerprint: String,
    source_order: i64,
    initial_status: &'static str,
}

struct StagedInsert {
    id: Uuid,
    sheet: String,
    row: i32,
    source_customer: Value,
    raw: Value,
    parsed: Option<Value>,
    issues: Value,
    review_status: &'static str,
    fingerprint: String,
    source_order: i64,
}

struct PreparedImportEntry {
    row_id: Uuid,
    customer_id: Uuid,
    ledger_id: Uuid,
    effective_date: chrono::NaiveDate,
    debit: i64,
    credit: i64,
    source_order: i64,
    description: Option<String>,
    quantity: Option<String>,
    unit_price_idr: Option<i64>,
}

struct PreparedLegacyAllocation {
    payment_entry_id: Uuid,
    debit_entry_id: Uuid,
    amount: i64,
}

fn commit_db_failure(request: RequestId, stage: &str, error: sqlx::Error) -> Failure {
    let sqlstate = error
        .as_database_error()
        .and_then(|database_error| database_error.code())
        .map(|code| code.to_string())
        .unwrap_or_else(|| "non_database".into());
    eprintln!(
        "level=error event=import_commit_db_failed request_id={} stage={} sqlstate={}",
        request.0, stage, sqlstate
    );
    request.internal()
}

pub async fn upload(
    State(pool): State<PgPool>,
    Extension(session): Extension<Session>,
    request: RequestId,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<ImportBatchResponse>), Failure> {
    let mut file: Option<(String, Vec<u8>)> = None;
    while let Some(field) = multipart.next_field().await.map_err(|_| {
        request.error(
            StatusCode::BAD_REQUEST,
            "INVALID_MULTIPART",
            "Formulir upload tidak valid",
        )
    })? {
        if field.name() != Some("file") || file.is_some() {
            return Err(request.error(
                StatusCode::BAD_REQUEST,
                "INVALID_MULTIPART",
                "Unggah tepat satu berkas pada field file",
            ));
        }
        let filename = field.file_name().unwrap_or("").to_owned();
        let bytes = field.bytes().await.map_err(|_| {
            request.error(
                StatusCode::BAD_REQUEST,
                "INVALID_MULTIPART",
                "Isi berkas tidak dapat dibaca",
            )
        })?;
        if bytes.len() > MAX_FILE_BYTES {
            return Err(request.error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "FILE_TOO_LARGE",
                "Ukuran berkas maksimal 25 MiB",
            ));
        }
        file = Some((filename, bytes.to_vec()));
    }
    let (filename, bytes) =
        file.ok_or_else(|| request.invalid("file", "Berkas XLSX wajib diunggah"))?;
    if !filename.to_ascii_lowercase().ends_with(".xlsx")
        || filename.len() > 255
        || filename.contains('\0')
    {
        return Err(request.invalid(
            "file",
            "Format berkas harus .xlsx dengan nama maksimal 255 karakter",
        ));
    }
    if bytes.len() < 4 || bytes[..2] != *b"PK" {
        return Err(request.invalid("file", "Isi berkas bukan workbook XLSX yang valid"));
    }
    zip_preflight(&bytes).map_err(|_| {
        request.invalid(
            "file",
            "Struktur arsip XLSX tidak valid atau melewati batas pemrosesan",
        )
    })?;
    let fingerprint = hex::encode(Sha256::digest(&bytes));
    let business_today: chrono::NaiveDate =
        sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Asia/Jakarta')::date")
            .fetch_one(&pool)
            .await
            .map_err(|_| request.internal())?;
    let parse_bytes = bytes.clone();
    let rows = tokio::task::spawn_blocking(move || parse_workbook(&parse_bytes, business_today))
        .await
        .map_err(|_| request.internal())?
        .map_err(|_| {
            request.invalid(
                "file",
                "Workbook tidak valid, melebihi batas, atau tidak dapat diproses",
            )
        })?;
    if rows.is_empty() {
        return Err(request.invalid("file", "Workbook tidak berisi baris sumber untuk staging"));
    }
    let mut fingerprints = BTreeMap::<String, usize>::new();
    for row in &rows {
        *fingerprints.entry(row.fingerprint.clone()).or_default() += 1;
    }
    let mut tx = pool.begin().await.map_err(|_| request.internal())?;
    let mut id = Uuid::new_v4();
    let safe_name = filename
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("workbook.xlsx")
        .trim();
    let safe_name = if safe_name.is_empty() {
        "workbook.xlsx"
    } else {
        safe_name
    };
    let inserted=sqlx::query("INSERT INTO import_batches(id,file_sha256,original_filename,raw_workbook,status,uploaded_by) VALUES($1,$2,$3,$4,'reviewing',$5) ON CONFLICT(file_sha256) DO NOTHING")
        .bind(id).bind(&fingerprint).bind(safe_name).bind(&bytes).bind(session.user_id).execute(&mut *tx).await.map_err(|_|request.internal())?;
    if inserted.rows_affected() == 0 {
        tx.rollback().await.map_err(|_| request.internal())?;
        let existing: (Uuid, String, i64, i64, i64, i64, i64) = sqlx::query_as(
            "SELECT b.id,b.status,count(r.id),count(r.id) FILTER(WHERE r.committed_entry_id IS NOT NULL),count(r.id) FILTER(WHERE r.sheet_name<>'Analisa'),count(r.id) FILTER(WHERE r.reviewed_by IS NOT NULL),count(r.id) FILTER(WHERE r.source_fingerprint=ANY($2)) FROM import_batches b LEFT JOIN import_rows r ON r.batch_id=b.id WHERE b.file_sha256=$1 GROUP BY b.id",
        )
        .bind(&fingerprint)
        .bind(fingerprints.keys().cloned().collect::<Vec<_>>())
        .fetch_one(&pool)
        .await
        .map_err(|_| request.internal())?;
        let (existing_id, status, old_count, committed, other_sheets, reviewed, same_fingerprints) =
            existing;
        if committed > 0 || status != "reviewing" {
            let summary = summary(&pool, existing_id, request).await?;
            return Ok((
                StatusCode::OK,
                Json(ImportBatchResponse {
                    data: summary,
                    meta: EmptyMeta::default(),
                }),
            ));
        }
        if other_sheets == 0 && old_count == rows.len() as i64 && same_fingerprints == old_count {
            let summary = summary(&pool, existing_id, request).await?;
            return Ok((
                StatusCode::OK,
                Json(ImportBatchResponse {
                    data: summary,
                    meta: EmptyMeta::default(),
                }),
            ));
        }
        if reviewed > 0 {
            return Err(request.error(
                StatusCode::CONFLICT,
                "IMPORT_REVIEW_ALREADY_STARTED",
                "Batch lama sudah direview; buat batch baru sebelum menerapkan aturan sheet Analisa",
            ));
        }
        let mut restage_tx = pool.begin().await.map_err(|_| request.internal())?;
        sqlx::query("DELETE FROM import_rows WHERE batch_id=$1 AND committed_entry_id IS NULL")
            .bind(existing_id)
            .execute(&mut *restage_tx)
            .await
            .map_err(|_| request.internal())?;
        sqlx::query("UPDATE import_batches SET status='reviewing',version=version+1,variance_idr=NULL,variance_reason=NULL,approved_by=NULL,approved_at=NULL,committed_at=NULL WHERE id=$1")
            .bind(existing_id)
            .execute(&mut *restage_tx)
            .await
            .map_err(|_| request.internal())?;
        sqlx::query("INSERT INTO audit_events(id,actor_id,action,entity_type,entity_id,reason,request_id,metadata) VALUES($1,$2,'import.restage_customer_sheets','import_batch',$3,'Terapkan ulang aturan import sheet pelanggan bernama',$4,jsonb_build_object('replaced_rows',$5,'kept_rows',$6))")
            .bind(Uuid::new_v4()).bind(session.user_id).bind(existing_id).bind(request.0).bind(old_count).bind(committed)
            .execute(&mut *restage_tx).await.map_err(|_| request.internal())?;
        restage_tx.commit().await.map_err(|_| request.internal())?;
        id = existing_id;
        tx = pool.begin().await.map_err(|_| request.internal())?;
    }
    let source_fingerprints = fingerprints.keys().cloned().collect::<Vec<_>>();
    let committed_fingerprints: BTreeSet<String> = sqlx::query_scalar(
        "SELECT DISTINCT source_fingerprint FROM import_rows WHERE source_fingerprint=ANY($1) AND committed_entry_id IS NOT NULL",
    )
    .bind(&source_fingerprints)
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| request.internal())?
    .into_iter()
    .collect();
    let staged: Vec<StagedInsert> = rows
        .into_iter()
        .map(|mut row| {
            let duplicate = fingerprints
                .get(&row.fingerprint)
                .copied()
                .unwrap_or_default()
                > 1
                || committed_fingerprints.contains(&row.fingerprint);
            if duplicate && let Some(items) = row.issues.as_array_mut() {
                items.push(json!("duplicate_source_row"));
                items.push(json!("transaction_data_issue"));
            }
            let review_status = if duplicate
                || row
                    .issues
                    .as_array()
                    .is_some_and(|v| v.iter().any(|x| x == "transaction_data_issue"))
            {
                "quarantined"
            } else {
                row.initial_status
            };
            Ok::<_, Failure>(StagedInsert {
                id: Uuid::new_v4(),
                sheet: row.sheet,
                row: row.row,
                source_customer: serde_json::to_value(row.source_customer)
                    .map_err(|_| request.internal())?,
                raw: row.raw,
                parsed: row.parsed,
                issues: row.issues,
                review_status,
                fingerprint: row.fingerprint,
                source_order: row.source_order,
            })
        })
        .collect::<Result<_, _>>()?;
    for chunk in staged.chunks(250) {
        let mut query = QueryBuilder::<Postgres>::new(
            "INSERT INTO import_rows(id,batch_id,sheet_name,row_number,source_customer,raw_values,parsed_values,issues,review_status,source_fingerprint,source_order) ",
        );
        query.push_values(chunk, |mut row, item| {
            row.push_bind(item.id)
                .push_bind(id)
                .push_bind(&item.sheet)
                .push_bind(item.row)
                .push_bind(item.source_customer.clone())
                .push_bind(item.raw.clone())
                .push_bind(item.parsed.clone())
                .push_bind(item.issues.clone())
                .push_bind(item.review_status)
                .push_bind(&item.fingerprint)
                .push_bind(item.source_order);
        });
        query
            .build()
            .execute(&mut *tx)
            .await
            .map_err(|_| request.internal())?;
    }
    tx.commit().await.map_err(|_| request.internal())?;
    Ok((
        StatusCode::CREATED,
        Json(ImportBatchResponse {
            data: summary(&pool, id, request).await?,
            meta: EmptyMeta::default(),
        }),
    ))
}

pub async fn list_batches(
    State(pool): State<PgPool>,
    request: RequestId,
) -> Result<Json<ImportBatchListResponse>, Failure> {
    let ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT id FROM import_batches ORDER BY created_at DESC,id DESC LIMIT 100",
    )
    .fetch_all(&pool)
    .await
    .map_err(|_| request.internal())?;
    let mut data = Vec::with_capacity(ids.len());
    for id in ids {
        data.push(summary(&pool, id, request).await?);
    }
    Ok(Json(ImportBatchListResponse {
        data,
        meta: PageMeta { next_cursor: None },
    }))
}

pub async fn get_batch(
    State(pool): State<PgPool>,
    request: RequestId,
    RouteId(id): RouteId,
) -> Result<Json<ImportBatchResponse>, Failure> {
    Ok(Json(ImportBatchResponse {
        data: summary(&pool, id, request).await?,
        meta: EmptyMeta::default(),
    }))
}

pub async fn list_customer_identities(
    State(pool): State<PgPool>,
    request: RequestId,
    RouteId(batch): RouteId,
) -> Result<Json<ImportCustomerIdentitiesResponse>, Failure> {
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM import_batches WHERE id=$1)")
            .bind(batch)
            .fetch_one(&pool)
            .await
            .map_err(|_| request.internal())?;
    if !exists {
        return Err(not_found(request));
    }
    let source_rows: Vec<ImportIdentityAggregate> = sqlx::query_as(
        "SELECT sheet_name AS source_sheet,source_customer,count(*)::bigint AS source_rows,coalesce(sum(nullif(parsed_values->>'debit_idr','')::bigint),0)::bigint AS source_debit_idr,coalesce(sum(nullif(parsed_values->>'credit_idr','')::bigint),0)::bigint AS source_credit_idr,coalesce(sum(nullif(parsed_values->>'debit_idr','')::bigint) FILTER(WHERE review_status<>'committed'),0)::bigint AS pending_debit_idr,coalesce(sum(nullif(parsed_values->>'credit_idr','')::bigint) FILTER(WHERE review_status<>'committed'),0)::bigint AS pending_credit_idr,coalesce(sum(nullif(parsed_values->>'debit_idr','')::bigint) FILTER(WHERE review_status IN ('candidate','accepted')),0)::bigint AS valid_debit_idr,coalesce(sum(nullif(parsed_values->>'credit_idr','')::bigint) FILTER(WHERE review_status IN ('candidate','accepted')),0)::bigint AS valid_credit_idr,count(*) FILTER(WHERE review_status='candidate')::bigint AS candidate_rows,count(*) FILTER(WHERE review_status='accepted')::bigint AS accepted_rows,count(*) FILTER(WHERE review_status IN ('candidate','accepted'))::bigint AS valid_rows,count(*) FILTER(WHERE review_status='quarantined')::bigint AS quarantined_rows,count(*) FILTER(WHERE review_status='excluded')::bigint AS excluded_rows,count(*) FILTER(WHERE review_status='committed')::bigint AS committed_rows,count(*) FILTER(WHERE review_status<>'committed' AND (parsed_values IS NULL OR parsed_values->>'debit_idr' IS NULL OR parsed_values->>'credit_idr' IS NULL))::bigint AS uncomputable_amount_rows,count(*) FILTER(WHERE review_status IN ('candidate','accepted') AND mapped_customer_id IS NOT NULL)::bigint AS mapped_valid_rows FROM import_rows WHERE batch_id=$1 GROUP BY sheet_name,source_customer ORDER BY sheet_name",
    )
    .bind(batch)
    .fetch_all(&pool)
    .await
    .map_err(|_| request.internal())?;
    let customers: Vec<ImportCustomerMatchRow> =
        sqlx::query_as("SELECT id,code,normalized_code,name,is_active FROM customers ORDER BY id")
            .fetch_all(&pool)
            .await
            .map_err(|_| request.internal())?;

    let sources: Vec<(String, ImportCustomerSource, ImportIdentityAggregate)> = source_rows
        .into_iter()
        .map(|aggregate| {
            Ok((
                aggregate.source_sheet.clone(),
                serde_json::from_value(aggregate.source_customer.clone())
                    .map_err(|_| request.internal())?,
                aggregate,
            ))
        })
        .collect::<Result<_, Failure>>()?;
    let mut names_to_codes = BTreeMap::<String, BTreeSet<String>>::new();
    let mut names_without_code = BTreeMap::<String, usize>::new();
    for (_, source, _) in &sources {
        if let Some(name) = source.name.as_deref() {
            let normalized_name = fold_customer_name(name);
            let normalized_code = source
                .code
                .as_deref()
                .map(str::trim)
                .filter(|code| !code.is_empty())
                .map(fold_customer_code);
            if let Some(code) = normalized_code {
                names_to_codes
                    .entry(normalized_name)
                    .or_default()
                    .insert(code);
            } else {
                *names_without_code.entry(normalized_name).or_default() += 1;
            }
        }
    }

    let data = sources
        .into_iter()
        .map(|(source_sheet, source, aggregate)| {
            let normalized_code = source
                .code
                .as_deref()
                .map(str::trim)
                .filter(|code| !code.is_empty())
                .map(fold_customer_code);
            let normalized_name = source.name.as_deref().map(fold_customer_name);
            let code_matches: Vec<_> = normalized_code
                .as_deref()
                .map(|code| {
                    customers
                        .iter()
                        .filter(|customer| customer.2.as_deref() == Some(code))
                        .collect()
                })
                .unwrap_or_default();
            if let Some(customer) = code_matches.iter().find(|customer| customer.4) {
                return ImportCustomerIdentityPreview {
                    source_sheet,
                    source_code: source.code,
                    source_name: source.name,
                    source_address: source.address,
                    source_rows: aggregate.source_rows,
                    source_debit_idr: aggregate.source_debit_idr.to_string(),
                    source_credit_idr: aggregate.source_credit_idr.to_string(),
                    pending_debit_idr: aggregate.pending_debit_idr.to_string(),
                    pending_credit_idr: aggregate.pending_credit_idr.to_string(),
                    valid_debit_idr: aggregate.valid_debit_idr.to_string(),
                    valid_credit_idr: aggregate.valid_credit_idr.to_string(),
                    candidate_rows: aggregate.candidate_rows,
                    accepted_rows: aggregate.accepted_rows,
                    valid_rows: aggregate.valid_rows,
                    quarantined_rows: aggregate.quarantined_rows,
                    excluded_rows: aggregate.excluded_rows,
                    committed_rows: aggregate.committed_rows,
                    uncomputable_amount_rows: aggregate.uncomputable_amount_rows,
                    mapped_valid_rows: aggregate.mapped_valid_rows,
                    resolution: "matched_code".into(),
                    customer_id: Some(customer.0),
                    customer_name: Some(customer.3.clone()),
                };
            }
            if !code_matches.is_empty() {
                return ImportCustomerIdentityPreview {
                    source_sheet,
                    source_code: source.code,
                    source_name: source.name,
                    source_address: source.address,
                    source_rows: aggregate.source_rows,
                    source_debit_idr: aggregate.source_debit_idr.to_string(),
                    source_credit_idr: aggregate.source_credit_idr.to_string(),
                    pending_debit_idr: aggregate.pending_debit_idr.to_string(),
                    pending_credit_idr: aggregate.pending_credit_idr.to_string(),
                    valid_debit_idr: aggregate.valid_debit_idr.to_string(),
                    valid_credit_idr: aggregate.valid_credit_idr.to_string(),
                    candidate_rows: aggregate.candidate_rows,
                    accepted_rows: aggregate.accepted_rows,
                    valid_rows: aggregate.valid_rows,
                    quarantined_rows: aggregate.quarantined_rows,
                    excluded_rows: aggregate.excluded_rows,
                    committed_rows: aggregate.committed_rows,
                    uncomputable_amount_rows: aggregate.uncomputable_amount_rows,
                    mapped_valid_rows: aggregate.mapped_valid_rows,
                    resolution: "inactive_match".into(),
                    customer_id: None,
                    customer_name: None,
                };
            }
            let name_matches: Vec<_> = normalized_name
                .as_deref()
                .map(|name| {
                    customers
                        .iter()
                        .filter(|customer| fold_customer_name(&customer.3) == name)
                        .collect()
                })
                .unwrap_or_default();
            let (resolution, customer_id, customer_name) =
                if name_matches.len() == 1 && name_matches[0].4 {
                    (
                        "matched_name",
                        Some(name_matches[0].0),
                        Some(name_matches[0].3.clone()),
                    )
                } else if !name_matches.is_empty() {
                    ("ambiguous", None, None)
                } else if source
                    .formula_fields
                    .iter()
                    .any(|field| matches!(field.as_str(), "code" | "name"))
                {
                    ("formula_identity", None, None)
                } else if let Some(name) = normalized_name.as_deref() {
                    let source_code_count = names_to_codes.get(name).map_or(0, BTreeSet::len);
                    let no_code_count = names_without_code.get(name).copied().unwrap_or_default();
                    if source_code_count > 1
                        || no_code_count > 1
                        || (source_code_count > 0 && no_code_count > 0)
                    {
                        ("ambiguous", None, None)
                    } else {
                        ("will_create", None, None)
                    }
                } else {
                    ("missing_identity", None, None)
                };
            ImportCustomerIdentityPreview {
                source_sheet,
                source_code: source.code,
                source_name: source.name,
                source_address: source.address,
                source_rows: aggregate.source_rows,
                source_debit_idr: aggregate.source_debit_idr.to_string(),
                source_credit_idr: aggregate.source_credit_idr.to_string(),
                pending_debit_idr: aggregate.pending_debit_idr.to_string(),
                pending_credit_idr: aggregate.pending_credit_idr.to_string(),
                valid_debit_idr: aggregate.valid_debit_idr.to_string(),
                valid_credit_idr: aggregate.valid_credit_idr.to_string(),
                candidate_rows: aggregate.candidate_rows,
                accepted_rows: aggregate.accepted_rows,
                valid_rows: aggregate.valid_rows,
                quarantined_rows: aggregate.quarantined_rows,
                excluded_rows: aggregate.excluded_rows,
                committed_rows: aggregate.committed_rows,
                uncomputable_amount_rows: aggregate.uncomputable_amount_rows,
                mapped_valid_rows: aggregate.mapped_valid_rows,
                resolution: resolution.into(),
                customer_id,
                customer_name,
            }
        })
        .collect();
    Ok(Json(ImportCustomerIdentitiesResponse {
        data,
        meta: EmptyMeta::default(),
    }))
}

pub async fn prepare_customer_identities(
    State(pool): State<PgPool>,
    Extension(session): Extension<Session>,
    request: RequestId,
    RouteId(batch): RouteId,
) -> Result<Json<PrepareImportCustomerIdentitiesResponse>, Failure> {
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM import_batches WHERE id=$1)")
            .bind(batch)
            .fetch_one(&pool)
            .await
            .map_err(|_| request.internal())?;
    if !exists {
        return Err(not_found(request));
    }
    let empty_sheets: Vec<String> = sqlx::query_scalar(
        "SELECT DISTINCT sheet_name FROM import_rows WHERE batch_id=$1 AND source_customer='{}'::jsonb ORDER BY sheet_name",
    )
    .bind(batch)
    .fetch_all(&pool)
    .await
    .map_err(|_| request.internal())?;
    if empty_sheets.is_empty() {
        return Ok(Json(PrepareImportCustomerIdentitiesResponse {
            data: PrepareImportCustomerIdentitiesData { updated_sheets: 0 },
            meta: EmptyMeta::default(),
        }));
    }
    let workbook: Option<Vec<u8>> =
        sqlx::query_scalar("SELECT raw_workbook FROM import_batches WHERE id=$1")
            .bind(batch)
            .fetch_optional(&pool)
            .await
            .map_err(|_| request.internal())?;
    let workbook = workbook.ok_or_else(|| not_found(request))?;
    let identities = tokio::task::spawn_blocking(move || parse_source_customer_map(&workbook))
        .await
        .map_err(|_| request.internal())?
        .map_err(|_| request.invalid("workbook", "Identitas sheet tidak dapat diproses"))?;

    let mut tx = pool.begin().await.map_err(|_| request.internal())?;
    let status: Option<String> =
        sqlx::query_scalar("SELECT status FROM import_batches WHERE id=$1 FOR UPDATE")
            .bind(batch)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|_| request.internal())?;
    let status = status.ok_or_else(|| not_found(request))?;
    if !matches!(status.as_str(), "reviewing" | "uploaded") {
        return Err(request.error(
            StatusCode::CONFLICT,
            "IMPORT_NOT_REVIEWABLE",
            "Batch sudah ditutup; identitas pelanggan tidak dapat disiapkan ulang",
        ));
    }
    let mut updated_sheets = 0_i64;
    for sheet in empty_sheets {
        let source = identities.get(&sheet).ok_or_else(|| {
            request.invalid("workbook", "Sheet staging tidak ditemukan di workbook")
        })?;
        let updated = sqlx::query("UPDATE import_rows SET source_customer=$3,issues=CASE WHEN $4 THEN issues||jsonb_build_array('formula_customer_identity','transaction_data_issue') ELSE issues END,review_status=CASE WHEN $4 AND review_status='candidate' THEN 'quarantined' ELSE review_status END,version=version+1 WHERE batch_id=$1 AND sheet_name=$2 AND source_customer='{}'::jsonb")
            .bind(batch)
            .bind(&sheet)
            .bind(serde_json::to_value(source).map_err(|_| request.internal())?)
            .bind(
                source
                    .formula_fields
                    .iter()
                    .any(|field| matches!(field.as_str(), "code" | "name")),
            )
            .execute(&mut *tx)
            .await
            .map_err(|_| request.internal())?;
        if updated.rows_affected() > 0 {
            updated_sheets += 1;
        }
    }
    if updated_sheets > 0 {
        sqlx::query("UPDATE import_batches SET version=version+1 WHERE id=$1")
            .bind(batch)
            .execute(&mut *tx)
            .await
            .map_err(|_| request.internal())?;
        sqlx::query("INSERT INTO audit_events(id,actor_id,action,entity_type,entity_id,reason,request_id,metadata) VALUES($1,$2,'import.customer_identity_prepare','import_batch',$3,'Siapkan preview identitas pelanggan',$4,jsonb_build_object('updated_sheets',$5))")
            .bind(Uuid::new_v4())
            .bind(session.user_id)
            .bind(batch)
            .bind(request.0)
            .bind(updated_sheets)
            .execute(&mut *tx)
            .await
            .map_err(|_| request.internal())?;
    }
    tx.commit().await.map_err(|_| request.internal())?;
    Ok(Json(PrepareImportCustomerIdentitiesResponse {
        data: PrepareImportCustomerIdentitiesData { updated_sheets },
        meta: EmptyMeta::default(),
    }))
}

fn parse_source_customer_map(bytes: &[u8]) -> Result<BTreeMap<String, ImportCustomerSource>, ()> {
    zip_preflight(bytes)?;
    let mut workbook = open_workbook_auto_from_rs(Cursor::new(bytes.to_vec())).map_err(|_| ())?;
    let sheets = workbook.sheet_names().to_owned();
    if sheets.is_empty() || sheets.len() > MAX_SHEETS {
        return Err(());
    }
    let mut identities = BTreeMap::new();
    for sheet in sheets {
        let range = workbook.worksheet_range(&sheet).map_err(|_| ())?;
        let formulas = workbook.worksheet_formula(&sheet).ok();
        let mut issues = BTreeSet::new();
        identities.insert(
            sheet,
            parse_source_customer(&range, formulas.as_ref(), &mut issues),
        );
    }
    Ok(identities)
}

pub async fn list_rows(
    State(pool): State<PgPool>,
    request: RequestId,
    RouteId(batch): RouteId,
    Params(query): Params<ImportRowsQuery>,
) -> Result<Json<ImportRowsResponse>, Failure> {
    let (limit, cursor) = customers::pagination(request, query.limit, query.cursor.as_deref())?;
    let status = query.review_status.as_deref();
    let sheet_name = query.sheet_name.as_deref();
    if sheet_name.is_some_and(|sheet| sheet.is_empty() || sheet.len() > 255 || sheet.contains('\0'))
    {
        return Err(request.invalid("sheet_name", "Filter sheet tidak valid"));
    }
    if status.is_some_and(|s| {
        !matches!(
            s,
            "candidate" | "accepted" | "valid" | "quarantined" | "excluded" | "committed"
        )
    }) {
        return Err(request.invalid("review_status", "Status filter tidak valid"));
    }
    let cursor_order = if let Some(cursor) = cursor {
        Some(
            sqlx::query_scalar::<_, i64>(
                "SELECT source_order FROM import_rows WHERE id=$1 AND batch_id=$2 AND ($3::text IS NULL OR sheet_name=$3)",
            )
            .bind(cursor)
            .bind(batch)
            .bind(sheet_name)
            .fetch_optional(&pool)
            .await
            .map_err(|_| request.internal())?
            .ok_or_else(|| request.invalid("cursor", "Cursor tidak valid untuk batch ini"))?,
        )
    } else {
        None
    };
    let rows:Vec<RowDb>=sqlx::query_as("SELECT id,sheet_name,row_number,source_customer,raw_values,parsed_values,issues,review_status,mapped_customer_id,review_note,version FROM import_rows WHERE batch_id=$1 AND ($2::text IS NULL OR ($2='valid' AND review_status IN ('candidate','accepted')) OR review_status=$2) AND ($3::text IS NULL OR sheet_name=$3) AND ($4::bigint IS NULL OR source_order>$4) ORDER BY source_order,id LIMIT $5")
        .bind(batch).bind(status).bind(sheet_name).bind(cursor_order).bind(i64::from(limit)+1).fetch_all(&pool).await.map_err(|_|request.internal())?;
    let mut data: Vec<ImportRow> = rows.into_iter().map(Into::into).collect();
    let next = customers::trim_page(&mut data, limit, |row| row.id);
    if data.is_empty() {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM import_batches WHERE id=$1)")
                .bind(batch)
                .fetch_one(&pool)
                .await
                .map_err(|_| request.internal())?;
        if !exists {
            return Err(not_found(request));
        }
    }
    Ok(Json(ImportRowsResponse {
        data,
        meta: PageMeta { next_cursor: next },
    }))
}

enum SourceCustomerResolution {
    Existing(Uuid),
    Create,
    Unresolved(&'static str),
}

#[derive(sqlx::FromRow)]
struct ImportIdentityAggregate {
    source_sheet: String,
    source_customer: Value,
    source_rows: i64,
    source_debit_idr: i64,
    source_credit_idr: i64,
    pending_debit_idr: i64,
    pending_credit_idr: i64,
    valid_debit_idr: i64,
    valid_credit_idr: i64,
    candidate_rows: i64,
    accepted_rows: i64,
    valid_rows: i64,
    quarantined_rows: i64,
    excluded_rows: i64,
    committed_rows: i64,
    uncomputable_amount_rows: i64,
    mapped_valid_rows: i64,
}

async fn resolve_source_customer(
    tx: &mut Transaction<'_, Postgres>,
    batch: Uuid,
    source_sheet: &str,
    source: &ImportCustomerSource,
    request: RequestId,
) -> Result<SourceCustomerResolution, Failure> {
    if source
        .formula_fields
        .iter()
        .any(|field| matches!(field.as_str(), "code" | "name"))
    {
        return Ok(SourceCustomerResolution::Unresolved(
            "Identitas sumber berisi formula; petakan pelanggan secara manual",
        ));
    }
    let normalized_code = source
        .code
        .as_deref()
        .map(str::trim)
        .filter(|code| !code.is_empty())
        .map(fold_customer_code);
    if let Some(code) = normalized_code.as_deref() {
        let matches: Vec<(Uuid, bool)> =
            sqlx::query_as("SELECT id,is_active FROM customers WHERE normalized_code=$1")
                .bind(code)
                .fetch_all(&mut **tx)
                .await
                .map_err(|_| request.internal())?;
        if let Some((id, true)) = matches.first() {
            return Ok(SourceCustomerResolution::Existing(*id));
        }
        if !matches.is_empty() {
            return Ok(SourceCustomerResolution::Unresolved(
                "Kode sumber cocok ke pelanggan nonaktif; pilih pelanggan aktif secara manual",
            ));
        }
    }
    let Some(normalized_name) = source
        .name
        .as_deref()
        .map(fold_customer_name)
        .filter(|name| !name.is_empty())
    else {
        return Ok(SourceCustomerResolution::Unresolved(
            "Nama sumber tidak tersedia; petakan pelanggan secara manual",
        ));
    };
    let mut lock_keys = BTreeSet::new();
    lock_keys.insert(format!("name:{normalized_name}"));
    if let Some(code) = normalized_code.as_deref() {
        lock_keys.insert(format!("code:{code}"));
    }
    for key in lock_keys {
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
            .bind(key)
            .execute(&mut **tx)
            .await
            .map_err(|_| request.internal())?;
    }
    if let Some(code) = normalized_code.as_deref() {
        let matches: Vec<(Uuid, bool)> =
            sqlx::query_as("SELECT id,is_active FROM customers WHERE normalized_code=$1")
                .bind(code)
                .fetch_all(&mut **tx)
                .await
                .map_err(|_| request.internal())?;
        if let Some((id, true)) = matches.first() {
            return Ok(SourceCustomerResolution::Existing(*id));
        }
        if !matches.is_empty() {
            return Ok(SourceCustomerResolution::Unresolved(
                "Kode sumber cocok ke pelanggan nonaktif; pilih pelanggan aktif secara manual",
            ));
        }
    }
    let existing: Vec<(Uuid, String, bool)> =
        sqlx::query_as("SELECT id,name,is_active FROM customers")
            .fetch_all(&mut **tx)
            .await
            .map_err(|_| request.internal())?;
    let name_matches: Vec<_> = existing
        .iter()
        .filter(|customer| fold_customer_name(&customer.1) == normalized_name)
        .collect();
    if name_matches.len() == 1 {
        if name_matches[0].2 {
            return Ok(SourceCustomerResolution::Existing(name_matches[0].0));
        }
        return Ok(SourceCustomerResolution::Unresolved(
            "Nama sumber cocok ke pelanggan nonaktif; pilih pelanggan aktif secara manual",
        ));
    }
    if !name_matches.is_empty() {
        return Ok(SourceCustomerResolution::Unresolved(
            "Nama sumber cocok ke beberapa pelanggan; petakan secara manual",
        ));
    }
    let batch_sources: Vec<(String, Value)> = sqlx::query_as(
        "SELECT DISTINCT sheet_name,source_customer FROM import_rows WHERE batch_id=$1",
    )
    .bind(batch)
    .fetch_all(&mut **tx)
    .await
    .map_err(|_| request.internal())?;
    for (sheet, value) in batch_sources {
        if sheet == source_sheet {
            continue;
        }
        let other: ImportCustomerSource =
            serde_json::from_value(value).map_err(|_| request.internal())?;
        if other.name.as_deref().map(fold_customer_name).as_deref()
            != Some(normalized_name.as_str())
        {
            continue;
        }
        let other_code = other
            .code
            .as_deref()
            .map(str::trim)
            .filter(|code| !code.is_empty())
            .map(fold_customer_code);
        if normalized_code.is_none() || other_code.is_none() || other_code != normalized_code {
            return Ok(SourceCustomerResolution::Unresolved(
                "Nama sumber tidak unik di workbook; petakan pelanggan secara manual",
            ));
        }
    }
    Ok(SourceCustomerResolution::Create)
}

fn source_identity_key(source: &ImportCustomerSource) -> Option<String> {
    source
        .code
        .as_deref()
        .map(str::trim)
        .filter(|code| !code.is_empty())
        .map(fold_customer_code)
        .map(|code| format!("code:{code}"))
        .or_else(|| {
            source
                .name
                .as_deref()
                .map(fold_customer_name)
                .filter(|name| !name.is_empty())
                .map(|name| format!("name:{name}"))
        })
}

async fn create_source_customer(
    tx: &mut Transaction<'_, Postgres>,
    batch: Uuid,
    source: &ImportCustomerSource,
    actor: Uuid,
    request: RequestId,
) -> Result<Uuid, Failure> {
    let identity = CustomerIdentity::new(
        source.name.as_deref().unwrap_or_default(),
        source.code.as_deref(),
        source.address.as_deref(),
    )
    .map_err(|_| {
        request.invalid(
            "mapped_customer_id",
            "Nama sumber kosong; mapping pelanggan Owner diperlukan",
        )
    })?;
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO customers(id,code,normalized_code,name,address,created_by) VALUES($1,$2,$3,$4,$5,$6)")
        .bind(id)
        .bind(identity.code)
        .bind(identity.normalized_code)
        .bind(identity.name)
        .bind(identity.address)
        .bind(actor)
        .execute(&mut **tx)
        .await
        .map_err(|_| request.error(
            StatusCode::CONFLICT,
            "IMPORT_CUSTOMER_CONFLICT",
            "Identitas pelanggan berubah sejak preview; muat ulang dan tinjau ulang batch",
        ))?;
    sqlx::query("INSERT INTO audit_events(id,actor_id,action,entity_type,entity_id,reason,request_id,metadata) VALUES($1,$2,'import.customer_create','customer',$3,'Created during Owner-approved import commit',$4,jsonb_build_object('batch_id',$5::uuid))")
        .bind(Uuid::new_v4())
        .bind(actor)
        .bind(id)
        .bind(request.0)
        .bind(batch)
        .execute(&mut **tx)
        .await
        .map_err(|_| request.internal())?;
    Ok(id)
}

pub async fn review_row(
    State(pool): State<PgPool>,
    Extension(session): Extension<Session>,
    request: RequestId,
    Path((batch, row_id)): Path<(Uuid, Uuid)>,
    Body(input): Body<ReviewImportRow>,
) -> Result<Json<ImportReviewResponse>, Failure> {
    let reason = clean_reason(request, &input.reason)?;
    if input.expected_version <= 0 {
        return Err(request.invalid("expected_version", "Versi baris tidak valid"));
    }
    if input
        .review_note
        .as_ref()
        .is_some_and(|n| n.len() > 1000 || n.contains('\0'))
    {
        return Err(request.invalid("review_note", "Catatan maksimal 1.000 karakter"));
    }
    if matches!(input.action, ImportReviewAction::Override)
        && input.overrides.as_ref().is_none_or(|v| !v.is_object())
    {
        return Err(request.invalid(
            "overrides",
            "Override harus berupa objek field yang diperiksa",
        ));
    }
    if !matches!(input.action, ImportReviewAction::Override) && input.overrides.is_some() {
        return Err(request.invalid("overrides", "Overrides hanya berlaku untuk aksi override"));
    }
    let status = match input.action {
        ImportReviewAction::Accept => "accepted",
        ImportReviewAction::Exclude => "excluded",
        ImportReviewAction::Quarantine => "quarantined",
        ImportReviewAction::Override => "accepted",
    };
    let mut tx = pool.begin().await.map_err(|_| request.internal())?;
    let current: Option<ImportReviewRow> = sqlx::query_as("SELECT version,issues,parsed_values,source_fingerprint,review_status,sheet_name,source_customer FROM import_rows WHERE id=$1 AND batch_id=$2 FOR UPDATE").bind(row_id).bind(batch).fetch_optional(&mut *tx).await.map_err(|_|request.internal())?;
    let (version, issues, parsed, fingerprint, old_status, source_sheet, source) =
        current.ok_or_else(|| not_found(request))?;
    if version != input.expected_version {
        return Err(request.error(
            StatusCode::CONFLICT,
            "VERSION_CONFLICT",
            "Baris staging sudah berubah",
        ));
    }
    let source_customer: ImportCustomerSource =
        serde_json::from_value(source).map_err(|_| request.internal())?;
    let mut mapped_customer_id = input.mapped_customer_id;
    if matches!(
        input.action,
        ImportReviewAction::Accept | ImportReviewAction::Override
    ) && mapped_customer_id.is_none()
    {
        mapped_customer_id =
            match resolve_source_customer(&mut tx, batch, &source_sheet, &source_customer, request)
                .await?
            {
                SourceCustomerResolution::Existing(id) => Some(id),
                SourceCustomerResolution::Create => None,
                SourceCustomerResolution::Unresolved(message) => {
                    return Err(request.invalid("mapped_customer_id", message));
                }
            };
    }
    if let Some(customer) = mapped_customer_id {
        let active: Option<bool> =
            sqlx::query_scalar("SELECT is_active FROM customers WHERE id=$1")
                .bind(customer)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|_| request.internal())?;
        if active != Some(true) {
            return Err(request.invalid("mapped_customer_id", "Pelanggan harus ada dan aktif"));
        }
    }
    if matches!(input.action, ImportReviewAction::Accept)
        && (old_status == "quarantined"
            || issues.as_array().is_some_and(|xs| {
                xs.iter()
                    .any(|x| x == "transaction_data_issue" || x == "duplicate_source_row")
            }))
    {
        return Err(request.invalid(
            "action",
            "Baris dengan exception memerlukan override Owner atau tetap dikarantina",
        ));
    }
    if matches!(
        input.action,
        ImportReviewAction::Accept | ImportReviewAction::Override
    ) && parsed.is_none()
    {
        return Err(request.invalid(
            "action",
            "Baris ini tidak memiliki kandidat transaksi yang dapat diterima",
        ));
    }
    let normalized = if let Some(overrides) = input.overrides {
        merge_override(request, parsed, overrides)?
    } else {
        parsed
    };
    let business_today: chrono::NaiveDate =
        sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Asia/Jakarta')::date")
            .fetch_one(&mut *tx)
            .await
            .map_err(|_| request.internal())?;
    if let Some(values) = normalized.as_ref() {
        validate_candidate(request, values, business_today)?;
    }
    sqlx::query("UPDATE import_rows SET review_status=$3,mapped_customer_id=$4,parsed_values=$5,reviewed_by=$6,review_reason=$7,review_note=$8,version=version+1 WHERE id=$1 AND batch_id=$2")
        .bind(row_id).bind(batch).bind(status).bind(mapped_customer_id).bind(normalized).bind(session.user_id).bind(reason).bind(input.review_note).execute(&mut *tx).await.map_err(|_|request.internal())?;
    sqlx::query("UPDATE import_batches SET version=version+1,status='reviewing' WHERE id=$1 AND status IN ('reviewing','uploaded')").bind(batch).execute(&mut *tx).await.map_err(|_|request.internal())?;
    sqlx::query("INSERT INTO audit_events(id,actor_id,action,entity_type,entity_id,reason,request_id,metadata) VALUES($1,$2,'import.row_review','import_row',$3,$4,$5,jsonb_build_object('batch_id',$6::uuid,'action',$7,'source_fingerprint',$8))")
        .bind(Uuid::new_v4()).bind(session.user_id).bind(row_id).bind(reason).bind(request.0).bind(batch).bind(format!("{input_action:?}",input_action=input.action).to_ascii_lowercase()).bind(fingerprint).execute(&mut *tx).await.map_err(|_|request.internal())?;
    tx.commit().await.map_err(|_| request.internal())?;
    let row:RowDb=sqlx::query_as("SELECT id,sheet_name,row_number,source_customer,raw_values,parsed_values,issues,review_status,mapped_customer_id,review_note,version FROM import_rows WHERE id=$1").bind(row_id).fetch_one(&pool).await.map_err(|_|request.internal())?;
    Ok(Json(ImportReviewResponse {
        data: row.into(),
        meta: EmptyMeta::default(),
    }))
}

pub async fn commit_batch(
    State(pool): State<PgPool>,
    Extension(session): Extension<Session>,
    request: RequestId,
    RouteId(batch): RouteId,
    headers: HeaderMap,
    Body(body): Body<Value>,
) -> Result<(StatusCode, Json<Value>), Failure> {
    let key = RetryKey::new(
        request,
        &headers,
        format!("import-batches:{batch}:commit"),
        &body,
    )?;
    let input: CommitImport = decode(request, body.clone())?;
    if input.row_ids.is_empty()
        || input.row_ids.len() > MAX_SOURCE_ROWS
        || input.expected_version <= 0
    {
        return Err(request.invalid("row_ids", "Pilih 1–25.000 baris dan versi batch yang valid"));
    }
    let unique: BTreeSet<Uuid> = input.row_ids.iter().copied().collect();
    if unique.len() != input.row_ids.len() {
        return Err(request.invalid("row_ids", "ID baris tidak boleh duplikat"));
    }
    let provided_variance = input
        .variance_idr
        .as_deref()
        .map(|v| parse_signed_money(request, v))
        .transpose()?;
    let variance_reason = input
        .variance_reason
        .as_deref()
        .map(|v| clean_reason(request, v))
        .transpose()?;
    let mut tx = pool
        .begin()
        .await
        .map_err(|error| commit_db_failure(request, "begin_transaction", error))?;
    if let Some(saved) = key.replay(&mut tx, session.user_id, request).await? {
        return Ok(saved);
    }
    let batch_row: Option<(String, i32)> =
        sqlx::query_as("SELECT status,version FROM import_batches WHERE id=$1 FOR UPDATE")
            .bind(batch)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|error| commit_db_failure(request, "lock_batch", error))?;
    let (batch_status, version) = batch_row.ok_or_else(|| not_found(request))?;
    if !matches!(batch_status.as_str(), "reviewing" | "uploaded" | "approved") {
        return Err(request.error(
            StatusCode::CONFLICT,
            "IMPORT_NOT_COMMITTABLE",
            "Batch sudah ditutup atau dikomit",
        ));
    }
    if version != input.expected_version {
        return Err(request.error(
            StatusCode::CONFLICT,
            "VERSION_CONFLICT",
            "Batch berubah sejak preview dimuat",
        ));
    }
    let rows: Vec<ImportCommitRow> = sqlx::query_as("SELECT id,mapped_customer_id,parsed_values,source_fingerprint,review_status,source_order,sheet_name,source_customer FROM import_rows WHERE batch_id=$1 AND id=ANY($2) ORDER BY source_order FOR UPDATE")
        .bind(batch).bind(&input.row_ids).fetch_all(&mut *tx).await.map_err(|error|commit_db_failure(request,"lock_import_rows",error))?;
    if rows.len() != input.row_ids.len()
        || rows
            .iter()
            .any(|r| !matches!(r.4.as_str(), "accepted" | "candidate"))
    {
        return Err(request.invalid(
            "row_ids",
            "Pilih semua baris kandidat valid/diterima; karantina dan pengecualian tetap di luar commit",
        ));
    }
    let selected_sheets = rows
        .iter()
        .map(|row| row.6.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let expected_row_ids: BTreeSet<Uuid> = sqlx::query_scalar(
        "SELECT id FROM import_rows WHERE batch_id=$1 AND sheet_name=ANY($2) AND review_status IN ('candidate','accepted')",
    )
    .bind(batch)
    .bind(&selected_sheets)
    .fetch_all(&mut *tx)
    .await
    .map_err(|error| commit_db_failure(request, "validate_selected_customer_rows", error))?
    .into_iter()
    .collect();
    if expected_row_ids != unique {
        return Err(request.invalid(
            "row_ids",
            "Pilih seluruh baris kandidat/diterima dari pelanggan yang dipilih; pelanggan lain dapat dikomit nanti",
        ));
    }
    // Capture the source side before selected rows transition to `committed` below.
    let (source_debit, source_credit, unknown_count): (i64, i64, i64) = sqlx::query_as(
        "SELECT coalesce(sum(coalesce(nullif(parsed_values->>'debit_idr','')::bigint,0)),0)::bigint,coalesce(sum(coalesce(nullif(parsed_values->>'credit_idr','')::bigint,0)),0)::bigint,count(*) FILTER(WHERE parsed_values IS NULL OR parsed_values->>'debit_idr' IS NULL OR parsed_values->>'credit_idr' IS NULL) FROM import_rows WHERE batch_id=$1 AND sheet_name=ANY($2) AND review_status<>'committed'",
    )
    .bind(batch)
    .bind(&selected_sheets)
    .fetch_one(&mut *tx)
    .await
    .map_err(|error| commit_db_failure(request, "reconcile_selected_customers", error))?;
    let source_fingerprints = rows.iter().map(|row| row.3.clone()).collect::<Vec<_>>();
    let committed_fingerprints: BTreeSet<String> = sqlx::query_scalar(
        "SELECT DISTINCT source_fingerprint FROM import_rows WHERE source_fingerprint=ANY($1) AND committed_entry_id IS NOT NULL",
    )
    .bind(&source_fingerprints)
    .fetch_all(&mut *tx)
    .await
    .map_err(|error| commit_db_failure(request, "check_committed_fingerprints", error))?
    .into_iter()
    .collect();
    if rows
        .iter()
        .any(|row| committed_fingerprints.contains(&row.3))
    {
        return Err(request.error(
            StatusCode::CONFLICT,
            "DUPLICATE_SOURCE_ROW",
            "Baris sumber pernah dikomit; Owner harus meninjau ulang dan membuat batch revision sesuai kebijakan",
        ));
    }
    let mapped_customer_ids = rows.iter().filter_map(|row| row.1).collect::<BTreeSet<_>>();
    let mapped_ids = mapped_customer_ids.iter().copied().collect::<Vec<_>>();
    let active_customer_ids: BTreeSet<Uuid> =
        sqlx::query_scalar("SELECT id FROM customers WHERE id=ANY($1) AND is_active")
            .bind(&mapped_ids)
            .fetch_all(&mut *tx)
            .await
            .map_err(|error| commit_db_failure(request, "validate_mapped_customers", error))?
            .into_iter()
            .collect();
    if active_customer_ids.len() != mapped_customer_ids.len() {
        return Err(request.invalid("mapped_customer_id", "Pelanggan tujuan harus ada dan aktif"));
    }
    let today: chrono::NaiveDate =
        sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Asia/Jakarta')::date")
            .fetch_one(&mut *tx)
            .await
            .map_err(|error| commit_db_failure(request, "read_business_date", error))?;
    let mut selected_net = 0_i64;
    let mut created_customers = 0_i64;
    let mut resolved_customers = BTreeMap::<String, Uuid>::new();
    let mut committed = Vec::<PreparedImportEntry>::with_capacity(rows.len());
    for (
        row_id,
        mapped_customer_id,
        values,
        _fingerprint,
        _,
        source_order,
        source_sheet,
        source_value,
    ) in &rows
    {
        let source_customer: ImportCustomerSource =
            serde_json::from_value(source_value.clone()).map_err(|_| request.internal())?;
        let customer_id = if let Some(id) = mapped_customer_id {
            *id
        } else {
            let identity_key = source_identity_key(&source_customer);
            if let Some(id) = identity_key
                .as_ref()
                .and_then(|key| resolved_customers.get(key))
            {
                *id
            } else {
                let id = match resolve_source_customer(
                    &mut tx,
                    batch,
                    source_sheet,
                    &source_customer,
                    request,
                )
                .await?
                {
                    SourceCustomerResolution::Existing(id) => id,
                    SourceCustomerResolution::Create => {
                        let id = create_source_customer(
                            &mut tx,
                            batch,
                            &source_customer,
                            session.user_id,
                            request,
                        )
                        .await?;
                        created_customers += 1;
                        id
                    }
                    SourceCustomerResolution::Unresolved(message) => {
                        sqlx::query("UPDATE import_rows SET review_status='quarantined',issues=issues||jsonb_build_array('customer_identity_unresolved','transaction_data_issue'),reviewed_by=$2,review_reason=$3,version=version+1 WHERE id=$1 AND review_status='candidate'")
                            .bind(row_id)
                            .bind(session.user_id)
                            .bind(message)
                            .execute(&mut *tx)
                            .await
                            .map_err(|_| request.internal())?;
                        continue;
                    }
                };
                if let Some(identity_key) = identity_key {
                    resolved_customers.insert(identity_key, id);
                }
                id
            }
        };
        let (date, debit, credit) = candidate_parts(request, values)?;
        if date.is_none() || debit < 0 || credit < 0 || (debit > 0) == (credit > 0) {
            return Err(request.invalid(
                "row_ids",
                "Kandidat transaksi harus memiliki tanggal dan tepat satu debit/kredit positif",
            ));
        }
        if date.is_some_and(|d| d > today) {
            return Err(request.invalid(
                "effective_date",
                "Tanggal historis tidak boleh di masa depan",
            ));
        }
        selected_net = selected_net
            .checked_add(debit - credit)
            .ok_or_else(|| request.internal())?;
        committed.push(PreparedImportEntry {
            row_id: *row_id,
            customer_id,
            ledger_id: Uuid::new_v4(),
            effective_date: date.expect("date was validated above"),
            debit,
            credit,
            source_order: *source_order,
            description: values
                .get("description")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned),
            quantity: values
                .get("quantity")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned),
            unit_price_idr: values
                .get("unit_price_idr")
                .and_then(Value::as_str)
                .and_then(|value| parse_signed_money_value(value).ok()),
        });
    }
    for chunk in committed.chunks(250) {
        let mut query = QueryBuilder::<Postgres>::new(
            "INSERT INTO receivable_entries(id,customer_id,entry_type,source_id,effective_date,debit_idr,credit_idr,source_order,created_by,description,quantity,unit_price_idr) ",
        );
        query.push_values(chunk, |mut row, entry| {
            row.push_bind(entry.ledger_id)
                .push_bind(entry.customer_id)
                .push_bind(if entry.debit > 0 {
                    "legacy_debit"
                } else {
                    "legacy_payment"
                })
                .push_bind(entry.row_id)
                .push_bind(entry.effective_date)
                .push_bind(entry.debit)
                .push_bind(entry.credit)
                .push_bind(entry.source_order)
                .push_bind(session.user_id)
                .push_bind(&entry.description)
                .push_bind(&entry.quantity)
                .push_bind(entry.unit_price_idr);
        });
        query
            .build()
            .execute(&mut *tx)
            .await
            .map_err(|error| commit_db_failure(request, "insert_receivable_entries", error))?;
    }
    for chunk in committed.chunks(250) {
        let mut query = QueryBuilder::<Postgres>::new(
            "INSERT INTO import_row_commits(import_row_id,ledger_entry_id) ",
        );
        query.push_values(chunk, |mut row, entry| {
            row.push_bind(entry.row_id).push_bind(entry.ledger_id);
        });
        query
            .build()
            .execute(&mut *tx)
            .await
            .map_err(|error| commit_db_failure(request, "insert_import_row_commits", error))?;
    }
    for chunk in committed.chunks(250) {
        let mut query = QueryBuilder::<Postgres>::new(
            "UPDATE import_rows AS ir SET review_status='committed',mapped_customer_id=v.customer_id,committed_entry_id=v.ledger_entry_id,reviewed_by=coalesce(ir.reviewed_by, ",
        );
        query.push_bind(session.user_id);
        query.push("),version=ir.version+1 FROM (");
        query.push_values(chunk, |mut row, entry| {
            row.push_bind(entry.row_id)
                .push_bind(entry.customer_id)
                .push_bind(entry.ledger_id);
        });
        query.push(") AS v(row_id,customer_id,ledger_entry_id) WHERE ir.id=v.row_id");
        query
            .build()
            .execute(&mut *tx)
            .await
            .map_err(|error| commit_db_failure(request, "update_import_rows", error))?;
    }
    let mut open_debits = BTreeMap::<Uuid, VecDeque<(Uuid, i64)>>::new();
    let mut open_credits = BTreeMap::<Uuid, VecDeque<(Uuid, i64)>>::new();
    let mut legacy_allocations = Vec::<PreparedLegacyAllocation>::new();
    for entry in &committed {
        if entry.debit > 0 {
            let mut remaining_debit = entry.debit;
            if let Some(credits) = open_credits.get_mut(&entry.customer_id) {
                while remaining_debit > 0 {
                    let Some((payment_id, open_credit)) = credits.pop_front() else {
                        break;
                    };
                    let allocation = remaining_debit.min(open_credit);
                    legacy_allocations.push(PreparedLegacyAllocation {
                        payment_entry_id: payment_id,
                        debit_entry_id: entry.ledger_id,
                        amount: allocation,
                    });
                    remaining_debit -= allocation;
                    if open_credit > allocation {
                        credits.push_front((payment_id, open_credit - allocation));
                    }
                }
            }
            if remaining_debit > 0 {
                open_debits
                    .entry(entry.customer_id)
                    .or_default()
                    .push_back((entry.ledger_id, remaining_debit));
            }
        } else {
            let mut remaining_credit = entry.credit;
            if let Some(debits) = open_debits.get_mut(&entry.customer_id) {
                while remaining_credit > 0 {
                    let Some((debit_id, open_debit)) = debits.pop_front() else {
                        break;
                    };
                    let allocation = remaining_credit.min(open_debit);
                    legacy_allocations.push(PreparedLegacyAllocation {
                        payment_entry_id: entry.ledger_id,
                        debit_entry_id: debit_id,
                        amount: allocation,
                    });
                    remaining_credit -= allocation;
                    if open_debit > allocation {
                        debits.push_front((debit_id, open_debit - allocation));
                    }
                }
            }
            if remaining_credit > 0 {
                open_credits
                    .entry(entry.customer_id)
                    .or_default()
                    .push_back((entry.ledger_id, remaining_credit));
            }
        }
    }
    for chunk in legacy_allocations.chunks(250) {
        let mut query = QueryBuilder::<Postgres>::new(
            "INSERT INTO legacy_credit_allocations(id,payment_entry_id,debit_entry_id,amount_idr,status,created_by) ",
        );
        query.push_values(chunk, |mut row, allocation| {
            row.push_bind(Uuid::new_v4())
                .push_bind(allocation.payment_entry_id)
                .push_bind(allocation.debit_entry_id)
                .push_bind(allocation.amount)
                .push_bind("active")
                .push_bind(session.user_id);
        });
        query.build().execute(&mut *tx).await.map_err(|error| {
            commit_db_failure(request, "insert_legacy_credit_allocations", error)
        })?;
    }
    if committed.is_empty() {
        return Err(request.invalid(
            "row_ids",
            "Tidak ada baris dengan transaksi dan identitas pelanggan yang dapat diimpor",
        ));
    }
    let known_variance = source_debit
        .checked_sub(source_credit)
        .and_then(|net| net.checked_sub(selected_net))
        .ok_or_else(|| request.internal())?;
    let variance = if unknown_count > 0 {
        provided_variance.ok_or_else(||request.invalid("variance_idr","Baris sumber yang belum dapat dihitung memerlukan variance Owner dan alasan eksplisit"))?
    } else {
        if known_variance != provided_variance.unwrap_or(0) {
            let message = format!(
                "Variance harus sama dengan selisih total sumber dan baris yang dipilih. Selisih terhitung: {known_variance} IDR.",
            );
            return Err(request.invalid("variance_idr", &message));
        }
        known_variance
    };
    if variance != 0 && variance_reason.is_none() {
        return Err(request.invalid(
            "variance_reason",
            "Variance nonzero memerlukan persetujuan dan alasan Owner",
        ));
    }
    let remaining_valid_rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM import_rows WHERE batch_id=$1 AND review_status IN ('candidate','accepted')",
    )
    .bind(batch)
    .fetch_one(&mut *tx)
    .await
    .map_err(|error| commit_db_failure(request, "count_remaining_valid_rows", error))?;
    let remaining_exception_rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM import_rows WHERE batch_id=$1 AND review_status IN ('quarantined','excluded')",
    )
    .bind(batch)
    .fetch_one(&mut *tx)
    .await
    .map_err(|error| commit_db_failure(request, "count_remaining_exceptions", error))?;
    let remaining_reviewable_rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM import_rows WHERE batch_id=$1 AND review_status IN ('candidate','accepted','quarantined')",
    )
    .bind(batch)
    .fetch_one(&mut *tx)
    .await
    .map_err(|error| commit_db_failure(request, "count_remaining_reviewable_rows", error))?;
    let selected_exception_rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM import_rows WHERE batch_id=$1 AND sheet_name=ANY($2) AND review_status IN ('quarantined','excluded')",
    )
    .bind(batch)
    .bind(&selected_sheets)
    .fetch_one(&mut *tx)
    .await
    .map_err(|error| commit_db_failure(request, "count_selected_customer_exceptions", error))?;
    let finalized = remaining_reviewable_rows == 0;
    let status = if !finalized {
        "reviewing"
    } else if remaining_exception_rows > 0 {
        "closed_with_exceptions"
    } else {
        "committed"
    };
    sqlx::query("UPDATE import_batches SET status=$2,approved_by=CASE WHEN $6 THEN $3 ELSE approved_by END,approved_at=CASE WHEN $6 THEN statement_timestamp() ELSE approved_at END,committed_at=CASE WHEN $6 THEN statement_timestamp() ELSE NULL END,variance_idr=CASE WHEN $6 THEN $4 ELSE variance_idr END,variance_reason=CASE WHEN $6 THEN $5 ELSE variance_reason END,version=version+1 WHERE id=$1")
        .bind(batch).bind(status).bind(session.user_id).bind(variance).bind(variance_reason).bind(finalized).execute(&mut *tx).await.map_err(|error|commit_db_failure(request,"update_batch_commit_status",error))?;
    sqlx::query("INSERT INTO audit_events(id,actor_id,action,entity_type,entity_id,reason,request_id,metadata) VALUES($1,$2,'import.commit','import_batch',$3,$4,$5,jsonb_build_object('committed_rows',$6,'excluded_rows',$7,'variance_idr',$8::text,'created_customers',$9,'remaining_valid_rows',$10,'selected_sheets',$11))")
        .bind(Uuid::new_v4()).bind(session.user_id).bind(batch).bind(variance_reason).bind(request.0).bind(committed.len() as i64).bind(selected_exception_rows).bind(variance.to_string()).bind(created_customers).bind(remaining_valid_rows).bind(selected_sheets.len() as i64).execute(&mut *tx).await.map_err(|error|commit_db_failure(request,"insert_import_commit_audit",error))?;
    let result = ImportCommitResponse {
        data: ImportCommitData {
            batch_id: batch,
            status: status.into(),
            committed_rows: committed.len() as i64,
            created_customers,
            variance_idr: variance.to_string(),
        },
        meta: EmptyMeta::default(),
    };
    let response = serde_json::to_value(result).map_err(|_| request.internal())?;
    key.commit(tx, session.user_id, StatusCode::OK, response, request)
        .await
}

#[derive(sqlx::FromRow)]
struct RowDb {
    id: Uuid,
    sheet_name: String,
    row_number: i32,
    source_customer: Value,
    raw_values: Value,
    parsed_values: Option<Value>,
    issues: Value,
    review_status: String,
    mapped_customer_id: Option<Uuid>,
    review_note: Option<String>,
    version: i32,
}
impl From<RowDb> for ImportRow {
    fn from(r: RowDb) -> Self {
        Self {
            id: r.id,
            sheet_name: r.sheet_name,
            row_number: r.row_number,
            source_customer: serde_json::from_value(r.source_customer).unwrap_or_default(),
            raw_values: r.raw_values,
            parsed_values: r.parsed_values,
            issues: r.issues,
            review_status: r.review_status,
            mapped_customer_id: r.mapped_customer_id,
            review_note: r.review_note,
            version: r.version,
        }
    }
}

async fn summary(
    pool: &PgPool,
    id: Uuid,
    request: RequestId,
) -> Result<ImportBatchSummary, Failure> {
    let head:Option<(String,String,String,chrono::DateTime<chrono::Utc>,i32)>=sqlx::query_as("SELECT file_sha256,original_filename,status,created_at,version FROM import_batches WHERE id=$1").bind(id).fetch_optional(pool).await.map_err(|_|request.internal())?;
    let (hash, filename, status, created, version) = head.ok_or_else(|| not_found(request))?;
    let stats:(i64,i64,i64,i64,i64,i64,i64,i64,i64,i64,i64)=sqlx::query_as("SELECT count(*),count(*) FILTER(WHERE review_status='candidate'),count(*) FILTER(WHERE review_status='quarantined'),count(*) FILTER(WHERE review_status='accepted'),count(*) FILTER(WHERE review_status='excluded'),count(*) FILTER(WHERE review_status='committed'),coalesce(sum(nullif(parsed_values->>'debit_idr','')::bigint),0)::bigint,coalesce(sum(nullif(parsed_values->>'credit_idr','')::bigint),0)::bigint,coalesce(sum(nullif(parsed_values->>'debit_idr','')::bigint) FILTER(WHERE review_status IN ('candidate','accepted','committed')),0)::bigint,coalesce(sum(nullif(parsed_values->>'credit_idr','')::bigint) FILTER(WHERE review_status IN ('candidate','accepted','committed')),0)::bigint,count(*) FILTER(WHERE parsed_values IS NULL OR parsed_values->>'debit_idr' IS NULL OR parsed_values->>'credit_idr' IS NULL) FROM import_rows WHERE batch_id=$1").bind(id).fetch_one(pool).await.map_err(|_|request.internal())?;
    Ok(ImportBatchSummary {
        id,
        file_sha256: hash,
        original_filename: filename,
        status,
        created_at: created,
        version,
        total_rows: stats.0,
        candidate_rows: stats.1,
        quarantined_rows: stats.2,
        accepted_rows: stats.3,
        excluded_rows: stats.4,
        committed_rows: stats.5,
        source_debit_idr: stats.6.to_string(),
        source_credit_idr: stats.7.to_string(),
        selected_debit_idr: stats.8.to_string(),
        selected_credit_idr: stats.9.to_string(),
        unresolved_count: stats.1 + stats.2,
        uncomputable_amount_rows: stats.10,
    })
}

fn zip_preflight(bytes: &[u8]) -> Result<(), ()> {
    let mut zip = ZipArchive::new(Cursor::new(bytes)).map_err(|_| ())?;
    if zip.len() > MAX_ZIP_ENTRIES {
        return Err(());
    }
    let mut total = 0_u64;
    let mut has_workbook = false;
    let mut has_content_types = false;
    for i in 0..zip.len() {
        let entry = zip.by_index(i).map_err(|_| ())?;
        let name = entry.name().map_err(|_| ())?;
        if name.split('/').any(|part| part == "..") || name.contains('\\') {
            return Err(());
        }
        if name.to_ascii_lowercase().ends_with("vbaproject.bin") {
            return Err(());
        }
        has_workbook |= name == "xl/workbook.xml";
        has_content_types |= name == "[Content_Types].xml";
        total = total.checked_add(entry.size()).ok_or(())?;
        if total > MAX_UNCOMPRESSED_BYTES {
            return Err(());
        }
    }
    if !has_workbook || !has_content_types {
        return Err(());
    }
    Ok(())
}

fn parse_workbook(bytes: &[u8], business_today: chrono::NaiveDate) -> Result<Vec<StagedRow>, ()> {
    let mut workbook = open_workbook_auto_from_rs(Cursor::new(bytes.to_vec())).map_err(|_| ())?;
    let sheets = workbook.sheet_names().to_owned();
    if sheets.is_empty() || sheets.len() > MAX_SHEETS {
        return Err(());
    }
    let mut rows = Vec::new();
    let mut source_order = 0_i64;
    for sheet in sheets {
        if sheet.trim().eq_ignore_ascii_case("Analisa") {
            continue;
        }
        if sheet.len() > 255 {
            return Err(());
        }
        let range = workbook.worksheet_range(&sheet).map_err(|_| ())?;
        let formulas = workbook.worksheet_formula(&sheet).ok();
        let (row_start, col_start) = range.start().unwrap_or((0, 0));
        let mut identity_issues = BTreeSet::new();
        let source_customer =
            parse_source_customer(&range, formulas.as_ref(), &mut identity_issues);
        if source_customer.name.as_deref().is_none_or(str::is_empty) {
            continue;
        }
        let mut previous_date: Option<String> = None;
        for (offset, cells) in range.rows().enumerate() {
            if cells.len() > MAX_COLUMNS {
                return Err(());
            }
            let absolute_row = row_start + u32::try_from(offset).map_err(|_| ())?;
            if absolute_row < 6 {
                continue;
            }
            let mut raw = Vec::new();
            let mut issues = identity_issues.clone();
            let mut values = vec![Value::Null; 12];
            let mut has_any = false;
            let mut formula_markers = [false; MAX_COLUMNS];
            for (offset_col, cell) in cells.iter().enumerate() {
                if matches!(cell, Data::Empty) {
                    continue;
                }
                has_any = true;
                let col = col_start + u32::try_from(offset_col).map_err(|_| ())?;
                let formula = formulas
                    .as_ref()
                    .and_then(|f| f.get_value((absolute_row, col)))
                    .filter(|v| !v.is_empty());
                let raw_value = data_json(cell);
                if (col as usize) < formula_markers.len() {
                    formula_markers[col as usize] = formula.is_some();
                }
                raw.push(json!({"column":column_name(col as usize),"value":raw_value,"formula":formula.is_some()}));
                let is_debit_formula_with_value = col == 5
                    && formula.is_some()
                    && parse_integer(&raw_value).is_some_and(|v| v > 0);
                if formula.is_some() && !is_debit_formula_with_value {
                    issues.insert("formula_value_ignored");
                }
                if let Data::Error(_) = cell {
                    issues.insert("cell_error");
                }
                if (col as usize) < values.len() {
                    values[col as usize] = if formula.is_some() && !is_debit_formula_with_value {
                        Value::Null
                    } else {
                        raw_value
                    };
                }
            }
            if !has_any {
                continue;
            }
            if values[0].as_str().is_some_and(|s| {
                matches!(
                    s.trim().to_ascii_lowercase().as_str(),
                    "tanggal" | "tgl" | "date"
                )
            }) {
                continue;
            }
            let has_literal_source_data = values[..=6].iter().any(|value| !value.is_null());
            if !has_literal_source_data {
                continue;
            }
            if rows.len() >= MAX_SOURCE_ROWS {
                return Err(());
            }
            let explicit_date = if values[0].is_null() {
                None
            } else {
                parse_date_value(&values[0])
            };
            let (date, inherited_date) = if let Some(date) = explicit_date {
                if chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d")
                    .is_ok_and(|date| date <= business_today)
                {
                    previous_date = Some(date.clone());
                }
                (Some(date), false)
            } else if values[0].is_null() && !formula_markers[0] {
                let inherited = previous_date.clone();
                (inherited.clone(), inherited.is_some())
            } else {
                (None, false)
            };
            if date.is_none() {
                issues.insert("invalid_or_missing_date");
            } else if inherited_date {
                issues.insert("inherited_previous_date");
            }
            let calculated_debit = || {
                let quantity = parse_quantity_milli(&values[3])?;
                let price = parse_integer(&values[4])?;
                let amount = (i128::from(quantity) * i128::from(price) + 500) / 1000;
                i64::try_from(amount).ok()
            };
            let debit = if formula_markers[5] {
                if parse_integer(&values[6]).is_some_and(|credit| credit > 0) {
                    Some(0)
                } else {
                    parse_integer(&values[5]).or_else(calculated_debit)
                }
            } else if values[5].is_null() {
                calculated_debit().or(Some(0))
            } else {
                parse_integer(&values[5])
            };
            let credit = if values[6].is_null() && !formula_markers[6] {
                Some(0)
            } else {
                parse_integer(&values[6])
            };
            if debit.is_none()
                || credit.is_none()
                || debit.is_some_and(|amount| amount < 0)
                || credit.is_some_and(|amount| amount < 0)
            {
                issues.insert("invalid_amount");
            }
            if debit.unwrap_or(0) == 0 && credit.unwrap_or(0) == 0 {
                issues.insert("invalid_debit_credit");
            }
            if date
                .as_deref()
                .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
                .is_some_and(|d| d > business_today)
            {
                issues.insert("future_business_date");
            }
            if debit.unwrap_or(0) > 0 && credit.unwrap_or(0) > 0 {
                issues.insert("mixed_debit_credit");
            }
            if debit.unwrap_or(0) > 0
                && !formula_markers[5]
                && let (Some(quantity), Some(price)) =
                    (parse_quantity_milli(&values[3]), parse_integer(&values[4]))
            {
                let calculated = (i128::from(quantity) * i128::from(price) + 500) / 1000;
                if calculated != i128::from(debit.unwrap_or_default()) {
                    issues.insert("quantity_price_mismatch");
                }
            }
            let blocking = issues.iter().any(|issue| {
                matches!(
                    *issue,
                    "invalid_or_missing_date"
                        | "invalid_amount"
                        | "invalid_debit_credit"
                        | "mixed_debit_credit"
                        | "quantity_price_mismatch"
                        | "future_business_date"
                        | "cell_error"
                        | "formula_customer_identity"
                )
            });
            let parsed = Some(
                json!({"effective_date":date,"reference_no":values[1],"description":values[2],"quantity":values[3],"unit_price_idr":values[4],"debit_idr":debit.map(|value|value.to_string()),"credit_idr":credit.map(|value|value.to_string())}),
            );
            let raw_json = Value::Array(raw);
            let serialized = serde_json::to_vec(&raw_json).map_err(|_| ())?;
            let fingerprint = hex::encode(Sha256::digest([sheet.as_bytes(), &serialized].concat()));
            source_order += 1;
            let initial_status = if blocking { "quarantined" } else { "candidate" };
            rows.push(StagedRow {
                sheet: sheet.clone(),
                row: i32::try_from(absolute_row + 1).map_err(|_| ())?,
                source_customer: source_customer.clone(),
                raw: raw_json,
                parsed,
                issues: json!(if blocking {
                    vec!["transaction_data_issue"]
                        .into_iter()
                        .chain(issues.iter().copied())
                        .collect::<Vec<_>>()
                } else {
                    issues.iter().copied().collect::<Vec<_>>()
                }),
                fingerprint,
                source_order,
                initial_status,
            });
        }
    }
    Ok(rows)
}

fn parse_source_customer(
    range: &Range<Data>,
    formulas: Option<&Range<String>>,
    issues: &mut BTreeSet<&'static str>,
) -> ImportCustomerSource {
    let mut formula_fields = Vec::new();
    let mut read = |position: (u32, u32), field: &str| {
        if formulas
            .and_then(|sheet| sheet.get_value(position))
            .is_some_and(|formula| !formula.is_empty())
        {
            let cached = range
                .get_value(position)
                .and_then(source_cell_text)
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty());
            if cached.is_none() {
                formula_fields.push(field.to_owned());
                if matches!(field, "code" | "name") {
                    issues.insert("formula_customer_identity");
                }
            }
            return cached;
        }
        range
            .get_value(position)
            .and_then(source_cell_text)
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    };
    let code = read((0, 1), "code");
    let name = read((1, 1), "name");
    let address = read((2, 1), "address");
    ImportCustomerSource {
        code,
        name,
        address,
        formula_fields,
    }
}

fn source_cell_text(value: &Data) -> Option<String> {
    match value {
        Data::String(value) => Some(value.clone()),
        Data::Int(value) => Some(value.to_string()),
        Data::Float(value) => Some(value.to_string()),
        Data::Bool(value) => Some(value.to_string()),
        Data::DateTime(value) => Some(value.to_string()),
        Data::DateTimeIso(value) | Data::DurationIso(value) => Some(value.clone()),
        Data::Empty | Data::Error(_) => None,
    }
}

fn data_json(data: &Data) -> Value {
    match data {
        Data::Empty => Value::Null,
        Data::String(v) => Value::String(v.clone()),
        Data::Int(v) => json!(v.to_string()),
        Data::Float(v) => Value::String(v.to_string()),
        Data::Bool(v) => json!(v),
        Data::DateTime(v) => json!(
            v.as_datetime()
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| v.to_string())
        ),
        Data::DateTimeIso(v) | Data::DurationIso(v) => Value::String(v.clone()),
        Data::Error(_) => json!({"cell_error":true}),
    }
}
fn parse_date_value(value: &Value) -> Option<String> {
    let text = value.as_str()?.trim();
    for format in ["%Y-%m-%d", "%d/%m/%Y", "%d-%m-%Y", "%m/%d/%Y"] {
        if let Ok(date) = chrono::NaiveDate::parse_from_str(text, format) {
            return Some(date.to_string());
        }
    }
    let serial = text.parse::<f64>().ok()?;
    if !serial.is_finite() || serial < 1.0 {
        return None;
    }
    chrono::NaiveDate::from_ymd_opt(1899, 12, 30)?
        .checked_add_signed(chrono::Duration::days(serial.floor() as i64))
        .map(|date| date.to_string())
}
fn parse_integer(value: &Value) -> Option<i64> {
    let text = value.as_str()?.trim();
    if let Ok(integer) = text.parse::<i64>() {
        return Some(integer);
    }
    let number = text.parse::<f64>().ok()?;
    (number.is_finite()
        && number.fract() == 0.0
        && number >= i64::MIN as f64
        && number <= i64::MAX as f64)
        .then_some(number as i64)
}
fn parse_quantity_milli(value: &Value) -> Option<i64> {
    let text = value.as_str()?.trim();
    let (whole, fraction) = text.split_once('.').unwrap_or((text, ""));
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.len() > 3
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let integer = whole.parse::<i64>().ok()?.checked_mul(1000)?;
    let scale = match fraction.len() {
        0 => 0,
        1 => fraction.parse::<i64>().ok()?.checked_mul(100)?,
        2 => fraction.parse::<i64>().ok()?.checked_mul(10)?,
        _ => fraction.parse::<i64>().ok()?,
    };
    integer.checked_add(scale)
}
fn column_name(mut col: usize) -> String {
    let mut s = String::new();
    loop {
        let rem = col % 26;
        s.insert(0, (b'A' + rem as u8) as char);
        if col < 26 {
            break;
        }
        col = col / 26 - 1;
    }
    s
}
fn validate_candidate(
    request: RequestId,
    v: &Value,
    business_today: chrono::NaiveDate,
) -> Result<(), Failure> {
    let (date, debit, credit) = candidate_parts(request, v)?;
    if date.is_none() || debit < 0 || credit < 0 || (debit > 0) == (credit > 0) {
        return Err(request.invalid(
            "overrides",
            "Kandidat harus memiliki tanggal dan tepat satu nilai debit/kredit positif",
        ));
    }
    if date.is_some_and(|d| d > business_today) {
        return Err(request.invalid(
            "effective_date",
            "Tanggal historis tidak boleh di masa depan",
        ));
    }
    Ok(())
}
fn candidate_parts(
    request: RequestId,
    v: &Value,
) -> Result<(Option<chrono::NaiveDate>, i64, i64), Failure> {
    let date = v
        .get("effective_date")
        .and_then(Value::as_str)
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let debit = v
        .get("debit_idr")
        .and_then(Value::as_str)
        .map(parse_signed_money_value)
        .transpose()
        .map_err(|_| request.invalid("debit_idr", "Nominal debit invalid"))?
        .unwrap_or(0);
    let credit = v
        .get("credit_idr")
        .and_then(Value::as_str)
        .map(parse_signed_money_value)
        .transpose()
        .map_err(|_| request.invalid("credit_idr", "Nominal kredit invalid"))?
        .unwrap_or(0);
    Ok((date, debit, credit))
}
fn merge_override(
    request: RequestId,
    parsed: Option<Value>,
    overrides: Value,
) -> Result<Option<Value>, Failure> {
    let Some(mut parsed) = parsed else {
        return Err(request.invalid(
            "overrides",
            "Baris tidak memiliki kandidat yang dapat di-override",
        ));
    };
    let allowed = [
        "effective_date",
        "reference_no",
        "description",
        "quantity",
        "unit_price_idr",
        "debit_idr",
        "credit_idr",
    ];
    let map = overrides
        .as_object()
        .ok_or_else(|| request.invalid("overrides", "Override harus objek"))?;
    if map.keys().any(|k| !allowed.contains(&k.as_str())) {
        return Err(request.invalid("overrides", "Field override tidak dikenal"));
    }
    for (k, v) in map {
        parsed[k] = v.clone();
    }
    Ok(Some(parsed))
}
fn parse_signed_money(request: RequestId, value: &str) -> Result<i64, Failure> {
    parse_signed_money_value(value)
        .map_err(|_| request.invalid("variance_idr", "Variance harus integer IDR bertanda"))
}
fn parse_signed_money_value(value: &str) -> Result<i64, ()> {
    if value.is_empty() || value.len() > 19 {
        return Err(());
    }
    let digits = value.strip_prefix('-').unwrap_or(value);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(());
    }
    value.parse().map_err(|_| ())
}
fn clean_reason(request: RequestId, reason: &str) -> Result<&str, Failure> {
    let r = reason.trim();
    if r.is_empty() || r.len() > 1000 || r.contains('\0') {
        Err(request.invalid("reason", "Alasan wajib maksimal 1.000 karakter"))
    } else {
        Ok(r)
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
fn not_found(request: RequestId) -> Failure {
    request.error(
        StatusCode::NOT_FOUND,
        "NOT_FOUND",
        "Batch import atau baris tidak ditemukan",
    )
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
