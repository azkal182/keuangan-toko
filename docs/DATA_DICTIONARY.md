# Data Dictionary — Model Final MVP

**Status:** model konseptual dan tipe/aturan dasar disepakati melalui ADR-007. Rincian tabel/relasi/constraint ada di [ERD.md](ERD.md); migration harus merealisasikan keduanya tanpa menyimpang diam-diam.

## Konvensi

- PostgreSQL; `UUID` primary keys; timestamp `TIMESTAMPTZ` UTC; tanggal usaha `DATE` dalam `Asia/Jakarta`.
- Nilai ledger `BIGINT` rupiah integer. API serialisasi nominal sebagai string digit integer. Quantity `NUMERIC(12,3)` dan dikirim sebagai string decimal.
- Boolean `is_active`, string status terbatas ke enum/check constraint. Financial source/ledger posted immutable; koreksi berupa reversal.
- Single shop pada MVP, sehingga tidak ada `tenant_id`.

## Entitas dan field bisnis

| Entitas | Field utama | Makna/aturan final |
|---|---|---|
| `users` | `id`, `username`, `password_hash`, `role`, `is_active`, `must_change_password`, `created_at`, `updated_at` | Akun personal; role `owner/operator/viewer`; password Argon2id. Password sementara memaksa perubahan saat login; minimal satu Owner aktif. |
| `sessions` | `id`, `user_id`, `token_hash`, `created_at`, `expires_at`, `revoked_at` | Server-side session; token mentah tidak disimpan; revoke pada logout/perubahan keamanan. |
| `customers` | `id`, `code`, `normalized_code`, `name`, `address`, `is_active`, `version` | UUID stabil; code opsional unik setelah trim/case-fold; name wajib; address opsional; tidak hard-delete. |
| `credit_sales` | `id`, `customer_id`, `effective_date`, `reference_no`, `status`, `total_idr`, `version`, `posted_by`, `posted_at` | `draft/posted/reversed`; invoice satu customer, banyak items; draft dapat diedit; posted immutable. |
| `credit_sale_items` | `id`, `sale_id`, `ordinal`, `description`, `quantity`, `unit_price_idr`, `line_total_idr` | Quantity >0 skala max 3; unit price integer IDR; line total dibulatkan half-up per item; diskon tidak termasuk MVP. |
| `payments` | `id`, `customer_id`, `amount_idr`, `received_date`, `method`, `reference`, `note`, `status`, `version`, `created_by`, `posted_at` | Nilai >0; method `cash`/`bank_transfer`; direct post setelah konfirmasi; alokasi FIFO untuk debit terbuka; surplus menjadi customer credit. Version naik pada reallocation Owner. |
| `receivable_entries` | `id`, `customer_id`, `entry_type`, `source_id`, `effective_date`, `debit_idr`, `credit_idr`, `source_order`, `description`, `quantity`, `unit_price_idr` | Ledger append-only. Tepat satu nilai debit/kredit positif. Balance = sum debit − credit. Legacy entry mempertahankan sheet/baris dan detail produk historis. |
| `payment_allocations` | `id`, `payment_id`, `debit_entry_id`, `amount_idr`, `status`, `created_by`, `reason` | Alokasi aktif mengurangi outstanding debit; trigger membatasi sum payment/debit. Kredit surplus diterapkan manual dengan alasan dan audit. Reversal/reallocation tersedia pada PH-07. |
| `import_batches` | `id`, `file_sha256`, `original_filename`, `raw_workbook`, `status`, `uploaded_by`, `approved_by`, `variance_idr`, `variance_reason`, `version` | Status `uploaded/reviewing/approved/committed/closed_with_exceptions/failed`; Owner approves; duplikasi hash membuka batch yang sama, bukan commit ulang. |
| `import_rows` | `id`, `batch_id`, `sheet_name`, `row_number`, `source_customer`, `raw_values`, `parsed_values`, `issues`, `review_status`, `mapped_customer_id`, `review_reason`, `review_note`, `source_fingerprint`, `source_order`, `committed_entry_id`, `version` | PH-06 hanya sheet pelanggan dengan nama B2 dan kode B1. `parsed_values` kandidat transaksi bertanggal: tanggal kosong mewarisi tanggal valid terakhir pada sheet yang sama (`inherited_previous_date`); debit literal atau hasil D×E half-up jika F formula/kosong; kredit literal G. Saldo formula/Analisa tidak digunakan. Sheet tanpa nama dilewati, baris transaksi anomali dikarantina; provenance formula/cell disimpan. |
| `import_row_commits` | `import_row_id`, `ledger_entry_id` | Lineage unik dari satu baris sumber ke maksimal satu event ledger; satu baris tidak dapat di-split menjadi beberapa event. |
| `legacy_credit_allocations` | `id`, `payment_entry_id`, `debit_entry_id`, `amount_idr`, `status`, `created_by`, `reversed_by`, `reversal_reason` | Alokasi pembayaran workbook tanpa mengarang metode tunai/transfer; aktif mengurangi debit dan dapat dibalik Owner dengan alasan audit. |
| `transaction_reversals` | `id`, `original_entry_id`, `reversal_entry_id`, `replacement_entry_id`, `source_type`, `source_id`, `reversed_by`, `reason`, `request_id`, `reversed_at` | Hubungan immutable antara event sumber, kontra-event, dan pengganti opsional. |
| `audit_events` | `id`, `actor_id`, `action`, `entity_type`, `entity_id`, `reason`, `request_id`, `occurred_at`, `metadata` | Append-only; wajib untuk posting, reversal, role change dan import review/commit. Tidak menyimpan secret/raw workbook. |
| `idempotency_records` | `id`, `actor_id`, `route_key`, `idempotency_key`, `request_hash`, `response_status`, `response_body` | Unique actor/route/key; body berbeda dengan key sama → conflict. Retensi mengikuti ledger finansial. |
| `business_settings` | `key`, `value`, `updated_by`, `updated_at` | Konfigurasi toko; MVP `business_timezone=Asia/Jakarta`; perubahan Owner-only dan diaudit. |

