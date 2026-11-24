> Versi: 1.0 | Disusun: 23 November 2026 | Status: baseline final kebijakan dan rencana; implementasi belum dimulai.

# Product Specification — Keuangan Toko

## Gambaran dan sumber fakta
**PRD-001 [DISEPAKATI]** Produk menggantikan buku piutang Excel per pelanggan menjadi aplikasi web; di fase lanjutan tersedia desktop Windows native GPUI dengan kemampuan offline-first. Backend terpisah dari kedua client. Target pengembangan pertama **web**, backend dan frontend dikembangkan **paralel per vertical slice**.

Sumber historis lokal: `docs/files/CATATAN KEUANGAN TOKO.xlsx` dan spesifikasi awal `spesifikasi-bisnis-keuangan-toko-v0.1.md`. Pemeriksaan struktur paket mengonfirmasi 104 worksheet dan formula; ini belum mengklasifikasi baris aktif atau membuktikan nilai transaksi. Header/row mapping, tanggal, identitas dan debit/kredit harus diprofilkan pada PH-06. Perbedaan formula `SALDO`/`Analisa` tidak menjadi saldo migrasi resmi; ledger dibangun dari baris sumber yang direview.

## Pengguna dan kebutuhan
| Aktor | Kebutuhan yang berasal dari alur yang dibahas | Status |
|---|---|---|
| Petugas/operator toko | mencari pelanggan, mencatat penjualan kredit/pembayaran, memeriksa saldo dan riwayat | [DISEPAKATI] role `operator`, izin di AUTHORIZATION_MATRIX |
| Pemilik toko | memantau piutang, mengelola akun, impor dan koreksi | [DISEPAKATI] role `owner`, seluruh kewenangan sensitif tercantum di AUTHORIZATION_MATRIX |
| Pembaca laporan | melihat pelanggan, saldo, buku piutang dan laporan tanpa mengubah data | [DISEPAKATI] role `viewer` read-only |
| Pengguna desktop Windows | bekerja offline secara terbatas melalui SQLCipher SQLite dan sinkronisasi | [DISEPAKATI] target PH-09; satu offline writer, maksimum 72 jam, sesuai ADR-007 |

## Tujuan, cakupan, eksklusi
- **PRD-002 [DISEPAKATI]** Menyediakan pencatatan pelanggan, transaksi debit/kredit, histori, saldo, dan umur piutang yang sebelumnya ada di Excel.
- **PRD-003 [DISEPAKATI]** Mengimpor data Excel lama dengan perhatian pada kualitas data dan rekonsiliasi, bukan sekadar copy nilai hasil formula.
- **PRD-004 [DISEPAKATI]** Website menjadi tempat memvalidasi proses bisnis dan data API sejak fase pertama.
- **PRD-005 [DISEPAKATI]** Desktop GPUI dan mekanisme offline-first diterapkan **setelah web stabil**, bukan jalur kritis fase web.
- **PRD-006 [DISEPAKATI]** MVP web fokus buku piutang, pelanggan, penjualan kredit, pembayaran dan laporan terkait. POS penjualan tunai lengkap di luar scope.
- **PRD-007 [DISEPAKATI]** MVP tidak mencakup stok, supplier, full accounting, pajak, multi-tenant, payroll, payment gateway atau printer.

## Modul produk
| ID | Modul | Prioritas fase web | Status dan referensi |
|---|---|---|---|
| MOD-01 | Pelanggan: daftar, detail, cari, tambah/ubah/nonaktif | PH-02 | [DISEPAKATI] lifecycle dan role final |
| MOD-02 | Penjualan kredit dan detail barang/jasa | PH-03 | [DISEPAKATI] draft→posted, immutable, reversal |
| MOD-03 | Pembayaran sebagian/lebih, FIFO dan kredit pelanggan | PH-04 | [DISEPAKATI] surplus tidak auto-apply |
| MOD-04 | Buku piutang, saldo dan status | PH-02–05 | [DISEPAKATI] ledger source of truth dan definisi saldo |
| MOD-05 | Impor Excel, staging, review, rekonsiliasi | PH-06 | [DISEPAKATI] partial commit dengan pengecualian eksplisit |
| MOD-06 | Analisis umur piutang, ringkasan | PH-05 | [DISEPAKATI] bucket dan unknown-date exception |
| MOD-07 | Identitas, otorisasi dan audit | lintas fitur | [DISEPAKATI] owner/operator/viewer dan audit append-only |
| MOD-08 | Desktop GPUI, SQLCipher SQLite, sync | PH-09 | [DISEPAKATI] dibatasi satu offline writer dan 72 jam |

## User flow produk
**WF-001 [DISEPAKATI]** Cari pelanggan → lihat histori/saldo → buat draft sale → review/post atau catat payment → lihat saldo terbaru dari server. Sale baru dapat mempunyai beberapa item; baris Excel tidak digabung otomatis.

