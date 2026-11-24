# Status Proyek — Keuangan Toko

**Diperbarui:** 24 November 2026
**Repo kerja:** `D:\azka\keuangan-toko` — dipilih pengguna.
**Sumber status:** bukti dokumen dan pemeriksaan filesystem/git; bukan klaim aplikasi berjalan.

## Ringkasan

- **Fase berikutnya:** `PH-08` — production readiness web.
- **Status aplikasi:** login, pelanggan, buku piutang, penjualan kredit, pembayaran, laporan aging, import staging/review/commit, serta koreksi dan audit telah terhubung ke API/PostgreSQL. PH-06 kini merekonsiliasi transaksi tiap sheet pelanggan bernama; summary per pelanggan dan dialog review telah dibangun. Parser baru belum diverifikasi dengan upload workbook aktual atau commit.
- **Perubahan layar pelanggan (24 November):** daftar utama kini berupa tabel ringkasan saldo piutang dan kredit per pelanggan dari ledger; klik nama membuka detail. Form tambah/ubah dan koreksi transaksi historis menggunakan dialog. Ini melengkapi PH-02; tidak mengubah kebijakan saldo historis.
- **Audit backend dan suite PostgreSQL (24 November):** `cargo test --locked -p toko-api --features postgres-tests --test postgres_foundation` berhasil pada PostgreSQL 18.7 setelah assertion pencarian pelanggan diselaraskan ke `CustomerSummary.customer.id` dan workbook sintetis memakai baris identitas serta transaksi sesuai kontrak parser. Temuan Clippy pada kondisi indeks, evaluasi `Option`, dan kompleksitas tiga tuple SQL telah diperbaiki; `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo fmt --all -- --check`, dan `cargo test --workspace --locked` berhasil dijalankan ulang. Audit ini belum mencakup pembuktian keamanan menyeluruh atau benchmark performa.
- **Draft deployment PH-08 (24 November):** Dockerfile API/web, Compose, Caddy untuk `piutang.banza.web.id`, dan workflow GitHub Actions CI/CD telah ditambahkan. Compose tervalidasi dan CI lokal untuk typecheck, build web, Clippy, integrasi PostgreSQL, serta OpenAPI berhasil. Build image Docker lokal terhalang sertifikat TLS proxy pada akses crates.io/npm (`SELF_SIGNED_CERT_IN_CHAIN`); verifikasi TLS tetap aktif. VPS belum disentuh. Deployment menunggu GitHub Actions secrets, akses dan konfigurasi VPS, tes build image di runner GitHub, serta backup/restore dan mekanisme purge.
- **Perbaikan blank frontend (24 November):** UI menerima bentuk item daftar pelanggan API lama maupun baru selama restart backend, sehingga item legacy tidak menyebabkan React crash; saldo ditampilkan `—` jika respons backend belum menyertakannya. Vite aktif di port 5180 menyajikan modul terbaru; `/api/v1/auth/me` merespons 401 normal tanpa sesi. `pnpm typecheck` dan `pnpm build` berhasil.
- **Alur homepage pelanggan (24 November):** diperjelas setelah umpan balik pengguna: homepage sekarang memakai tabel ringkasan selebar halaman; detail hanya dibuka setelah nama diklik, dengan aksi kembali ke ringkasan. Implementasi awal masih mempertahankan daftar samping dan tidak memenuhi alur yang diminta.
- **Keputusan tersedia:** seluruh OQ awal ditutup di ADR-007; web-first; Rust/Axum, React/TypeScript, PostgreSQL; Tailwind CSS + shadcn/ui (ADR-006); layout/workspace, API, ERD, auth, security, quality dan operations baseline tersedia. Skills berada di `.agents/skills/`.
- **Dokumentasi:** baseline kebijakan, kontrak, schema, keamanan, operasi, fase, dan skill proyek telah direkonsiliasi per 23 November 2026; semua OQ awal ditutup. Bukti implementasi dan acceptance criteria per fase tercantum pada register dan bagian bukti berikut.
- **Workbook:** tersedia lokal di `docs/files/CATATAN KEUANGAN TOKO.xlsx`, diabaikan oleh Git. Pemeriksaan struktur bukan validasi data atau rekonsiliasi.

