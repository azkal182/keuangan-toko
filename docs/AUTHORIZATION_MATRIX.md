# Authorization Matrix — Final MVP

**Status:** disepakati melalui ADR-007. Semua permission ditegakkan di Axum untuk setiap request; UI hiding bukan kontrol keamanan. Default deny.

## Identitas dan role

Login memakai akun lokal individual username/password. Role final: `owner`, `operator`, `viewer`. Setiap mutasi mencatat `actor_id`; akun bersama dilarang. Hanya Owner mengelola akun/role dan melakukan reset password pengguna lain.

| Aksi / permission | Owner | Operator | Viewer |
|---|:---:|:---:|:---:|
| Melihat pelanggan, saldo, ledger, payment dan sale (`customers:read`, `ledger:read`, `payments:read`, `sales:read`) | Ya | Ya | Ya |
| Membuat/mengubah identitas pelanggan (`customers:write`) | Ya | Ya | Tidak |
| Merge pelanggan duplikat (`customers:merge`) | Ya | Tidak | Tidak |
| Menonaktifkan customer dengan open AR (`customers:write`) | Ya | Ya | Tidak |
| Membuat draft, edit draft dan posting penjualan (`sales:write`) | Ya | Ya | Tidak |
| Mencatat pembayaran (`payments:write`) | Ya | Ya | Tidak |
| Mengalokasikan customer credit secara manual (`credits:apply`) | Ya | Ya | Tidak |
| Membaca laporan aging (`reports:read`) | Ya | Ya | Ya |
| Upload workbook (`imports:upload`) | Ya | Ya | Tidak |
| Melihat batch/staging dan memetakan baris (`imports:read`, `imports:review`) | Ya | Tidak | Tidak |
| Override baris karantina dan catat usulan split sebagai review note (`imports:override`) | Ya | Tidak | Tidak |
| Menyetujui variance dan commit import (`imports:commit`) | Ya | Tidak | Tidak |
| Reversal transaksi (`transactions:reverse`) | Ya | Tidak | Tidak |
| Reallocation alokasi (`credits:reallocate`) | Ya | Tidak | Tidak |
| Mengelola user/role dan business settings (`users:manage`, `settings:write`) | Ya | Tidak | Tidak |
| Mengelola device offline / menangani sync conflict (PH-09) | Ya | Tidak | Tidak |

## Aturan penegakan

- Tidak ada akses jika tidak login, akun nonaktif, atau permission tidak tercantum.
- Request ke customer/sale/payment/import di luar scope menghasilkan `404` atau `403` tanpa membocorkan keberadaan data yang tidak boleh dilihat.
- Minimum satu Owner aktif selalu harus ada. Perubahan role/aktivasi akun membatalkan sesi pengguna yang terdampak.
- Viewer dapat melihat data bisnis yang sama dengan batas baca; tidak menerima endpoint/import raw access.
- Desktop PH-09 memakai permission cached dari login terakhir maksimal 72 jam; offline tidak mendukung admin, import commit, reversal, atau role mutation.

Detail session dan audit mengikuti [API_SPEC](API_SPEC.md), [SECURITY](SECURITY.md), dan [ADR-007](decisions/ADR-007-mvp-and-operating-policies.md).
