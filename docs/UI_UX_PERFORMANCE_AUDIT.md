# Audit UI/UX dan performa frontend

**Tanggal:** 24 November 2026
**Cakupan:** login, ringkasan dan detail pelanggan, penjualan kredit, pembayaran, buku piutang, koreksi, aging, import, navigasi, serta konfigurasi build web.
**Metode:** peninjauan source React/TypeScript, konfigurasi static hosting, pemeriksaan typecheck, dan production build. Browser visual, Lighthouse, serta Core Web Vitals production belum diukur.

## Ringkasan hasil

| Area | Temuan | Status |
|---|---|---|
| Navigasi | Tab pelanggan, aging, dan import sebelumnya mengubah state; pemilihan pelanggan/penjualan mengubah History API secara manual. Deep link, tombol Back/Forward, dan route tidak konsisten. | Diperbaiki dengan React Router. |
| URL yang dapat dibagikan | Detail pelanggan, penjualan, batch import, rincian bucket aging, pencarian dan halaman daftar pelanggan sebelumnya tidak memiliki URL yang memulihkan state. | Diperbaiki; filter aging serta pencarian/cursor daftar berada di query string. |
| Navigasi lama | URL lama `/?pelanggan=...&penjualan=...` tidak cocok dengan route baru. | Redirect kompatibilitas ditambahkan. |
| Pemisahan kode | Halaman laporan, import, penjualan, pembayaran, buku piutang dan koreksi sebelumnya masuk ke graph import awal. | Dipecah dengan lazy loading per fitur. |
| Status data | Halaman memakai status loading, kosong, error/retry dan notice yang relevan; nilai uang diformat dari string IDR menggunakan `BigInt`. | Pola dipertahankan. |
| Tabel lebar | Ringkasan pelanggan, buku piutang, penjualan, dan import memiliki banyak kolom. | Di layar sempit tabel berubah menjadi kartu berlabel; tabel desktop mempertahankan heading sticky, dengan identitas/tanggal tetap terlihat saat scroll. Pemeriksaan visual browser belum tersedia. |

## Route frontend

- `/login`
- `/customers?search=...&page_cursor=...`
- `/customers`
- `/customers/:customerId`
- `/customers/:customerId/sales/:saleId`
- `/reports/aging?as_of=YYYY-MM-DD&bucket=...`
- `/imports`
- `/imports/:batchId`

Nginx sudah memakai `try_files ... /index.html`, sehingga refresh pada path SPA dilayani oleh frontend. Route laporan mempertahankan tanggal dan bucket melalui query string; pemilihan batch import memiliki path tersendiri.

## Catatan performa

Production build setelah pemisahan fitur menghasilkan entry `271.45 kB` (`86.66 kB` gzip), chunk workspace pelanggan `17.48 kB` (`5.10 kB` gzip), Import `28.39 kB` (`7.70 kB` gzip), Sales `7.71 kB` (`2.70 kB` gzip), Payments `7.45 kB` (`2.47 kB` gzip), Ledger `6.80 kB` (`1.86 kB` gzip), dan Reports `4.64 kB` (`1.72 kB` gzip). Angka ini ukuran aset build, bukan hasil pengukuran kecepatan perangkat atau jaringan. Workspace pelanggan dan halaman fitur kini diunduh sesuai kebutuhan; entry utama masih mencakup React, React Router, dan autentikasi.

## Tindak lanjut

1. Ukur Lighthouse/Web Vitals pada deployment dan perangkat sasaran; rekam ukuran/latensi API dengan data operasional sintetis.
2. Periksa layout kartu responsif dan tabel layar lebar di browser desktop/ponsel; pertahankan akses keyboard pada tabel desktop yang dapat di-scroll.
3. Lakukan pemeriksaan manual keyboard, pembaca layar, zoom 200%, kontras, sticky header, dan dialog setelah tersedia preview browser yang dapat diuji.
