# Operations — Final Web Baseline

**Status:** kebijakan operasi disepakati melalui ADR-007; provider konkret dan deployment belum dilakukan.

## Environment dan konfigurasi

Pisahkan `development`, `test`, `staging`, dan `production`. Gunakan nama konfigurasi baseline `APP_ENV`, `DATABASE_URL`, `PUBLIC_APP_ORIGIN`, `RUST_LOG`, `UPLOAD_DIR`, `SESSION_IDLE_TIMEOUT`, dan `SESSION_ABSOLUTE_TIMEOUT`. Values dan credentials berasal dari environment/secret manager; `.env.example` hanya berisi placeholder.

Tidak ada hardcoded production secret, credential, URL, atau data pelanggan dalam repo.

## Build, runtime dan deployment

- Monorepo memakai Cargo workspace dan pnpm workspace; pipeline CI default GitHub Actions.
- API Axum dan web React dibuat sebagai OCI containers pada Linux. Target hosting region Indonesia bila tersedia; PostgreSQL terkelola pada region yang sama diutamakan.
- Vendor deployment dipilih pada PH-08 setelah pembandingan biaya, data residency, backup dan restore; desain aplikasi tidak bergantung pada API proprietary provider.
- Deployment memiliki staging dan production terpisah, migration review, health/readiness checks, canary atau staged rollout untuk perubahan berisiko dan rollback artefak versi sebelumnya.
## Perintah pengembangan lokal

Toolchain repo dikunci ke Rust 1.100.0, Node.js 26.13.1 (LTS), dan pnpm 12.21.0. Jalankan perintah dari root workspace.

| Tujuan | Perintah |
|---|---|
| Pasang dependensi web sesuai lockfile | `pnpm install --frozen-lockfile` |
| Web development server | `pnpm dev` |
| Jalankan API | `cargo run --locked -p toko-api` |
| Format Rust | `cargo fmt --all -- --check` |
| Lint Rust | `cargo clippy --workspace --all-targets --locked -- -D warnings` |
| Test Rust | `cargo test --workspace --locked` |
| Build Rust | `cargo build --workspace --locked` |
| Typecheck web | `pnpm typecheck` |
| Build web | `pnpm build` |
| Terapkan migrasi PostgreSQL | `cargo run --locked -p toko-api --bin migrate-postgres` |
| Uji integrasi PostgreSQL | `cargo test --locked -p toko-api --features postgres-tests --test postgres_foundation` |
| Regenerasi OpenAPI dan typed client | `pnpm api:generate` |
| Periksa kesesuaian artefak OpenAPI | `pnpm api:check` |

Setel `DATABASE_URL` di environment proses API dan migrator sesuai perannya. API memakai `API_BIND_ADDR=127.0.0.1:3000` secara default dan tidak menerapkan migrasi saat startup. API dan migrator tidak memuat file `.env` secara otomatis. Contoh variabel ada di [`.env.example`](../.env.example).

Untuk pengembangan lokal, terapkan migrasi dan buat akun Owner pertama pada database kosong sebelum menjalankan API dan web. Bootstrap menerima kata sandi via stdin dan menolak bootstrap ulang saat akun sudah ada. Contoh Bash dari root repo dengan `DATABASE_URL` yang sudah diekspor:

```bash
cargo run --locked -p toko-api --bin migrate-postgres
read -rsp 'Kata sandi Owner: ' toko_owner_password
printf '\n'
printf '%s\n' "$toko_owner_password" | cargo run --locked -p toko-api --bin bootstrap-owner -- owner
unset toko_owner_password
```

Jalankan `cargo run --locked -p toko-api` dan `pnpm dev` di terminal terpisah. Buka URL HTTPS yang dicetak Vite. Vite memakai sertifikat lokal pengembangan dan meneruskan `/api` ke `http://127.0.0.1:3000`. Untuk sertifikat lokal yang sudah dipercaya browser, setel `DEV_TLS_CERT` dan `DEV_TLS_KEY` ke path absolut. Setel `DEV_API_ORIGIN` jika bind API memakai port lain. Kredensial lokal tidak boleh memakai kredensial production.

