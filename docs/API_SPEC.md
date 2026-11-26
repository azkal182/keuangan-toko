# API Specification — Kontrak Web v1

**Status:** Kontrak baseline disepakati melalui ADR-007. Implementasi bertahap dicatat di [PROJECT_STATUS](PROJECT_STATUS.md). Breaking change memerlukan `/api/v2` atau ADR migrasi. Rute desktop sync tidak termasuk API v1 web.

## Konvensi

- Base path `/api/v1`; JSON UTF-8. Semua response sukses memakai `{"data": ..., "meta": {}}`; `meta` boleh `{}`.
- ID UUID string. Tanggal bisnis `YYYY-MM-DD` pada zona `Asia/Jakarta`; timestamp RFC3339 UTC.
- Uang IDR dikirim sebagai string integer digit, misalnya `"40000"`, untuk menghindari kehilangan presisi pada JavaScript. Quantity dikirim sebagai string desimal maksimal 3 digit skala.
- Cursor pagination: `cursor` opaque, `limit` default 50 dan maksimum 100. Urutan stabil ditentukan server; response `meta.next_cursor` nullable.
- Error: `{"error":{"code":"VALIDATION_FAILED","message":"Input tidak valid","fields":{"name":"Wajib diisi"}},"request_id":"<uuid>"}`. Kode publik stabil, pesan aman, tidak menyertakan SQL/stack/secret.
- Status: `400` JSON/query malformed; `401` belum login; `403` tidak berizin; `404` tidak ditemukan; `409` konflik/version/idempotency; `413` file terlalu besar; `422` validasi/domain; `429` rate limit; `500` kegagalan internal.
- Browser auth menggunakan cookie sesi server-side `__Host-session` (`Secure`, `HttpOnly`, `SameSite=Strict`, `Path=/`, tanpa `Domain`), HTTPS wajib; mutasi memerlukan CSRF token. GET tidak mengubah data bisnis. Aktivitas sesi dapat memperbarui metadata `last_seen_at`.
- `Idempotency-Key` wajib pada create/post sale, payment, credit application, reversal, dan commit import. Key + body sama mengulang response awal; key sama dengan body berbeda menghasilkan `409 IDEMPOTENCY_CONFLICT`.

## Authentication dan user management

| ID | Method dan path | Izin | Request / hasil |
|---|---|---|---|
| API-001 | `POST /api/v1/auth/login` | publik, rate-limited | `username`, `password`; set cookie sesi, response user, role, `must_change_password`, dan `csrf_token`. |
| API-002 | `POST /api/v1/auth/logout` | login | revoke sesi; response `data: {"logged_out":true}`. |
| API-003 | `GET /api/v1/auth/me` | login | user aktif, role, `must_change_password`, expiry sesi dan `csrf_token` untuk mutasi berikutnya. |
| API-004 | `POST /api/v1/auth/password` | login | `current_password`, `new_password` (min. 15 karakter); verifikasi password kini dan revoke sesi lain, sesi aktif dipertahankan. |
| API-005 | `GET /api/v1/users` | `users:manage` | daftar akun (`id`, `username`, `role`, `is_active`, timestamp); tanpa hash password. |
| API-006 | `POST /api/v1/users` | `users:manage` | `username`, `role`, `password` sementara (min. 15 karakter); akun harus mengganti kata sandi pada login pertama. |
| API-007 | `PATCH /api/v1/users/{id}` | `users:manage` | ubah `username`, `role`, `is_active`; perubahan role/status revoke seluruh sesi terdampak. Tidak dapat mengubah role/nonaktifkan akun sendiri atau menonaktifkan Owner aktif terakhir. |
| API-008 | `POST /api/v1/users/{id}/password-reset` | `users:manage` | `temporary_password` min. 15 karakter; Owner mereset akun lain, sesi target dicabut, dan penggantian diwajibkan sebelum mengakses fitur lain. |
| API-009 | `DELETE /api/v1/users/{id}` | `users:manage` | nonaktifkan akun (soft-delete), pertahankan histori/audit dan revoke sesi; akun sendiri dan Owner aktif terakhir dilindungi. |

