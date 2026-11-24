> Versi: 1.0 | Disusun: 23 November 2026 | Status: baseline final kebijakan dan rencana; implementasi belum dimulai.

# ADR-001 — Rust + Axum untuk Backend

**Status: Diterima [DISEPAKATI]** | Tanggal: 23 November 2026

## Konteks
Awalnya backend Go dibahas; kemudian pengguna secara eksplisit memilih backend Rust.

## Keputusan
Gunakan Rust + Axum untuk API, PostgreSQL pusat; tidak membangun backend Go.

## Konsekuensi
Rust dapat membagi sebagian domain dengan GPUI kelak; toolchain/dependency versions dikunci saat bootstrap, sementara API v1 ditetapkan di ADR-007 dan API_SPEC.

## Batas keputusan
Rincian kebijakan produk, API dan skema yang dahulu belum diputuskan kini ditetapkan oleh ADR-007 beserta API_SPEC dan ERD. ADR ini menjadi catatan keputusan teknologi dan tetap tunduk pada baseline terbaru tersebut.
