use std::{env, path::Path};

use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, StatusCode, header},
    middleware,
    response::Response,
    routing::{get, post},
};
use serde_json::Value;
use sqlx::{PgPool, migrate::Migrator, postgres::PgPoolOptions};
use toko_api::auth::{AccessPolicy, request_context, require_access};
use toko_application::Permission;
use tower::ServiceExt;
use uuid::Uuid;
use zip::{ZipWriter, write::SimpleFileOptions};

#[tokio::test]
async fn postgres_foundation() {
    let url = env::var("TEST_DATABASE_URL")
        .expect("Set TEST_DATABASE_URL to a disposable PostgreSQL database");
    let admin = PgPool::connect(&url)
        .await
        .expect("Cannot connect to test PostgreSQL");
    let schema = format!("ph01_{}", Uuid::new_v4().simple());
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&admin)
        .await
        .unwrap();

    let search_path = schema.clone();
    let pool = PgPoolOptions::new()
        .after_connect(move |connection, _| {
            let search_path = search_path.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path', $1, false)")
                    .bind(search_path)
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .unwrap();

    let test_pool = pool.clone();
    let result = tokio::spawn(async move {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../migrations/postgres");
        let migrator = Migrator::new(path.as_path()).await.unwrap();
        migrator.run(&test_pool).await.unwrap();
        migrator.run(&test_pool).await.unwrap();
        sqlx::raw_sql(include_str!("fixtures/synthetic.sql"))
            .execute(&test_pool)
            .await
            .unwrap();

        current_session_rejects_missing_expired_revoked_and_inactive_sessions(test_pool.clone())
            .await;
        access_guard_enforces_read_write_merge_and_csrf(test_pool.clone()).await;
        migration_enforces_customer_and_ledger_constraints(test_pool.clone()).await;
        customer_slice(test_pool.clone()).await;
        payment_slice(test_pool.clone()).await;
        credit_sale_slice(test_pool.clone()).await;
        aging_report_slice(test_pool.clone()).await;
        import_slice(test_pool.clone()).await;
        correction_slice(test_pool.clone()).await;
        login_and_logout(test_pool).await;
    })
    .await;

    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&admin)
        .await
        .unwrap();
    result.unwrap();
}