Setiap request mutasi browser menyertakan `X-CSRF-Token` yang cocok dengan token sesi; login menerbitkan token, `/auth/me` menyediakannya kembali. API menolak token hilang/tidak cocok dengan `403 CSRF_FAILED`.

## Pelanggan dan buku piutang

| ID | Method dan path | Izin | Request / hasil |
|---|---|---|---|
| API-101 | `GET /api/v1/customers` | `customers:read` | query `search?`, `cursor?`, `limit?`; tiap item berisi customer, `outstanding_idr`, dan `customer_credit_idr` dari ledger/alokasi aktif; berikutnya `next_cursor`. |
| API-102 | `POST /api/v1/customers` | `customers:write` | `code?`, `name`, `address?`; response customer. Kode ternormalisasi duplikat `409`. |
| API-103 | `GET /api/v1/customers/{id}` | `customers:read` | customer, saldo outstanding, saldo kredit terpisah. |
| API-104 | `PATCH /api/v1/customers/{id}` | `customers:write` | identitas, `expected_version`; hanya field identitas/status, bukan ledger; konflik `409`. |
| API-107 | `POST /api/v1/customers/{id}/merge` | `customers:merge` | `target_customer_id`, `reason`, `expected_version`; merge hanya Owner, customer sumber dinonaktifkan, referensi historis dialihkan atomik dan diaudit; tidak menggabungkan ledger event. |
| API-105 | `GET /api/v1/customers/{id}/ledger` | `ledger:read` | `cursor?`, `limit?`, `from?`, `to?`, `order?` (`asc`/`desc`, default `asc`); entries berurutan, `source_order` tie-breaker, `allocated_idr`, `is_reversed`, summary, balances dan next cursor. |
| API-106 | `GET /api/v1/customers/{id}/payments` | `payments:read` | cursor; histori penerimaan dan alokasi. |

## Penjualan dan pembayaran

| ID | Method dan path | Izin | Request / hasil dan efek |
|---|---|---|---|
| API-201 | `POST /api/v1/credit-sales` | `sales:write` | `customer_id`, `effective_date`, `reference_no?`, `backdate_reason?`, `items[]` (`description`, `quantity`, `unit_price_idr`); membuat draft. Idempotency wajib. |
| API-202 | `PATCH /api/v1/credit-sales/{id}` | `sales:write` | field draft yang diubah dan `expected_version`; `items` yang disertakan menggantikan semua item. Sale posted tidak bisa diubah. |
| API-203 | `POST /api/v1/credit-sales/{id}/post` | `sales:write` | `expected_version`; posting atomik, ledger debit satu kali, response total/status/balance. Idempotency wajib. |
| API-204 | `GET /api/v1/credit-sales/{id}` | `sales:read` | sale, items, status dan ledger reference. |
| API-301 | `POST /api/v1/payments` | `payments:write` | `customer_id`, `amount_idr` (string digit IDR), `received_date`, `method` (`cash`/`bank_transfer`), `reference?`, `note?`; direct post, alokasi FIFO, sisa menjadi customer credit. Memerlukan CSRF dan Idempotency-Key. |
| API-302 | `POST /api/v1/customers/{id}/credit-applications` | `credits:apply` | `payment_id` dapat berupa ID payment normal atau `source_id` entry `legacy_payment`; `debit_entry_id`, `amount_idr`, `reason`; alokasi manual same-customer, validasi saldo, audit. Memerlukan CSRF dan Idempotency-Key. |
| API-305 | `POST /api/v1/payments/{id}/reallocation` | `credits:reallocate` (Owner) | `allocations[]` (`debit_entry_id`, `amount_idr`), `reason`, `expected_version`; membalik alokasi terdampak dan membuat allocation pengganti secara atomik; payment/customer/amount tervalidasi, audit wajib. Idempotency wajib. |
| API-303 | `GET /api/v1/payments/{id}` | `payments:read` | payment, allocations, unallocated amount, status. |
| API-304 | `POST /api/v1/transactions/{id}/reversal` | `transactions:reverse` (Owner) | `id` adalah ID sale/payment atau ID ledger entry legacy; `reason` wajib; membuat kontra-entry dan audit; tidak menghapus event asal. Idempotency wajib. |
| API-306 | `POST /api/v1/transactions/{id}/replacement` | `transactions:reverse` (Owner) | Tautkan transaksi pengganti ke koreksi yang sudah dibalik; tidak mengubah ledger. Memerlukan alasan dan Idempotency-Key. |

