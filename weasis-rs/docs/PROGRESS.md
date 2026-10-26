# Progres Rewrite Weasis

File ini diperbarui **setiap kali satu langkah selesai**. Sesi baru membaca file ini
untuk tahu harus lanjut dari mana. Rencana lengkap: [`PLAN.md`](PLAN.md).

## Status sekarang

- **Fase:** 0 — Persiapan
- **Langkah terakhir selesai:** 0.8 (`FEATURES.md`) + dokumen pendukung
- **Langkah berikutnya:** 0.1 Buat workspace `weasis-rs/` — **tunggu konfirmasi pemilik untuk mulai coding**
- **Ditunda (D12):** 0.5 golden dari aplikasi Java, uji integrasi PACS
- **Hambatan:** tidak ada

## Lingkungan

| Alat | Versi |
|---|---|
| cargo / rustc | 1.97.0 |
| JDK / Maven | 21.0.12 / 3.9.11 (Weasis master butuh JDK 25+) |
| Trunk | belum dipasang (langkah 0.2) |
| target wasm32 | belum dipasang (langkah 0.2) |

## Hasil uji codec WASM (langkah 0.4)

| Transfer syntax | Di WASM | Catatan |
|---|---|---|
| (diisi di langkah 0.4) | | |

## Sub-langkah sedang dikerjakan

(Sebelum memulai fase, tulis `x.y.a`, `x.y.b`, … di sini dengan checkbox.)

## Log sesi

| Tanggal | Langkah | Catatan |
|---|---|---|
| 2026-10-26 | — | Rencana dibuat berdasarkan Weasis 1.21 (branch 1.2.x) |
| 2026-10-26 | — | Rencana ditulis ulang: acuan pindah ke `master` (4.8.0-SNAPSHOT, `32fef3339`), WADO & semua modul masuk scope, branch kerja dibuat ulang dari `master` |
| 2026-10-26 | 0.8 | Dokumen pendukung: `FEATURES.md`, `CODING-GUIDE.md`; acuan `weasis-dicom-tools` (`73a359e`); D12 (uji coba ditunda), D13 (standar kode) |
