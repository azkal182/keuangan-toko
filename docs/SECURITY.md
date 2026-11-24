# Security Baseline — Final MVP

**Status:** baseline kebijakan final melalui ADR-007; seluruh kontrol tetap harus dibuktikan pada implementasi dan tidak ada klaim audit/security certification.

## Klasifikasi data

Nama/alamat/kode pelanggan, histori piutang, pembayaran, akun, workbook, staging dan audit diklasifikasikan **internal-konfidensial**. Kumpulkan field seperlunya; alamat opsional. Workbook pelanggan tidak boleh di-commit, dikirim ke log/fixture, atau diekspos sebagai HTML aktif.

## Authentication dan authorization

- Akun lokal individual username/password; role `owner/operator/viewer`; default deny dan authorization Axum per aksi.
- Hash password dengan Argon2id mengikuti OWASP Password Storage; parameter produksi ditune pada hardware, paling rendah baseline rekomendasi OWASP saat implementasi.
- Session server-side opaque; cookie `__Host-session; Secure; HttpOnly; SameSite=Strict; Path=/`, HTTPS-only, CSRF token pada state changes. Sesi web 12 jam absolut, 30 menit idle; logout/reset/perubahan role/disable mencabut sesi terdampak. Password sementara hanya boleh mengakses `/auth/me`, logout, dan ganti password sampai password baru dibuat.
- Login dibatasi rate, password disimpan tidak pernah plaintext/encrypted reversible. MFA bukan kebutuhan MVP; akun Owner memakai password unik dan tersedia audit perubahan akses.

## Aplikasi, API dan database

- HTTPS untuk seluruh sesi; HSTS pada production; CORS same-origin allowlist. Cookie policy dan CSRF diuji bersama; semua private response `Cache-Control: no-store`.
- Validasi ukuran/bentuk semua input di API dan domain; batas workbook 25 MiB; parse XLSX defensif, tidak menjalankan macro/formula; escaped output.
- SQL parameterized; akun DB aplikasi least-privilege; schema/migration hanya melalui pipeline review. Mutasi finansial transaksi atomik, idempotent dan diaudit.
- Secrets hanya di environment/secret manager, tidak di source, `.env.example`, image, browser storage atau logs. Tidak ada token di `localStorage`.
- Error publik tidak membocorkan SQL, stack trace, secret atau raw row. Request ID menghubungkan response/log.

## Audit, import dan retensi

- Audit append-only untuk login/role changes, posting payment/sale, allocation, reversal, customer merge, import review/variance/commit. Catat actor, action, entity, waktu UTC, request ID, reason dan reference; jangan masukkan password/session/raw file/PII yang tidak perlu.
- Upload XLSX dibatasi ukuran (25 MiB), struktur ZIP, 2.048 entry, 128 MiB hasil ekstraksi, 250 sheet, 25.000 baris sumber, dan 256 kolom per rentang baris. Formula dan cached value tidak digunakan sebagai nilai ledger. File asli dan raw row hanya dibaca lewat permission import; akses review/commit Owner-only, upload juga dapat dilakukan Operator. Semua mutasi commit/reversal/reallocation memerlukan CSRF; operasi posting sensitif idempotent.
- Pencocokan identitas import membandingkan kode/nama ternormalisasi; identitas ambigu tidak auto-map. Pembuatan customer baru hanya saat commit Owner, dalam transaksi yang sama dengan event ledger dan audit sehingga kegagalan tidak meninggalkan customer parsial.
- Financial records dan audit disimpan 10 tahun minimum sejak akhir tahun buku sebagai kebijakan baseline, sejajar dengan UU 8/1997 Pasal 11 sepanjang berlaku. File workbook raw dihapus 30 hari setelah batch final; batch gagal/tidak aktif dihapus raw values 90 hari setelah aktivitas terakhir. Log aplikasi redacted 90 hari. Legal hold mengalahkan purge sesuai proses Owner.
- Worker atau scheduler purge untuk menerapkan jangka retensi tersebut belum tersedia pada PH-06/PH-07; implementasi dan bukti penghapusannya menjadi exit gate PH-08 sebelum menyimpan data workbook di production.
- Pelindungan data mengikuti prinsip minimisasi dan pemusnahan/penghapusan sesuai tujuan/retensi dengan memperhatikan kewajiban penyimpanan dokumen; lihat UU 27/2022. Owner melakukan review retensi sebelum PH-08.
- Backup terenkripsi; encryption keys dipisahkan dari data backup. Restore diuji sebelum production dan setiap perubahan besar pada prosedur backup.

## Desktop PH-09

Device provisioning online; SQLCipher untuk SQLite dan key dalam Windows Credential Manager/DPAPI; offline session maksimal 72 jam. Cache hanya minimum data, offline admin/import approval/reversal tidak tersedia. Cabut device dan hentikan akses pada koneksi berikutnya.

## Acceptance keamanan

1. Unauthenticated/forbidden action tidak mengubah DB; lintas customer access diuji.
2. Invalid payload/post failure tidak menghasilkan transaksi parsial.
3. Replay key sama tidak menggandakan financial event; body berbeda ditolak.
4. Secret/PII raw tidak muncul di response, logs, traces atau error.
5. Workbook abnormal ditolak/karantina dan formula tidak dieksekusi.
6. Audit memiliki actor dan lineage; tidak dapat diedit via UI.

Rujukan: [UU 8/1997](https://peraturan.bpk.go.id/Home/Download/34482/UU%20Nomor%208%20Tahun%201997.pdf), [UU 27/2022](https://peraturan.bpk.go.id/Home/Download/224884/UU%20Nomor%2027%20Tahun%202022.pdf), OWASP [Password Storage](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html), [Session Management](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html), [CSRF Prevention](https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html).
