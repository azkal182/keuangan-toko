> Versi: 1.0 | Disusun: 23 November 2026 | Status: baseline final kebijakan dan rencana; implementasi belum dimulai.

# Instruksi AI Coding Agent

## 1. Misi dan prioritas
**GOV-001 [DISEPAKATI]** Bangun aplikasi pencatatan keuangan/piutang toko menggantikan workbook pelanggan. Target utama sekarang **website**: backend **Rust/Axum**, frontend **React + TypeScript**; desktop **Rust/GPUI offline-first** hanya setelah web stabil. Database pusat PostgreSQL; database lokal desktop SQLite. Jangan membuat desktop pada tugas fase web tanpa instruksi eksplisit.

**GOV-002 [DISEPAKATI]** Kerjakan per **vertical slice**, dengan UI dan API berkembang bersamaan; hindari strategi seluruh backend selesai sebelum UI. Jadikan UI sebagai validasi kebutuhan data. Gunakan mock data sementara secara eksplisit dan gantikan saat API siap.

**GOV-003 [DISEPAKATI]** Excel merupakan sumber historis berisiko; jangan percaya kolom hasil formula sebagai sumber saldo definitif. Semua impor melewati validasi dan rekonsiliasi yang status aturannya ditandai di dokumentasi.

## 2. Urutan membaca
1. [README-DOCS.md](README-DOCS.md) — indeks dan hirarki dokumen.
2. [PRODUCT_SPEC](docs/PRODUCT_SPEC.md), [FEATURE_SPECIFICATIONS](docs/FEATURE_SPECIFICATIONS.md) — definisi bisnis, cakupan, AC.
3. [OPEN_QUESTIONS](docs/OPEN_QUESTIONS.md) — register keputusan tertutup dan tindak lanjut fase.
4. [ARCHITECTURE](docs/ARCHITECTURE.md), [API_SPEC](docs/API_SPEC.md), [DATA_DICTIONARY](docs/DATA_DICTIONARY.md) — batas modul, kontrak baseline, atribut.
5. [AUTHORIZATION_MATRIX](docs/AUTHORIZATION_MATRIX.md), [SECURITY](docs/SECURITY.md), [QUALITY](docs/QUALITY.md), [OPERATIONS](docs/OPERATIONS.md).
6. [ADR](docs/decisions/) — ADR-007 adalah baseline kebijakan final; rencana fase ada di [docs/IMPLEMENTATION_PLAN.md](docs/IMPLEMENTATION_PLAN.md), ERD di [docs/ERD.md](docs/ERD.md).

**GOV-004 [DISEPAKATI]** Arti label legacy: `[DISEPAKATI]` keputusan eksplisit sesi; `[USULAN]` rancangan yang memerlukan persetujuan; `[ASUMSI]` interpretasi kerja yang belum dikonfirmasi; `[PERLU KEPUTUSAN]` blocker kebijakan/kontrak. ADR-007 menutup seluruh OQ awal; ADR-007 dan dokumen final yang ditautkannya mengungguli label lama. Detail eksekusi yang dijadwalkan fase bukan blocker kebijakan.

## 3. Struktur repo dan pembatasan
**ARCH-010 [DISEPAKATI melalui ADR-007]** Monorepo target: `apps/api/`, `apps/web/`, `apps/desktop/` (placeholder sampai fase GPUI), `crates/domain/`, `crates/application/`, `crates/money/`, `crates/contracts/`, `packages/api-client/`, `docs/`, `migrations/postgres/`, `infra/`. Root repo kanonis `D:\azka\keuangan-toko`; Cargo workspace dan pnpm workspace. Periksa isi repo sebelum membuat direktori; target layout bukan bukti implementasi sudah ada.

Skill project berada di `.agents/skills/`; lihat skill yang relevan sebelum task pengembangan dan perbarui `docs/PROJECT_STATUS.md` saat ada bukti kemajuan atau perubahan blocker.

**GOV-005 [DISEPAKATI melalui ADR-007]** Domain dan application service Rust tidak bergantung pada HTTP atau DB; adapter Axum, PostgreSQL, dan kelak SQLite dipisahkan. Frontend mengakses backend melalui typed API client, bukan query DB langsung. Jangan mengimpor crate Rust ke React.