**WF-002 [DISEPAKATI]** Operator membuat draft penjualan kredit → memeriksa kuantitas/harga → post eksplisit → backend mencatat debit atomik → UI memperbarui saldo.

**WF-003 [DISEPAKATI]** Operator memilih pelanggan → memasukkan pembayaran tunai/transfer → backend mengalokasikan FIFO ke debit eligible tertua → UI menampilkan outstanding dan saldo kredit terpisah. Kredit tidak auto-apply.

**WF-004 [DISEPAKATI]** Unggah workbook → pratinjau kualitas data dan hasil pemetaan → review anomali → rekonsiliasi dengan Excel → persetujuan → data historis tersedia pada aplikasi. Status final dan otoritas persetujuan masih dibutuhkan.

**WF-005 [DISEPAKATI]** Desktop PH-09 menyimpan event lokal idempotent dalam outbox terenkripsi; satu device offline writer maksimal 72 jam; status provisional sampai server menerima; conflict masuk review Owner.

## Business rules
| ID | Ketentuan | Status |
|---|---|---|
| BR-001 | Debit pada buku pelanggan menambah piutang; kredit mengurangi posisi piutang | [DISEPAKATI] pola Excel |
| BR-002 | Saldo berjalan berasal dari transaksi debit-kredit, bukan hasil copy kolom `SALDO` | [DISEPAKATI] prinsip migrasi |
| BR-003 | FIFO saat pembayaran: tanggal usaha tertua, `posted_at`, UUID; import mengikuti source order; backdated tidak mengubah alokasi lama otomatis | [DISEPAKATI] |
| BR-004 | Sale/payment posted immutable; koreksi hanya reversal dan audit oleh Owner | [DISEPAKATI] |
| BR-005 | Rupiah integer BIGINT; quantity NUMERIC(12,3), rounding half-up per item; no discount in MVP | [DISEPAKATI] |
| BR-006 | Surplus tetap customer credit dan hanya diaplikasikan melalui aksi manual eksplisit | [DISEPAKATI] |
| BR-007 | Aging 0–15, 16–30, 31–45, 46–60, >60 sejak tanggal usaha; unknown/future dipisah dan masuk total | [DISEPAKATI] |
| BR-008 | Tanggal/identitas/mixed debit-credit anomaly dikarantina; override/split hanya review Owner dengan alasan | [DISEPAKATI] |
| BR-009 | Setiap baris import memiliki sheet/row lineage yang immutable | [DISEPAKATI] |
| BR-010 | Server jadi source of truth; sync event offline idempotent, conflict tidak last-write-wins | [DISEPAKATI] |
| BR-011 | Sale baru dapat memiliki banyak item; legacy source rows tidak otomatis digabung | [DISEPAKATI] |
| BR-012 | Business date Asia/Jakarta terpisah dari UTC timestamp; backdate Owner-only dengan alasan; future date ditolak | [DISEPAKATI] |

## Pengecualian bisnis
- `EX-001 [DISEPAKATI]` Ada tanggal Excel kosong/tidak valid dan rumus error; kolom turunan H/J/K/L bukan fakta resmi.
- `EX-002 [DISEPAKATI]` Kode pelanggan/faktur lama tidak selalu lengkap/unik; import harus menahan data ambigu untuk keputusan manusia.
- `EX-003 [DISEPAKATI]` Pembayaran surplus ditampilkan sebagai customer credit, tidak membuat piutang negatif dan tidak auto-apply.
- `EX-004 [DISEPAKATI]` Baris Excel debit+kredit masuk karantina; pemecahan hanya reviewer Owner dengan alasan.
- `EX-005 [DISEPAKATI]` Backdate hanya Owner dengan alasan; reversal immutable; period close dan due date tidak termasuk MVP.

## Glosarium
**Debit piutang:** penambah kewajiban pelanggan kepada toko. **Kredit piutang:** pengurang piutang. **Buku piutang:** histori per pelanggan. **Outstanding:** sisa debit belum dilunasi. **FIFO:** pembayaran ke debit tertua berdasar urutan yang harus ditetapkan. **Saldo kredit pelanggan:** uang diterima belum teralokasi pada debit. **Posting:** pengesahan transaksi (rancangan, belum ditetapkan). **Reversal:** transaksi pembalik, bukan menghapus histori. **Staging:** lokasi data impor sebelum persetujuan. **Initial sync:** muat data pusat untuk desktop baru. **Vertical slice:** satu fitur dengan UI+API+persistensi+test.

Seluruh keputusan terbuka dikelola di [OPEN_QUESTIONS.md](OPEN_QUESTIONS.md); spesifikasi UI per fitur berada di [FEATURE_SPECIFICATIONS.md](FEATURE_SPECIFICATIONS.md).