## Register fase

| Fase | Status | Bukti sekarang | Kekurangan / exit berikut |
|---|---|---|---|
| `PH-00` Repo & tata kerja | **Selesai** | Cargo/pnpm workspace dan lockfile, toolchain terkunci, scaffold Axum/web, pemeriksaan baseline tercatat di bawah. | Gate berikut: `PH-01`. |
| `PH-01` Fondasi slice pelanggan | **Selesai** | Migrasi 0001, kontrak Rust/OpenAPI/TypeScript, guard izin, dan fixture sintetis. Bukti exit gate di bawah. | Gate berikut: `PH-02`. |
| `PH-02` Pelanggan & buku piutang baca | **Selesai** | UI, Axum, PostgreSQL dan typed client terhubung. `AC-001`, `AC-002`, `AC-007`, `AC-008` terverifikasi. | Gate berikut: `PH-03`. |
| `PH-03` Penjualan kredit | **Selesai** | Draft, review, posting atomik dan riwayat penjualan terhubung. `AC-003` dan `AC-004` terverifikasi. | Exit gate selesai; lanjut PH-04. |
| `PH-04` Pembayaran & alokasi | **Selesai** | Migrasi 0004, domain FIFO, API pembayaran/aplikasi kredit, UI riwayat/form/review, dan OpenAPI/client tersedia. AC-005/006 serta surplus/aplikasi kredit diverifikasi lewat Chrome UI→Axum→PostgreSQL; atomicity, retry, dan concurrency diverifikasi lewat suite PostgreSQL. | Exit gate terpenuhi; lanjut PH-05. |
| `PH-05` Aging & ringkasan | **Selesai** | API-501/502, UI laporan dan rincian pelanggan, bucket/tanggal Jakarta serta suite PostgreSQL untuk AC-009/010. Bukti exit gate di bawah. | Gate selesai; lanjut PH-06. |
| `PH-06` Import workbook | **Berjalan** | OQ-22/23/24: Owner preview; match kode lalu nama unik; create customer atomik; tanggal kosong mewarisi tanggal valid terakhir per sheet dengan issue provenance. Parser mengabaikan `Analisa`, menghitung transaksi sheet pelanggan bernama, debit literal atau D×E, kredit literal. UI memiliki ringkasan batch dan net per pelanggan dalam tabel, review melalui dialog. Staging dan commit memakai lookup duplikasi satu kali serta insert/update berkelompok; alokasi FIFO dihitung berurutan per pelanggan lalu ditulis berkelompok. `cargo check --locked --workspace` berhasil. | Verifikasi ulang commit manual setelah optimasi, rekonsiliasi Owner, dan pengecekan perilaku batch lama. |
| `PH-07` Koreksi, audit & akses | **Selesai** | Permission Owner-only; suite menguji Operator ditolak, reallocation membalik alokasi dan menaikkan versi, reversal sale/payment menghasilkan kontra-entry/audit, replacement link tersimpan; ledger juga mengekspos reversal state untuk koreksi legacy di UI. | Gate terverifikasi; lanjut PH-08. |
| `PH-08` Production readiness web | Belum mulai | Draft Docker Compose dan workflow CI/CD untuk VPS Ubuntu 24 serta domain Cloudflare sudah disiapkan; belum diuji di GitHub atau VPS. Mekanisme purge otomatis sesuai retensi workbook/raw staging belum tersedia. | Selesaikan PH-06 sesuai aturan satu fase aktif; kemudian uji workflow, siapkan secret/deploy access, backup/restore/observability, purge, deploy dan buktikan gates. |
| `PH-09` Desktop offline-first | Ditunda | Policy sync/72h/Windows 11 final; belum dimulai. | Mulai setelah web stabil; benchmark hardware dan sync verification. |

