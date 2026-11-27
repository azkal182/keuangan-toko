# Prototipe desktop

Prototipe Rust/GPUI menggunakan data sintetis dalam memori. Login dan peran hanya simulasi; tidak ada autentikasi server, API, penyimpanan, sinkronisasi, atau transaksi keuangan nyata. Setiap peluncuran memulai ulang data contoh.

Dari root workspace pada Windows dengan Rust dan toolchain C++ yang tersedia:

```powershell
cargo run --locked -p toko-desktop
```

Pilih akun Owner/Operator/Viewer contoh dan gunakan kata sandi awal `contoh123`. Jangan masukkan kredensial asli. Klik kolom pencarian dan ketik nama/kode; klik pelanggan untuk melihat detail. Form mendukung pengetikan dasar, Backspace, Ctrl+A dan Tab; input IME, clipboard dan navigasi keyboard lengkap belum tersedia. Prototipe bukan bukti kesiapan aksesibilitas atau offline-first. Impor workbook dikecualikan dari pekerjaan saat ini.