## Definisi saldo

- **Gross receivable:** jumlah seluruh debit posted.
- **Outstanding receivable:** jumlah debit posted dikurangi alokasi aktif ke debit tersebut.
- **Customer credit:** jumlah pembayaran posted dikurangi seluruh alokasi aktifnya.
- **Ledger balance:** total debit posted dikurangi total credit posted; customer credit ditampilkan terpisah dan tidak membuat outstanding negatif.
- Reversal entry mengimbangi event asal dan tetap terhubung ke source. Kolom H/J/K/L Excel hanya pembanding, bukan sumber saldo.

## Pemetaan sumber historis

| Workbook | Target / keputusan |
|---|---|
| B1/B2/B3 | Kode/nama/alamat kandidat pelanggan; ID internal baru; code opsional; identitas ambigu dikarantina. |
| A TANGGAL | `effective_date`; kosong/invalid tetap unknown dan exception, tidak auto-fill. |
| B FAKTUR | `reference_no`; tidak dianggap unik untuk data historis. |
| C URAIAN, D JUMLAH, E HARGA | Data item bila lengkap; quantity decimal skala 3; discrepancy `D×E` vs F perlu review. |
| F DEBET | Debit historis kandidat; jumlah asli dipertahankan, setelah review diposting sebagai legacy debit. |
| G KREDIT | Payment legacy; dialokasikan FIFO secara kronologis (`effective_date`, `source_order`) ke debit terlama; surplus tetap customer credit yang menyerap debit transaksi berikutnya. |
| H SALDO, J Keterangan, K Sisa, L Umur Piutang | Formula/hasil turunan; bukan nilai otoritatif dan tidak menjadi mutasi ledger otomatis. |

Baris dengan debit dan kredit bersamaan, tanggal/faktur/identitas tidak jelas masuk quarantine. Evaluasi formula dibatasi secara ketat hanya pada Kolom F (Debet) yang menghasilkan nilai integer positif (misal `=2*5000`); formula template kosong diabaikan agar tidak membanjiri karantina. Workbook asli disimpan sebagai BYTEA terbatas Owner untuk review/reproduksi hingga kebijakan retensi berlaku; jangan menyalin PII mentah ke log/fixture. Baris yang di-commit memiliki lineage `import_row_commits`; pembayaran legacy dialokasikan FIFO kronologis (`effective_date`, `source_order`) dan sisanya tetap kredit pelanggan.
