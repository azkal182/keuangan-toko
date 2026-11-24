# Quality Gates — Final MVP

**Status:** acceptance dan target baseline disepakati melalui ADR-007. Ini adalah target verifikasi, bukan klaim aplikasi telah dibangun atau lulus.

## Definition of Done per slice

- Satu user flow berjalan dari React ke Axum, PostgreSQL, kembali ke UI; mock diberi label dan tidak menjadi bukti integrasi.
- UI menampilkan loading, empty, success, validation error, network/server error dan forbidden state yang sesuai.
- Server/domain validation, permission, audit, atomicity dan idempotency sesuai keputusan berlaku.
- Nilai uang dan saldo UI konsisten dengan response server; tidak dihitung sebagai otoritas di browser.
- API, data dictionary/ERD, feature spec dan status proyek sesuai implementasi.
- AC slice diverifikasi dengan unit/integration/UI test yang relevan; hasil nyata dilaporkan.
- Migrations diperiksa forward dan rollback/restore plan diuji pada database nonproduction.

## Quality targets

| ID | Target final |
|---|---|
| NFR-001 | Bahasa UI Bahasa Indonesia; desktop web responsive, minimum viewport 1024×768. |
| NFR-002 | WCAG 2.2 AA sebagai target: keyboard flow, focus terlihat, semantic labels, kontras, screen-reader smoke check. |
| NFR-003 | Browser Chrome dan Edge dua versi stable major terbaru; target Windows 11 x64 untuk desktop fase berikut. |
| NFR-004 | P95 API ≤500 ms untuk request normal pada 50 concurrent users dan dataset benchmark 100.000 ledger entries; import dikecualikan dari latency request normal. |
| NFR-005 | Interaksi utama daftar/pencarian/detail terasa selesai ≤2 s pada benchmark environment yang dicatat. |
| NFR-006 | Workbook upload maksimum 25 MiB dan 25.000 baris sumber; di atas limit mendapat pesan aman. |
| NFR-007 | Tidak ada SLA availability sebelum production host, monitoring dan periode pengukuran ditetapkan pada PH-08. |

## Acceptance test suite

| ID | Verifikasi wajib | Fase |
|---|---|---|
| QA-001 | Debit/kredit, saldo dan reversal; no negative receivable from overpayment | PH-02–07 |
| QA-002 | FIFO, backdated debit tidak rewrite allocation lama; reallocation manual ter-audit | PH-04 |
| QA-003 | Atomic posting, concurrent allocation, idempotent retry | PH-03/04/07 |
| QA-004 | Import fixtures anonymized: missing/invalid/future dates, duplicate, mixed debit-credit, formula error, quantity-price mismatch | PH-06 |
| QA-005 | Reconciliation per customer and source sheet/row lineage; explicit variance approval | PH-06 |
| QA-006 | Auth/RBAC/IDOR, CSRF, request tampering, rate limit, upload size/type | PH-02–08 |
| QA-007 | UI/E2E customer, sale, payment, aging and import critical flows | PH-02–06 |
| QA-008 | Desktop restart/outbox/replay/device revoke/conflict | PH-09 |

## Numerical acceptance examples

- Debit Rp100.000, payment Rp40.000 → outstanding Rp60.000.
- Debit Rp100.000 + Rp200.000, payment Rp150.000 → debit pertama lunas, debit kedua outstanding Rp150.000 under FIFO.
- Outstanding Rp100.000, payment Rp140.000 → outstanding Rp0, customer credit Rp40.000; no auto-apply to future sales.
- Age 15 vs 16 days → buckets 0–15 vs 16–30.
- Replayed identical idempotency key/body → same result, one financial effect; same key/different body → 409.

Perintah baseline yang tersedia dicatat di [OPERATIONS](OPERATIONS.md#perintah-pengembangan-lokal). Hasil verifikasi aktual dicatat di [PROJECT_STATUS](PROJECT_STATUS.md).