## Laporan aging

| ID | Method dan path | Izin | Request / hasil |
|---|---|---|---|
| API-501 | `GET /api/v1/reports/receivables-aging` | `reports:read` | query `as_of?` default tanggal hari ini Asia/Jakarta. Balas buckets `days_0_15`, `days_16_30`, `days_31_45`, `days_46_60`, `days_over_60`, `unknown_date`, `future_date`, dan `total_open_idr`; nominal string. |
| API-502 | `GET /api/v1/reports/receivables-aging/customers` | `reports:read` | query `as_of?`, `bucket`, `cursor?`, `limit?`; rincian piutang terbuka per pelanggan di bucket terpilih, nominal string. |

`as_of` hanya mengubah perhitungan umur kalender, menggunakan saldo piutang terbuka saat laporan dibaca; tanggal masa depan ditolak. Tanggal tidak diketahui dan tanggal transaksi setelah `as_of` adalah exception terpisah, tetapi tetap termasuk `total_open_idr`. Bucket merujuk tanggal usaha debit. Kredit pelanggan yang belum dialokasikan tidak di-aging.

## Import workbook

| ID | Method dan path | Izin | Request / hasil dan efek |
|---|---|---|---|
| API-601 | `POST /api/v1/import-batches` | `imports:upload` | multipart `.xlsx`, maksimum 25 MiB; staging hanya sheet pelanggan bernama (kode B1/nama B2), mengabaikan `Analisa`; tanggal kosong mewarisi tanggal valid terakhir dalam sheet yang sama dan diberi issue `inherited_previous_date`. Saldo direkonstruksi dari transaksi: debit literal atau D×E half-up bila F formula/kosong, kredit literal G. Saldo formula tidak digunakan. Re-upload hash identik mengembalikan batch lama atau memperbarui batch belum direview/dikomit ke aturan ini. |
| API-602 | `GET /api/v1/import-batches/{id}` | `imports:read` | status, summary, variance, exception counts, dan `uncomputable_amount_rows` agar Owner tahu apakah variance diperlukan sebelum commit. |
| API-606 | `GET /api/v1/import-batches/{id}/customer-identities` | `imports:read` (Owner) | Preview identitas per sheet; resolution `matched_code`, `matched_name`, `will_create`, `ambiguous`, atau `missing_identity`. Tidak membuat customer atau ledger. |
| API-603 | `GET /api/v1/import-batches/{id}/rows` | `imports:read` | filter status, `review_status=valid` (candidate + accepted), sheet dan cursor; raw row hanya untuk user berizin, escaped pada UI. |
| API-604 | `PATCH /api/v1/import-batches/{id}/rows/{row_id}` | `imports:review` (Owner) | Mapping/override, accept/exclude/quarantine dan reason wajib; simpan reviewer, versi review dan provenance. Usulan split disimpan sebagai catatan saja: satu row maksimal satu event. |
| API-605 | `POST /api/v1/import-batches/{id}/commit` | `imports:commit` (Owner) | `row_ids[]` mencakup semua `candidate` dan `accepted` untuk sheet pelanggan yang dipilih; sheet pelanggan lain dapat dikomit kemudian selama batch masih `reviewing`. Request atomik memakai `expected_version` dan variance acknowledgement/reason jika diperlukan. Karantina/pengecualian tetap di luar commit. Pelanggan baru yang identitasnya jelas dibuat dalam transaksi yang sama dengan ledger. Batch tetap `reviewing` selama masih ada candidate/accepted/quarantined yang dapat ditinjau; final menjadi `closed_with_exceptions` bila hanya excluded yang tersisa, atau `committed` bila semua baris selesai tanpa exception. Response berisi `created_customers`. Satu row maksimal satu ledger entry, lineage, alokasi FIFO kronologis `(effective_date, source_order)`, audit dan idempotency. |
| API-606 | `GET /api/v1/import-batches/{id}/customer-identities` | `imports:read` (Owner) | Preview identitas per sheet pelanggan, jumlah valid/kandidat/diterima/karantina/dikecualikan/committed, jumlah nominal tak terhitung, debit/kredit valid dan pending, baris valid yang sudah dimapping, serta resolution (`matched_code`, `matched_name`, `will_create`, `ambiguous`, `missing_identity`, `formula_identity`, `inactive_match`). Formula saldo tidak ikut agregat. Endpoint baca saja dan tidak membuat customer/ledger. |
| API-607 | `POST /api/v1/import-batches/{id}/customer-identities/prepare` | `imports:review` (Owner) | Backfill identitas B1/B2/B3 dari raw workbook untuk row legacy yang metadata-nya kosong; idempotent, audit, tidak membuat customer atau ledger. Batch lama dengan sheet Analisa harus diunggah ulang sebelum review untuk restaging ke kebijakan OQ-23. |