## 4. Aturan implementasi
- **GOV-006 [DISEPAKATI]** Ikuti konvensi bahasa: Rust `snake_case` modul/fungsi dan `PascalCase` tipe; React `PascalCase` komponen, `camelCase` fungsi/variabel; nama endpoint/tabel hanya setelah kontrak/schema disetujui. Istilah stabil: *pelanggan*, *penjualan kredit*, *pembayaran*, *piutang*, *buku piutang*, *alokasi FIFO*, *saldo kredit pelanggan*, *import staging*.
- **GOV-007 [DISEPAKATI]** Validasi di batas input **dan** domain; jangan mengandalkan validasi frontend. Hindari float untuk nilai uang. Ikuti representasi dan rounding ADR-007.
- **GOV-008 [DISEPAKATI]** Error terstruktur: kode stabil, pesan aman untuk pengguna, detail korelasi untuk log; bedakan kesalahan input, otorisasi, konflik, dan kegagalan infrastruktur. Jangan membocorkan SQL, secret, dan stack trace pada respons publik.
- **GOV-009 [DISEPAKATI]** Log terstruktur dengan request/correlation ID; redaksi data pelanggan dan referensi sensitif; audit mutasi uang dipisah dari application log. Jangan log credential maupun isi mentah seluruh Excel.
- **GOV-010 [DISEPAKATI]** Konfigurasi dari environment dan pengelola secret, bukan hardcode. Commit `.env.example` **tanpa** credential; jangan commit `.env` asli, dump produksi atau berkas pelanggan.
- **GOV-011 [DISEPAKATI]** Terapkan parameterized SQL, least privilege, CORS terbatas, migrasi versi, transaksi atomik untuk posting, dan idempotency ketika endpoint sensitif diulang.
- **GOV-012 [DISEPAKATI]** Jangan mengubah keputusan bisnis, status `[DISEPAKATI]`, kontrak API publik, atau format historis secara sepihak. Buat catatan dampak, tandai permintaan keputusan, tunggu persetujuan sebelum perubahan breaking.

## 5. Protokol tiap tugas agent
1. Sebutkan feature ID, dokumen sumber, status keputusan dan dependency yang relevan. OQ awal telah diputuskan; kebijakan baru dicatat sebagai OQ baru dan tidak mengubah keputusan final tanpa instruksi pengguna.
2. Uraikan perubahan terkecil yang menyelesaikan satu user flow; jika UI/API diperlukan bersamaan, implementasikan keduanya dalam satu vertical slice atau dua PR yang saling terlacak.
3. Tentukan AC yang ingin diluluskan, contoh payload dan state UI (loading, empty, error, success).
4. Tinjau kompatibilitas frontend, server, migrasi, kontrak, validation dan permission; jangan ubah modul di luar scope tanpa alasan.
5. Jalankan verifikasi yang **tersedia di repo**; bila tidak tersedia, laporkan `belum tersedia`—jangan mengarang hasil.
6. Perbarui dokumen yang relevan dalam PR yang sama, termasuk `OPEN_QUESTIONS` bila menemukan gap. Tandai mock/placeholder.

### Perintah yang diketahui
**OPS-010 [FAKTA REPO, periksa ulang saat bootstrap]** Perintah `lint`, `typecheck`, `test`, `build`, `migrate up/down`, `dev`, dan CI **belum diketahui** (tidak ada konfigurasi repo aplikasi yang diverifikasi). Jangan membuat daftar command seolah sudah tersedia. Cari `Cargo.toml`, `package.json`, konfigurasi migration dan workflow aktual sebelum menjalankannya.

## 6. Format laporan penyelesaian agent
```text
Feature/Task ID:
Status: implemented | partial | blocked
Scope dan file berubah:
Keputusan acuan: ADR-007 dan spesifikasi final; catat OQ baru bila ada kebijakan baru
Perilaku UI dan API yang dapat ditinjau:
Perubahan kontrak/migrasi: none | detail
AC terverifikasi: ID — metode — hasil / belum dijalankan
Pemeriksaan: lint / typecheck / test / build / migration (command nyata + hasil)
Risiko, data yang belum aman, dan rollback:
Pertanyaan terbuka dan asumsi baru:
Dokumen yang diperbarui:
```
**GOV-013 [DISEPAKATI]** Hindari kata “lulus” untuk pengujian yang belum dilakukan. Setiap perubahan kontrak, aturan bisnis, dan data perlu perubahan dokumen/ADR yang relevan sebelum merge.