**Catatan PH-06 (24 November 2026):** troubleshooting upload lama membuat staging 18.142 baris dengan parser generik; angka tersebut bukan hasil parser final dan batch harus diunggah ulang bila belum direview/dikomit. Profil XML tanpa menampilkan nilai pelanggan menemukan sheet `Analisa` dan 103 sheet lain; 34 sheet memiliki nama B2 tersimpan, dengan tanggal A, jumlah D, harga E, debit F, kredit G, dan saldo formula H. Profil menemukan 9.418 baris bernama dengan data transaksi literal setelah header; baris formula-only diabaikan. Parser terbaru melewati `Analisa` dan sheet tanpa nama B2, memakai debit F literal atau menghitung D×E saat F formula/kosong, memakai hanya kredit G literal, serta tidak memakai saldo H. Verifikasi ingest baru, review Owner, rekonsiliasi dan commit belum dilakukan.

Implementasi PH-06 menambahkan endpoint preview identitas dan agregat per pelanggan, auto-match kode lalu nama ternormalisasi, auto-create customer ketika Owner commit, serta pewarisan tanggal kosong per sheet dengan provenance. Commit tetap satu transaksi untuk customer, ledger, allocation, dan audit; kandidat yang gagal resolve saat commit dikarantina. Perubahan tanggal lolos `cargo check --locked --workspace`, `pnpm typecheck`, dan `pnpm build`; parser baru belum dijalankan terhadap workbook aktual. Investigasi request commit 500 menemukan bulk insert alokasi legacy menyebut kolom `status` tanpa nilai; kini setiap alokasi ditulis dengan status `active`. Request berikutnya mengidentifikasi SQLSTATE `42601` pada update bulk import rows: `coalesce(...)` tidak ditutup sebelum klausa `version`; sintaks telah diperbaiki. Logging tahap dan SQLSTATE tersedia untuk operasi database commit tanpa query atau nilai data. UI PH-06 menampilkan ringkasan per pelanggan, membuka baris valid melalui dialog, dan mendukung commit parsial; batch tetap `reviewing` selama ada baris valid atau karantina yang bisa diproses. Tabel mempertahankan debit/kredit/net sumber dan menambahkan count valid/exception, net valid, variance pending, pencocokan, tujuan serta aksi buka baris. Request `328f31b3-9912-4539-97ca-d3a394e5f859` mengungkap aggregate sumber commit dihitung sesudah row status diubah menjadi `committed`, sehingga sumber terbaca nol; aggregate kini diambil sebelum transisi status. UI menjelaskan rumus variance, menghitung dan mengisi variance otomatis bila nominal lengkap; mismatch API menyertakan angka hitung server. `cargo fmt --all -- --check`, `cargo check --locked --workspace`, `pnpm typecheck`, dan `pnpm build` berhasil. API dijalankan pada port 3002; proxy Vite merespons auth/me `401 application/json` tanpa sesi. Commit workbook nyata belum diverifikasi ulang.

## Bukti exit gate PH-00

