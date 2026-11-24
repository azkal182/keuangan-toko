> Versi: 1.0 | Disusun: 23 November 2026 | Status: baseline final kebijakan dan rencana; implementasi belum dimulai.

# ADR-003 — GPUI sebagai Desktop Native Windows

**Status: Diterima [DISEPAKATI]** | Tanggal: 23 November 2026

## Konteks
Tauri pernah direkomendasikan, lalu pengguna memilih GPUI demi target efisiensi.

## Keputusan
Desktop Windows menggunakan Rust + GPUI, bukan Tauri. Offline-first dengan SQLite.

## Konsekuensi
Ekosistem UI dan native Windows perlu diuji; benchmark resource belum menjadi target angka; implementasi desktop ditunda.

## Batas keputusan
Rincian kebijakan produk, API dan skema yang dahulu belum diputuskan kini ditetapkan oleh ADR-007 beserta API_SPEC dan ERD. ADR ini menjadi catatan keputusan teknologi dan tetap tunduk pada baseline terbaru tersebut.
