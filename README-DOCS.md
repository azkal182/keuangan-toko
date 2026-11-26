> Versi: 1.0 | Disusun: 23 November 2026 | Status: baseline final kebijakan dan rencana; implementasi belum dimulai.

# Panduan Paket Dokumentasi — Keuangan Toko

**Versi paket:** 1.0-final · **Tanggal:** 23 November 2026 · **Tujuan:** baseline keputusan dan implementasi untuk AI coding agent dan reviewer manusia. Status scaffold dan hasil verifikasi aktual tersedia di [PROJECT_STATUS](docs/PROJECT_STATUS.md).

## 1. Inventaris dan urutan baca
| Urutan | Dokumen | Tanggung jawab |
|---|---|---|
| 1 | [AGENTS.md](AGENTS.md) | aturan wajib bagi coding agent, workflow vertical slice, pelaporan |
| 2 | [PRODUCT_SPEC](docs/PRODUCT_SPEC.md) | tujuan, cakupan, istilah, aturan bisnis |
| 3 | [FEATURE_SPECIFICATIONS](docs/FEATURE_SPECIFICATIONS.md) | alur fitur dan acceptance criteria |
| 4 | [OPEN_QUESTIONS](docs/OPEN_QUESTIONS.md) | register keputusan OQ; semua pertanyaan awal ditutup atau dijadwalkan eksekusinya |
| 5 | [ARCHITECTURE](docs/ARCHITECTURE.md) | baseline komponen, workspace dan aliran data |
| 6 | [API_SPEC](docs/API_SPEC.md) | kontrak API v1 yang disepakati dan artefak OpenAPI |
| 7 | [DATA_DICTIONARY](docs/DATA_DICTIONARY.md), [ERD](docs/ERD.md) | kamus data dan baseline relasi PostgreSQL final |
| 8 | [AUTHORIZATION_MATRIX](docs/AUTHORIZATION_MATRIX.md) | role/permission baseline, belum dibuktikan di kode |
| 9 | [SECURITY](docs/SECURITY.md) | kontrol keamanan dan retensi baseline |
| 10 | [QUALITY](docs/QUALITY.md) | target dan acceptance; bukan hasil test |
| 11 | [OPERATIONS](docs/OPERATIONS.md) | baseline deployment/backup/observability; deployment belum dilakukan |
| 12 | [ADR](docs/decisions/) | alasan dan konsekuensi keputusan final |
| 13 | [IMPLEMENTATION_PLAN](docs/IMPLEMENTATION_PLAN.md) | urutan fase web-first, acceptance gate, dan acuan workbook |
| 14 | [PROJECT_STATUS](docs/PROJECT_STATUS.md) | status aktual, fase berjalan, blocker, dan langkah berikut |
| 15 | [UI_UX_PERFORMANCE_AUDIT](docs/UI_UX_PERFORMANCE_AUDIT.md) | temuan review frontend, route, pemisahan bundle, dan tindak lanjut pengukuran |
| 16 | [DESKTOP_IMPLEMENTATION_PLAN](docs/DESKTOP_IMPLEMENTATION_PLAN.md) | rencana PH-09 dengan prototipe data sintetis sebelum integrasi API |
| 17 | [DESKTOP_WEB_FEATURE_MAP](docs/DESKTOP_WEB_FEATURE_MAP.md) | matriks pemetaan fitur web terhadap kesiapan prototipe mock desktop |

## Skill khusus repo

Skill Codex yang dipakai untuk pengembangan proyek disimpan di `.agents/skills/` agar ikut versi repo:

- `toko-project-status` — laporan fase, progress, bukti dan blocker.
- `toko-web-slice` — implementasi vertical slice React/Axum/PostgreSQL.
- `toko-frontend` — UI React, Tailwind CSS dan shadcn/ui.
- `toko-financial-domain` — ledger, uang, FIFO, aging dan koreksi.
- `toko-excel-migration` — profiling aman, staging dan rekonsiliasi workbook.
- `toko-production-readiness` — security, operasi dan kesiapan rilis.

Pilih skill sesuai task; `AGENTS.md` tetap instruksi proyek utama.

## 2. Hirarki kebenaran / konflik
1. **Keputusan eksplisit terbaru pengguna di sesi** menang atas rekomendasi/opsi sebelumnya; ADR merekam keputusan final tersebut.
2. `AGENTS.md` mengatur **cara kerja agent**; `PRODUCT_SPEC` mengatur ruang lingkup bisnis; `FEATURE_SPECIFICATIONS` mengatur behavior dan AC.
3. `ADR-007` menutup keputusan kebijakan awal. `API_SPEC`, `DATA_DICTIONARY`, dan `ERD` adalah baseline final untuk implementasi; perubahan berikutnya memerlukan keputusan/ADR baru.
4. `OPEN_QUESTIONS` adalah register keputusan yang sudah ditutup, termasuk item yang detail eksekusinya sengaja dijadwalkan ke PH-08/PH-09. `AUTHORIZATION_MATRIX` adalah policy baseline; kode tetap wajib menegakkannya.
5. Konflik antar dokumen dicatat sebagai isu/keputusan sebelum implementasi; jangan memilih diam-diam berdasarkan urutan nama file.

