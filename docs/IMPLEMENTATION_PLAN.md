# Rencana Implementasi — Keuangan Toko

**Status:** baseline eksekusi fase; status aktual hanya dicatat di [PROJECT_STATUS.md](PROJECT_STATUS.md).
**Repo kerja yang disepakati:** `D:\azka\keuangan-toko`. Skill khusus proyek disimpan di `.agents/skills/`.
**Sumber:** `AGENTS.md`, spesifikasi produk/fitur, `OPEN_QUESTIONS.md`, arsitektur, ADR, dan workbook lokal.

## Tujuan dan keputusan teknis

Bangun aplikasi pencatatan piutang web sebagai prioritas menggunakan Rust/Axum, React/TypeScript, PostgreSQL, Tailwind CSS dan shadcn/ui. Kembangkan UI dan API bersama per vertical slice. Saldo dan alokasi final berasal dari server. Workbook historis masuk staging dan direkonsiliasi sebelum posting. Desktop Rust/GPUI + SQLite dikerjakan setelah web stabil.

Platform, web-first, vertical slice, styling, root repo, target monorepo/workspace, ERD/schema, kontrak API v1, auth/role, dan aturan bisnis MVP telah diputuskan (GOV-001/002; ADR-001–007). Toolchain versi stabil dicatat saat scaffold; vendor production dan detail perangkat uji adalah aktivitas terjadwal di PH-08/PH-09, bukan blocker kebijakan.

## Fase proyek

Status fase tidak diedit di tabel rencana; gunakan satu register pada `PROJECT_STATUS.md` dengan status `Belum mulai`, `Berjalan`, `Terblokir`, `Selesai`, atau `Ditunda`. Fase hanya boleh `Selesai` bila bukti exit gate dicatat.

| ID | Fase dan hasil | Acuan utama | Exit gate / bukti selesai |
|---|---|---|---|
| **PH-00** | Repo & tata kerja proyek. Root, skills, fase, workspace target, toolchain dan command aktual. | GOV-001–005; ADR-007 | Cargo/pnpm manifests dibuat sesuai arsitektur; toolchain dikunci; command yang tersedia ditemukan dan baseline checks dijalankan; tidak ada code/secret workbook ikut commit. |
| **PH-01** | Fondasi slice pelanggan. Hubungkan API/ERD final ke model/use case/client contract; siapkan baseline fixtures non-PII. | `FR-001`, `FR-004`; ADR-007; `ERD.md`, `API_SPEC.md` | Migration dan typed contract sesuai dokumen; auth/permission behavior diuji; fixture aman; command verifikasi tersedia. |
| **PH-02** | Vertical slice pelanggan & buku piutang baca. Pencarian/daftar, detail, ledger dan saldo dari API/persistensi. | `FR-001`, `FR-004`; `AC-001`, `AC-002`, `AC-007`, `AC-008` | AC terkait diverifikasi UI→API→PostgreSQL; loading/empty/error tersedia; server menolak input tak valid; tidak ada saldo mock dianggap nyata. |
| **PH-03** | Vertical slice penjualan kredit. Input, review, posting atomik, histori dan saldo bertambah. | `FR-002`; `AC-003`, `AC-004`; ADR-007 §§01/05/08/11 | Implementasi mengikuti policy final; AC posting satu kali dan tanpa mutasi parsial diverifikasi; docs/API/schema sesuai kode. |
| **PH-04** | Vertical slice pembayaran dan alokasi. Penerimaan, FIFO, surplus/saldo kredit, status dan histori. | `FR-003`; `AC-005`, `AC-006`; ADR-007 §§02–05/08/11 | Metode/FIFO/surplus final; skenario numerik diverifikasi di domain/API; atomicity/retry dicakup. |
| **PH-05** | Aging dan ringkasan piutang. Buckets, tanggal acuan, exception dan drill-down sesuai kebijakan. | `FR-005`; `AC-009`, `AC-010`; ADR-007 §§08/12/21 | Bucket/timezone final; batas bucket dan total ke outstanding diverifikasi. |
| **PH-06** | Import workbook historis. Profil, mapping, staging, review, rekonsiliasi, approval dan commit lineage. | `FR-006`; `AC-011`–`AC-013`; ADR-007 §§06/07/13/18 | Baris bermasalah dapat ditelusuri; formula bukan saldo otoritatif; dedupe/partial rules diimplementasi; batch disetujui Owner. |
| **PH-07** | Koreksi, audit dan akses lengkap. Reversal/koreksi terlacak dan otorisasi diuji. | `FR-007`; `AC-014`, `AC-015`; ADR-007 §§09/11/18 | Mutasi sesuai role; histori tertaut; audit dan retensi diuji. |
| **PH-08** | Kesiapan production dan operasi web. Keamanan, deployment, backup/restore, observability, runbook dan rollback. | `QUALITY`, `SECURITY`, `OPERATIONS`; ADR-007 §§18/19/21 | Provider/owner/secret/pipeline dipasang; backup/restore drill, target operasional dan rollback memiliki bukti. Readiness tidak sama dengan deploy. |
| **PH-09** | Desktop offline-first sebagai fase lanjutan. GPUI, SQLite, outbox, sync, konflik, pemulihan lokal. | `FR-008`; `AC-016`, `AC-017`; ADR-007 §§14/15/20 | PH-08/web stabil; default one-writer/72h final; restart/replay/conflict diuji dan hardware benchmark dicatat. |

## Aturan pelaksanaan dan monitoring

- Kerjakan satu fase aktif pada satu waktu; fase berikut boleh direncanakan, tetapi tidak dianggap berjalan tanpa perubahan dan bukti.
- Setiap task mencantumkan `PH-*`, `FR-*`, `AC-*`, dependency `OQ-*`, file berubah, dan hasil pemeriksaan nyata.
- Di awal task, baca `AGENTS.md`, `PROJECT_STATUS.md`, fase aktif, skill relevan dalam `.agents/skills/`, dan dokumen sumber.
- Untuk slice UI/API, siapkan payload contoh serta loading/empty/success/validation-error/network-error. Mock diberi label dan tidak membuktikan integrasi.
- Jika ditemukan gap atau konflik baru pada uang, akses, kontrak publik atau data historis, catat OQ/ADR baru dan jangan menimpa baseline final diam-diam.
- Perbarui `PROJECT_STATUS.md` pada perubahan fase atau bukti kemajuan: status, bukti (path/AC/command), blocker, dan langkah berikut. Jangan tandai `Selesai` hanya dari dokumentasi atau rencana.
- Cari perintah dari manifest/workflow aktual. Bila belum ada, tulis `belum tersedia`; jangan karang perintah.

## Workbook dan privasi

Workbook `docs/files/CATATAN KEUANGAN TOKO.xlsx` adalah acuan historis lokal dan tidak boleh ikut commit, log, fixture, atau dokumentasi berisi data pelanggan. Pemeriksaan struktur XLSX pada 23 November 2026 mengonfirmasi 104 worksheet dan 104.263 elemen formula; 31 worksheet memiliki lebih dari 60 baris XML. Ini bukan verifikasi transaksi/header/identitas. Profil semantik dilakukan pada PH-06 dengan agregat aman; cached formula tidak menjadi saldo definitif.

## Kondisi awal

Baseline kebijakan dan ERD tersedia sebelum scaffold. Bukti exit gate, status fase, dan perintah yang sudah diverifikasi dicatat di [PROJECT_STATUS](PROJECT_STATUS.md).
