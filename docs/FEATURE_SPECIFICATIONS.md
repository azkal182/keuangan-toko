# Feature Specifications — Final MVP Baseline

**Status:** behavior and decision dependencies finalized by ADR-007. Paths and payloads are final in [API_SPEC](API_SPEC.md); role enforcement in [AUTHORIZATION_MATRIX](AUTHORIZATION_MATRIX.md); physical model in [ERD](ERD.md). AC below are verification targets, not results already achieved.

## FR-001 — Daftar dan detail pelanggan (PH-02)

**Aktor:** owner/operator/viewer sesuai permission. Cari melalui nama/kode; detail menampilkan identitas, saldo outstanding, customer credit dan ledger. Nama wajib, code opsional dan unik setelah trim/case-fold, alamat opsional. Tidak ada hard-delete; nonaktif tidak menghapus riwayat. Merge hanya Owner dengan audit.

- **AC-001:** pengguna berizin mencari pelanggan dan membuka detail yang sesuai.
- **AC-002:** create/update valid tersimpan; server menolak nama kosong/kode duplikat dan konflik version dengan field error; setelah refresh nilai sama.
- **Dependencies:** PH-01 contract dan schema; `OQ-09`, `OQ-10`, `OQ-16`, `OQ-17` sudah diputuskan di ADR-007.

## FR-002 — Penjualan kredit / debit piutang (PH-03)

Operator memilih customer aktif, memasukkan tanggal, reference optional dan satu atau lebih item; quantity max 3 decimal, unit price IDR integer; line total half-up per item. Sale disimpan draft, ditinjau, lalu post eksplisit. Posted sale immutable. Future date ditolak; backdate hanya Owner dengan reason.

- **AC-003:** sale berhasil diposting menambah piutang tepat satu kali, terlihat pada ledger/detail.
- **AC-004:** kegagalan posting tidak menghasilkan sale/ledger parsial; request replay tidak menggandakan debit.
- **Permission:** `sales:write`; reversal hanya `transactions:reverse` Owner.
- **Dependencies:** `OQ-01`, `OQ-05`, `OQ-08`, `OQ-11` final melalui ADR-007.

## FR-003 — Penerimaan pembayaran / kredit piutang (PH-04)

Operator mencatat nominal >0, tanggal terima, method `cash`/`bank_transfer`, reference/note opsional, kemudian konfirmasi. Backend posting payment atomik dan mengalokasikan FIFO ke debit yang terbuka saat itu; urutan tanggal usaha, posted_at, UUID. Sisa menjadi customer credit. Kredit tidak auto-apply; Operator/Owner dapat memilih debit dan menerapkan sebagian/sisa dengan audit. Backdated sale tidak mengubah payment allocations lama.

- **AC-005:** debit Rp100.000 dan payment Rp40.000 menghasilkan outstanding Rp60.000.
- **AC-006:** debit Rp100.000 + Rp200.000 dan payment Rp150.000 mengalokasikan debit pertama penuh dan menyisakan Rp150.000 pada debit kedua.
- **Permission:** `payments:write`, `credits:apply`; reversal Owner-only.
- **Dependencies:** `OQ-02`–`OQ-05`, `OQ-08`, `OQ-11` final melalui ADR-007.

## FR-004 — Buku piutang, saldo dan status (PH-02–04)

Ledger per customer menampilkan tanggal usaha, source/reference, description, debit, credit, allocation status dan running balance. Nilai dari API menjadi source of truth; saldo dihitung dari event posted, customer credit terpisah.

- **AC-007:** total posisi piutang dapat direkonsiliasi dari ledger; kolom H Excel tidak dipakai sebagai saldo final.
- **AC-008:** query/customer boundary mencegah transaksi A muncul di B.
- **Permission:** `ledger:read`; semua role boleh read sesuai matrix.

## FR-005 — Analisis umur piutang dan ringkasan (PH-05)

Laporan dan ringkasan daftar pelanggan memakai tanggal kalender Asia/Jakarta (default hari ini). Aging per outstanding debit dari tanggal usaha, tanpa due date: 0–15, 16–30, 31–45, 46–60, >60. Kolom `Jumlah` adalah total piutang terbuka. Piutang bertanggal tidak diketahui atau bertanggal masa depan diringkas dalam kolom pengecualian agar jumlah lima bucket dan pengecualian tetap dapat direkonsiliasi ke `Jumlah`; keduanya tidak dimasukkan ke bucket umur. Nominal berupa string IDR.

- **AC-009:** debit berumur 15 dan 16 hari berada di bucket berbeda.
- **AC-010:** buckets + exception menjumlah tepat ke outstanding total.
- **Permission:** `reports:read`.
- **Dependencies:** `OQ-06`, `OQ-08`, `OQ-12` final melalui ADR-007.

## FR-006 — Impor dan rekonsiliasi Excel (PH-06)

