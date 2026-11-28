# Prototipe GPUI Kit Windows

Eksperimen terisolasi untuk redesign/refactor desktop. Memakai 120 pelanggan sintetis dalam memori tanpa API, penyimpanan, atau transaksi. Input memfilter tabel; tombol menampilkan contoh kesalahan dan dialog. DataTable menyediakan seleksi sel, kolom yang dapat diurutkan dan diubah lebarnya. Ini hanya prototipe, bukan implementasi finansial.

Jalankan dari root: `cargo run --locked --manifest-path experiments/gpui-kit-prototype/Cargo.toml`.

## Hasil 27 November 2026

- GPUI Kit 0.11.0 dengan `gpui-pre` 0.3.14 dibangun pada Windows menggunakan Rust/Cargo 1.100.0. Aplikasi utama masih memakai GPUI 0.2.2 dan tidak berbagi tipe atau workspace dengan prototipe. Migrasi aplikasi utama belum dilakukan; versi ini kandidat, bukan bukti kompatibilitas migrasi.
- `cargo check`, `cargo build` dan `cargo clippy ... -- -D warnings` untuk manifest prototipe berhasil. Jendela terbuka pada perangkat Lenovo model 83DV; resolusi dan skala tidak terukur pada pemeriksaan ini.
- Pada versi pertama DataTable, input `xyz` menghasilkan state kosong, input `001` menghasilkan satu baris, dialog terbuka dan Escape menutupnya. Pada build berikutnya, pohon aksesibilitas memperlihatkan grid, header dan teks sel setelah teks dibungkus Label. Ini bukan uji screen reader menyeluruh.
- Pemeriksaan otomasi UI tidak konsisten: penentuan posisi tombol pengurutan gagal dan pembacaan state sesekali tidak tersedia. Tab tidak terbukti berpindah dari input pada pengamatan awal; perlu investigasi ulang dengan fokus dan interaksi langsung. Seleksi sel, tombol panah, urutan data, resize kolom, error, clipboard, IME dan pengembalian fokus belum terverifikasi.
- Ukuran 1024×768 dan 1366×768 serta skala 100%, 125%, 150% belum diverifikasi secara terpisah oleh otomasi; hasil tiap kombinasi belum dicatat.

Periksa ulang menggunakan data contoh: pencarian, hasil kosong, error, pemilihan dan navigasi tabel, sort naik/turun, resize kolom, Tab/Shift+Tab, clipboard/IME, dialog dan fokus setelah ditutup. Periksa keterbacaan, clipping dan resize pada ukuran serta skala yang tersedia; catat pengaturan dan hasil.

Pemeriksaan lanjutan: binding Tab dan Shift+Tab ditambahkan. Di Windows, Tab terlihat berpindah dari input ke tabel; tombol panah bawah memilih baris pertama lalu kedua. Uji unit memeriksa filter dan urutan nominal naik/turun. Tombol sort melalui UI belum terbukti, dan pemindahan kolom dinonaktifkan karena mapping prototipe berdasarkan posisi. Pemeriksaan skala, IME, clipboard, resize, dialog/fokus dan aksesibilitas menyeluruh tetap terbuka. Pengujian ini tidak membuktikan migrasi desktop utama.

Pemeriksaan kompatibilitas salinan: `powershell -NoProfile -ExecutionPolicy Bypass -File experiments/gpui-kit-prototype/check-desktop-migration.ps1` membuat salinan sementara di `target/migration-check`, memasangkan GPUI Kit 0.11 dan `gpui-pre` 0.3.14, lalu menjalankan `cargo check --offline`. Pada 27 November 2026 hasilnya berhasil setelah tiga adaptasi pada salinan: inisialisasi `gpui_kit::application()` dan `inset: false` pada dua BoxShadow. Pemeriksaan kompilasi sendiri tidak membuktikan interaksi, regresi, atau integrasi komponen Kit. Script memerlukan dependensi yang sudah tersedia secara lokal.

Seleksi tabel dihapus saat pencarian berubah supaya baris pilihan lama tidak secara diam-diam merujuk hasil lain. Langkah untuk pemeriksaan langsung tersedia di [UJI_MANUAL.md](UJI_MANUAL.md).

Umpan balik pengguna: pengujian manual dilaporkan berhasil, dengan pengecualian tabel tidak dapat digulir horizontal ketika jendela sempit. Perbaikan memberikan lebar minimum nol pada wadah tabel dan menampilkan scrollbar. Pada jendela uji 480×700 logical pixels di Windows, scrollbar terlihat dan dapat diseret ke kanan untuk melihat kolom piutang lalu kembali ke kiri. Pengujian pada konfigurasi dan skala lain tidak disimpulkan dari hasil ini. Opsi `--narrow` membuka ukuran uji tersebut.

Pemeriksaan lanjutan salinan migrasi: `cargo build --offline` dan `cargo test --offline` pada `target/migration-check/Cargo.toml` berhasil; 16 unit test selesai. Salinan dijalankan dengan `TOKO_DESKTOP_ADAPTER=mock` dan Windows melaporkan jendela terbuka. Alat pemeriksaan UI kehilangan sesi ketika membaca jendela, sehingga login, navigasi, detail dan transaksi mock pada salinan belum diverifikasi melalui UI. Tes unit tidak menggantikan verifikasi itu. Token jarak awal kini terpusat di `src/theme.rs`.

Setelah pemeriksaan tersebut, pengguna melaporkan pengujian salinan desktop mock berhasil. Rincian setiap langkah, skala yang digunakan dan konfirmasi terpisah atas scrollbar hasil perbaikan tidak dilaporkan; jangan menganggap seluruh kombinasi perangkat dan aksesibilitas telah teruji. Otomasi sebelumnya melihat scrollbar prototipe bergerak ke kanan dan kembali pada jendela 480×700.
