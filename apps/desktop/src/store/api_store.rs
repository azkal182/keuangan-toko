//! HttpApiAdapter: implementasi `TokoStoreAdapter` yang berbicara dengan API web v1 nyata
//! (lihat docs/API_SPEC.md dan crates/contracts). Autentikasi memakai cookie sesi server-side
//! (`__Host-session`) yang dikelola agent HTTP, ditambah header `X-CSRF-Token`, `Idempotency-Key`
//! dan `Origin` pada setiap mutasi.
//!
//! Catatan desain:
//! - Pemanggilan bersifat blocking (adapter sinkron); hasil baca di-cache singkat agar `render`
//!   GPUI tidak memanggil jaringan pada setiap frame. Setiap mutasi menghapus seluruh cache.
//! - Nominal uang di wire berupa string integer IDR; kuantitas string desimal.
//! - Tidak ada kredensial yang disimpan atau di-log; sesi hanya hidup di memori proses.

use super::*;
use serde_json::{Value, json};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use uuid::Uuid;

const PAGE_LIMIT: u32 = 100;
const MAX_PAGES: usize = 50;
const OK_TTL: Duration = Duration::from_secs(60);
const ERR_TTL: Duration = Duration::from_secs(8);
const AGING_KEYS: [&str; 7] = [
    "days_0_15",
    "days_16_30",
    "days_31_45",
    "days_46_60",
    "days_over_60",
    "unknown_date",
    "future_date",
];

struct ApiSession {
    user: SessionUser,
    csrf: String,
}

/// Parse respons `GET /auth/me` menjadi pengguna + token CSRF.
/// Menerima `data.user{...}` maupun field pengguna langsung di `data`.
fn parse_me(resp: &Value) -> Option<(SessionUser, String)> {
    let d = &resp["data"];
    let u = if d["user"].is_object() { &d["user"] } else { d };
    let user = SessionUser {
        id: vs(u, "id")?,
        username: vs(u, "username").unwrap_or_default(),
        role: parse_role(&vs(u, "role").unwrap_or_default()),
        must_change_password: d["must_change_password"]
            .as_bool()
            .or_else(|| u["must_change_password"].as_bool())
            .unwrap_or(false),
    };
    Some((user, vs(d, "csrf_token").unwrap_or_default()))
}

type CacheMap = HashMap<String, (Instant, Box<dyn Any + Send>)>;

pub struct HttpApiAdapter {
    base_url: String,
    agent: ureq::Agent,
    session: Mutex<Option<ApiSession>>,
    cache: Mutex<CacheMap>,
    last_error: Mutex<Option<String>>,
}