Owner/Operator upload `.xlsx` max 25 MiB. PH-06 mengabaikan worksheet `Analisa` dan membaca sheet pelanggan yang memiliki nama sumber di B2; kode pelanggan berasal dari B1. Sistem menjumlahkan debit/kredit transaksi bertanggal dari tiap sheet dan mengabaikan saldo formula seperti kolom H. Debit literal F dipakai sebagai sumber; bila F formula/kosong, debit dihitung dari kuantitas D × harga E dengan rounding half-up per item sesuai ADR-007 §05. Kredit hanya berasal dari nilai literal G. Tanggal transaksi kosong mengikuti tanggal valid terakhir pada sheet pelanggan yang sama; tanggal warisan ditandai pada preview. Tanggal tidak valid/formula atau tidak ada tanggal terdahulu untuk diwarisi tetap dikarantina. Formula kredit, quantity, atau harga tidak dipakai; baris yang memerlukannya masuk karantina. Baris tanpa nama pelanggan/sheet tidak diimpor. Raw cells/formula provenance tetap disimpan. Pencocokan kode ternormalisasi didahulukan, lalu nama unik; pelanggan baru dibuat atomik saat commit. Preview Owner menampilkan ringkasan keseluruhan dan tabel rekonsiliasi per pelanggan dengan kolom sumber yang sudah ada (sheet, kode, nama, jumlah baris, debit/kredit/net sumber, pencocokan dan tujuan) serta jumlah valid, kandidat, diterima, karantina, pengecualian, nominal belum terhitung, net valid dan variance pending. Owner membuka baris melalui aksi pada baris pelanggan, meninjau valid/exception di dialog, memilih pelanggan, lalu commit seluruh baris kandidat/accepted dari pelanggan terpilih secara atomik. Pelanggan lain tetap di staging untuk diproses kemudian; batch tetap `reviewing` sementara masih ada baris kandidat, accepted, atau karantina yang dapat ditinjau, lalu ditutup saat tak ada baris reviewable yang tersisa. UI menjelaskan variance sebagai `(debit sumber - kredit sumber) - (debit valid - kredit valid)`, menampilkan nilai hasil hitung dan mengisi otomatis bila semua nominal tersedia; error mismatch menyertakan nilai selisih server. Re-upload identik tidak menggandakan ledger dan memperbarui batch lama ke aturan ini hanya jika belum direview/commit.
Saat commit, uraian, jumlah, dan harga transaksi disimpan sebagai detail buku piutang. Buku piutang menampilkan tanggal, uraian, jumlah, harga, debit, kredit, saldo berjalan, jumlah lunas/alokasi, sisa, dan umur dalam hari; nomor faktur/referensi tidak ditampilkan. Migrasi mengisi detail tersebut untuk baris historis yang sumber staging-nya masih tersedia.

- **AC-011:** user berizin melihat sheet/row dan source value baris bermasalah tanpa mutasi sumber.
- **AC-012:** file/batch yang sama tidak menggandakan ledger; replay commit idempotent.
- **AC-013:** preview menampilkan rekonsiliasi; commit tidak menggunakan formula saldo sebagai sumber final; excluded rows/variance disetujui eksplisit.
- **Permission:** `imports:upload`, `imports:read`, `imports:review`, `imports:commit` sesuai matrix.
- **Dependencies:** `OQ-06`, `OQ-07`, `OQ-09`, `OQ-13`, `OQ-22`–`OQ-24` final melalui ADR-007.

## FR-007 — Koreksi, audit dan akses (lintas fitur/PH-07)

Posted sale/payment/ledger tidak diedit atau dihapus. Owner melakukan reversal dengan reason wajib; reversal membuat contra-entry dan audit link ke original. Transaksi pengganti dibuat sebagai event baru. Backend mengecek role setiap operasi; audit mencatat actor/request/time/target/reason.

- **AC-014:** request tanpa permission ditolak dan tidak mengubah saldo.
- **AC-015:** koreksi menyimpan tautan transaksi asal, reversal dan transaksi pengganti.
- **Permission:** `transactions:reverse` Owner-only; user/role changes `users:manage` Owner-only.

## FR-008 — Desktop GPUI offline-first (PH-09)

Dimulai setelah web stabil. Windows 11 x64, SQLite SQLCipher, key DPAPI/Windows Credential Manager. Device perlu provision online; satu offline writer per toko; cache rights/data minimum dan offline session max 72h. Sale/payment operation memiliki UUID idempotent, status provisional, outbox; server menerima/reject; conflict ditinjau Owner. Offline user/role management, import commit dan reversal tidak tersedia.

- **AC-016:** transaksi offline tetap ada setelah restart dan ditandai provisional hingga ack server.
- **AC-017:** replay operation yang sudah berhasil tidak menghasilkan efek ganda.
- **Dependencies:** PH-08 web stabil; `OQ-14`, `OQ-15`, `OQ-20` sudah diputuskan baseline di ADR-007, detail benchmark diverifikasi pada perangkat target.

Verifikasi lintas fitur mengikuti [QUALITY](QUALITY.md); status aktual per fase ada di [PROJECT_STATUS](PROJECT_STATUS.md).
