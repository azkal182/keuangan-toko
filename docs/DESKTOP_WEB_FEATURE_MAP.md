# Peta fitur web ke prototipe desktop

Ditinjau 26 November 2026 dari route, komponen React dan spesifikasi; daftar ini membedakan fitur web dari kemampuan desktop saat ini. Semua implementasi desktop tahap ini memakai data sintetis dalam memori, tanpa autentikasi, API, penyimpanan permanen, atau transaksi riil.

| Area web / sumber | Alur web yang perlu dipetakan | Status desktop |
|---|---|---|
| Sesi dan akun — `app.tsx`, `users.tsx` | Login/logout, sesi, wajib ganti kata sandi, ganti kata sandi sendiri | Simulasi lokal tersedia; bukan autentikasi server, belum seluruhnya diuji melalui UI |
| Pelanggan — `customers.tsx` | Pencarian, daftar saldo dan bucket aging, detail, tambah/ubah/nonaktif, gabung pelanggan | Pencarian, saldo, detail, tambah/ubah/status dan gabung contoh tersedia; bucket dalam daftar belum tersedia |
| Buku piutang — `ledger.tsx` | Riwayat, kolom transaksi dan saldo, filter, urutan serta halaman; relasi ke penjualan | Kolom, filter tanggal dan urutan mock tersedia; pagination dan relasi detail penjualan belum tersedia |
| Penjualan — `record-sale-dialog.tsx`, `sales.tsx` | Catat belanja multi-item, draft, review, posting dan detail | Form multi-item dan review simulasi tersedia; draft tersimpan dan detail penjualan belum tersedia |
| Pembayaran — `payments.tsx` | Riwayat, catat pembayaran, FIFO dan aplikasi saldo kredit | Form/review, rincian alokasi, kredit per pembayaran dan aplikasi ke debit tertentu tersedia dalam simulasi; belum terintegrasi API atau diuji menyeluruh melalui UI |
| Aging — `reports.tsx` | Tanggal acuan, bucket dan pengecualian, rincian pelanggan | Ringkasan dan drill-down contoh tersedia; belum diuji menyeluruh melalui UI |
| Import — `imports.tsx` | Upload XLSX, batch staging, preview per pelanggan/baris, review dan commit | Dikecualikan sementara atas permintaan pengguna. |
| Pengguna — `users.tsx` | Owner melihat, membuat, mengubah, mengaktifkan/menonaktifkan dan reset kata sandi | Simulasi lokal tersedia; belum diuji menyeluruh melalui UI |
| Koreksi — `corrections.tsx` | Komponen reversal, tautan pengganti, realokasi dan aplikasi kredit historis ada di source; tidak seluruhnya ditampilkan di alur detail saat ini | Pembalikan contoh tersedia; tautan pengganti dan realokasi belum tersedia |

Perlu dibangun bertahap dengan state validasi/loading/error/success dan data mock terisolasi; tindakan mock tidak menyimulasikan jaminan atomisitas, audit, otorisasi, idempotensi, maupun sinkronisasi. Integrasi dan kebijakan offline tetap memerlukan pengujian tersendiri. Jangan menganggap peta fitur sebagai bukti bahwa implementasi desktop sudah setara dengan web.