- Workspace [Cargo](../Cargo.toml) dan [pnpm](../pnpm-workspace.yaml) tersedia bersama [Cargo.lock](../Cargo.lock) dan [pnpm-lock.yaml](../pnpm-lock.yaml). Rust 1.100.0, Node.js 26.13.1 (LTS), dan pnpm 12.21.0 terkunci di konfigurasi repo.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, dan `cargo build --workspace --locked` berhasil. `cargo test --workspace --locked` berhasil dengan 0 kasus uji.
- `pnpm install --frozen-lockfile --offline`, `pnpm typecheck`, dan `pnpm build` berhasil.
- Smoke test HTTP lokal menggunakan `cargo run --locked -p toko-api` dengan port loopback sementara mengembalikan 404 dari router kosong.
- Pemeriksaan kandidat Git mengonfirmasi `.env` dan workbook dikecualikan dan `.env.example` termasuk kandidat. Tidak ada commit yang dibuat pada tugas ini.
- Perintah yang tersedia dicatat di [OPERATIONS](OPERATIONS.md#perintah-pengembangan-lokal).

## Bukti exit gate PH-01

- Migrator `cargo run --locked -p toko-api --bin migrate-postgres` berhasil dijalankan dua kali di PostgreSQL 18.7 lokal terisolasi.
- `cargo test --locked -p toko-api --features postgres-tests --test postgres_foundation` berhasil. Satu suite integrasi memeriksa constraint migrasi, validasi sesi pada API-003, serta akses baca, tulis, merge, dan CSRF melalui guard yang dipakai API.
- Smoke test HTTP loopback ke `GET /api/v1/auth/me` tanpa sesi mengembalikan `401 UNAUTHENTICATED`, request ID yang konsisten, dan `Cache-Control: no-store`.
- [Fixture SQL](../apps/api/tests/fixtures/synthetic.sql) sepenuhnya sintetis dan dimuat ke skema uji. Workbook historis tidak digunakan.
- `cargo test --workspace --locked` berhasil. Unit test memeriksa normalisasi nama/kode pelanggan dan semantik patch nullable.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo build --workspace --locked`, `pnpm typecheck` dan `pnpm build` berhasil.
- `pnpm api:generate` dan `pnpm api:check` berhasil. Artefak [OpenAPI](../packages/api-client/openapi.json) dan [tipe TypeScript](../packages/api-client/src/schema.ts) tersedia.

## Bukti exit gate PH-02

| AC | Metode | Hasil |
|---|---|---|
| `AC-001` | Chromium 1024×768 melalui HTTPS Vite, Axum, dan PostgreSQL 18.7 | Pencarian kode/nama, detail yang sesuai, serta baca oleh Viewer berhasil. |
| `AC-002` | Chromium dan suite integrasi PostgreSQL | Tambah dan ubah tersimpan setelah refresh. Nama kosong, kode duplikat setelah normalisasi, dan versi usang ditolak dengan field error. UI dapat memuat ulang detail setelah konflik. |
| `AC-007` | Chromium dan fixture sintetis PostgreSQL | Debit Rp100.000 dengan pembayaran teralokasi Rp40.000 menghasilkan outstanding Rp60.000. Pembayaran yang belum dialokasikan tampil terpisah sebagai kredit pelanggan. Angka berasal dari ledger dan alokasi. |
| `AC-008` | Chromium dan suite integrasi PostgreSQL | Riwayat A dan B terpisah. Cursor riwayat A ditolak untuk B. |

- API dan UI juga mencakup login/logout, pembatasan role, penonaktifan, dan merge Owner dengan audit. Uji Chromium dijalankan dengan Playwright sementara. Suite integrasi memverifikasi CSRF, pemindahan riwayat, dan pencatatan audit merge.
- Migrator dijalankan dua kali pada basis data uji kosong dengan migrasi 0001 dan 0002. Pengujian memakai fixture sintetis yang tidak berasal dari workbook.
- Perintah pemeriksaan berhasil meliputi `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo test --locked -p toko-api --features postgres-tests --test postgres_foundation`, `cargo build --workspace --locked`, `pnpm api:check`, `pnpm typecheck`, dan `pnpm build`. Lihat [OPERATIONS](OPERATIONS.md#perintah-pengembangan-lokal).

## Bukti exit gate PH-03

| AC | Metode | Hasil |
|---|---|---|
| `AC-003` | Chromium melalui HTTPS Vite, Axum, dan PostgreSQL 18.7 | Draft dan edit tidak mengubah saldo. Setelah review dua item dan posting, saldo naik dari Rp60.000 menjadi Rp72.552. Tautan buku piutang membuka detail penjualan. |
| `AC-004` | Suite integrasi PostgreSQL dan Chromium | Kegagalan audit dipaksakan setelah update status dan insert debit. Draft, versi, ledger, audit, serta record idempotensi tetap seperti sebelum posting. Dua permintaan posting konkuren dan retry browser setelah respons hilang menghasilkan satu debit. Isi berbeda dengan kunci sama ditolak dengan 409. |

- Pengujian juga mencakup izin Viewer/Operator/Owner, tanggal lampau dan masa depan, versi draft, proteksi item setelah posting, serta relink sale/ledger saat merge. Regresi Chromium untuk pelanggan dan buku piutang juga berhasil.
- Migrasi 0001–0003 berhasil diterapkan dua kali pada database kosong. Pengujian memakai fixture sintetis dan cluster terisolasi.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo test --locked -p toko-api --features postgres-tests --test postgres_foundation`, `cargo build --workspace --locked`, `pnpm api:check`, `pnpm typecheck`, dan `pnpm build` berhasil.
- Draft dapat dibuka kembali melalui URL detail. Penjualan posted juga dapat dibuka dari buku piutang. API-201–204 menggunakan schema [OpenAPI](../packages/api-client/openapi.json).

## Bukti exit gate PH-04

| AC / invariant | Metode | Hasil |
|---|---|---|
| `AC-005` | Chrome headless pada host Windows, Vite, Axum, PostgreSQL 16.15 | Dari customer detail, pembayaran Rp40.000 pada debit Rp100.000 diposting; UI menampilkan outstanding Rp60.000. PostgreSQL mengonfirmasi ledger balance/outstanding Rp60.000. |
| `AC-006` | Chrome headless pada host Windows, Vite, Axum, PostgreSQL 16.15 | Dari customer detail, pembayaran Rp150.000 pada debit Rp100.000 + Rp200.000 mengalokasikan Rp100.000 dan Rp50.000; UI menampilkan outstanding Rp150.000. PostgreSQL mengonfirmasi kedua allocation tersebut. |
| Surplus dan kredit manual | Chrome UI→API→PostgreSQL 16.15 | Pembayaran Rp150.000 pada debit Rp100.000 menampilkan surplus Rp50.000. Setelah debit baru Rp20.000 tersedia, UI menerapkan Rp10.000 dengan alasan; saldo UI/DB menjadi outstanding Rp10.000 dan kredit pelanggan Rp40.000, dengan audit `credit.apply`. |
| Retry, atomicity, concurrency | Suite integrasi PostgreSQL | Retry key sama mengulang respons; body beda menghasilkan 409. Audit failure tidak meninggalkan payment, ledger, atau idempotency parsial. Dua pembayaran konkuren hanya mengalokasikan total saldo debit Rp100.000. |

- `migrations/postgres/0004_payment_posting.sql` diterapkan dua kali pada skema PostgreSQL sementara oleh suite integrasi bersama migrasi 0001–0003. Fixture tetap sintetis.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --locked`, `cargo test --locked -p toko-api --features postgres-tests --test postgres_foundation`, `cargo build --workspace --locked`, `pnpm api:check`, `pnpm typecheck`, dan `pnpm build` berhasil.
- API-301–303 dan API-302 tercermin pada [OpenAPI](../packages/api-client/openapi.json) dan typed client. UI menampilkan state loading, kosong, error/retry dan konfirmasi sebelum posting. Browser harness berjalan sementara di luar repo menggunakan Chrome 157 di host Windows; runner E2E otomatis permanen belum ditambahkan.

## Bukti exit gate PH-05

| AC | Metode | Hasil |
|---|---|---|
| `AC-009` | Unit test Rust dan suite PostgreSQL terisolasi | Hari ke-15 masuk `days_0_15`, hari ke-16 masuk `days_16_30`; batas 30/31, 45/46, 60/61 serta tanggal tak diketahui/masa depan juga diperiksa. |
| `AC-010` | Suite PostgreSQL melalui API dengan fixture sintetis | Jumlah tujuh bucket sama dengan `total_open_idr`; rincian per pelanggan cocok dengan nilai pada setiap bucket. Tanggal masa depan untuk `as_of` ditolak. |

- API-501 dan API-502 tersedia di OpenAPI serta typed client. Laporan mengambil piutang terbuka dari debit ledger dikurangi alokasi aktif pembayaran posted; saldo kredit belum teralokasi tidak di-aging. Penghitungan memakai tanggal usaha dan nominal integer IDR.
- UI laporan mencakup pemilihan tanggal acuan, ringkasan total, bucket yang membuka rincian per pelanggan, pagination, loading, empty, error/retry. Build/typecheck diverifikasi; browser belum dijalankan khusus untuk PH-05.
- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --locked`, suite `postgres_foundation`, `pnpm api:check`, `pnpm typecheck`, dan `pnpm build` berhasil. Suite PostgreSQL memakai skema sementara terisolasi di cluster dev; fixture hanya sintetis.

## Bukti exit gate PH-06/PH-07

| AC | Metode | Hasil |
|---|---|---|
| `AC-011` | `postgres_foundation`, upload workbook XLSX sintetis dan API staging | Owner melihat dua baris beserta raw/provenance; formula terdeteksi. Operator dapat upload, tetapi review Owner-only ditolak untuk Operator. |
| `AC-012` | Suite PostgreSQL pada endpoint upload dan commit | Upload dengan SHA-256 sama mengembalikan batch yang sama; replay commit dengan key/body sama mengembalikan respons sama; ledger tetap berisi dua event, tidak ganda. |
| `AC-013` | Ringkasan batch dan commit PostgreSQL | Preview melaporkan debit 100.000 dan kredit 120.000; commit menghasilkan dua event dengan net −20.000, alokasi FIFO legacy 100.000; sisa kredit legacy 20.000 lalu 10.000 diterapkan manual dengan audit. Formula saldo tidak digunakan. Satu row menghasilkan satu event. |
| `AC-014` | Suite PostgreSQL permission middleware dan route | Operator ditolak untuk review import, reallocation, reversal, serta permission Owner-only; tidak ada mutasi pada request yang ditolak. |
| `AC-015` | Suite PostgreSQL API reversal, reallocation dan replacement link | Reallocation menaikkan payment version dan membalik alokasi lama; reversal sale/payment menyimpan kontra-event serta audit; replacement entry tertaut ke koreksi asal. |

- [Migrasi 0005](../migrations/postgres/0005_imports_reversals.sql) diterapkan dua kali bersama migrasi 0001–0004 pada schema sementara. Workbook uji dibuat di dalam test dari data sintetis; workbook pelanggan lokal tidak dibaca atau dipakai sebagai fixture.
- Perintah verifikasi: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features --offline -- -D warnings`, `cargo test --workspace --offline`, `TEST_DATABASE_URL=... cargo test --locked -p toko-api --features postgres-tests --test postgres_foundation`, `pnpm api:check`, `pnpm typecheck`, dan `pnpm build` berhasil. Hasil suite integrasi: 1 test berhasil.

## Langkah berikut

Mulai `PH-08` untuk menyiapkan keamanan, operasi, backup/restore, observability, deployment dan rollback berbasis bukti aktual. Jangan gunakan workbook pelanggan nyata sebagai fixture.

## Cara memperbarui

Gunakan `.agents/skills/toko-project-status/SKILL.md`. Setiap pembaruan harus membaca ulang keadaan repo dan menautkan bukti. Gunakan status `Belum mulai`, `Berjalan`, `Terblokir`, `Selesai`, `Ditunda`; fase `Selesai` memerlukan AC/exit gate terverifikasi. Pisahkan dokumen siap dari kode jadi, dan kode ada dari perilaku teruji.