## 3. Status ketentuan
- `[DISEPAKATI]`: benar-benar dipilih/disetujui eksplisit dalam sesi atau fakta perilaku workbook yang telah dibahas.
- `[USULAN]`: label dari rancangan terdahulu; ADR-007 mengungguli label ini.
- `[ASUMSI]`: label asumsi terdahulu yang perlu diverifikasi sebelum digunakan.
- `[PERLU KEPUTUSAN]`: label blocker pada dokumen terdahulu; OQ awal kini ditutup oleh ADR-007.

## 4. Audit sumber dan batas verifikasi
Sumber: percakapan sesi ini, workbook referensi lokal `docs/files/CATATAN KEUANGAN TOKO.xlsx`, dan berkas awal `spesifikasi-bisnis-keuangan-toko-v0.1.md`. Workbook kini ada di workspace; jangan masukkan file pelanggan ke commit, log, fixture, atau dokumen. Pemeriksaan paket XLSX read-only pada 23 November 2026 mengonfirmasi 104 worksheet, 104.263 elemen formula, dan 31 worksheet dengan lebih dari 60 baris XML. Ini bukan validasi isi/transaksi atau posisi header; verifikasi semantik tetap bagian fase importer. Hasil pemeriksaan scaffold dicatat di [PROJECT_STATUS](docs/PROJECT_STATUS.md). ERD baseline tersedia di `docs/ERD.md`.

## 5. Matriks kelengkapan dokumentasi
| Area | Status | Penjelasan |
|---|---|---|
| Platform/urutan pengembangan | **Final baseline** | Rust Axum, React TS, GPUI setelah web stabil; monorepo/workspace ditetapkan |
| Produk dan transaksi | **Final baseline** | scope MVP, saldo, FIFO, surplus, rounding, tanggal, lifecycle diputuskan |
| Flow pelanggan/sale/payment | **Final baseline** | FR/AC dan role final; implementasi belum ada |
| Migrasi Excel | **Final baseline** | staging, quarantine, partial commit dan approval final; profiling semantik menunggu PH-06 |
| API | **Final baseline** | REST v1, routes/envelopes/payloads ditetapkan; artefak OpenAPI awal dan status endpoint tercatat |
| Data dictionary / ERD | **Final baseline** | ERD PostgreSQL dan kamus data tersedia bersama migrasi foundation |
| Authorization | **Final baseline** | owner/operator/viewer; guard foundation sudah diuji |
| Security | **Final baseline** | auth/session/retensi/controls ditetapkan; belum diuji |
| Quality | **Final baseline** | target NFR/AC terukur; belum diverifikasi aplikasi |
| Operations | **Final baseline** | strategi provider-neutral/backup/observability; infra/deployment belum dipasang |
| Desktop sync | **Final baseline, PH-09** | one offline writer, 72h, SQLCipher; implementasi ditunda sampai web stabil |

## 6. Cara menggunakan oleh coding agent
Mulai dari fase aktif `PH-*`; periksa `FR-*`, `BR-*`, `AC-*`, ADR-007, API dan ERD; buat UI/API/persistensi bersama per vertical slice; jalankan pemeriksaan nyata dan perbarui `PROJECT_STATUS`. Scope baru atau konflik dengan baseline dicatat sebagai OQ/ADR baru sebelum diimplementasikan.

## 7. Konflik yang sudah terselesaikan
- Backend Go → **Rust + Axum** (`ADR-001`).
- Desktop Tauri → **Rust + GPUI** (`ADR-003`).
- Backend dulu → **React + Rust per vertical slice** (`ADR-004`).
- Mulai web atau desktop bersamaan → **web stabil lebih dulu, GPUI sesudahnya** (`ADR-004`).
- Keputusan MVP, uang, akses, API, schema, security, operations dan target quality → **ADR-007**.

**Catatan:** rencana fase resmi proyek berada di [IMPLEMENTATION_PLAN](docs/IMPLEMENTATION_PLAN.md); ERD baseline ada di [ERD](docs/ERD.md). Temuan konflik atau kebutuhan baru dicatat di `OPEN_QUESTIONS` untuk ADR/keputusan berikutnya.
