# Rencana Implementasi Desktop — Mock Data Terlebih Dahulu

**Task:** PH-09 / FR-008 · **Tanggal:** 26 November 2026 · **Status:** prototipe mock D1 dimulai atas permintaan pengguna; integrasi dan offline belum diimplementasikan.

## Tujuan dan batas

Siapkan aplikasi desktop Windows 11 x64 dengan Rust dan GPUI untuk alur pelanggan dan piutang. Pengembangan dimulai dengan data sintetis lokal agar alur dan antarmuka dapat ditinjau sebelum integrasi API. Mock tidak menyimpan transaksi nyata, tidak membuktikan integrasi atau kemampuan offline-first, dan harus terlihat jelas sebagai data contoh.

Sumber keputusan: `PRODUCT_SPEC.md` (FR dan aturan bisnis), `FEATURE_SPECIFICATIONS.md` (FR-001–005, FR-007–008), `API_SPEC.md`, `AUTHORIZATION_MATRIX.md`, `SECURITY.md`, `ARCHITECTURE.md`, ADR-003, ADR-004 dan ADR-007 §§14, 15, 20. Keputusan OQ-14, OQ-15 dan OQ-20 sudah final; benchmark perangkat adalah kegiatan PH-09. Pengguna meminta prototipe dimulai sebelum web stabil; ini tidak menyatakan gate PH-08 terpenuhi atau membuktikan kesiapan rilis. Status aktual ada di `PROJECT_STATUS.md`.

## Arsitektur yang direncanakan

`apps/desktop/` menjadi composition root GPUI saat PH-09 dimulai. Tampilan memanggil antarmuka layanan aplikasi; implementasi mock sintetis dan adapter API dipisahkan agar penggantian sumber tidak memerlukan penulisan ulang layar. Gunakan kembali domain/money/contract Rust yang sesuai, tanpa dependensi domain pada GPUI, HTTP, atau database. Periksa kebutuhan kontrak API terhadap implementasi saat integrasi; jangan mengubah kontrak publik atau aturan bisnis secara sepihak.

Mock memakai data dan identitas fiktif, sumber deterministik, serta operasi simulasi dalam memori. Penanda “Data contoh — tidak tersimpan di server” terlihat di seluruh prototipe. Data awal dan skenario reset mudah diulang; jangan memakai workbook, kredensial, atau data pelanggan nyata. Perilaku simulasi tidak diberi label transaksi resmi dan tidak dapat mengirim request finansial.

Pada tahap sesungguhnya, server/PostgreSQL tetap sumber kebenaran. Cache SQLite terenkripsi SQLCipher dengan kunci di Windows Credential Manager/DPAPI; penyimpanan lokal dan outbox dipisah dari adapter jaringan. Event offline bersifat provisional sampai diterima server. Rancangan skema lokal dan protokol sinkronisasi harus ditinjau terhadap kontrak yang tersedia sebelum implementasi; detail yang membutuhkan kebijakan baru dicatat sebagai OQ baru.

## Tahapan dan gate

| Tahap | Hasil yang direncanakan | Gate / bukti |
|---|---|---|
| D0 — Persiapan | Periksa toolchain, kompatibilitas GPUI di Windows 11 x64, dependensi dan perangkat uji; buat scaffold `apps/desktop` dalam Cargo workspace. | Aplikasi kosong terbuka pada perangkat target; command aktual dan hasil build dicatat. Angka minimum hardware belum ditetapkan. |
| D1 — Pelanggan mock | Shell, navigasi, daftar/pencarian pelanggan, detail, saldo dan buku piutang sintetis. | Alur FR-001/FR-004 dapat ditinjau, termasuk loading, kosong dan error simulasi; tidak ada koneksi API atau data riil. |
| D2 — Transaksi dan laporan mock | Form penjualan multi-item, review/konfirmasi simulasi, pembayaran, saldo kredit terpisah dan aging. Mutasi hanya pada state mock. | Validasi dan skenario saldo/FIFO/aging dicocokkan dengan domain dan kebijakan; simulasi tidak diklaim sebagai posting, audit, atau bukti AC finansial server. |
| D3 — Integrasi online | Ganti adapter mock per flow dengan API v1: autentikasi, sesi dan permission, pelanggan/ledger, penjualan, pembayaran, laporan. Tangani CSRF, error, timeout dan retry aman sesuai kontrak aktual. | Setiap flow dibuktikan UI → API → PostgreSQL dengan data uji sintetis; tidak ada fallback diam-diam ke mock saat API gagal. Role dan idempotensi diverifikasi. |
| D4 — Offline-first | Provision online, cache minimal terenkripsi, hak akses dan batas 72 jam, satu device offline writer, outbox atomik dan antrean konflik. Admin pengguna/role, commit import dan reversal tetap online-only. | Restart mempertahankan transaksi provisional (AC-016); replay tidak menggandakan efek (AC-017); uji crash, jaringan putus, concurrent/retry, konflik, revoke dan kedaluwarsa sesi. Tidak gunakan last-write-wins. |
| D5 — Kesiapan rilis | Validasi keamanan, pemulihan data lokal, packaging, instalasi/update/rollback serta benchmark pada perangkat target. | Rekam hasil benchmark dan uji operasional, batasan serta prosedur pemulihan; target hardware ditentukan berdasarkan bukti perangkat. |

