# Fondasi Desain Desktop

**PH-09 / FR-008 — baseline kerja untuk redesign, 27 November 2026.** Dokumen ini memberi arah awal, bukan bukti aksesibilitas atau kesiapan rilis. Rujuk [rencana fase](DESKTOP_REDESIGN_REFACTOR_PLAN.md) dan hasil [prototipe](../experiments/gpui-kit-prototype/README.md).

## Arah visual dan perilaku

Gunakan aplikasi Windows native GPUI dengan permukaan terang netral, aksen biru seperlunya, garis halus, sudut sedikit membulat dan jarak yang jelas. Pertahankan kepadatan tabel yang nyaman tanpa mengecilkan teks secara berlebihan. Awali dari tema bawaan GPUI Kit; nilai warna, jarak dan tipografi final ditetapkan setelah uji pada perangkat dan skala target. Jangan mencampur nilai hardcoded baru ke setiap layar; konsolidasikan sebagai token semantik di modul tema saat migrasi.

Token awal prototipe (logical pixels): padding halaman 24, jarak antarseksi 16, jarak antartombol 12, tinggi minimum tabel 150; diterapkan terpusat di `experiments/gpui-kit-prototype/src/theme.rs`. Untuk kontrol dan warna, baseline awal adalah tema Default Light GPUI Kit 0.11: background putih, primary neutral-900, teks neutral-950, border neutral-200, header tabel `#fafafa`, latar seleksi `#bfdbfe33` dan garis seleksi `#60a5fa`. Default tema memakai font sistem 16 px, radius 6 px dan radius besar 8 px. Biarkan GPUI Kit memasok warna semantik/ukuran kontrol sampai disesuaikan dalam tema bersama; jangan menduplikasi nilainya di tiap view. Ini baseline kerja untuk ditinjau, bukan klaim kontras atau keterbacaan pada semua skala. Warna status selalu disertai teks.

Pola dasar: tombol utama satu per konteks, aksi sekunder terpisah, input dengan label dan error dekat kolom, dialog dengan judul dan aksi jelas serta Escape sesuai konteks, tabel dengan header jelas dan angka rata kanan. Fokus harus terlihat; Tab/Shift+Tab mengelilingi kontrol dan panah menavigasi tabel. Loading, kosong, error dan sukses mesti memiliki penjelasan dan aksi yang sesuai. Data mock diberi penanda yang terlihat, tanpa fallback diam-diam dari API.

## Batas modul dan migrasi

`app` mengelola komposisi, navigasi dan siklus aplikasi; setiap fitur memiliki state, handler dan tampilan sendiri; `components` menyimpan pola umum; `theme` menyimpan token; `store` mempertahankan antarmuka dan adapter mock/HTTP. Domain dan aturan finansial tidak bergantung pada GPUI. Hindari memindahkan kode semata untuk mengurangi jumlah baris tanpa memisahkan tanggung jawab. Inisialisasi dan tipe navigasi telah dipisahkan; sisanya dikerjakan bersama alur pada fase selanjutnya.

**Strategi kandidat:** GPUI Kit 0.11.0 dengan snapshot GPUI 0.3.14, dikunci bersama melalui satu dependensi Kit; bangun aplikasi desktop dengan satu keluarga tipe GPUI dalam setiap executable. Prototipe menggunakan workspace terpisah dari GPUI 0.2.2 yang saat ini dipakai desktop. Migrasi adalah penggantian dan adaptasi bertahap per alur yang dibuktikan build dan pengujian, bukan mencampur tipe GPUI 0.2.2 dan gpui-pre pada view yang sama. Salinan desktop lolos pemeriksaan kompilasi setelah adaptasi startup dan dua BoxShadow; [eksperimen](../experiments/gpui-kit-prototype/README.md) belum membuktikan perilaku runtime atau integrasi komponen Kit. Gate interaksi dan regresi tetap berlaku.

DataTable prototipe memakai kolom yang dapat diubah lebar dan diurutkan. Pemindahan kolom dinonaktifkan karena mapping data masih berdasarkan posisi. Implementasi produksi memerlukan identitas kolom stabil, format rupiah dan pengujian navigasi/sort/resizing di Windows. Dialog dan input perlu pemeriksaan fokus, keyboard, clipboard dan IME. Gunakan `rfd` hanya saat alur file diperlukan; Storybook/DevTools opsional setelah kompatibilitasnya dibuktikan.
