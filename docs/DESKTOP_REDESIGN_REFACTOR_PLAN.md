# Rencana Redesign dan Refactor Desktop

**ID:** PH-09 / FR-008 · **Tanggal:** 27 November 2026 · **Status:** fase 1 selesai sebagai fondasi; fase 2 berjalan, parsial.
**Branch kerja awal:** `codex/desktop-redesign-refactor`.

## Tujuan dan batas

Matangkan UI/UX aplikasi desktop Windows agar terasa native, modern dan mudah digunakan, berdasarkan prototipe GPUI Kit yang telah ditinjau dan disukai pengguna. Pecah struktur kode menurut tanggung jawab, kurangi file yang terlalu besar, dan gunakan komponen baru secara bertahap tanpa kehilangan perilaku yang telah tersedia.

Dokumen ini merinci pekerjaan redesign/refactor di dalam PH-09; bukan pengganti [rencana implementasi desktop](DESKTOP_IMPLEMENTATION_PLAN.md), [spesifikasi fitur](FEATURE_SPECIFICATIONS.md), [kontrak API](API_SPEC.md), [matriks otorisasi](AUTHORIZATION_MATRIX.md), atau ADR-007. Keputusan bisnis dan API publik tidak berubah secara sepihak. Gate stabilitas web PH-08 dan kesiapan rilis desktop tetap berlaku. Offline-first, SQLCipher, outbox dan sinkronisasi bukan hasil yang diklaim oleh pekerjaan desain ini.

Prototipe terisolasi `experiments/gpui-kit-prototype` menggunakan GPUI Kit 0.11, DataTable dan data sintetis. Build dan pengujian terbatas telah dilakukan; hasil dan kekurangan tercatat di [README eksperimen](../experiments/gpui-kit-prototype/README.md). GPUI Kit memakai snapshot GPUI berbeda dari `gpui = 0.2.2` pada aplikasi utama. Salinan desktop yang diadaptasi berhasil dibangun, diuji unit, dan pengujian manualnya dilaporkan berhasil oleh pengguna; migrasi aplikasi utama dikerjakan bertahap pada fase 2.

## Prinsip desain dan implementasi

- Pertahankan kesan aplikasi Windows: navigasi dan shortcut yang konsisten, tabel yang mudah dibaca, fokus yang terlihat, dialog jelas dan interaksi keyboard yang nyaman.
- Gunakan tampilan netral dan modern, hierarki teks yang jelas, garis secukupnya dan aksen warna yang membantu tindakan utama. Warna status disertai teks.
- Dahulukan alur mencari pelanggan, memahami saldo, mencatat penjualan atau pembayaran dan meninjau hasil. Pisahkan piutang dan saldo kredit secara jelas.
- Kembangkan per alur yang dapat ditinjau: loading, kosong, error dan sukses; label dan validasi yang jelas; konfirmasi untuk tindakan relevan. UI bukan pengganti validasi dan otorisasi server/domain.
- Gunakan data sintetis untuk prototipe dan pengujian; mode mock ditandai jelas, tidak mengirim transaksi finansial dan tidak menjadi fallback diam-diam saat API gagal. Jangan gunakan workbook atau data pelanggan asli.
- Pertahankan pemisahan adapter mock dan HTTP, domain dari GPUI/HTTP/DB, dan kontrak API yang telah disetujui. Catat konflik atau kebijakan baru sesuai GOV-012 sebelum perubahan breaking.

## Fase 1 — Fondasi komponen dan struktur

**Tujuan:** membuktikan kecocokan komponen GPUI Kit di Windows dan menyiapkan batas modul yang dapat dirawat.

1. Lengkapi prototipe terisolasi dengan DataTable interaktif, input, dialog serta contoh state kosong dan error. Periksa navigasi tabel, seleksi, urutan/ukuran kolom bila relevan, dan perilaku dengan data sintetis yang cukup.
2. Periksa Windows: build dan jendela, Tab/Shift+Tab, fokus, pengetikan, clipboard dan IME; pembukaan/penutupan dialog, Escape dan pengembalian fokus; resize, keterbacaan dan clipping pada 1024×768 dan 1366×768 serta skala 100%, 125%, 150% jika tersedia. Catat perangkat, pengaturan, hasil dan apa yang belum diuji; uji aksesibilitas dengan alat yang tersedia.
3. Catat versi dan kompatibilitas GPUI Kit, GPUI, toolchain dan komponen. Pilih strategi migrasi hanya setelah bukti build dan interaksi; jangan mencampur tipe dari salinan GPUI yang berbeda secara tidak sengaja.
4. Siapkan dan terapkan pemisahan bertahap untuk inisialisasi/navigasi/state aplikasi, fitur, komponen bersama, tema dan adapter data. Pisahkan perilaku dan state menurut tanggung jawab, bukan hanya memecah file berdasarkan jumlah baris.
5. Tetapkan token visual dan pola tombol, input, dialog, tabel dan penanda mock. Pertahankan alur yang ada saat komponen diganti.