Setiap tahap dikerjakan sebagai slice yang dapat ditinjau, bukan seluruh tampilan sekaligus. Urutan D1–D2 dapat disesuaikan dengan umpan balik pengguna tanpa mengklaim tahap D3–D5 selesai.

## Slice pertama yang dapat ditinjau

**ID:** PH-09-D1 · **Acuan:** FR-001, FR-004, AC-001, AC-007, AC-008; OQ-14/15/20 sudah diputuskan. **Dependency:** D0 untuk pembuktian berjalan; PH-08/web stabil tetap diperlukan sebelum kesiapan rilis dinyatakan. **Scope:** daftar pelanggan fiktif → cari → buka detail → baca ledger dan saldo. Tidak ada mutasi uang.

Contoh representasi mock internal (bukan kontrak API baru):

```json
{
  "customer": { "id": "mock-001", "name": "Pelanggan Contoh", "code": "CONTOH-01" },
  "outstanding_idr": "60000",
  "customer_credit_idr": "0",
  "ledger": [
    { "description": "Penjualan contoh", "debit_idr": "100000", "credit_idr": "0" },
    { "description": "Pembayaran contoh", "debit_idr": "0", "credit_idr": "40000" }
  ]
}
```

State yang harus ditinjau: loading saat sumber simulasi disiapkan; empty untuk daftar atau pencarian tanpa hasil; error simulasi dengan aksi coba lagi; success menampilkan identitas, ledger, outstanding dan kredit terpisah. Data pelanggan lain tidak boleh muncul pada detail yang dipilih. Seluruh state menampilkan penanda mock. Skenario Rp100.000 dikurangi Rp40.000 menghasilkan Rp60.000; ini hanya validasi prototipe, bukan bukti AC-007 end-to-end.

## Strategi verifikasi dan transisi

- D0–D2: jalankan format, lint, unit test domain yang relevan, build dan pemeriksaan manual UI Windows menggunakan command aktual yang ditemukan saat scaffold. Periksa navigasi keyboard, fokus, keterbacaan, ukuran jendela dan state error/kosong. Belum ada command desktop di repo; jangan mengarang hasil.
- D3: gunakan server/database uji terisolasi dengan fixture sintetis; periksa role Owner/Operator/Viewer, input tidak valid, konflik versi, request berulang, transaksi atomik dan recovery saat jaringan gagal. Verifikasi AC terkait secara end-to-end sebelum menyebutnya terpenuhi.
- D4: uji simpan atomik event + outbox, restart sebelum/sesudah kirim, replay berulang, respons hilang, konflik dan rekonsiliasi server. Periksa enkripsi, pengelolaan kunci, batas sesi dan larangan operasi offline. AC-016/017 hanya tercapai dengan implementasi dan bukti nyata.
- D5: ukur startup, penggunaan memori dan respons alur utama pada perangkat target; catat spesifikasi serta metode, tanpa menetapkan angka target sepihak. Lakukan uji install, upgrade, rollback dan pemulihan dari backup sesuai prosedur yang disetujui.

Integrasi mengganti mock per flow melalui adapter eksplisit. Mode mock hanya untuk pengembangan/review dan harus dipisahkan dari konfigurasi rilis; kegagalan API ditampilkan sebagai error, bukan data contoh seolah data nyata. Tidak ada migrasi dari state mock ke data produksi. Rollback integrasi menggunakan build sebelumnya dan prosedur pemulihan teruji; perubahan schema produksi mengikuti migrasi maju atau restore yang telah diuji.

## Hal yang harus dipastikan saat implementasi

- Kesiapan dan kompatibilitas GPUI/Windows, versi dependensi serta command build pada saat D0; belum diverifikasi dalam tugas perencanaan ini.
- Kecukupan endpoint dan kontrak untuk provisioning device, sinkronisasi, konflik, dan revoke; jangan mengasumsikan endpoint tersebut sudah ada. Kekurangan yang membutuhkan perubahan kontrak atau keputusan bisnis perlu dokumentasi dan persetujuan sesuai GOV-012.
- Perangkat untuk benchmark, distribusi/update dan prosedur recovery dipastikan di fase terkait tanpa mengubah kebijakan yang sudah final.
