# Desktop Keuangan Toko

Fase 2 redesign menggunakan GPUI Kit secara bertahap. Daftar pelanggan memakai DataTable dan pencarian Kit; beberapa layar, tabel ledger dan input form masih menggunakan komponen sebelumnya. Status dan batas pengujian tersedia di [PROJECT_STATUS](../../docs/PROJECT_STATUS.md).

Untuk pengujian lokal dengan data sintetis dalam memori di Windows:

```powershell
$env:TOKO_DESKTOP_ADAPTER = "mock"
cargo run --locked -p toko-desktop
```

Mode mock menampilkan penanda **DATA CONTOH**, menggunakan pelanggan sintetis, dan mereset data saat aplikasi ditutup. Akun contoh: `owner`, kata sandi `password1234567`. Jangan masukkan data atau kredensial asli. Mode ini bukan bukti posting ke server, audit, sinkronisasi, atau offline-first.

Tanpa variabel tersebut, desktop memilih adapter HTTP; `TOKO_API_BASE_URL` menentukan alamat API jika dikonfigurasi. Gunakan lingkungan API dan database uji yang terisolasi untuk pengujian integrasi; jangan gunakan data produksi.

Pada daftar, pilih satu baris lalu gunakan **Buka Detail** atau klik dua kali. Tabel memiliki scrollbar horizontal. Setelah mengubah pencarian atau halaman, pilih pelanggan kembali. Belanja dan pembayaran memerlukan tinjauan dan konfirmasi; pemeriksaan keamanan, aksesibilitas, integrasi dan regresi menyeluruh masih diperlukan sebelum rilis.