**Gate:** hasil uji komponen dan kompatibilitas tercatat; fondasi dapat dibangun; alur terdampak diperiksa terhadap perilaku sebelumnya. Kekurangan yang menghalangi migrasi dicatat beserta langkah berikutnya. Jangan menyatakan fase selesai hanya karena prototipe dapat dikompilasi.

**Hasil fase 1:** pengguna melaporkan pengujian salinan desktop mock berhasil. Prototipe dibangun dan diuji; scroll horizontal pada jendela sempit diperbaiki dan diamati melalui otomasi. Token awal dan batas modul tersedia. Pengujian lengkap screen reader, semua skala dan regresi seluruh fitur belum dibuktikan; verifikasi dilanjutkan per slice pada fase 2 dan tidak disamakan dengan kesiapan rilis atau offline-first. Pengguna belum memberikan laporan terpisah tentang perbaikan scrollbar setelah build baru.

Struktur berikut adalah arah, bukan kewajiban memindahkan semua kode sekaligus:

```text
apps/desktop/src/
├── app/          # inisialisasi, navigasi, state aplikasi
├── features/     # pelanggan, transaksi, laporan, akun
├── components/   # komponen bersama
├── theme/        # token visual
└── store/        # antarmuka dan adapter data
```

## Fase 2 — Redesign alur utama toko

**Tujuan:** menerapkan desain dan struktur baru pada satu alur utuh yang paling sering digunakan.

Kerjakan sebagai slice yang dapat ditinjau: (1) shell dan navigasi; (2) daftar dan pencarian pelanggan; (3) detail pelanggan, saldo dan buku piutang; (4) penjualan dan pembayaran, termasuk validasi dan tinjauan sebelum konfirmasi. Gunakan komponen baru yang telah dievaluasi; rapikan state, handler dan tampilan fitur bersamaan. Pastikan informasi pelanggan terpilih, saldo dan transaksi tetap tepat, serta error, loading dan kondisi kosong terlihat.

**Gate:** alur daftar → detail → tinjau transaksi dapat diperiksa dengan data sintetis; fungsi, validasi dan hak akses yang ada diperiksa untuk tiap slice. Uji integrasi HTTP yang tersedia dengan lingkungan dan data uji terisolasi; simulasi bukan bukti posting, FIFO atau transaksi riil. Tidak mengubah kontrak/migrasi atau aturan finansial tanpa proses keputusan proyek.

**Status: selesai menurut kriteria uji fungsi dan visual yang ditetapkan pengguna (27 November 2026).** Desktop memakai GPUI Kit untuk pencarian, DataTable pelanggan dan tombol utama; shell/dialog diperbarui, dan view pelanggan, ledger, pembayaran serta dialog dipisah ke modul fitur. Uji Windows dengan data mock sintetis memeriksa pencarian, detail, belanja Rp5.000, pembayaran Rp1.000, tinjau, saldo, riwayat, gulir horizontal dan tampilan Viewer. Uji adapter HTTP terpisah berhasil membuat pelanggan dan transaksi sintetis di server uji pengguna dan membaca saldo yang diharapkan. Uji tersebut tidak mencakup UI yang terhubung ke server.

**Tindak lanjut Fase 3:** migrasi komponen form dan tabel ledger/riwayat belum menyeluruh; pemeriksaan aksesibilitas, error, seluruh role dan berbagai skala layar belum lengkap. Penyelesaian Fase 2 tidak menyatakan kesiapan rilis atau offline-first.

## Fase 3 — Layar pendukung dan pemantapan

**Tujuan:** meluaskan pola yang terbukti ke layar desktop lainnya dan menguji konsistensi.

