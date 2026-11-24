> Versi: 1.0 | Disusun: 23 November 2026 | Status: baseline final kebijakan dan rencana; implementasi belum dimulai.

# ADR-004 — Web-first, Vertical Slices Paralel

**Status: Diterima [DISEPAKATI]** | Tanggal: 23 November 2026

## Konteks
Usulan sebelumnya menempatkan backend/domain selesai lebih dahulu; pengguna menolak jeda itu karena ingin memantau flow frontend dan kebutuhan API.

## Keputusan
Bangun React dan Rust secara paralel per fitur, web prioritas; setelah backend+web stabil baru GPUI.

## Konsekuensi
Butuh draft kontrak API, mock dengan label, pengujian integrasi per slice dan sinkronisasi dokumentasi.

## Batas keputusan
Rincian kebijakan produk, API dan skema yang dahulu belum diputuskan kini ditetapkan oleh ADR-007 beserta API_SPEC dan ERD. ADR ini menjadi catatan keputusan teknologi dan tetap tunduk pada baseline terbaru tersebut.