impl HttpApiAdapter {
    pub fn new(base_url: impl Into<String>) -> Self {
        let base_url = base_url.into().trim().trim_end_matches('/').to_string();
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(20))
            .user_agent("keuangan-toko-desktop/0.1")
            .build();
        Self {
            base_url,
            agent,
            session: Mutex::new(None),
            cache: Mutex::new(HashMap::new()),
            last_error: Mutex::new(None),
        }
    }

    fn clear_cache(&self) {
        self.cache.lock().unwrap().clear();
    }

    fn cached<T: Clone + Send + 'static>(
        &self,
        key: String,
        load: impl FnOnce() -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        if let Some((at, boxed)) = self.cache.lock().unwrap().get(&key) {
            if let Some(res) = boxed.downcast_ref::<Result<T, AppError>>() {
                let ttl = if res.is_ok() { OK_TTL } else { ERR_TTL };
                if at.elapsed() < ttl {
                    return res.clone();
                }
            }
        }
        let res = load();
        if let Err(e) = &res {
            *self.last_error.lock().unwrap() = Some(e.0.clone());
        }
        self.cache
            .lock()
            .unwrap()
            .insert(key, (Instant::now(), Box::new(res.clone())));
        res
    }

    fn api_error(&self, status: u16, body: &Value) -> AppError {
        let err = &body["error"];
        let mut msg = err["message"]
            .as_str()
            .map(String::from)
            .unwrap_or_else(|| format!("Server menolak permintaan (HTTP {status})"));
        if let Some(fields) = err["fields"].as_object() {
            for (k, v) in fields {
                if let Some(text) = v.as_str() {
                    msg.push_str(&format!(" • {k}: {text}"));
                }
            }
        }
        if status == 401 {
            *self.session.lock().unwrap() = None;
            self.clear_cache();
        }
        AppError(msg)
    }

    fn request(
        &self,
        method: &str,
        path: &str,
        query: &[(&str, String)],
        body: Option<Value>,
        mutating: bool,
    ) -> Result<Value, AppError> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self
            .agent
            .request(method, &url)
            .set("Accept", "application/json");
        for (k, v) in query {
            req = req.query(k, v);
        }
        if mutating {
            req = req
                .set("Origin", &self.base_url)
                .set("Idempotency-Key", &Uuid::new_v4().to_string());
            if let Some(s) = self.session.lock().unwrap().as_ref() {
                req = req.set("X-CSRF-Token", &s.csrf);
            }
        }
        let result = match body {
            Some(b) => req.send_json(b),
            None => req.call(),
        };
        match result {
            Ok(resp) => {
                let text = resp
                    .into_string()
                    .map_err(|e| AppError(format!("Respons server tidak terbaca: {e}")))?;
                if text.trim().is_empty() {
                    return Ok(Value::Null);
                }
                serde_json::from_str(&text)
                    .map_err(|e| AppError(format!("Respons server tidak valid: {e}")))
            }
            Err(ureq::Error::Status(code, resp)) => {
                let v = resp
                    .into_string()
                    .ok()
                    .and_then(|t| serde_json::from_str::<Value>(&t).ok())
                    .unwrap_or(Value::Null);
                Err(self.api_error(code, &v))
            }
            Err(ureq::Error::Transport(t)) => Err(AppError(format!(
                "Tidak dapat terhubung ke server ({}): {}",
                self.base_url, t
            ))),
        }
    }

    fn get(&self, path: &str, query: &[(&str, String)]) -> Result<Value, AppError> {
        self.request("GET", path, query, None, false)
    }

    fn send(&self, method: &str, path: &str, body: Value) -> Result<Value, AppError> {
        let res = self.request(method, path, &[], Some(body), true);
        if res.is_ok() {
            self.clear_cache();
        }
        res
    }

    fn fetch_pages(&self, path: &str, query: &[(&str, String)]) -> Result<Vec<Value>, AppError> {
        let mut pages = Vec::new();
        let mut cursor: Option<String> = None;
        for _ in 0..MAX_PAGES {
            let mut q: Vec<(&str, String)> = query.to_vec();
            q.push(("limit", PAGE_LIMIT.to_string()));
            if let Some(c) = &cursor {
                q.push(("cursor", c.clone()));
            }
            let body = self.get(path, &q)?;
            cursor = body["meta"]["next_cursor"].as_str().map(String::from);
            pages.push(body);
            if cursor.is_none() {
                break;
            }
        }
        Ok(pages)
    }

    fn require_login(&self) -> Result<(), AppError> {
        if self.session.lock().unwrap().is_some() {
            Ok(())
        } else {
            Err(AppError(
                "Sesi belum aktif. Silakan masuk terlebih dahulu.".into(),
            ))
        }
    }

    fn customer_version(&self, id: &str) -> Result<i32, AppError> {
        let body = self.get(&format!("/api/v1/customers/{id}"), &[])?;
        Ok(body["data"]["customer"]["version"].as_i64().unwrap_or(1) as i32)
    }
}

// ---------------------------------------------------------------------------
// Parser nilai JSON → tipe domain desktop
// ---------------------------------------------------------------------------

