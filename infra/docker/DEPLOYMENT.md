# Deployment Docker VPS

Konfigurasi ini untuk satu VPS Ubuntu 24 dengan Docker Compose, Cloudflare proxy, dan domain `piutang.banza.web.id`. Cloudflare harus memakai SSL/TLS **Full (strict)**. Caddy mengambil sertifikat origin publik melalui ACME HTTP-01, sehingga port TCP 80 dan 443 perlu dapat dijangkau melalui Cloudflare. Jangan buka port PostgreSQL atau API ke internet.

## Persiapan pertama di VPS

1. Pasang Docker Engine dan Compose plugin dari repositori resmi Docker. Buat pengguna deploy non-root yang dapat menjalankan Docker. Batasi SSH ke alamat administrator dan izinkan TCP 80/443 dari Cloudflare.
2. Pastikan DNS `piutang.banza.web.id` menunjuk ke IPv4 VPS dan status proxy Cloudflare aktif. Atur SSL/TLS ke Full (strict).
3. Buat `/opt/keuangan-toko/.env` dengan permission `600`:

   ```dotenv
   POSTGRES_PASSWORD=isi_dengan_hex_acak_minimal_48_karakter
   API_IMAGE=ghcr.io/azkal182/keuangan-toko-api
   WEB_IMAGE=ghcr.io/azkal182/keuangan-toko-web
   IMAGE_TAG=latest
   ```

   Buat password menggunakan generator kriptografis di VPS, jangan gunakan contoh teks di atas. Simpan salinan pemulihan secret di password manager organisasi.
4. Pastikan pengguna deploy dapat membaca `/opt/keuangan-toko/.env` dan menjalankan Compose dari direktori tersebut. Workflow GitHub menyalin `compose.yaml` dan Caddyfile, tetapi tidak menimpa `.env`.
5. Buat GitHub personal access token classic dengan scope `read:packages`, yang hanya dapat membaca package milik akun penerbit. Simpan token itu sebagai Actions secret `GHCR_READ_TOKEN`. Package GHCR tetap private.
6. Jalankan workflow `CI and deploy` pada branch `main`. Setelah image diterbitkan, workflow menyalin konfigurasi, login ke GHCR melalui SSH, menerapkan migrasi, lalu mengganti container ke image commit tersebut.
7. Buat akun Owner pertama satu kali, dari direktori deployment:

   ```bash
   cd /opt/keuangan-toko
   docker compose run --rm -T api bootstrap-owner owner
   ```

   Perintah meminta password melalui stdin. Gunakan password unik minimal 15 karakter dari password manager. Jangan letakkan password pada argumen shell atau log CI.

## GitHub Actions secrets

Simpan nilai ini di pengaturan repository, bukan sebagai file repository:

- `VPS_HOST`: alamat IP VPS.
- `VPS_USER`: pengguna deploy non-root.
- `VPS_SSH_PRIVATE_KEY`: private key khusus deployment.
- `VPS_KNOWN_HOSTS`: host key VPS yang diverifikasi di luar workflow.
- `GHCR_READ_TOKEN`: GitHub PAT classic dengan scope `read:packages`.

Gunakan environment protection dan persetujuan manual sebelum deploy production jika tersedia. Deploy berjalan otomatis sesudah semua job verifikasi dan publish sukses pada push ke `main`. Pull request hanya menjalankan verifikasi dan build image tanpa publish atau deploy.

## Pemeriksaan pascadeploy

- Buka `https://piutang.banza.web.id` dan verifikasi sertifikat valid.
- Pastikan login Owner, daftar pelanggan, dan operasi baca API berfungsi.
- Periksa `docker compose ps` dan `docker compose logs --since=10m api web caddy` tanpa menyalin log yang berisi data pelanggan.
- Pastikan backup database otomatis yang terenkripsi dan pemulihan ke database terisolasi telah diuji sebelum memasukkan data pelanggan nyata. Konfigurasi Compose ini belum memasang sistem backup.

Image deployment diberi tag `sha-<commit>`. Workflow memakai Docker credential directory sementara dan menghapusnya setelah deployment. Untuk rollback aplikasi, ubah `IMAGE_TAG` di `.env` ke `sha-<commit-sebelumnya>` lalu jalankan `docker compose pull`, `docker compose up -d`. Migrator saat ini forward-only, jadi rollback schema mengikuti prosedur restore/recovery dan tidak dilakukan dengan menurunkan versi database secara otomatis.
