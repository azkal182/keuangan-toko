> Versi: 1.0 | Disusun: 23 November 2026 | Status: baseline final kebijakan dan rencana; implementasi belum dimulai.

# ADR-005 — PostgreSQL Pusat dan SQLite Desktop

**Status: Diterima [DISEPAKATI]** | Tanggal: 23 November 2026

## Konteks
Satu backend melayani web online dan desktop offline-first.

## Keputusan
Gunakan PostgreSQL untuk backend pusat; SQLite untuk data lokal desktop.

## Konsekuensi
Perlu migrasi berbeda dan sync design lanjutan; PostgreSQL tetap source of truth setelah sinkronisasi.

## Batas keputusan
Rincian kebijakan produk, API dan skema yang dahulu belum diputuskan kini ditetapkan oleh ADR-007 beserta API_SPEC dan ERD. ADR ini menjadi catatan keputusan teknologi dan tetap tunduk pada baseline terbaru tersebut.