## Idempotency, concurrency, dan privasi

- Sale post, payment, credit application, reversal dan import commit berjalan dalam transaksi PostgreSQL atomik; gagal berarti tidak ada efek parsial.
- Posting payment mengunci customer dan memvalidasi alokasi melalui trigger PostgreSQL untuk mencegah over-allocation saat concurrent requests. Balances server-authoritative. Payment gagal jika penulisan payment, allocation, ledger, audit, atau idempotency gagal; retry key + body sama mengembalikan respons tersimpan, body berbeda menghasilkan `409 IDEMPOTENCY_CONFLICT`.
- Draft update menggunakan `expected_version`; mismatch menghasilkan `409 VERSION_CONFLICT`.
- Setiap response membawa `request_id`; log aplikasi tidak menyimpan password, cookie, token, raw workbook, atau nilai pelanggan lengkap.
- Private responses memakai `Cache-Control: no-store`. CORS dibatasi same-origin.

## Payload contoh final

```json
{
  "customer_id": "9f5f4eac-a846-4c4b-a988-2a0e4cb5ef70",
  "amount_idr": "40000",
  "received_date": "2026-11-23",
  "method": "bank_transfer",
  "reference": "TRF-001"
}
```

## Implementasi PH-02

[OpenAPI](../packages/api-client/openapi.json) dan [tipe TypeScript](../packages/api-client/src/schema.ts) digenerasi dari `toko-contracts`. Implementasi mencakup API-001–009, API-101–105, dan API-107. Login menerbitkan cookie sesi; `/auth/me` memberikan role, `must_change_password`, expiry dan token CSRF. Password sementara memblokir rute bisnis sampai diganti. Pengelolaan user hanya Owner, menonaktifkan akun tanpa menghapus lineage, mempertahankan minimal satu Owner aktif, dan mencabut sesi setelah reset/perubahan akses. API-106 tetap mengikuti kontrak baseline.

API-102 mengembalikan HTTP 201. Untuk API-104, payload sintetis `{"expected_version":1,"code":null}` menghapus kode opsional. Field yang absen tidak diubah. Kode duplikat menghasilkan `409 CUSTOMER_CODE_CONFLICT` dengan error field `code`. Versi usang menghasilkan `409 VERSION_CONFLICT` dengan error field `expected_version`.

API-103 memisahkan `outstanding_idr` dan `customer_credit_idr`. API-105 mengurutkan ledger dari tanggal usaha terlama, lalu nomor baris sumber (`source_order`) dan waktu pencatatan, dengan tanggal yang tidak diketahui di akhir. Parameter `order=desc` menyediakan pengurutan terbaru di atas secara konsisten langsung dari server. `from` dan `to` memfilter entri secara inklusif. Saldo berjalan memperhitungkan entri sebelum rentang filter dan ringkasan mencakup semua tanggal. Saldo bersih ledger dan saldo berjalan dikirim sebagai string integer yang dapat bernilai negatif (pada UI ditampilkan dengan format kurung akuntansi merah). Cursor ledger hanya berlaku bagi pelanggan yang mengeluarkannya.

