# Pemeriksaan Manual Prototipe Desktop

Prototipe hanya memakai data contoh. Jangan masukkan nama atau data pelanggan sungguhan. Jalankan dari root repositori:

```powershell
cargo run --locked --manifest-path experiments/gpui-kit-prototype/Cargo.toml
```

Catat versi Windows, ukuran/skala layar yang digunakan, dan hasil setiap langkah (berhasil / gagal / belum dicoba). Skala yang biasa dipakai cukup untuk mulai; ukuran dan skala lain dicatat terpisah.

1. Ketik `001` di pencarian: hanya pelanggan contoh 001 ditampilkan. Ketik `xyz`: muncul pesan tidak ada pelanggan yang cocok. Kosongkan lagi.
2. Pilih teks contoh di pencarian, salin, hapus lalu tempel kembali dengan Ctrl+C dan Ctrl+V. Pastikan teks yang sama muncul. Jika memakai IME, coba memasukkan beberapa karakter dan menghapusnya.
3. Dari input, tekan Tab untuk masuk tabel, lalu panah bawah dua kali. Periksa fokus terlihat dan pilihan bergerak. Gunakan Shift+Tab untuk kembali. Setelah filter berubah, pastikan pilihan lama tidak menandai pelanggan lain.
4. Gunakan ikon urut pada header Piutang: periksa nominal berubah urut; ulangi untuk arah sebaliknya. Seret batas kolom untuk mengubah lebarnya dan pastikan header serta data tetap sejajar. Pemindahan urutan kolom memang dinonaktifkan pada prototipe.
5. Klik **Tampilkan error contoh**: pesan kesalahan tampil. Klik lagi: pesan hilang. Klik **Uji dialog**, lalu tekan Escape: dialog menutup. Buka lagi dan tutup melalui tombol X; periksa fokus kembali ke kontrol yang masuk akal.
6. Ubah ukuran jendela: periksa teks, tabel dan tombol tetap terbaca dan tidak terpotong. Bila tersedia, coba area jendela 1024×768 dan 1366×768 serta skala 100%, 125% dan 150%; kembalikan pengaturan awal setelah selesai.
7. Persempit jendela hingga kolom piutang terpotong. Seret scrollbar di bagian bawah tabel ke kanan untuk melihat kolom tersebut, lalu kembali ke kiri untuk melihat nama. Untuk langsung membuka jendela sempit, tambahkan `-- --narrow` pada perintah di atas.

Laporkan langkah yang gagal beserta apa yang terjadi. Ini uji prototipe, bukan pengujian aplikasi keuangan atau bukti kesiapan aksesibilitas penuh.
