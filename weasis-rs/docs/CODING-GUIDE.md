# Standar Penulisan Kode `weasis-rs`

Tujuan utama: kode **mudah dibaca dan mudah diperbaiki sendiri** oleh pemilik repo tanpa
harus membaca seluruh proyek. Setiap aturan di bawah melayani tujuan itu.

## 1. Format & lint (wajib, dicek CI)

- `cargo fmt` dengan `rustfmt.toml` proyek: `max_width = 100`, `edition = "2024"`.
- `cargo clippy --workspace --all-targets -- -D warnings`. Lint tambahan di `Cargo.toml` workspace:
  - `unwrap_used`, `expect_used`, `panic` = **deny** di kode non-test.
  - `missing_docs` = **warn** untuk item `pub`.
  - `unsafe_code` = **forbid** (pengecualian harus dicatat sebagai keputusan di `PLAN.md` §2).
- Tidak ada warning yang dibiarkan; tidak ada `#[allow(...)]` tanpa komentar alasannya.

## 2. Struktur file & modul

- Struktur modul Rust **mengikuti paket Java acuan**, supaya mudah mencari padanannya.
  Contoh: `org.weasis.dicom.codec.geometry.GeometryOfSlice` →
  `crates/weasis-core/src/dicom/geometry/geometry_of_slice.rs`.
- **Satu konsep per file.** Batas lunak: file ≤ 400 baris, fungsi ≤ 50 baris. Kalau lewat,
  pecah menjadi fungsi/modul yang namanya menjelaskan isinya.
- Setiap file diawali komentar modul yang menyebut **apa isinya** dan **file Java acuannya**:

```rust
//! Window/level presets loaded from `windowPresets.json`.
//!
//! Java reference: `weasis-dicom/weasis-dicom-codec/src/main/java/org/weasis/dicom/codec/display/WindowPresetRegistry.java`
```

- `mod.rs` hanya berisi deklarasi modul dan `pub use`, tanpa logika.

## 3. Penamaan

- Ikuti konvensi Rust: `snake_case` fungsi/variabel/modul, `PascalCase` tipe, `SCREAMING_CASE` konstanta.
- Nama tipe utama **sama dengan nama kelas Java** bila konsepnya sama (`GeometryOfSlice`,
  `WindowPreset`), supaya mudah dicocokkan. Prefiks Java yang tidak perlu dibuang (`TagD` → `Tag`
  boleh, asal disebut di komentar modul).
- Nama menjelaskan maksud: `window_center`, bukan `wc`; `pixel_spacing_mm`, bukan `ps`.
  Satuan dimasukkan ke nama bila ada (`_mm`, `_deg`, `_ms`, `_px`).
- Boolean dimulai `is_`, `has_`, `should_`.

## 4. Komentar & dokumentasi

- Setiap item `pub` punya doc comment `///` satu kalimat yang menjelaskan **maksud**, bukan
  mengulang nama.
- Komentar di dalam fungsi menjelaskan **kenapa**, bukan apa. Kode yang perlu komentar "apa"
  sebaiknya ditulis ulang supaya jelas.
- Rumus dan perilaku dari standar DICOM wajib menyebut sumbernya:
  `// DICOM PS3.3 C.11.2.1.2.1: linear VOI function`.
- Penyimpangan sengaja dari perilaku Java atau dari standar diberi komentar `// NOTE:` + alasan.
- Tidak ada `TODO` tanpa nomor langkah: `// TODO(3.12): apply sharpen kernel`.
- Bahasa komentar & doc: **Inggris** (dokumen di `docs/` berbahasa Indonesia).

## 5. Error handling

- Setiap crate punya satu tipe `Error` (`thiserror`) di `error.rs`; fungsi yang bisa gagal
  mengembalikan `Result<T, Error>`.
- Pesan error menjelaskan apa yang gagal dan konteksnya:
  `"cannot decode frame {frame} of {sop_instance_uid}: unsupported transfer syntax {ts}"`.
- Tidak ada `unwrap()`/`expect()` di kode non-test. Gunakan `?`, `ok_or`, atau nilai default
  yang dijelaskan di komentar.
- Di UI, error ditampilkan ke pengguna (notifikasi/status bar) dan dicatat dengan `tracing`, tidak di-panic.

## 6. Desain kode

- Utamakan **fungsi murni** di `weasis-core`: input → output, tanpa state global. Ini membuat
  logika mudah dites dan di-debug.
- Tidak ada state global yang bisa diubah (`static mut`, `lazy_static` mutable). Konfigurasi
  dimuat sekali dan dioper sebagai nilai.
- Abstraksi (trait, generic) hanya dibuat jika sudah ada **≥ 2 pemakai nyata**.
- Hindari struct raksasa: kelompokkan field yang berhubungan ke struct kecil
  (`WindowLevel { center, width }`, bukan dua field lepas di mana-mana).
- Gunakan tipe untuk mencegah salah pakai: `enum` untuk pilihan tetap, newtype untuk satuan
  bila tertukar berbahaya (`Millimeters(f64)`).

## 7. Frontend (Leptos)

- Satu komponen per file di `weasis-web/src/<area>/`, nama file = nama komponen dalam snake_case.
- Komponen hanya mengurus tampilan & event; logika ada di `weasis-core` atau modul `state` terpisah.
- State bersama disimpan di satu struct context per area (`ViewerState`, `ExplorerState`),
  bukan sinyal yang tersebar.
- Kerja berat (parse, decode, MPR) **selalu di Web Worker**, tidak di thread UI.
- CSS memakai variabel dari tema (`--weasis-*`, diturunkan dari `Weasis.theme.json`), tanpa
  warna yang ditulis langsung di komponen.
- Teks yang tampil ke pengguna selalu lewat i18n (`t!("key")`), tidak ditulis langsung.

## 8. Backend (axum)

- Satu file per kelompok route di `weasis-server/src/routes/`; handler tipis, logika di `weasis-net`.
- Kredensial dari konfigurasi/variabel lingkungan; tidak pernah di-log dan tidak dikirim ke browser.

## 9. Test

- Unit test di file yang sama (`#[cfg(test)] mod tests`) untuk logika kecil; test integrasi di `tests/`.
- Nama test menjelaskan perilaku: `window_level_clamps_output_to_8_bit_range`.
- File uji dibaca dari `weasis-rs/testdata/`, tidak dari path absolut.
- Uji coba terhadap aplikasi Java dan server PACS **hanya dijalankan atas instruksi pemilik repo** (D12).

## 10. Dependensi

- Crate baru hanya ditambah jika benar-benar dipakai; versinya dikunci di `Cargo.lock`.
- Dependensi dideklarasikan sekali di `[workspace.dependencies]` dan dipakai dengan `workspace = true`.

## 11. Commit

- Satu langkah rencana = satu atau beberapa commit kecil dengan prefiks `weasis-rs: [x.y] ...`.
- Pesan commit dalam bahasa Inggris, kalimat perintah: `weasis-rs: [1.4] add VOI LUT sigmoid shape`.