## Implementasi PH-03

API-201 hingga API-204 tersedia di [OpenAPI](../packages/api-client/openapi.json). API-201 mengembalikan HTTP 201 dan draft di `data`. API-202 dan API-204 mengembalikan penjualan dan item di `data` dengan total hasil kalkulasi server. API-203 mengembalikan `data.sale` dan snapshot `data.balance` setelah posting. Nilai rupiah dan kuantitas dikirim sebagai string. Contoh dua item adalah `{"description":"Produk A","quantity":"1.250","unit_price_idr":"10001"}` dan `{"description":"Produk B","quantity":"0.500","unit_price_idr":"101"}`. Total item adalah `"12501"` dan `"51"`, sehingga total penjualan `"12552"`.

Tanggal usaha diperiksa di Asia/Jakarta saat membuat, mengubah, dan memposting draft. Tanggal lampau memerlukan Owner dan `backdate_reason` pada draft. Posting menggunakan alasan yang sudah tersimpan dan menyertakannya di audit. Jika draft berubah, `expected_version` usang menghasilkan `409 VERSION_CONFLICT`.

API-201 dan API-203 menyimpan status HTTP dan isi respons sukses bersama efek transaksi. Lingkup kunci adalah akun, operasi, dan ID penjualan untuk posting. JSON dengan isi sama mengulang respons awal walaupun urutan properti berbeda; isi berbeda dengan kunci yang sama menghasilkan `409 IDEMPOTENCY_CONFLICT`. Jika transaksi gagal, kunci dapat dipakai lagi karena belum tersimpan.

## Implementasi PH-05

API-501 merangkum debit terbuka yang berasal dari `legacy_debit` dan penjualan kredit posted, dikurangi alokasi aktif dari pembayaran posted. Laporan memakai snapshot PostgreSQL read-only repeatable-read; nilai uang dijumlahkan sebagai integer dan diserialisasi sebagai string IDR. `as_of` adalah tanggal acuan umur terhadap outstanding saat ini, bukan snapshot historis saldo. API-502 mengelompokkan rincian per pelanggan dan memakai cursor UUID stabil. Owner, Operator, dan Viewer memiliki izin baca.

Contoh ringkasan: `{"data":{"as_of":"2026-11-23","days_0_15":"60000","days_16_30":"0","days_31_45":"0","days_46_60":"0","days_over_60":"0","unknown_date":"0","future_date":"0","total_open_idr":"60000"},"meta":{}}`.

## Implementasi PH-06/PH-07

API-601–605 menangani staging workbook dengan hash SHA-256, penyimpanan raw/provenance, parser XLSX defensif, evaluasi formula terbatas strictly pada Kolom F (Debet) bernilai bilangan bulat positif, review berversi Owner, rekonsiliasi dan commit atomik. Satu baris sumber membuat paling banyak satu `legacy_debit` atau `legacy_payment`; kredit historis dialokasikan FIFO secara kronologis berdasarkan `(effective_date, source_order)` sehingga nota lama dilunasi lebih dahulu dan sisa kelebihan bayar mengalir menyerap debit berikutnya. Formula saldo bukan input ledger. Upload hash identik dan replay commit tidak menggandakan efek.

API-304 membalik payment, sale atau entry legacy melalui kontra-entry serta mengembalikan alokasi aktif; reversal sale mensyaratkan alokasi debit telah dipindahkan. API-305 membalik alokasi payment lama lalu menulis pilihan baru dengan `expected_version`; surplus boleh tetap menjadi customer credit. API-306 menyimpan tautan ke event pengganti tanpa mengubah event keuangan. Semua mutasi Owner-only, beralasan, idempotent, dan tercatat dalam audit.
