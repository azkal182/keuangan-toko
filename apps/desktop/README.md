# Piutang Reza Putra — Aplikasi Desktop

Aplikasi desktop menggunakan komponen GPUI Kit untuk kontrol utama, formulir, dialog dan tabel. Status dan batas pengujian tersedia di [PROJECT_STATUS](../../docs/PROJECT_STATUS.md).

Untuk pengujian lokal dengan data sintetis dalam memori di Windows:

```powershell
$env:TOKO_DESKTOP_ADAPTER = "mock"
cargo run --locked -p toko-desktop
```

Mode mock menampilkan penanda **DATA CONTOH**, menggunakan pelanggan sintetis, dan mereset data saat aplikasi ditutup. Akun contoh: `owner`, kata sandi `password1234567`. Jangan masukkan data atau kredensial asli. Mode ini bukan bukti posting ke server, audit, sinkronisasi, atau offline-first.

Tanpa variabel tersebut, desktop memilih adapter HTTP; `TOKO_API_BASE_URL` menentukan alamat API jika dikonfigurasi. Gunakan lingkungan API dan database uji yang terisolasi untuk pengujian integrasi; jangan gunakan data produksi.

## Installer Windows beta

Pasang Inno Setup Compiler terlebih dahulu. Dari root repository, jalankan:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File apps/desktop/installer/build-installer.ps1
```

Script membangun executable profil release dan membuat installer di `dist/desktop/`. Gunakan `-IsccPath "C:\path\ISCC.exe"` jika compiler tidak terdeteksi, atau `-Profile debug` untuk paket pengujian yang namanya berakhiran `-debug`. Installer memasang aplikasi untuk seluruh pengguna dan meminta izin administrator. Aplikasi beta menggunakan API dan memerlukan internet; installer tidak menyediakan mode offline. File installer dan executable belum ditandatangani secara digital. Pastikan hak penggunaan Inno Setup sesuai tujuan distribusi.

Pada daftar, pilih satu baris lalu gunakan **Buka Detail** atau klik dua kali. Tabel memiliki scrollbar horizontal. Setelah mengubah pencarian atau halaman, pilih pelanggan kembali. Belanja dan pembayaran memerlukan tinjauan dan konfirmasi; pemeriksaan keamanan, aksesibilitas, integrasi dan regresi menyeluruh masih diperlukan sebelum rilis.
