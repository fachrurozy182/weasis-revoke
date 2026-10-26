# Progres Rewrite Weasis

File ini diperbarui **setiap kali satu langkah selesai**. Sesi baru membaca file ini
untuk tahu harus lanjut dari mana. Rencana lengkap: [`PLAN.md`](PLAN.md).

## Status sekarang

- **Fase:** 0 — Persiapan
- **Langkah terakhir selesai:** 0.4 Uji codec di WASM (keputusan D14)
- **Langkah berikutnya:** 0.6 Halaman Leptos "hello" disajikan axum (0.5 ditunda, D12)
- **Ditunda (D12):** 0.5 golden dari aplikasi Java, uji integrasi PACS
- **Celah diketahui:** belum ada contoh JPEG Lossless `.4.57` dan HEVC `.4.107` (lihat `testdata/README.md`)
- **Hambatan:** tidak ada

## Lingkungan

| Alat | Versi |
|---|---|
| cargo / rustc | 1.97.0 |
| JDK / Maven | 21.0.12 / 3.9.11 (Weasis master butuh JDK 25+) |
| Trunk | 0.22.1 (`cargo install trunk --locked`, ±9 menit) |
| target wasm32 | `wasm32-unknown-unknown` (otomatis dari `rust-toolchain.toml`) |
| wasm-bindgen-cli | 0.2.129 (harus sama dengan versi `wasm-bindgen` di `Cargo.lock`) |
| Node.js | 22.22 (menjalankan test WASM) |

## Hasil uji codec WASM (langkah 0.4)

Test: `crates/weasis-core/tests/codec_support.rs`, lulus native dan `--target wasm32-unknown-unknown`.
Akurasi dibandingkan dengan pydicom + codec C (libjpeg, CharLS, OpenJPEG) pada frame pertama.

| Transfer syntax | Crate | Di WASM | Akurasi vs codec C |
|---|---|---|---|
| Implicit/Explicit LE, Explicit BE | `dicom-pixeldata` | ya | — (tanpa kompresi) |
| Deflated Explicit LE, Deflated Image Frame | `dicom-pixeldata` | ya | — |
| RLE Lossless | `dicom-pixeldata` | ya | — |
| JPEG Baseline `.4.50` | `oxiarc-jpeg` | ya | identik |
| JPEG Extended 12-bit `.4.51` | `oxiarc-jpeg` | ya | selisih ≤ 1 pada 1,4% sampel (pembulatan IDCT) |
| JPEG Lossless `.4.70` | `oxiarc-jpeg` | ya | identik (3 file) |
| JPEG Lossless `.4.57` | `oxiarc-jpeg` | belum diuji | tidak ada sampel |
| JPEG-LS `.4.80`/`.4.81` 1 komponen | `pure_jpegls` | ya | identik (3 file, termasuk 15-bit bertanda & palette) |
| JPEG-LS 3 komponen (RGB) | — | **tidak** | `pure_jpegls` menolak Nf ≠ 1 → server (D14) |
| JPEG 2000 `.4.90` | `oxideav-jpeg2000` | ya | identik |
| JPEG 2000 `.4.91` | `oxideav-jpeg2000` | ya | selisih ≤ 1 (lossy), identik untuk multiframe 12-bit |
| HTJ2K `.4.201` | `oxideav-jpeg2000` | ya | identik |
| HTJ2K `.4.203` | `oxideav-jpeg2000` | ya | selisih ≤ 1 (lossy) |
| Video MPEG-2/H.264/HEVC | — | — | bukan decode frame; lihat D14 |

Crate yang ditolak: `openjp2` (bawaan dicom-rs, panic *undefined behavior* di debug),
`hayro-jpeg2000` (tidak mendukung HTJ2K, nilai lossy tidak di-clamp), `jpeg-decoder`
(tidak mendukung JPEG 12-bit).
Catatan: `pydicom/JPEG-lossy.dcm` rusak (codec C pun gagal), tidak dipakai.

## Sub-langkah sedang dikerjakan

(Sebelum memulai fase, tulis `x.y.a`, `x.y.b`, … di sini dengan checkbox.)

## Log sesi

| Tanggal | Langkah | Catatan |
|---|---|---|
| 2026-10-26 | — | Rencana dibuat berdasarkan Weasis 1.21 (branch 1.2.x) |
| 2026-10-26 | — | Rencana ditulis ulang: acuan pindah ke `master` (4.8.0-SNAPSHOT, `32fef3339`), WADO & semua modul masuk scope, branch kerja dibuat ulang dari `master` |
| 2026-10-26 | 0.8 | Dokumen pendukung: `FEATURES.md`, `CODING-GUIDE.md`; acuan `weasis-dicom-tools` (`73a359e`); D12 (uji coba ditunda), D13 (standar kode) |
| 2026-10-26 | — | Acuan `weasis-core-img` (`10f88e4`, versi 5.0.0.1) ditambahkan; Q2 selesai |
| 2026-10-26 | 0.1 | Workspace 4 crate (`weasis-core`, `weasis-net`, `weasis-web`, `weasis-server`), toolchain 1.97.0 dikunci, lint sesuai CODING-GUIDE |
| 2026-10-26 | 0.2 | Trunk 0.22.1 & target wasm32 terpasang |
| 2026-10-26 | 0.3 | `testdata/` 9 MB: pydicom + pydicom-data (MIT), weasis-dicom-tools, sintetis dari `tools/testdata/generate.py` |
| 2026-10-26 | 0.4 | Codec murni Rust lulus di WASM (D14); JPEG-LS RGB → server |