fn vs(v: &Value, k: &str) -> Option<String> {
    v.get(k)
        .and_then(|x| x.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
}

fn vm(v: &Value, k: &str) -> i64 {
    v.get(k)
        .and_then(|x| {
            x.as_str()
                .and_then(|s| s.trim().parse().ok())
                .or_else(|| x.as_i64())
        })
        .unwrap_or(0)
}

fn vdate(v: &Value, k: &str) -> Option<NaiveDate> {
    vs(v, k).and_then(|s| NaiveDate::parse_from_str(&s[..s.len().min(10)], "%Y-%m-%d").ok())
}

fn parse_role(s: &str) -> UserRole {
    match s {
        "owner" => UserRole::Owner,
        "viewer" => UserRole::Viewer,
        _ => UserRole::Operator,
    }
}

fn trim_qty(q: String) -> String {
    if q.contains('.') {
        q.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        q
    }
}

fn parse_summary(item: &Value) -> Option<CustomerSummary> {
    let c = &item["customer"];
    Some(CustomerSummary {
        id: vs(c, "id")?,
        code: vs(c, "code"),
        name: vs(c, "name")?,
        address: vs(c, "address"),
        is_active: c["is_active"].as_bool().unwrap_or(true),
        version: c["version"].as_i64().unwrap_or(1) as i32,
        outstanding_idr: vm(item, "outstanding_idr"),
        customer_credit_idr: vm(item, "customer_credit_idr"),
        aging: AgingBuckets {
            days_0_15: vm(item, "days_0_15_idr"),
            days_16_30: vm(item, "days_16_30_idr"),
            days_31_45: vm(item, "days_31_45_idr"),
            days_46_60: vm(item, "days_46_60_idr"),
            days_over_60: vm(item, "days_over_60_idr"),
            unknown_date: vm(item, "unknown_date_idr"),
            future_date: vm(item, "future_date_idr"),
        },
    })
}

fn parse_ledger_entry(e: &Value, order: i64) -> Option<LedgerEntry> {
    let debit = vm(e, "debit_idr");
    let allocated = vm(e, "allocated_idr");
    let remaining = vm(e, "remaining_idr");
    let entry_type = vs(e, "entry_type").unwrap_or_default();
    let status = if debit > 0 {
        Some(if remaining == 0 {
            AllocationStatus::Allocated
        } else if allocated > 0 {
            AllocationStatus::PartiallyAllocated
        } else {
            AllocationStatus::Unallocated
        })
    } else {
        None
    };
    let fallback_desc = match entry_type.as_str() {
        "credit_sale" => "Penjualan kredit",
        "payment" => "Pembayaran",
        "legacy_debit" => "Debit historis",
        "legacy_payment" => "Pembayaran historis",
        "reversal" => "Pembalikan",
        _ => "Transaksi",
    };
    Some(LedgerEntry {
        id: vs(e, "id")?,
        source_id: vs(e, "source_id").unwrap_or_default(),
        effective_date: vdate(e, "effective_date"),
        reference: vs(e, "reference"),
        description: vs(e, "description").unwrap_or_else(|| fallback_desc.to_string()),
        quantity: vs(e, "quantity").map(trim_qty),
        unit_price_idr: vs(e, "unit_price_idr").and_then(|s| s.parse().ok()),
        debit_idr: debit,
        credit_idr: vm(e, "credit_idr"),
        running_balance_idr: vm(e, "running_balance_idr"),
        allocated_idr: allocated,
        remaining_idr: remaining,
        age_days: e["age_days"].as_i64().map(|d| d as i32),
        allocation_status: status,
        is_reversed: e["is_reversed"].as_bool().unwrap_or(false),
        reversal_reason: None,
        replacement_entry_id: None,
        source_order: order,
        entry_type,
    })
}

fn parse_payment(p: &Value) -> Option<PaymentRow> {
    let amount = vm(p, "amount_idr");
    let unallocated = vm(p, "unallocated_idr");
    let allocations = p["allocations"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|a| {
                    Some(PaymentAllocationItem {
                        debit_entry_id: vs(a, "debit_entry_id")?,
                        amount_idr: vm(a, "amount_idr"),
                        debit_description: "Nota piutang".into(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Some(PaymentRow {
        id: vs(p, "id")?,
        customer_id: vs(p, "customer_id").unwrap_or_default(),
        amount_idr: amount,
        received_date: vdate(p, "received_date")?,
        method: vs(p, "method").unwrap_or_else(|| "cash".into()),
        reference: vs(p, "reference"),
        note: vs(p, "note"),
        status: vs(p, "status").unwrap_or_else(|| "posted".into()),
        version: p["version"].as_i64().unwrap_or(1) as i32,
        allocated_idr: amount - unallocated,
        unallocated_credit_idr: unallocated,
        allocations,
    })
}

fn parse_user(u: &Value) -> Option<UserItem> {
    Some(UserItem {
        id: vs(u, "id")?,
        username: vs(u, "username")?,
        role: parse_role(&vs(u, "role").unwrap_or_default()),
        is_active: u["is_active"].as_bool().unwrap_or(true),
        must_change_password: false,
        created_at: vdate(u, "created_at").unwrap_or_default(),
    })
}

fn opt_json(s: &Option<String>) -> Value {
    match s.as_ref().map(|x| x.trim()).filter(|x| !x.is_empty()) {
        Some(t) => Value::String(t.to_string()),
        None => Value::Null,
    }
}

impl TokoStoreAdapter for HttpApiAdapter {
    fn adapter_label(&self) -> String {
        format!(
            "API {}",
            self.base_url
                .trim_start_matches("https://")
                .trim_start_matches("http://")
        )
    }

    fn invalidate_cache(&self) {
        self.clear_cache();
    }

    fn take_last_error(&self) -> Option<String> {
        self.last_error.lock().unwrap().take()
    }

    // ----- Sesi & pengguna -----

    fn current_session(&self) -> Option<SessionUser> {
        self.session
            .lock()
            .unwrap()
            .as_ref()
            .map(|s| s.user.clone())
    }

    fn restore_session(&mut self) -> Option<SessionUser> {
        // GET /auth/me: sesi masih valid? Sekaligus menyegarkan role, flag ganti sandi dan CSRF.
        match self.get("/api/v1/auth/me", &[]) {
            Ok(resp) => {
                let (user, csrf) = parse_me(&resp)?;
                // CSRF baru dari server; bila kosong, pertahankan token sesi sebelumnya.
                let csrf = if csrf.is_empty() {
                    self.session
                        .lock()
                        .unwrap()
                        .as_ref()
                        .map(|s| s.csrf.clone())
                        .unwrap_or_default()
                } else {
                    csrf
                };
                *self.session.lock().unwrap() = Some(ApiSession {
                    user: user.clone(),
                    csrf,
                });
                Some(user)
            }
            Err(_) => {
                *self.session.lock().unwrap() = None;
                self.clear_cache();
                None
            }
        }
    }

    fn login(&mut self, username: &str, password: &str) -> Result<SessionUser, AppError> {
        if username.trim().is_empty() || password.is_empty() {
            return Err(AppError("Nama pengguna dan kata sandi wajib diisi.".into()));
        }
        let resp = self.request(
            "POST",
            "/api/v1/auth/login",
            &[],
            Some(json!({ "username": username.trim(), "password": password })),
            true,
        )?;
        let d = &resp["data"];
        let u = &d["user"];
        let user = SessionUser {
            id: vs(u, "id").ok_or_else(|| AppError("Respons login tidak lengkap.".into()))?,
            username: vs(u, "username").unwrap_or_else(|| username.trim().to_string()),
            role: parse_role(&vs(u, "role").unwrap_or_default()),
            must_change_password: d["must_change_password"].as_bool().unwrap_or(false),
        };
        let csrf = vs(d, "csrf_token").unwrap_or_default();
        *self.session.lock().unwrap() = Some(ApiSession {
            user: user.clone(),
            csrf,
        });
        self.clear_cache();
        Ok(user)
    }

    fn logout(&mut self) {
        let _ = self.request("POST", "/api/v1/auth/logout", &[], Some(json!({})), true);
        *self.session.lock().unwrap() = None;
        self.clear_cache();
    }

    fn switch_session_role(&mut self, _role: UserRole) {
        // Peran ditentukan server; simulasi peran hanya untuk adapter mock.
    }

    fn change_password(&mut self, old_pw: &str, new_pw: &str) -> Result<(), AppError> {
        self.require_login()?;
        self.send(
            "POST",
            "/api/v1/auth/password",
            json!({ "current_password": old_pw, "new_password": new_pw }),
        )?;
        if let Some(s) = self.session.lock().unwrap().as_mut() {
            s.user.must_change_password = false;
        }
        Ok(())
    }

    // ----- Pelanggan -----

    fn list_customers(&self, query: &CustomerQuery) -> Vec<CustomerSummary> {
        let all = self
            .cached("customers".into(), || {
                let pages = self.fetch_pages("/api/v1/customers", &[])?;
                let mut out = Vec::new();
                for p in &pages {
                    if let Some(items) = p["data"]["items"].as_array() {
                        out.extend(items.iter().filter_map(parse_summary));
                    }
                }
                out.sort_by_key(|a| a.name.to_lowercase());
                Ok(out)
            })
            .unwrap_or_default();

        let term = query.search.as_deref().unwrap_or("").trim().to_lowercase();
        all.into_iter()
            .filter(|c| !query.active_only || c.is_active)
            .filter(|c| {
                term.is_empty()
                    || c.name.to_lowercase().contains(&term)
                    || c.code
                        .as_deref()
                        .map(|x| x.to_lowercase().contains(&term))
                        .unwrap_or(false)
                    || c.address
                        .as_deref()
                        .map(|x| x.to_lowercase().contains(&term))
                        .unwrap_or(false)
            })
            .collect()
    }

    fn get_customer(&self, id: &str) -> Result<CustomerDetail, AppError> {
        let v = self.cached(format!("customer|{id}"), || {
            Ok(self.get(&format!("/api/v1/customers/{id}"), &[])?["data"].clone())
        })?;
        let c = &v["customer"];
        Ok(CustomerDetail {
            id: vs(c, "id").unwrap_or_else(|| id.to_string()),
            code: vs(c, "code"),
            name: vs(c, "name").unwrap_or_default(),
            address: vs(c, "address"),
            is_active: c["is_active"].as_bool().unwrap_or(true),
            version: c["version"].as_i64().unwrap_or(1) as i32,
            outstanding_idr: vm(&v, "outstanding_idr"),
            customer_credit_idr: vm(&v, "customer_credit_idr"),
        })
    }

    fn create_customer(&mut self, input: &CreateCustomerInput) -> Result<CustomerDetail, AppError> {
        self.require_login()?;
        let resp = self.send(
            "POST",
            "/api/v1/customers",
            json!({
                "code": opt_json(&input.code),
                "name": input.name.trim(),
                "address": opt_json(&input.address),
            }),
        )?;
        let id = vs(&resp["data"], "id")
            .ok_or_else(|| AppError("Respons pembuatan pelanggan tidak lengkap.".into()))?;
        self.get_customer(&id)
    }

    fn update_customer(
        &mut self,
        id: &str,
        input: &UpdateCustomerInput,
    ) -> Result<CustomerDetail, AppError> {
        self.require_login()?;
        let version = self.customer_version(id)?;
        self.send(
            "PATCH",
            &format!("/api/v1/customers/{id}"),
            json!({
                "expected_version": version,
                "name": input.name.trim(),
                "code": opt_json(&input.code),
                "address": opt_json(&input.address),
                "is_active": input.is_active,
            }),
        )?;
        self.get_customer(id)
    }

    fn toggle_customer_active(&mut self, id: &str) -> Result<bool, AppError> {
        self.require_login()?;
        let detail = self.get_customer(id)?;
        let next = !detail.is_active;
        self.send(
            "PATCH",
            &format!("/api/v1/customers/{id}"),
            json!({ "expected_version": detail.version, "is_active": next }),
        )?;
        Ok(next)
    }

    fn merge_customers(
        &mut self,
        source_id: &str,
        target_id: &str,
        reason: &str,
    ) -> Result<(), AppError> {
        self.require_login()?;
        let version = self.customer_version(source_id)?;
        self.send(
            "POST",
            &format!("/api/v1/customers/{source_id}/merge"),
            json!({
                "target_customer_id": target_id,
                "reason": reason.trim(),
                "expected_version": version,
            }),
        )?;
        Ok(())
    }

    // ----- Buku piutang & penjualan -----

    fn get_ledger(&self, customer_id: &str, query: &LedgerQuery) -> Result<LedgerPage, AppError> {
        let key = format!(
            "ledger|{customer_id}|{:?}|{:?}|{}",
            query.from, query.to, query.order_desc
        );
        let (entries, summary) = self.cached(key, || {
            let mut q: Vec<(&str, String)> = vec![(
                "order",
                if query.order_desc { "desc" } else { "asc" }.to_string(),
            )];
            if let Some(f) = query.from {
                q.push(("from", f.to_string()));
            }
            if let Some(t) = query.to {
                q.push(("to", t.to_string()));
            }
            let pages = self.fetch_pages(&format!("/api/v1/customers/{customer_id}/ledger"), &q)?;
            let mut entries = Vec::new();
            for p in &pages {
                if let Some(arr) = p["data"]["entries"].as_array() {
                    for e in arr {
                        if let Some(entry) = parse_ledger_entry(e, entries.len() as i64) {
                            entries.push(entry);
                        }
                    }
                }
            }
            let s = pages
                .first()
                .map(|p| p["data"]["summary"].clone())
                .unwrap_or(Value::Null);
            let summary = LedgerSummary {
                outstanding_idr: vm(&s, "outstanding_idr"),
                customer_credit_idr: vm(&s, "customer_credit_idr"),
                ledger_balance_idr: vm(&s, "ledger_balance_idr"),
            };
            Ok((entries, summary))
        })?;
        Ok(LedgerPage { entries, summary })
    }

    fn create_instant_sale(
        &mut self,
        customer_id: &str,
        input: &InstantSaleInput,
    ) -> Result<i64, AppError> {
        self.require_login()?;
        let today = chrono::Local::now().date_naive();
        let backdate = if input.effective_date < today {
            opt_json(&input.backdate_reason)
        } else {
            Value::Null
        };
        let items: Vec<&SaleItemInput> = input
            .items
            .iter()
            .filter(|i| !(i.description.trim().is_empty() && i.unit_price_idr <= 0))
            .collect();
        if items.is_empty() {
            return Err(AppError(
                "Minimal masukkan 1 barang dengan nama, kuantitas, dan harga satuan valid.".into(),
            ));
        }
        // Sama seperti web: setiap barang diposting sebagai penjualan sendiri agar tampil per baris.
        let mut total = 0_i64;
        let count = items.len();
        for (idx, item) in items.iter().enumerate() {
            let partial = |e: AppError| {
                if idx > 0 {
                    AppError(format!(
                        "{} ({} dari {} barang sudah terposting; periksa buku piutang sebelum mengulang.)",
                        e.0, idx, count
                    ))
                } else {
                    e
                }
            };
            let draft = self
                .send(
                    "POST",
                    "/api/v1/credit-sales",
                    json!({
                        "customer_id": customer_id,
                        "effective_date": input.effective_date.to_string(),
                        "reference_no": opt_json(&input.reference_no),
                        "backdate_reason": backdate,
                        "items": [{
                            "description": item.description.trim(),
                            "quantity": item.quantity.trim().replace(',', "."),
                            "unit_price_idr": item.unit_price_idr.to_string(),
                        }],
                    }),
                )
                .map_err(partial)?;
            let sale_id = vs(&draft["data"], "id")
                .ok_or_else(|| AppError("Respons draf penjualan tidak lengkap.".into()))?;
            let version = draft["data"]["version"].as_i64().unwrap_or(1);
            let posted = self
                .send(
                    "POST",
                    &format!("/api/v1/credit-sales/{sale_id}/post"),
                    json!({ "expected_version": version }),
                )
                .map_err(partial)?;
            total += vm(&posted["data"]["sale"], "total_idr");
        }
        Ok(total)
    }

    fn reverse_transaction(
        &mut self,
        customer_id: &str,
        entry_id: &str,
        reason: &str,
    ) -> Result<(), AppError> {
        self.require_login()?;
        let page = self.get_ledger(
            customer_id,
            &LedgerQuery {
                from: None,
                to: None,
                order_desc: false,
            },
        )?;
        let entry = page
            .entries
            .iter()
            .find(|e| e.id == entry_id)
            .ok_or_else(|| AppError("Transaksi buku piutang tidak ditemukan.".into()))?;
        // API-304: sale/payment memakai ID sumbernya; entri legacy memakai ID entri ledger.
        let target = match entry.entry_type.as_str() {
            "credit_sale" | "payment" if !entry.source_id.is_empty() => entry.source_id.clone(),
            _ => entry.id.clone(),
        };
        self.send(
            "POST",
            &format!("/api/v1/transactions/{target}/reversal"),
            json!({ "reason": reason.trim() }),
        )?;
        Ok(())
    }

    // ----- Pembayaran & saldo kredit -----

    fn list_payments(&self, customer_id: &str) -> Result<Vec<PaymentRow>, AppError> {
        self.cached(format!("payments|{customer_id}"), || {
            let pages =
                self.fetch_pages(&format!("/api/v1/customers/{customer_id}/payments"), &[])?;
            let mut out = Vec::new();
            for p in &pages {
                if let Some(arr) = p["data"].as_array() {
                    out.extend(arr.iter().filter_map(parse_payment));
                }
            }
            out.sort_by_key(|a| std::cmp::Reverse(a.received_date));
            Ok(out)
        })
    }

    fn record_payment(
        &mut self,
        customer_id: &str,
        input: &RecordPaymentInput,
    ) -> Result<PaymentRow, AppError> {
        self.require_login()?;
        let method = match input.method.trim().to_lowercase().as_str() {
            "bank_transfer" | "transfer bank" | "transfer" => "bank_transfer",
            _ => "cash",
        };
        let resp = self.send(
            "POST",
            "/api/v1/payments",
            json!({
                "customer_id": customer_id,
                "amount_idr": input.amount_idr.to_string(),
                "received_date": input.received_date.to_string(),
                "method": method,
                "reference": opt_json(&input.reference),
                "note": opt_json(&input.note),
            }),
        )?;
        parse_payment(&resp["data"])
            .ok_or_else(|| AppError("Respons pembayaran tidak lengkap.".into()))
    }

    fn apply_credit(
        &mut self,
        customer_id: &str,
        input: &ApplyCreditInput,
    ) -> Result<(), AppError> {
        self.require_login()?;
        self.send(
            "POST",
            &format!("/api/v1/customers/{customer_id}/credit-applications"),
            json!({
                "payment_id": input.payment_id,
                "debit_entry_id": input.debit_entry_id,
                "amount_idr": input.amount_idr.to_string(),
                "reason": input.reason.trim(),
            }),
        )?;
        Ok(())
    }

    fn reallocate_payment(
        &mut self,
        _customer_id: &str,
        input: &ReallocatePaymentInput,
    ) -> Result<(), AppError> {
        self.require_login()?;
        let current = self.get(&format!("/api/v1/payments/{}", input.payment_id), &[])?;
        let version = current["data"]["version"].as_i64().unwrap_or(1);
        let allocations = match input.debit_entry_id.as_deref().map(str::trim) {
            Some(d) if !d.is_empty() && input.amount_idr > 0 => {
                json!([{ "debit_entry_id": d, "amount_idr": input.amount_idr.to_string() }])
            }
            _ => json!([]),
        };
        self.send(
            "POST",
            &format!("/api/v1/payments/{}/reallocation", input.payment_id),
            json!({
                "expected_version": version,
                "allocations": allocations,
                "reason": input.reason.trim(),
            }),
        )?;
        Ok(())
    }

    // ----- Laporan umur piutang -----

    fn get_aging_report(&self, as_of: NaiveDate) -> Result<AgingReport, AppError> {
        self.cached(format!("aging|{as_of}"), || {
            let q = vec![("as_of", as_of.to_string())];
            let summary = self.get("/api/v1/reports/receivables-aging", &q)?["data"].clone();
            let meta: HashMap<String, (Option<String>, bool)> = self
                .list_customers(&CustomerQuery {
                    search: None,
                    active_only: false,
                })
                .into_iter()
                .map(|c| (c.id, (c.code, c.is_active)))
                .collect();

            let mut rows: Vec<AgingReportCustomerItem> = Vec::new();
            let mut index: HashMap<String, usize> = HashMap::new();
            for key in AGING_KEYS {
                if vm(&summary, key) <= 0 {
                    continue;
                }
                let mut bq = q.clone();
                bq.push(("bucket", key.to_string()));
                let pages = self.fetch_pages("/api/v1/reports/receivables-aging/customers", &bq)?;
                for p in &pages {
                    let Some(arr) = p["data"].as_array() else {
                        continue;
                    };
                    for item in arr {
                        let Some(cid) = vs(item, "customer_id") else {
                            continue;
                        };
                        let amount = vm(item, "amount_idr");
                        let pos = *index.entry(cid.clone()).or_insert_with(|| {
                            let (code, active) = meta.get(&cid).cloned().unwrap_or((None, true));
                            rows.push(AgingReportCustomerItem {
                                customer_id: cid.clone(),
                                customer_code: code,
                                customer_name: vs(item, "customer_name").unwrap_or_default(),
                                is_active: active,
                                total_outstanding_idr: 0,
                                days_0_15: 0,
                                days_16_30: 0,
                                days_31_45: 0,
                                days_46_60: 0,
                                days_over_60: 0,
                                unknown_date: 0,
                                future_date: 0,
                            });
                            rows.len() - 1
                        });
                        let r = &mut rows[pos];
                        match key {
                            "days_0_15" => r.days_0_15 += amount,
                            "days_16_30" => r.days_16_30 += amount,
                            "days_31_45" => r.days_31_45 += amount,
                            "days_46_60" => r.days_46_60 += amount,
                            "days_over_60" => r.days_over_60 += amount,
                            "unknown_date" => r.unknown_date += amount,
                            _ => r.future_date += amount,
                        }
                        r.total_outstanding_idr += amount;
                    }
                }
            }
            rows.sort_by_key(|a| std::cmp::Reverse(a.total_outstanding_idr));
            Ok(AgingReport {
                as_of,
                total_open_idr: vm(&summary, "total_open_idr"),
                days_0_15: vm(&summary, "days_0_15"),
                days_16_30: vm(&summary, "days_16_30"),
                days_31_45: vm(&summary, "days_31_45"),
                days_46_60: vm(&summary, "days_46_60"),
                days_over_60: vm(&summary, "days_over_60"),
                unknown_date: vm(&summary, "unknown_date"),
                future_date: vm(&summary, "future_date"),
                customers: rows,
            })
        })
    }

    // ----- Manajemen pengguna (Owner) -----

    fn list_users(&self) -> Result<Vec<UserItem>, AppError> {
        self.cached("users".into(), || {
            let body = self.get("/api/v1/users", &[])?;
            Ok(body["data"]
                .as_array()
                .map(|a| a.iter().filter_map(parse_user).collect())
                .unwrap_or_default())
        })
    }

    fn create_user(&mut self, input: &CreateUserInput) -> Result<UserItem, AppError> {
        self.require_login()?;
        let resp = self.send(
            "POST",
            "/api/v1/users",
            json!({
                "username": input.username.trim(),
                "password": input.temporary_password,
                "role": input.role.as_str(),
            }),
        )?;
        parse_user(&resp["data"])
            .ok_or_else(|| AppError("Respons pembuatan akun tidak lengkap.".into()))
    }

    fn update_user(&mut self, id: &str, input: &UpdateUserInput) -> Result<UserItem, AppError> {
        self.require_login()?;
        let resp = self.send(
            "PATCH",
            &format!("/api/v1/users/{id}"),
            json!({
                "username": input.username.trim(),
                "role": input.role.as_str(),
                "is_active": input.is_active,
            }),
        )?;
        parse_user(&resp["data"])
            .ok_or_else(|| AppError("Respons pembaruan akun tidak lengkap.".into()))
    }

    fn reset_password(&mut self, id: &str, temporary_password: &str) -> Result<(), AppError> {
        self.require_login()?;
        self.send(
            "POST",
            &format!("/api/v1/users/{id}/password-reset"),
            json!({ "temporary_password": temporary_password }),
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_customer_summary_from_contract_shape() {
        let v = json!({
            "customer": {"id":"c1","code":"P017","name":"DESA","address":null,"is_active":true,"version":3},
            "outstanding_idr":"1938200","customer_credit_idr":"0",
            "days_0_15_idr":"0","days_16_30_idr":"0","days_31_45_idr":"0","days_46_60_idr":"0",
            "days_over_60_idr":"1938200","unknown_date_idr":"0","future_date_idr":"0"
        });
        let s = parse_summary(&v).unwrap();
        assert_eq!(s.outstanding_idr, 1_938_200);
        assert_eq!(s.aging.days_over_60, 1_938_200);
        assert_eq!(s.version, 3);
        assert!(s.address.is_none());
    }

    #[test]
    fn parses_ledger_entry_with_negative_running_balance_and_status() {
        let v = json!({
            "id":"e1","entry_type":"legacy_debit","source_id":"s1","is_reversed":false,
            "effective_date":"2025-09-23","reference":null,"description":"FC","quantity":"51.000",
            "unit_price_idr":"300","debit_idr":"15300","credit_idr":"0","allocated_idr":"15300",
            "remaining_idr":"0","age_days":null,"allocation_status":"allocated","running_balance_idr":"-3600"
        });
        let e = parse_ledger_entry(&v, 0).unwrap();
        assert_eq!(e.running_balance_idr, -3600);
        assert_eq!(e.quantity.as_deref(), Some("51"));
        assert_eq!(e.allocation_status, Some(AllocationStatus::Allocated));
        assert_eq!(e.effective_date, NaiveDate::from_ymd_opt(2025, 9, 23));
    }

    #[test]
    fn parses_auth_me_nested_and_flat() {
        let nested = json!({"data":{"user":{"id":"u1","username":"owner","role":"owner"},
            "must_change_password":true,"csrf_token":"tok"}});
        let (u, c) = parse_me(&nested).unwrap();
        assert_eq!(
            (u.id.as_str(), c.as_str(), u.must_change_password),
            ("u1", "tok", true)
        );
        let flat = json!({"data":{"id":"u2","username":"kasir","role":"staff"}});
        let (u, c) = parse_me(&flat).unwrap();
        assert_eq!(u.username, "kasir");
        assert!(c.is_empty());
        assert!(parse_me(&json!({"data":{}})).is_none());
    }

    #[test]
    fn api_error_includes_field_messages_and_drops_session_on_401() {
        let a = HttpApiAdapter::new("https://example.invalid/");
        assert_eq!(a.base_url, "https://example.invalid");
        *a.session.lock().unwrap() = Some(ApiSession {
            user: SessionUser {
                id: "u".into(),
                username: "x".into(),
                role: UserRole::Owner,
                must_change_password: false,
            },
            csrf: "t".into(),
        });
        let err = a.api_error(
            401,
            &json!({"error":{"code":"UNAUTHENTICATED","message":"Sesi tidak aktif","fields":{"name":"Wajib diisi"}}}),
        );
        assert!(err.0.contains("Sesi tidak aktif") && err.0.contains("name: Wajib diisi"));
        assert!(a.current_session().is_none());
    }

    #[test]
    fn parses_payment_amounts_and_allocations() {
        let v = json!({
            "id":"p1","customer_id":"c1","amount_idr":"500000","received_date":"2026-11-01",
            "method":"cash","reference":null,"note":null,"status":"posted","version":2,
            "allocations":[{"debit_entry_id":"d1","amount_idr":"150000"}],"unallocated_idr":"350000"
        });
        let p = parse_payment(&v).unwrap();
        assert_eq!(p.allocated_idr, 150_000);
        assert_eq!(p.unallocated_credit_idr, 350_000);
        assert_eq!(p.allocations.len(), 1);
    }
}