Untuk membuka dev server dari host Windows saat repo berjalan di WSL, pertahankan config proyek agar plugin React/Tailwind dan proxy API tetap aktif. Dari direktori `apps/web`, gunakan:

```bash
DEV_API_ORIGIN=http://127.0.0.1:3000 pnpm exec vite --config ./vite.config.ts --host 0.0.0.0 --port 5180 --strictPort
```

Buka URL HTTPS yang dicetak Vite, biasanya `https://localhost:5180`. `--host 0.0.0.0` mengikat semua interface WSL; gunakan hanya untuk dev server. Jangan menunjuk `--config` ke config kosong: config tersebut menggantikan `apps/web/vite.config.ts`, sehingga proxy `/api` hilang dan Vite memberi fallback HTML untuk request API. Pengujian PH-04 pada port 5180 menghasilkan `200 text/html` untuk `/api/v1/auth/me`; memakai config proyek menghasilkan `401 application/json` (respons API tanpa sesi), seperti yang diharapkan.

Tes integrasi membaca `TEST_DATABASE_URL` dari environment proses. Gunakan database PostgreSQL khusus uji dengan role yang boleh membuat skema. Tes membuat skema sementara, menerapkan migrasi, mengisi fixture sintetis, lalu menghapus skema uji. Suite mencakup posting penjualan/pembayaran, FIFO dan surplus, aplikasi kredit manual, idempotency, retry konkuren, serta kegagalan audit yang dipaksakan. PH-04 juga diverifikasi lewat Chrome host pada lingkungan dev terisolasi; runner browser/UI otomatis di repo, CI, dan deploy belum tersedia. Migrator forward-only. Pemulihan skema yang sudah dipakai mengikuti migrasi kompensasi atau restore teruji.

## Database, backup dan pemulihan

- PostgreSQL production dengan akun aplikasi least-privilege; migrasi versioned memakai SQLx.
- Backup otomatis harian terenkripsi; retensi 35 salinan harian dan 12 bulanan. Encryption key disimpan terpisah dan akses backup dibatasi.
- Target RPO 24 jam dan RTO 8 jam untuk MVP. Owner operasional menjalankan restore drill minimal per kuartal dan sebelum perubahan infrastruktur besar.
- Migration destructive dilarang tanpa backup terbaru tervalidasi, rehearsal restore, dan rollback/recovery steps.
- Ledger dan audit retensi minimum 10 tahun; workbook raw dan logs mengikuti [SECURITY](SECURITY.md).

## Observability dan incident response

- Structured application logs, request/correlation ID, metrics latency/error rate/DB saturation, health checks dan alert untuk availability, backup failure, migration failure dan error spike.
- Application logs disimpan 90 hari, redacted dan tanpa customer content/auth secrets. Financial audit trail disimpan terpisah 10 tahun.
- Owner bertanggung jawab atas incident triage, pembatasan akses, restore decision dan notifikasi yang relevan. Kegagalan data/privacy dicatat dan ditangani sesuai kebijakan/aturan yang berlaku.

## File import

File maksimal 25 MiB; staging terisolasi; file tidak disajikan publik atau dieksekusi. Raw workbook dihapus 30 hari setelah batch final; raw values batch terbengkalai 90 hari dari aktivitas terakhir. Sisa variance/lineage disimpan sebagai record tanpa mempertahankan raw source yang tidak diperlukan.

## Desktop PH-09

Windows 11 x64; updater dan backup SQLite diputuskan saat fase desktop. SQLite encrypted SQLCipher, key Windows Credential Manager/DPAPI. Printer tidak termasuk scope.

## Non-goals operasional saat ini

Belum ada sistem production, provider, CI run, backup, atau restore drill. Jangan melaporkan readiness/deployment sebagai selesai tanpa bukti nyata.
