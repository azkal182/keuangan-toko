# ADR-006 — Tailwind CSS dan shadcn/ui untuk Styling Web

**Status: Diterima [DISEPAKATI]** | Tanggal: 23 November 2026

## Konteks

Frontend produk menggunakan React + TypeScript (ADR-002), tetapi pilihan styling/component system masih berupa usulan. Pengguna secara eksplisit meminta Tailwind CSS dan shadcn/ui dikunci dengan versi resmi terbaru.

## Keputusan

- Gunakan **Tailwind CSS** sebagai sistem utility CSS untuk web.
- Gunakan **shadcn/ui** sebagai sumber komponen UI React yang dapat dikustomisasi di dalam aplikasi.
- Pada saat bootstrap frontend, pilih rilis stabil resmi terbaru, tanpa prerelease/canary. Rekam versi yang benar-benar terpasang di manifest dan lockfile repo; pembaruan versi berikutnya dilakukan secara terkontrol.
- Ikuti panduan instalasi resmi shadcn/ui dan Tailwind CSS untuk React + Vite; gunakan pnpm workspace dan simpan versi terpasang pada manifest serta lockfile.

Pada pemeriksaan dokumentasi resmi 23 November 2026, Tailwind CSS v4.3 merupakan rilis terbaru yang ditampilkan pada kanal rilis resmi. Dokumentasi shadcn/ui saat ini menggunakan CLI `shadcn@latest`; versi dan hasil komponen akan dikunci melalui artefak package manager ketika frontend dibuat, bukan dengan menebak versi sekarang.

## Konsekuensi

- Komponen yang dipakai perlu diperiksa dan disesuaikan dengan kebutuhan UI; penambahan komponen bukan pengganti review aksesibilitas dan perilaku.
- Vite dan pnpm workspace dipilih di ADR-007. React version, tema, dan library form/state ditetapkan saat scaffold sesuai rilis stabil kompatibel dan dicatat pada manifest/lockfile.
- Kontrak visual, token warna/typography, dan komponen dasar perlu dikembangkan bersama vertical slice UI.

## Referensi resmi

- [Tailwind CSS — Latest Updates](https://tailwindcss.com/blog)
- [Tailwind CSS — Install with Vite](https://tailwindcss.com/docs/installation/using-vite)
- [shadcn/ui — Vite Installation](https://ui.shadcn.com/docs/installation/vite)
- [shadcn/ui — Official Website and Source](https://ui.shadcn.com/docs/official)