async fn aging_report_slice(pool: PgPool) {
    let app = toko_api::app(pool.clone());
    let today: chrono::NaiveDate =
        sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Asia/Jakarta')::date")
            .fetch_one(&pool)
            .await
            .unwrap();
    let user: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE username='fixture-owner'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let customer = Uuid::new_v4();
    sqlx::query("INSERT INTO customers (id,name,created_by) VALUES ($1,'Uji Aging PH05',$2)")
        .bind(customer)
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    let dates = [
        Some(today),
        Some(today - chrono::Days::new(15)),
        Some(today - chrono::Days::new(16)),
        Some(today - chrono::Days::new(30)),
        Some(today - chrono::Days::new(31)),
        Some(today - chrono::Days::new(45)),
        Some(today - chrono::Days::new(46)),
        Some(today - chrono::Days::new(60)),
        Some(today - chrono::Days::new(61)),
        None,
        Some(today + chrono::Days::new(1)),
    ];
    for (index, date) in dates.into_iter().enumerate() {
        sqlx::query("INSERT INTO receivable_entries (id,customer_id,entry_type,source_id,effective_date,debit_idr,created_by) VALUES ($1,$2,'legacy_debit',$3,$4,$5,$6)")
            .bind(Uuid::new_v4()).bind(customer).bind(Uuid::new_v4()).bind(date).bind((index+1) as i64).bind(user)
            .execute(&pool).await.unwrap();
    }
    let summary = json(
        request(
            &app,
            Method::GET,
            "/api/v1/reports/receivables-aging",
            Some('c'),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(summary["data"]["as_of"], today.to_string());
    for (bucket, value) in [
        ("days_0_15", "3"),
        ("days_16_30", "7"),
        ("days_31_45", "11"),
        ("days_46_60", "15"),
        ("days_over_60", "9"),
        ("unknown_date", "10"),
        ("future_date", "11"),
    ] {
        let details = json(
            request(
                &app,
                Method::GET,
                &format!("/api/v1/reports/receivables-aging/customers?bucket={bucket}"),
                Some('c'),
                None,
            )
            .await,
        )
        .await;
        let row = details["data"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["customer_id"] == customer.to_string())
            .unwrap();
        assert_eq!(row["amount_idr"], value);
    }
    let buckets = [
        "days_0_15",
        "days_16_30",
        "days_31_45",
        "days_46_60",
        "days_over_60",
        "unknown_date",
        "future_date",
    ];
    let sum: i64 = buckets
        .iter()
        .map(|key| {
            summary["data"][*key]
                .as_str()
                .unwrap()
                .parse::<i64>()
                .unwrap()
        })
        .sum();
    assert_eq!(
        summary["data"]["total_open_idr"]
            .as_str()
            .unwrap()
            .parse::<i64>()
            .unwrap(),
        sum
    );
    let future = today + chrono::Days::new(1);
    let invalid = json(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/reports/receivables-aging?as_of={future}"),
            Some('c'),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(
        invalid["error"]["fields"]["as_of"],
        "Tanggal acuan tidak boleh di masa depan"
    );
}

fn synthetic_xlsx() -> Vec<u8> {
    let mut writer = ZipWriter::new(std::io::Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default();
    let files = [
        (
            "[Content_Types].xml",
            r#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/><Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/></Types>"#,
        ),
        (
            "_rels/.rels",
            r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<?xml version="1.0" encoding="UTF-8"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Uji" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/></Relationships>"#,
        ),
        (
            "xl/sharedStrings.xml",
            r#"<?xml version="1.0" encoding="UTF-8"?><sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="4" uniqueCount="4"><si><t>2026-01-01</t></si><si><t>Tanggal</t></si><si><t>UJI-IMPORT</t></si><si><t>Baris sintetis</t></si></sst>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><dimension ref="A1:H8"/><sheetData><row r="1"><c r="A1" t="s"><v>1</v></c></row><row r="2"><c r="B2" t="s"><v>2</v></c></row><row r="3"><c r="B3" t="s"><v>3</v></c></row><row r="7"><c r="A7" t="s"><v>0</v></c><c r="B7" t="s"><v>2</v></c><c r="C7" t="s"><v>3</v></c><c r="F7"><v>100000</v></c><c r="H7"><f>F7+G7</f><v>100000</v></c></row><row r="8"><c r="A8" t="s"><v>0</v></c><c r="B8" t="s"><v>2</v></c><c r="C8" t="s"><v>3</v></c><c r="G8"><v>120000</v></c></row></sheetData></worksheet>"#,
        ),
    ];
    for (name, content) in files {
        writer.start_file(name, options).unwrap();
        std::io::Write::write_all(&mut writer, content.as_bytes()).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

async fn upload_synthetic(app: &Router, csrf: &str, bytes: &[u8]) -> Response {
    let boundary = "synthetic-xlsx-boundary";
    let mut body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"synthetic.xlsx\"\r\nContent-Type: application/vnd.openxmlformats-officedocument.spreadsheetml.sheet\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    app.clone()
        .oneshot(
            axum::http::Request::post("/api/v1/import-batches")
                .header(header::COOKIE, format!("__Host-session={}", "b".repeat(64)))
                .header("x-csrf-token", csrf)
                .header(
                    "content-type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn import_slice(pool: PgPool) {
    let app = toko_api::app(pool.clone());
    let operator_csrf = csrf(&app, 'b').await;
    let owner_csrf = csrf(&app, 'a').await;
    let user: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE username='fixture-owner'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let customer = Uuid::new_v4();
    sqlx::query("INSERT INTO customers(id,name,created_by) VALUES($1,'Uji import sintetis',$2)")
        .bind(customer)
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    let workbook = synthetic_xlsx();
    let uploaded = upload_synthetic(&app, &operator_csrf, &workbook).await;
    let upload_status = uploaded.status();
    let batch = json(uploaded).await;
    assert_eq!(upload_status, StatusCode::CREATED, "{batch}");
    let batch_id = batch["data"]["id"].as_str().unwrap();
    assert_eq!(batch["data"]["source_debit_idr"], "100000");
    assert_eq!(batch["data"]["source_credit_idr"], "120000");

    let operator_review = send_json(
        &app,
        Method::PATCH,
        &format!("/api/v1/import-batches/{batch_id}/rows/{}", Uuid::new_v4()),
        Some(&format!("__Host-session={}", "b".repeat(64))),
        Some(&operator_csrf),
        serde_json::json!({}),
    )
    .await;
    assert_eq!(operator_review.status(), StatusCode::FORBIDDEN);

    let rows = json(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/import-batches/{batch_id}/rows?limit=100"),
            Some('a'),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(rows["data"].as_array().unwrap().len(), 2);
    let first_raw = &rows["data"][0]["raw_values"];
    assert!(
        first_raw
            .as_array()
            .unwrap()
            .iter()
            .any(|cell| cell["formula"] == true)
    );
    assert!(
        rows["data"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["review_status"] == "candidate")
    );
    for row in rows["data"].as_array().unwrap() {
        let body = serde_json::json!({
            "expected_version":row["version"], "action":"accept", "mapped_customer_id":customer,
            "reason":"Uji sintetis review", "review_note":""
        });
        let reviewed = send_json(
            &app,
            Method::PATCH,
            &format!(
                "/api/v1/import-batches/{batch_id}/rows/{}",
                row["id"].as_str().unwrap()
            ),
            Some(&format!("__Host-session={}", "a".repeat(64))),
            Some(&owner_csrf),
            body,
        )
        .await;
        assert_eq!(reviewed.status(), StatusCode::OK);
    }
    let latest = json(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/import-batches/{batch_id}"),
            Some('a'),
            None,
        )
        .await,
    )
    .await;
    let row_ids: Vec<&str> = rows["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap())
        .collect();
    let commit_body =
        serde_json::json!({ "expected_version":latest["data"]["version"], "row_ids":row_ids });
    let request_commit = |key: &str| {
        app.clone().oneshot(
            axum::http::Request::post(format!("/api/v1/import-batches/{batch_id}/commit"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, format!("__Host-session={}", "a".repeat(64)))
                .header("x-csrf-token", &owner_csrf)
                .header("idempotency-key", key)
                .body(Body::from(commit_body.to_string()))
                .unwrap(),
        )
    };
    let committed = request_commit("ph06-commit").await.unwrap();
    let commit_status = committed.status();
    let commit_data = json(committed).await;
    assert_eq!(commit_status, StatusCode::OK, "{commit_data}");
    assert_eq!(commit_data["data"]["committed_rows"], 2);
    let replay = request_commit("ph06-commit").await.unwrap();
    assert_eq!(replay.status(), StatusCode::OK);
    assert_eq!(json(replay).await, commit_data);
    let duplicate = upload_synthetic(&app, &operator_csrf, &workbook).await;
    assert_eq!(duplicate.status(), StatusCode::OK);
    assert_eq!(json(duplicate).await["data"]["id"], batch_id);
    let totals: (i64, i64, i64) = sqlx::query_as("SELECT count(*),coalesce(sum(debit_idr),0)::bigint,coalesce(sum(credit_idr),0)::bigint FROM receivable_entries WHERE source_id IN (SELECT id FROM import_rows WHERE batch_id=$1)")
        .bind(batch_id.parse::<Uuid>().unwrap()).fetch_one(&pool).await.unwrap();
    assert_eq!(totals, (2, 100000, 120000));
    let allocation: i64 = sqlx::query_scalar("SELECT coalesce(sum(amount_idr),0)::bigint FROM legacy_credit_allocations WHERE debit_entry_id IN (SELECT ledger_entry_id FROM import_row_commits WHERE import_row_id IN (SELECT id FROM import_rows WHERE batch_id=$1))")
        .bind(batch_id.parse::<Uuid>().unwrap()).fetch_one(&pool).await.unwrap();
    assert_eq!(allocation, 100000);
    let later_debit = Uuid::new_v4();
    let owner: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE username='fixture-owner'")
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO receivable_entries(id,customer_id,entry_type,source_id,effective_date,debit_idr,created_by) VALUES($1,$2,'legacy_debit',$3,'2026-02-01',25000,$4)")
        .bind(later_debit).bind(customer).bind(Uuid::new_v4()).bind(owner).execute(&pool).await.unwrap();
    let payment_source_id = Uuid::parse_str(rows["data"][1]["id"].as_str().unwrap()).unwrap();
    let applied = sale_request(&app, Method::POST, &format!("/api/v1/customers/{customer}/credit-applications"), 'b', &operator_csrf, Some("ph06-legacy-credit"), &serde_json::json!({"payment_id":payment_source_id,"debit_entry_id":later_debit,"amount_idr":"10000","reason":"Uji penerapan kredit legacy"})).await;
    assert_eq!(applied.status(), StatusCode::OK);
    let manual_allocation: i64 = sqlx::query_scalar(
        "SELECT sum(amount_idr)::bigint FROM legacy_credit_allocations WHERE debit_entry_id=$1",
    )
    .bind(later_debit)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(manual_allocation, 10000);
    let audit: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_events WHERE entity_id=$1 AND action='import.commit'",
    )
    .bind(batch_id.parse::<Uuid>().unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit, 1);
}

async fn correction_slice(pool: PgPool) {
    let app = toko_api::app(pool.clone());
    let owner_csrf = csrf(&app, 'a').await;
    let operator_csrf = csrf(&app, 'b').await;
    let payment: Uuid = sqlx::query_scalar("SELECT id FROM payments WHERE reference='AC-005'")
        .fetch_one(&pool)
        .await
        .unwrap();
    let reallocate_body =
        serde_json::json!({"expected_version":1,"allocations":[],"reason":"Uji koreksi alokasi"});
    let denied = sale_request(
        &app,
        Method::POST,
        &format!("/api/v1/payments/{payment}/reallocation"),
        'b',
        &operator_csrf,
        Some("ph07-denied"),
        &reallocate_body,
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    let reallocated = sale_request(
        &app,
        Method::POST,
        &format!("/api/v1/payments/{payment}/reallocation"),
        'a',
        &owner_csrf,
        Some("ph07-reallocate"),
        &reallocate_body,
    )
    .await;
    assert_eq!(reallocated.status(), StatusCode::OK);
    let version: i32 = sqlx::query_scalar("SELECT version FROM payments WHERE id=$1")
        .bind(payment)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(version, 2);
    let reversed_allocations: i64 = sqlx::query_scalar("SELECT count(*) FROM payment_allocations WHERE payment_id=$1 AND status='reversed' AND reversed_by IS NOT NULL AND reversal_reason IS NOT NULL")
        .bind(payment).fetch_one(&pool).await.unwrap();
    assert_eq!(reversed_allocations, 1);
    let reversed = sale_request(
        &app,
        Method::POST,
        &format!("/api/v1/transactions/{payment}/reversal"),
        'a',
        &owner_csrf,
        Some("ph07-payment-reverse"),
        &serde_json::json!({"reason":"Uji pembalikan payment"}),
    )
    .await;
    assert_eq!(reversed.status(), StatusCode::OK);
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_events WHERE entity_id=$1 AND action='transaction.reverse'",
    )
    .bind(payment)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit_count, 1);
    let sale: Uuid =
        sqlx::query_scalar("SELECT id FROM credit_sales WHERE reference_no='UJI-KREDIT-SALE' AND status='posted' ORDER BY created_at DESC LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
    let sale_reverse = sale_request(
        &app,
        Method::POST,
        &format!("/api/v1/transactions/{sale}/reversal"),
        'a',
        &owner_csrf,
        Some("ph07-sale-reverse"),
        &serde_json::json!({"reason":"Uji pembalikan penjualan"}),
    )
    .await;
    let sale_reverse_status = sale_reverse.status();
    let sale_reverse_body = json(sale_reverse).await;
    assert_eq!(sale_reverse_status, StatusCode::OK, "{sale_reverse_body}");
    let sale_status: String = sqlx::query_scalar("SELECT status FROM credit_sales WHERE id=$1")
        .bind(sale)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(sale_status, "reversed");

    let customer_id: Uuid = sqlx::query_scalar("SELECT customer_id FROM credit_sales WHERE id=$1")
        .bind(sale)
        .fetch_one(&pool)
        .await
        .unwrap();
    let today: chrono::NaiveDate =
        sqlx::query_scalar("SELECT (now() AT TIME ZONE 'Asia/Jakarta')::date")
            .fetch_one(&pool)
            .await
            .unwrap();
    let replacement_id = Uuid::new_v4();
    let owner_id: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE username='fixture-owner'")
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO credit_sales(id,customer_id,effective_date,reference_no,status,total_idr,version,created_by,posted_by,posted_at) VALUES($1,$2,$3,'UJI-REPLACEMENT','posted',12552,2,$4,$4,statement_timestamp())")
        .bind(replacement_id).bind(customer_id).bind(today).bind(owner_id).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO receivable_entries(id,customer_id,entry_type,source_id,effective_date,debit_idr,created_by) VALUES($1,$2,'credit_sale',$3,$4,12552,$5)")
        .bind(Uuid::new_v4()).bind(customer_id).bind(replacement_id).bind(today).bind(owner_id).execute(&pool).await.unwrap();
    let linked = sale_request(&app, Method::POST, &format!("/api/v1/transactions/{sale}/replacement"), 'a', &owner_csrf, Some("ph07-replacement-link"), &serde_json::json!({"replacement_transaction_id":replacement_id,"reason":"Uji tautan transaksi pengganti"})).await;
    assert_eq!(linked.status(), StatusCode::OK);
    let linked_entry: Option<Uuid> = sqlx::query_scalar(
        "SELECT replacement_entry_id FROM transaction_reversals WHERE source_id=$1",
    )
    .bind(sale)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(linked_entry.is_some());
}

async fn request(
    app: &Router,
    method: Method,
    path: &str,
    token: Option<char>,
    csrf: Option<&str>,
) -> Response {
    let mut request = axum::http::Request::builder().method(method).uri(path);
    if let Some(token) = token {
        request = request.header(
            header::COOKIE,
            format!("__Host-session={}", token.to_string().repeat(64)),
        );
    }
    if let Some(csrf) = csrf {
        request = request.header("x-csrf-token", csrf);
    }
    app.clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap()
}

async fn json(response: Response) -> Value {
    let body = to_bytes(response.into_body(), 8192).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}

async fn current_session_rejects_missing_expired_revoked_and_inactive_sessions(pool: PgPool) {
    let app = toko_api::app(pool);
    for token in [None, Some('1'), Some('d'), Some('e'), Some('f'), Some('0')] {
        let response = request(&app, Method::GET, "/api/v1/auth/me", token, None).await;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        let request_id = response.headers()["x-request-id"]
            .to_str()
            .unwrap()
            .to_owned();
        let body = json(response).await;
        assert_eq!(body["error"]["code"], "UNAUTHENTICATED");
        assert_eq!(body["request_id"], request_id);
    }

    let response = request(&app, Method::GET, "/api/v1/auth/me", Some('c'), None).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    let body = json(response).await;
    assert_eq!(body["data"]["user"]["role"], "viewer");
    assert!(
        body["data"]["csrf_token"]
            .as_str()
            .is_some_and(|token| token.len() == 64)
    );
    assert_eq!(body["meta"], serde_json::json!({}));
}

async fn access_guard_enforces_read_write_merge_and_csrf(pool: PgPool) {
    let app = toko_api::app(pool.clone());
    for (permission, method, permitted) in [
        (Permission::CustomersRead, Method::GET, [true, true, true]),
        (Permission::LedgerRead, Method::GET, [true, true, true]),
        (Permission::PaymentsRead, Method::GET, [true, true, true]),
        (Permission::ReportsRead, Method::GET, [true, true, true]),
        (Permission::ImportsUpload, Method::GET, [true, true, false]),
        (Permission::ImportsRead, Method::GET, [true, false, false]),
        (Permission::ImportsReview, Method::GET, [true, false, false]),
        (
            Permission::ImportsOverride,
            Method::GET,
            [true, false, false],
        ),
        (Permission::ImportsCommit, Method::GET, [true, false, false]),
        (
            Permission::TransactionsReverse,
            Method::GET,
            [true, false, false],
        ),
        (
            Permission::CreditsReallocate,
            Method::GET,
            [true, false, false],
        ),
        (Permission::PaymentsWrite, Method::POST, [true, true, false]),
        (Permission::CreditsApply, Method::POST, [true, true, false]),
        (
            Permission::CustomersWrite,
            Method::POST,
            [true, true, false],
        ),
        (
            Permission::CustomersMerge,
            Method::POST,
            [true, false, false],
        ),
    ] {
        // Exercise the production middleware without registering unfinished API routes.
        let probe = Router::new()
            .route(
                "/probe",
                get(|| async { StatusCode::NO_CONTENT })
                    .merge(post(|| async { StatusCode::NO_CONTENT })),
            )
            .route_layer(middleware::from_fn_with_state(
                AccessPolicy::new(pool.clone(), permission),
                require_access,
            ))
            .layer(middleware::from_fn(request_context));
        for (token, allowed) in ['a', 'b', 'c'].into_iter().zip(permitted) {
            let me =
                json(request(&app, Method::GET, "/api/v1/auth/me", Some(token), None).await).await;
            let csrf = me["data"]["csrf_token"].as_str().unwrap();
            let expected = if allowed {
                StatusCode::NO_CONTENT
            } else {
                StatusCode::FORBIDDEN
            };
            assert_eq!(
                request(&probe, method.clone(), "/probe", Some(token), Some(csrf))
                    .await
                    .status(),
                expected
            );
        }
        if method == Method::POST {
            for csrf in [None, Some("invalid")] {
                let response = request(&probe, Method::POST, "/probe", Some('a'), csrf).await;
                assert_eq!(response.status(), StatusCode::FORBIDDEN);
                assert_eq!(json(response).await["error"]["code"], "CSRF_FAILED");
            }
        }
    }
}

async fn migration_enforces_customer_and_ledger_constraints(pool: PgPool) {
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM customers")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 2);

    for (sql, expected) in [
        ("UPDATE customers SET normalized_code = 'uji-a', code = 'UJI-A' WHERE code IS NULL", "23505"),
        ("UPDATE customers SET name = E' \\t' WHERE code IS NULL", "23514"),
        ("DELETE FROM customers WHERE code IS NULL", "23514"),
        ("UPDATE receivable_entries SET debit_idr = 1 WHERE debit_idr > 0", "23514"),
        ("DELETE FROM receivable_entries WHERE credit_idr > 0", "23514"),
        ("INSERT INTO receivable_entries (id, customer_id, entry_type, source_id, debit_idr, credit_idr, created_by)
          SELECT gen_random_uuid(), customer_id, entry_type, gen_random_uuid(), 1, 1, created_by
          FROM receivable_entries LIMIT 1", "23514"),
        ("INSERT INTO receivable_entries (id, customer_id, entry_type, source_id, debit_idr, credit_idr, created_by)
          SELECT gen_random_uuid(), customer_id, entry_type, source_id, debit_idr, credit_idr, created_by
          FROM receivable_entries LIMIT 1", "23505"),
    ] {
        let error = sqlx::query(sql).execute(&pool).await.unwrap_err();
        assert_eq!(error.as_database_error().unwrap().code().as_deref(), Some(expected));
    }
}

const CUSTOMER_A: &str = "00000000-0000-4000-8000-000000002001";
const CUSTOMER_B: &str = "00000000-0000-4000-8000-000000002002";

async fn send_json(
    app: &Router,
    method: Method,
    path: &str,
    cookie: Option<&str>,
    csrf: Option<&str>,
    body: Value,
) -> Response {
    let mut request = axum::http::Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    if let Some(csrf) = csrf {
        request = request.header("x-csrf-token", csrf);
    }
    app.clone()
        .oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap()
}

async fn csrf(app: &Router, token: char) -> String {
    json(request(app, Method::GET, "/api/v1/auth/me", Some(token), None).await).await
        ["data"]["csrf_token"].as_str().unwrap().to_owned()
}

async fn customer_slice(pool: PgPool) {
    let app = toko_api::app(pool.clone());
    let search = request(
        &app,
        Method::GET,
        "/api/v1/customers?search=UjI-A",
        Some('c'),
        None,
    )
    .await;
    assert_eq!(search.status(), StatusCode::OK);
    let search = json(search).await;
    assert_eq!(search["data"]["items"].as_array().unwrap().len(), 1);
    assert_eq!(search["data"]["items"][0]["customer"]["id"], CUSTOMER_A);
    let aging_row = &search["data"]["items"][0];
    let aged_total: i64 = [
        "days_0_15_idr",
        "days_16_30_idr",
        "days_31_45_idr",
        "days_46_60_idr",
        "days_over_60_idr",
        "unknown_date_idr",
        "future_date_idr",
    ]
    .iter()
    .map(|field| aging_row[*field].as_str().unwrap().parse::<i64>().unwrap())
    .sum();
    assert_eq!(aged_total.to_string(), aging_row["outstanding_idr"]);

    for (id, outstanding, credit, balance) in [
        (CUSTOMER_A, "60000", "0", "60000"),
        (CUSTOMER_B, "17000", "20000", "-3000"),
    ] {
        let detail = json(
            request(
                &app,
                Method::GET,
                &format!("/api/v1/customers/{id}"),
                Some('c'),
                None,
            )
            .await,
        )
        .await;
        assert_eq!(detail["data"]["customer"]["id"], id);
        assert_eq!(detail["data"]["outstanding_idr"], outstanding);
        assert_eq!(detail["data"]["customer_credit_idr"], credit);
        let ledger = json(
            request(
                &app,
                Method::GET,
                &format!("/api/v1/customers/{id}/ledger"),
                Some('c'),
                None,
            )
            .await,
        )
        .await;
        assert_eq!(ledger["data"]["summary"]["ledger_balance_idr"], balance);
        assert_eq!(ledger["data"]["entries"].as_array().unwrap().len(), 2);
        assert!(ledger["data"]["entries"][0].get("remaining_idr").is_some());
        assert!(ledger["data"]["entries"][0].get("age_days").is_some());
        assert_eq!(ledger["data"]["entries"][1]["running_balance_idr"], balance);
        assert!(
            ledger["data"]["entries"]
                .as_array()
                .unwrap()
                .iter()
                .all(|entry| {
                    let reference = entry["reference"].as_str().unwrap_or_default();
                    reference.is_empty()
                        || reference.ends_with(if id == CUSTOMER_A { 'A' } else { 'B' })
                })
        );
    }
    let first = json(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/customers/{CUSTOMER_A}/ledger?limit=1"),
            Some('a'),
            None,
        )
        .await,
    )
    .await;
    let cursor = first["meta"]["next_cursor"].as_str().unwrap();
    let cross_customer = request(
        &app,
        Method::GET,
        &format!("/api/v1/customers/{CUSTOMER_B}/ledger?cursor={cursor}"),
        Some('a'),
        None,
    )
    .await;
    assert_eq!(cross_customer.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let filtered = json(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/customers/{CUSTOMER_A}/ledger?from=2026-01-02&cursor={cursor}"),
            Some('a'),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(filtered["data"]["entries"][0]["reference"], "UJI-BAYAR-A");
    assert_eq!(
        filtered["data"]["entries"][0]["running_balance_idr"],
        "60000"
    );

    let viewer_cookie = format!("__Host-session={}", "c".repeat(64));
    let op_cookie = format!("__Host-session={}", "b".repeat(64));
    let op_csrf = csrf(&app, 'b').await;
    let viewer_csrf = csrf(&app, 'c').await;

    let payload =
        serde_json::json!({"name":" Pelanggan Baru ", "code":" Baru-3 ", "address":"Alamat Uji"});
    let denied = send_json(
        &app,
        Method::POST,
        "/api/v1/customers",
        Some(&viewer_cookie),
        Some(&viewer_csrf),
        payload.clone(),
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    for (body, status, field) in [
        (
            serde_json::json!({"name":"  "}),
            StatusCode::UNPROCESSABLE_ENTITY,
            "name",
        ),
        (
            serde_json::json!({"name":"Bertabrakan","code":" uJi-A "}),
            StatusCode::CONFLICT,
            "code",
        ),
    ] {
        let response = send_json(
            &app,
            Method::POST,
            "/api/v1/customers",
            Some(&op_cookie),
            Some(&op_csrf),
            body,
        )
        .await;
        assert_eq!(response.status(), status);
        assert!(json(response).await["error"]["fields"][field].is_string());
    }
    let created = send_json(
        &app,
        Method::POST,
        "/api/v1/customers",
        Some(&op_cookie),
        Some(&op_csrf),
        payload,
    )
    .await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let created = json(created).await;
    assert_eq!(created["data"]["name"], "Pelanggan Baru");
    let id = created["data"]["id"].as_str().unwrap();
    let path = format!("/api/v1/customers/{id}");
    let edit = serde_json::json!({"expected_version":1,"name":"Pelanggan Diperbarui","code":null,"is_active":false});
    let updated = send_json(
        &app,
        Method::PATCH,
        &path,
        Some(&op_cookie),
        Some(&op_csrf),
        edit.clone(),
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    let conflict = send_json(
        &app,
        Method::PATCH,
        &path,
        Some(&op_cookie),
        Some(&op_csrf),
        edit,
    )
    .await;
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
    assert!(json(conflict).await["error"]["fields"]["expected_version"].is_string());
    let refreshed = json(request(&app, Method::GET, &path, Some('c'), None).await).await;
    assert_eq!(
        refreshed["data"]["customer"]["name"],
        "Pelanggan Diperbarui"
    );
    assert_eq!(refreshed["data"]["customer"]["is_active"], false);
    assert_eq!(refreshed["data"]["customer"]["code"], Value::Null);

    let merge_path = format!("/api/v1/customers/{CUSTOMER_A}/merge");
    let payload = serde_json::json!({"target_customer_id":CUSTOMER_B,"expected_version":1,"reason":"Data sintetis duplikat"});
    let denied = send_json(
        &app,
        Method::POST,
        &merge_path,
        Some(&op_cookie),
        Some(&op_csrf),
        payload.clone(),
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    let owner_cookie = format!("__Host-session={}", "a".repeat(64));
    let owner_csrf = csrf(&app, 'a').await;
    let merged = send_json(
        &app,
        Method::POST,
        &merge_path,
        Some(&owner_cookie),
        Some(&owner_csrf),
        payload,
    )
    .await;
    assert_eq!(merged.status(), StatusCode::OK);
    assert_eq!(json(merged).await["data"]["is_active"], false);
    let moved = json(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/customers/{CUSTOMER_B}/ledger"),
            Some('c'),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(moved["data"]["entries"].as_array().unwrap().len(), 4);
    assert_eq!(moved["data"]["summary"]["outstanding_idr"], "77000");
    assert_eq!(moved["data"]["summary"]["customer_credit_idr"], "20000");
    let audit: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_events WHERE action = 'customer.merge' AND entity_id = $1",
    )
    .bind(Uuid::parse_str(CUSTOMER_A).unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit, 1);
}

async fn login_and_logout(pool: PgPool) {
    let password = "Disposable fixture credential";
    let hash = toko_api::auth::hash_password(password).unwrap();
    sqlx::query("UPDATE users SET password_hash = $1 WHERE username = 'fixture-owner'")
        .bind(hash)
        .execute(&pool)
        .await
        .unwrap();
    let app = toko_api::app(pool.clone());
    let bad = send_json(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        None,
        None,
        serde_json::json!({"username":"fixture-owner","password":"wrong"}),
    )
    .await;
    assert_eq!(bad.status(), StatusCode::UNAUTHORIZED);
    let session = send_json(
        &app,
        Method::POST,
        "/api/v1/auth/login",
        None,
        None,
        serde_json::json!({"username":"fixture-owner","password":password}),
    )
    .await;
    assert_eq!(session.status(), StatusCode::OK);
    let set_cookie = session.headers()[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .to_owned();
    assert!(set_cookie.contains("; Secure; HttpOnly; SameSite=Strict; Path=/"));
    assert!(!set_cookie.contains("Domain="));
    let cookie = set_cookie.split(';').next().unwrap();
    let session = json(session).await;
    let csrf = session["data"]["csrf_token"].as_str().unwrap();
    let me_request = axum::http::Request::get("/api/v1/auth/me")
        .header(header::COOKIE, cookie)
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone().oneshot(me_request).await.unwrap().status(),
        StatusCode::OK
    );
    let denied = send_json(
        &app,
        Method::POST,
        "/api/v1/auth/logout",
        Some(cookie),
        None,
        Value::Null,
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    let logout = send_json(
        &app,
        Method::POST,
        "/api/v1/auth/logout",
        Some(cookie),
        Some(csrf),
        Value::Null,
    )
    .await;
    assert_eq!(logout.status(), StatusCode::OK);
    assert!(
        logout.headers()[header::SET_COOKIE]
            .to_str()
            .unwrap()
            .contains("Max-Age=0")
    );
    let revoked = send_json(
        &app,
        Method::POST,
        "/api/v1/auth/logout",
        Some(cookie),
        Some(csrf),
        Value::Null,
    )
    .await;
    assert_eq!(revoked.status(), StatusCode::UNAUTHORIZED);
}

async fn sale_request(
    app: &Router,
    method: Method,
    path: &str,
    token: char,
    csrf: &str,
    key: Option<&str>,
    body: &Value,
) -> Response {
    let mut request = axum::http::Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json")
        .header(
            header::COOKIE,
            format!("__Host-session={}", token.to_string().repeat(64)),
        )
        .header("x-csrf-token", csrf);
    if let Some(key) = key {
        request = request.header("Idempotency-Key", key);
    }
    app.clone()
        .oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap()
}

async fn payment_request(
    app: &Router,
    token: char,
    csrf: &str,
    key: &str,
    body: &Value,
) -> Response {
    app.clone()
        .oneshot(
            axum::http::Request::post("/api/v1/payments")
                .header(header::CONTENT_TYPE, "application/json")
                .header(
                    header::COOKIE,
                    format!("__Host-session={}", token.to_string().repeat(64)),
                )
                .header("x-csrf-token", csrf)
                .header("idempotency-key", key)
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

async fn payment_slice(pool: PgPool) {
    let app = toko_api::app(pool.clone());
    let operator = csrf(&app, 'b').await;
    let viewer = csrf(&app, 'c').await;
    let today: chrono::NaiveDate =
        sqlx::query_scalar("SELECT (clock_timestamp() AT TIME ZONE 'Asia/Jakarta')::date")
            .fetch_one(&pool)
            .await
            .unwrap();
    let owner_id = Uuid::parse_str("00000000-0000-4000-8000-000000000001").unwrap();
    let one_debit_customer = Uuid::from_u128(0x2003);
    let fifo_customer = Uuid::from_u128(0x2004);
    for (customer, name) in [
        (one_debit_customer, "Pembayaran AC005"),
        (fifo_customer, "Pembayaran AC006"),
    ] {
        sqlx::query("INSERT INTO customers (id,name,created_by) VALUES ($1,$2,$3)")
            .bind(customer)
            .bind(name)
            .bind(owner_id)
            .execute(&pool)
            .await
            .unwrap();
    }
    let old_debit = Uuid::from_u128(0x3005);
    sqlx::query("INSERT INTO receivable_entries (id,customer_id,entry_type,source_id,effective_date,debit_idr,created_by) VALUES ($1,$2,'legacy_debit',$3,'2026-01-01',100000,$4)")
        .bind(old_debit).bind(one_debit_customer).bind(Uuid::from_u128(0x4005)).bind(owner_id)
        .execute(&pool).await.unwrap();
    let first = Uuid::from_u128(0x3006);
    let second = Uuid::from_u128(0x3007);
    for (debit, source, date, amount) in [
        (first, Uuid::from_u128(0x4006), 1, 100000_i64),
        (second, Uuid::from_u128(0x4007), 2, 200000_i64),
    ] {
        sqlx::query("INSERT INTO receivable_entries (id,customer_id,entry_type,source_id,effective_date,debit_idr,created_by) VALUES ($1,$2,'legacy_debit',$3,$4,$5,$6)")
            .bind(debit).bind(fifo_customer).bind(source).bind(chrono::NaiveDate::from_ymd_opt(2026, 1, date).unwrap()).bind(amount).bind(owner_id)
            .execute(&pool).await.unwrap();
    }

    let first_payment = serde_json::json!({
        "customer_id": one_debit_customer, "amount_idr":"40000", "received_date":today,
        "method":"cash", "reference":"AC-005"
    });
    assert_eq!(
        payment_request(&app, 'c', &viewer, "viewer-payment", &first_payment)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let posted = payment_request(&app, 'b', &operator, "ph04-ac005", &first_payment).await;
    assert_eq!(posted.status(), StatusCode::CREATED);
    let posted_body = json(posted).await;
    assert_eq!(
        posted_body["data"]["allocations"][0]["debit_entry_id"],
        old_debit.to_string()
    );
    assert_eq!(posted_body["data"]["allocations"][0]["amount_idr"], "40000");
    let replay = payment_request(&app, 'b', &operator, "ph04-ac005", &first_payment).await;
    assert_eq!(replay.status(), StatusCode::CREATED);
    let payment_history = json(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/customers/{one_debit_customer}/payments"),
            Some('c'),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(payment_history["data"][0]["id"], posted_body["data"]["id"]);
    let mut changed = first_payment.clone();
    changed["amount_idr"] = serde_json::json!("45000");
    assert_eq!(
        payment_request(&app, 'b', &operator, "ph04-ac005", &changed)
            .await
            .status(),
        StatusCode::CONFLICT
    );
    let detail = json(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/customers/{one_debit_customer}"),
            Some('c'),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(detail["data"]["outstanding_idr"], "60000");

    let surplus_body = serde_json::json!({
        "customer_id": one_debit_customer, "amount_idr":"80000", "received_date":today,
        "method":"cash", "reference":"uji-surplus"
    });
    let surplus =
        json(payment_request(&app, 'b', &operator, "ph04-surplus", &surplus_body).await).await;
    assert_eq!(surplus["data"]["unallocated_idr"], "20000");
    let later_debit = Uuid::from_u128(0x3008);
    sqlx::query("INSERT INTO receivable_entries (id,customer_id,entry_type,source_id,effective_date,debit_idr,created_by) VALUES ($1,$2,'legacy_debit',$3,'2026-02-01',25000,$4)")
        .bind(later_debit).bind(one_debit_customer).bind(Uuid::from_u128(0x4008)).bind(owner_id)
        .execute(&pool).await.unwrap();
    let credit_apply = serde_json::json!({
        "payment_id":surplus["data"]["id"], "debit_entry_id":later_debit,
        "amount_idr":"10000", "reason":"Alokasi manual sesuai konfirmasi toko"
    });
    let applied = app
        .clone()
        .oneshot(
            axum::http::Request::post(format!(
                "/api/v1/customers/{one_debit_customer}/credit-applications"
            ))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::COOKIE, format!("__Host-session={}", "b".repeat(64)))
            .header("x-csrf-token", &operator)
            .header("idempotency-key", "ph04-credit-application")
            .body(Body::from(credit_apply.to_string()))
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(applied.status(), StatusCode::OK);
    let payment_detail = json(
        request(
            &app,
            Method::GET,
            &format!(
                "/api/v1/payments/{}",
                surplus["data"]["id"].as_str().unwrap()
            ),
            Some('c'),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(payment_detail["data"]["unallocated_idr"], "10000");
    let refreshed = json(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/customers/{one_debit_customer}"),
            Some('c'),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(refreshed["data"]["outstanding_idr"], "15000");
    assert_eq!(refreshed["data"]["customer_credit_idr"], "10000");

    let rollback_customer = Uuid::from_u128(0x2005);
    let rollback_debit = Uuid::from_u128(0x3009);
    sqlx::query("INSERT INTO customers (id,name,created_by) VALUES ($1,'Pembayaran rollback',$2)")
        .bind(rollback_customer)
        .bind(owner_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO receivable_entries (id,customer_id,entry_type,source_id,effective_date,debit_idr,created_by) VALUES ($1,$2,'legacy_debit',$3,'2026-03-01',10000,$4)")
        .bind(rollback_debit).bind(rollback_customer).bind(Uuid::from_u128(0x4009)).bind(owner_id)
        .execute(&pool).await.unwrap();
    sqlx::raw_sql("CREATE FUNCTION fail_payment_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.action='payment.post' THEN RAISE EXCEPTION 'forced payment audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER fail_payment_audit BEFORE INSERT ON audit_events FOR EACH ROW EXECUTE FUNCTION fail_payment_audit();")
        .execute(&pool).await.unwrap();
    let rollback_body = serde_json::json!({
        "customer_id":rollback_customer, "amount_idr":"5000", "received_date":today,
        "method":"cash"
    });
    assert_eq!(
        payment_request(&app, 'b', &operator, "ph04-rollback", &rollback_body)
            .await
            .status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    let payment_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM payments WHERE customer_id=$1")
            .bind(rollback_customer)
            .fetch_one(&pool)
            .await
            .unwrap();
    let ledger_count: i64 = sqlx::query_scalar("SELECT count(*) FROM receivable_entries WHERE source_id IN (SELECT id FROM payments WHERE customer_id=$1)").bind(rollback_customer).fetch_one(&pool).await.unwrap();
    let retry_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM idempotency_records WHERE idempotency_key='ph04-rollback'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((payment_count, ledger_count, retry_count), (0, 0, 0));
    sqlx::raw_sql(
        "DROP TRIGGER fail_payment_audit ON audit_events; DROP FUNCTION fail_payment_audit();",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        payment_request(&app, 'b', &operator, "ph04-rollback", &rollback_body)
            .await
            .status(),
        StatusCode::CREATED
    );

    let concurrent_customer = Uuid::from_u128(0x2006);
    sqlx::query("INSERT INTO customers (id,name,created_by) VALUES ($1,'Pembayaran konkuren',$2)")
        .bind(concurrent_customer)
        .bind(owner_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO receivable_entries (id,customer_id,entry_type,source_id,effective_date,debit_idr,created_by) VALUES ($1,$2,'legacy_debit',$3,'2026-04-01',100000,$4)")
        .bind(Uuid::from_u128(0x3010)).bind(concurrent_customer).bind(Uuid::from_u128(0x4010)).bind(owner_id)
        .execute(&pool).await.unwrap();
    let concurrent_body = serde_json::json!({
        "customer_id":concurrent_customer, "amount_idr":"70000", "received_date":today,
        "method":"cash"
    });
    let (left, right) = tokio::join!(
        payment_request(&app, 'b', &operator, "ph04-concurrent-a", &concurrent_body),
        payment_request(&app, 'b', &operator, "ph04-concurrent-b", &concurrent_body),
    );
    assert_eq!(
        (left.status(), right.status()),
        (StatusCode::CREATED, StatusCode::CREATED)
    );
    let allocation_total: i64 = sqlx::query_scalar("SELECT coalesce(sum(a.amount_idr),0)::bigint FROM payment_allocations a JOIN payments p ON p.id=a.payment_id WHERE p.customer_id=$1 AND a.status='active'")
        .bind(concurrent_customer).fetch_one(&pool).await.unwrap();
    assert_eq!(allocation_total, 100000);

    let fifo_payment = serde_json::json!({
        "customer_id": fifo_customer, "amount_idr":"150000", "received_date":today,
        "method":"bank_transfer", "note":"uji alokasi berurutan"
    });
    let posted = payment_request(&app, 'b', &operator, "ph04-ac006", &fifo_payment).await;
    assert_eq!(posted.status(), StatusCode::CREATED);
    let response = json(posted).await;
    assert_eq!(
        response["data"]["allocations"][0]["debit_entry_id"],
        first.to_string()
    );
    assert_eq!(response["data"]["allocations"][0]["amount_idr"], "100000");
    assert_eq!(
        response["data"]["allocations"][1]["debit_entry_id"],
        second.to_string()
    );
    assert_eq!(response["data"]["allocations"][1]["amount_idr"], "50000");
    let audit_allocations: String = sqlx::query_scalar(
        "SELECT metadata->>'allocation_count' FROM audit_events WHERE action='payment.post' AND entity_id=$1",
    )
    .bind(Uuid::parse_str(response["data"]["id"].as_str().unwrap()).unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit_allocations, "2");
    let detail = json(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/customers/{fifo_customer}"),
            Some('c'),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(detail["data"]["outstanding_idr"], "150000");
}

async fn credit_sale_slice(pool: PgPool) {
    let app = toko_api::app(pool.clone());
    let operator = csrf(&app, 'b').await;
    let owner = csrf(&app, 'a').await;
    let viewer = csrf(&app, 'c').await;
    let today: chrono::NaiveDate =
        sqlx::query_scalar("SELECT (clock_timestamp() AT TIME ZONE 'Asia/Jakarta')::date")
            .fetch_one(&pool)
            .await
            .unwrap();
    let initial = serde_json::json!({
        "customer_id": CUSTOMER_B, "effective_date": today, "reference_no":"UJI-KREDIT-SALE",
        "items":[{"description":"Item penjualan A","quantity":"1.250","unit_price_idr":"10001"}]
    });
    let base = "/api/v1/credit-sales";
    let create =
        |token, csrf, key, body| sale_request(&app, Method::POST, base, token, csrf, key, body);

    let denied = create('c', &viewer, Some("denied"), &initial).await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        create('b', &operator, None, &initial).await.status(),
        StatusCode::BAD_REQUEST
    );
    let mut inactive = initial.clone();
    inactive["customer_id"] = serde_json::json!(CUSTOMER_A);
    assert_eq!(
        create('b', &operator, Some("inactive"), &inactive)
            .await
            .status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );

    let mut dates = initial.clone();
    dates["effective_date"] = serde_json::json!(today.succ_opt().unwrap());
    assert_eq!(
        create('a', &owner, Some("future"), &dates).await.status(),
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mut dates = initial.clone();
    dates["effective_date"] = serde_json::json!(today.pred_opt().unwrap());
    assert_eq!(
        create('b', &operator, Some("backdate-op"), &dates)
            .await
            .status(),
        StatusCode::CREATED
    );
    let mut dated_with_reason = dates.clone();
    dated_with_reason["backdate_reason"] = serde_json::json!("Pembukuan hari sebelumnya");
    assert_eq!(
        create('a', &owner, Some("backdate-owner"), &dated_with_reason)
            .await
            .status(),
        StatusCode::CREATED
    );

    let created = create('b', &operator, Some("draft"), &initial).await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let draft = json(created).await;
    let duplicate = create('b', &operator, Some("draft"), &initial).await;
    assert_eq!(duplicate.status(), StatusCode::CREATED);
    assert_eq!(json(duplicate).await, draft);
    let mut changed = initial.clone();
    changed["reference_no"] = serde_json::json!("CHANGED");
    let conflict = create('b', &operator, Some("draft"), &changed).await;
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
    assert_eq!(
        json(conflict).await["error"]["code"],
        "IDEMPOTENCY_CONFLICT"
    );

    let id = draft["data"]["id"].as_str().unwrap();
    let sale_id = Uuid::parse_str(id).unwrap();
    let path = format!("{base}/{id}");
    let posting = format!("{path}/post");
    let ledger_before: i64 =
        sqlx::query_scalar("SELECT count(*) FROM receivable_entries WHERE source_id = $1")
            .bind(sale_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(ledger_before, 0);

    let edit = serde_json::json!({"expected_version":1,"items":[
        {"description":"Item penjualan A","quantity":"1.250","unit_price_idr":"10001"},
        {"description":"Item penjualan B","quantity":"0.500","unit_price_idr":"101"}
    ]});
    let updated = sale_request(&app, Method::PATCH, &path, 'b', &operator, None, &edit).await;
    assert_eq!(updated.status(), StatusCode::OK);
    let updated = json(updated).await;
    assert_eq!(updated["data"]["total_idr"], "12552");
    assert_eq!(updated["data"]["items"][1]["line_total_idr"], "51");
    assert_eq!(
        sale_request(&app, Method::PATCH, &path, 'b', &operator, None, &edit)
            .await
            .status(),
        StatusCode::CONFLICT
    );
    let reviewed = request(&app, Method::GET, &path, Some('c'), None).await;
    assert_eq!(reviewed.status(), StatusCode::OK);
    assert_eq!(json(reviewed).await, updated);

    // Fail after the sale update and ledger insertion by rejecting this transaction's audit row.
    sqlx::query("ALTER TABLE audit_events ADD CONSTRAINT fixture_force_post_failure CHECK (action <> 'credit_sale.post')")
        .execute(&pool).await.unwrap();
    let expected = serde_json::json!({"expected_version":2});
    let failure = sale_request(
        &app,
        Method::POST,
        &posting,
        'b',
        &operator,
        Some("post"),
        &expected,
    )
    .await;
    assert_eq!(failure.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let persisted: (String, i32, i64, i64, i64) = sqlx::query_as(
        "SELECT status, version,
            (SELECT count(*) FROM receivable_entries WHERE source_id = $1),
            (SELECT count(*) FROM audit_events WHERE entity_id = $1),
            (SELECT count(*) FROM idempotency_records WHERE route_key = $2)
         FROM credit_sales WHERE id = $1",
    )
    .bind(sale_id)
    .bind(format!("credit-sales:{id}:post"))
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(persisted, ("draft".into(), 2, 0, 0, 0));
    sqlx::query("ALTER TABLE audit_events DROP CONSTRAINT fixture_force_post_failure")
        .execute(&pool)
        .await
        .unwrap();

    let (first, concurrent) = tokio::join!(
        sale_request(
            &app,
            Method::POST,
            &posting,
            'b',
            &operator,
            Some("post"),
            &expected
        ),
        sale_request(
            &app,
            Method::POST,
            &posting,
            'b',
            &operator,
            Some("post"),
            &expected
        ),
    );
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(concurrent.status(), StatusCode::OK);
    let posted = json(first).await;
    assert_eq!(json(concurrent).await, posted);
    assert_eq!(posted["data"]["sale"]["status"], "posted");
    // Sesuai OQ-28 (Auto-apply deposit): Saldo kredit 20.000 otomatis menyerap penjualan sehingga outstanding menjadi 69552 dan deposit menjadi 0
    assert_eq!(posted["data"]["balance"]["outstanding_idr"], "69552");
    assert_eq!(posted["data"]["balance"]["customer_credit_idr"], "0");

    let replay = sale_request(
        &app,
        Method::POST,
        &posting,
        'b',
        &operator,
        Some("post"),
        &expected,
    )
    .await;
    assert_eq!(json(replay).await, posted);
    let conflict = sale_request(
        &app,
        Method::POST,
        &posting,
        'b',
        &operator,
        Some("post"),
        &serde_json::json!({"expected_version":3}),
    )
    .await;
    assert_eq!(
        json(conflict).await["error"]["code"],
        "IDEMPOTENCY_CONFLICT"
    );
    assert_eq!(
        sale_request(&app, Method::PATCH, &path, 'b', &operator, None, &edit)
            .await
            .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        sale_request(
            &app,
            Method::POST,
            &posting,
            'b',
            &operator,
            Some("different-key"),
            &expected
        )
        .await
        .status(),
        StatusCode::CONFLICT
    );

    let ledger = json(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/customers/{CUSTOMER_B}/ledger"),
            Some('c'),
            None,
        )
        .await,
    )
    .await;
    let entries: Vec<_> = ledger["data"]["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["source_id"] == id)
        .collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["debit_idr"], "12552");
    assert_eq!(entries[0]["reference"], "UJI-KREDIT-SALE");
    assert_eq!(entries[0]["description"], "Item penjualan A");
    let audit_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_events WHERE entity_id = $1 AND action = 'credit_sale.post'",
    )
    .bind(sale_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit_count, 1);
    assert!(
        sqlx::query("UPDATE credit_sale_items SET description = 'forbidden' WHERE sale_id = $1")
            .bind(sale_id)
            .execute(&pool)
            .await
            .is_err()
    );

    let merged = send_json(
        &app, Method::POST, &format!("/api/v1/customers/{CUSTOMER_B}/merge"),
        Some(&format!("__Host-session={}", "a".repeat(64))), Some(&owner),
        serde_json::json!({"target_customer_id": CUSTOMER_A, "expected_version": 2, "reason": "Uji relink penjualan"}),
    ).await;
    assert_eq!(merged.status(), StatusCode::OK);
    let moved = json(request(&app, Method::GET, &path, Some('c'), None).await).await;
    assert_eq!(moved["data"]["customer_id"], CUSTOMER_A);
    assert_eq!(
        moved["data"]["ledger_entry_id"],
        posted["data"]["sale"]["ledger_entry_id"]
    );
    assert_eq!(moved["data"]["version"], posted["data"]["sale"]["version"]);
    let ledger = json(
        request(
            &app,
            Method::GET,
            &format!("/api/v1/customers/{CUSTOMER_A}/ledger"),
            Some('c'),
            None,
        )
        .await,
    )
    .await;
    assert_eq!(
        ledger["data"]["entries"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|entry| entry["source_id"] == id && entry["reference"] == "UJI-KREDIT-SALE")
            .count(),
        1
    );

    let pdf_resp = request(
        &app,
        Method::GET,
        &format!("/api/v1/customers/{CUSTOMER_A}/ledger/export-pdf"),
        Some('c'),
        None,
    )
    .await;
    assert_eq!(pdf_resp.status(), StatusCode::OK);
    assert_eq!(
        pdf_resp.headers().get("content-type").unwrap(),
        "application/pdf"
    );
    let pdf_bytes = axum::body::to_bytes(pdf_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    assert!(pdf_bytes.starts_with(b"%PDF-1.4"));
    assert!(pdf_bytes.ends_with(b"%%EOF\n") || pdf_bytes.ends_with(b"%%EOF"));
}