Terapkan pola desain dan struktur pada laporan aging, riwayat pembayaran, akun dan alur koreksi yang sudah tersedia. Bersihkan komponen lama setelah penggantinya dibuktikan; fitur yang belum dipindah tetap dipertahankan. Periksa Owner/Operator/Viewer, validasi dan error API, keyboard/fokus dialog, keterbacaan, resize dan skala layar. Jalankan pemeriksaan repo yang relevan dan tersedia, serta perbarui peta fitur dan status proyek berdasarkan bukti aktual.

**Gate:** layar dalam cakupan konsisten, perilaku sebelumnya tidak hilang tanpa keputusan yang sesuai, hasil uji dan keterbatasan tercatat, dan perubahan dapat ditinjau serta dibatalkan melalui versi branch sebelumnya. Kesiapan rilis dan offline-first tetap memerlukan gate terpisah PH-09.

**Status: selesai menurut kriteria uji fungsi dan visual pengguna (27 November 2026).** Laporan umur dan akun dipisah ke modul fitur, tabelnya mendapat gulir horizontal, dan error saat memuat akun tidak lagi disamarkan sebagai daftar kosong. Uji Windows mock mengamati laporan, filter, gulir, akun Owner, tampilan Operator/Viewer, validasi dan pembalikan transaksi, serta pintasan F6 → F2 → F8. Ringkasan mock diperbaiki agar pembalikan tidak menciptakan saldo kredit fiktif; pengujian unit mencakup pembalikan belanja dan pembayaran yang dialokasikan. Perubahan tidak mengubah kontrak atau migrasi.

**Batas hasil:** belum ada pengujian menyeluruh seluruh ukuran/skala dan screen reader, simulasi error API di UI, maupun pembalikan melalui server. Sebagian form dan tabel masih komponen lama; pekerjaan tersebut dan kesiapan offline-first/produksi tetap memerlukan tindak lanjut terpisah.

## Library dan keputusan lanjutan

- **GPUI Kit / gpui-component / gpui-base / aset:** kandidat utama untuk komponen dan fondasi; pemakaian produksi bergantung pada gate fase 1, termasuk versi, aksesibilitas dan interaksi Windows.
- **rfd:** pertimbangkan saat fitur pemilihan atau penyimpanan file dikerjakan; bukan prasyarat alur utama.
- **rust-embed:** opsional jika aset/font khusus diperlukan; periksa aset yang sudah disediakan Kit.
- **gpui-storybook dan gpui-devtools:** opsional untuk development; pastikan kompatibilitas versi dan dukungan workflow Windows sebelum mengadopsi.

## Verifikasi dan pelaporan per slice

Rujuk FR/AC yang relevan dalam [FEATURE_SPECIFICATIONS](FEATURE_SPECIFICATIONS.md), catat perubahan UI/API dan state yang diperiksa, lalu jalankan command nyata yang sesuai dengan slice. Untuk alur uang, periksa validasi, permission, ketepatan nominal serta respons sukses/gagal menggunakan data uji yang aman. Cantumkan hasil aktual dan `belum dijalankan` untuk pemeriksaan yang belum tersedia. Update [PROJECT_STATUS](PROJECT_STATUS.md) saat ada kemajuan atau blocker; perbarui peta fitur jika kemampuan berubah. Hindari mengklaim AC-016/017 tanpa uji restart/outbox/replay yang sesungguhnya.

**Langkah berikutnya:** tindak lanjuti pengujian dan migrasi komponen yang belum menyeluruh serta gate kesiapan desktop PH-09 secara terpisah.

## Penyempurnaan lanjutan yang diminta pengguna

**Status: berjalan, parsial.** Halaman masuk terpisah, Logout, daftar pelanggan yang dibuka dalam sesi pada sidebar, penyembunyian aksi pembalikan dan realokasi, serta contoh placeholder nominal mulai diterapkan. Login, daftar pelanggan, laporan umur dan penyimpanan belanja/pembayaran berjalan di worker latar. Hasil simpan gagal dikunci pada tinjauan dan pengguna diminta memeriksa buku/riwayat sebelum mencoba lagi. Pemeriksaan mock mencakup dua pelanggan, login, pencarian, laporan dan pembayaran. Form login masih memakai input lama.

**Lanjutan 28 November:** detail pelanggan, buku piutang, riwayat pembayaran, daftar akun, logout, penyegaran sesi dan perubahan status akun juga berjalan di worker latar. Respons usang dari permintaan detail dan 401 sesi lama tidak menimpa pilihan atau sesi baru. Uji mock memeriksa detail, buku piutang dan riwayat; 21 unit test berhasil.

**Lanjutan 28 November (form dan operasi):** penyimpanan pelanggan, penggabungan pelanggan, pengelolaan akun dan kata sandi, penerapan saldo kredit, dan pembacaan data untuk form penerapan kredit dipindah ke worker dengan indikator proses. Date picker GPUI Kit digunakan untuk tanggal belanja, pembayaran, filter tanggal dan laporan umur. Uji Windows mock memeriksa pilihan tanggal pembayaran, pembersihan filter, validasi tanggal laporan serta perpindahan fokus Tab dari tanggal ke kolom lain. `cargo test --locked --offline -p toko-desktop` berhasil: 21 test, satu test server diabaikan. Build dan Clippy desktop juga berhasil.

**Tabel dan pagination:** rincian umur menggunakan DataTable GPUI Kit; klik dua kali membuka detail. Pagination menggunakan tombol GPUI Kit dan membatasi halaman saat hasil menyusut; perubahan filter/urutan serta pelanggan mereset halaman. Uji Windows mock mengamati tabel umur, pembukaan detail, dan pagination satu halaman/kosong. Unit test mencakup batas halaman saat hasil menyusut (22 berhasil, satu test server diabaikan). Tabel buku piutang dan riwayat masih grid lama; perpindahan lebih dari satu halaman belum diuji di UI.

**Lanjutan tabel detail:** buku piutang dan riwayat pembayaran dipindah ke DataTable GPUI Kit; pilih pembayaran bersaldo lebih untuk mengaktifkan tombol penerapan kredit. Uji Windows mock memeriksa tampilan nilai, seleksi dan pembukaan form dengan pembayaran terpilih. Pengujian perpindahan lebih dari satu halaman tetap diperlukan.

**Input login:** kolom nama pengguna dan kata sandi menggunakan Input GPUI Kit dengan kata sandi tersamarkan. Uji Windows mock mencakup logout, fokus awal, pengetikan, Tab/Shift+Tab dan login dengan Enter. Input dan dialog form lainnya serta login HTTP masih perlu diuji dan dimigrasikan.

**Input pelanggan:** form tambah dan ubah memakai Input GPUI Kit. Uji Windows mock memeriksa validasi, Tab, Enter, pembuatan dan penyuntingan; nama sidebar ikut diperbarui setelah disimpan. Build terpisah, Clippy dan 22 unit test berhasil; satu test server diabaikan. Form transaksi dan dialog lain belum seluruhnya dimigrasikan.

**Input form dan bagian dialog:** input umum dan baris belanja memakai GPUI Kit. Header, judul, isi dan footer memakai komponen dialog GPUI Kit; overlay dan pengelolaan modal masih khusus aplikasi, sehingga migrasi Dialog sebagai root belum selesai. Uji Windows mock mencatat belanja dua barang Rp5.000 dan pembayaran Rp1.000 (saldo Rp104.000), navigasi Tab, tinjauan dan Escape pada form pelanggan. Seluruh form/role dan perilaku focus trap belum diuji menyeluruh.

**Lanjutan root dialog:** modal sekarang menggunakan pengelola Dialog GPUI Kit dengan view terpisah. Uji Windows mock memeriksa pembukaan dan penyimpanan pelanggan, date picker, Escape dari tinjauan dan konfirmasi pembayaran dengan Enter; saldo sesudah pembayaran Rp1.000 adalah Rp99.000. Pemeriksaan lengkap seluruh form, role, ukuran layar dan focus trap masih diperlukan.

**Audit komponen:** tabel akun menggunakan DataTable; tombol tindakan, tab, sidebar, role mock, hapus barang, dan kartu filter umur menggunakan Button GPUI Kit. Uji Windows mock memeriksa tabel akun, pembatasan akun sendiri, sidebar, detail dan filter umur. Layout dan konten noninteraktif tetap menggunakan primitive GPUI. Pemeriksaan komprehensif semua role, ukuran, aksesibilitas, HTTP dan jaringan lambat masih diperlukan.

**Belum selesai:** migrasikan input, dialog dan tabel tersisa ke komponen GPUI Kit; audit seluruh jalur request yang masih sinkron, termasuk kode fitur yang disembunyikan; uji loading/error/timeout dan pencegahan klik ganda dengan koneksi lambat, transaksi dengan server uji, interaksi keyboard secara menyeluruh, banyak pelanggan, dan resize. Belum ada jaminan idempotency lintas percobaan sesudah respons hilang; pengguna perlu memeriksa hasil transaksi sebelum mencoba lagi.
