# Progres Rewrite Weasis

File ini diperbarui **setiap kali satu langkah selesai**. Sesi baru membaca file ini
untuk tahu harus lanjut dari mana. Rencana lengkap: [`PLAN.md`](PLAN.md).

## Status sekarang

- **Fase:** 1 — Inti DICOM (Fase 0 selesai; 0.5 ditunda, D12)
- **Langkah terakhir selesai:** 1.12.a Info pojok
- **Langkah berikutnya:** 1.13.a PDF terenkapsulasi & video
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
| Leptos / axum / tokio | 0.8.22 / 0.8.9 / 1.54.1 |

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

### Fase 1 — Inti DICOM (`weasis-core`)

Modul Rust di `crates/weasis-core/src/`: `model/` (1.1), `dicom/` (1.2–1.13), `image/` (1.3–1.6, 1.14).
Sub-langkah boleh dipecah lagi bila saat dikerjakan ternyata besar.

- [x] **1.1.a** `model::Level` (Patient/Study/Series/Instance/Frame) + daftar tag per level — `TagD.Level`, `DicomMediaIO` (blok `static`)
- [x] **1.1.b** Identitas pasien (`pseudo_uid`, normalisasi nama PS3.5 §6.2.1) — `PatientComparator`
- [x] **1.1.c** Pohon Patient → Study → Series → Instance: tambah, cari, hapus, gabung pasien/studi — `DicomModel`, `MediaSeriesGroupNode`, `TreeModelNode`
- [x] **1.2.a** Baca file/bytes → objek DICOM (preamble otomatis, file tanpa meta, error jelas) — `DicomMediaIO.readMetaData`
- [x] **1.2.b** Ekstrak tag tiap level ke model + atribut instance (SOP, instance number, TS, concatenation) — `DicomMediaIO.writeInstanceTags`
- [x] **1.2.c** Charset (Specific Character Set) — `CharsetEncoding`
- [x] **1.2.d** Jenis objek (gambar, video, PR, KO, SEG, SR, dokumen, waveform, tidak didukung) dari SOP Class — `DicomMediaIO` (MIME types), `DicomSpecialElementFactory`
- [x] **1.3.a** Ekstraksi bytes per frame dari pixel data terenkapsulasi (offset table, multi-fragmen) — `ExtendSegmentedInputImageStream`
- [x] **1.3.b** Modul codec: dispatch per transfer syntax (D14), buffer piksel bertipe (u8/u16/i16/i32/f32/RGB) — `DicomImageReader`
- [x] **1.3.c** Fotometri: MONOCHROME1/2, RGB, YBR_FULL(_422), palette color, planar configuration — `PaletteColorUtils`, `ImageConversion`
- [x] **1.3.d** Pindahkan `tests/codec_support.rs` ke modul codec
- [x] **1.4.a** Modality LUT (rescale slope/intercept, Modality LUT Sequence), pixel padding — `ModalityLutModule`, `RescaleUtils`
- [x] **1.4.b** VOI: W/L linear, linear exact, sigmoid; VOI LUT Sequence; invert; MONOCHROME1 — `VoiLutModule`, `LutShape`, `WlParams`, `ImageRendering`
- [x] **1.5.a** Anatomi gambar (body part, region group, selector) — `weasis-dicom-tools` `ref/*`, `regionGroups.json`
- [x] **1.5.b** Parse `windowPresets.json` + daftar preset per gambar (urutan, layer, percent) — `WindowPresetRegistry`, `WindowPreset`, `PresetWindowLevel`, `Window-Presets.md`
- [x] **1.6.a** Parse `colormaps.json` + sampling color map — `ColorMapRegistry`, `ColorMapCompiler`, `ColorMap-Design.md`
- [x] **1.7.a** Urutan stack — `SortSeriesStack`
- [x] **1.7.b** Aturan pemecahan seri (subseries) — `SplittingRules`, `SplittingModalityRules`
- [x] **1.8.a** Orientasi & label (A/P/L/R/H/F) — `ImageOrientation`, `PatientOrientation`, `geom/*`
- [x] **1.8.b** Geometri irisan & perpotongan (lokalizer) — `GeometryOfSlice`, `SlicePlaneGeometry`, `IntersectSlice`, `IntersectVolume`, `LocalizerPoster`
- [x] **1.9.a** Overlay 60xx & embedded — `OverlayOp`, `OverlayData`, `EmbeddedOverlay`
- [x] **1.9.b** Shutter (rectangular, circular, polygonal, bitmap) — `ShutterOp`
- [x] **1.10.a** DICOMDIR — `DicomDirLoader`
- [x] **1.10.b** ZIP — `DicomZipCodec`
- [x] **1.11.a1** Presentation State: tipe, referensi, VOI, Modality/Presentation LUT, overlay, shutter, blending & palette — `PrDicomObject`, `PresentationStateReader`
- [x] **1.11.a2** Presentation State: anotasi grafik & teks, layer, displayed area, rotasi/flip — `PresentationStateReader`, `PrGraphicUtil`
- [x] **1.11.b** Key Object, rejected KO, hidden series — `KOSpecialElement`, `RejectedKOSpecialElement`, `HiddenSeriesManager`
- [x] **1.12.a** Info pojok per modalitas — `ModalityView`, `ModalityViewJson`, `CornerInfoData`
- [ ] **1.13.a** PDF terenkapsulasi & video (keputusan transcode D14) — `DicomEncapDocElement`, `DicomVideoElement`
- [ ] **1.13.b** Region ultrasound — `Ultrasound`, `SpecialElementRegion`
- [ ] **1.14.a** Statistik & histogram — `ImageStatistics`, `ValueHistogram`, `ImageAnalyzer`
- [ ] **1.14.b** Unit & besaran — `Unit`, `QuantityKind`
- [ ] **1.15.a** Profil masking identitas — `Identity-Masking.md`, `identityMasking*.json`
- [ ] **1.15.b** Redaksi piksel — `Redaction`, `KarnakMasks`, `PixelPrivacy`

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
| 2026-10-26 | 0.6 | Leptos CSR (`weasis-web`) dibangun Trunk, disajikan `weasis-server` (axum, fallback ke `index.html`); diverifikasi Playwright tanpa error console |
| 2026-10-26 | 0.7 | Workflow `.github/workflows/weasis-rs.yml` dibuat (action dikunci SHA), run pertama berjalan |
| 2026-10-26 | 1.1 | `weasis_core::model`: `Level`, `PatientIdentity`, `Attributes`, `DicomModel` (tambah/cari/hapus/gabung), 15 unit test |
| 2026-10-26 | 1.2.a | `weasis_core::dicom::read_dicom` (preamble otomatis, tanpa file meta: tebak Implicit/Explicit LE), `weasis_core::error::Error` |
| 2026-10-26 | 1.2.b | `weasis_core::dicom::new_instance`; studi yang sudah ada tetap di pasiennya (seperti `LoadDicom`); UID studi/seri hilang → "UNKNOWN", SOP UID hilang → error |
| 2026-10-26 | 1.2.c | dicom-rs tidak mendukung ISO 2022 (Jepang/Korea/Cina): decoder sendiri `dicom::charset` di atas `encoding_rs`; 13 file charset pydicom lulus (`tests/charset_support.rs`); CI: `concurrency` membatalkan run lama |
| 2026-10-26 | 0.7 | CI run pertama hijau (±10 menit): fmt, clippy native+WASM, test native+WASM, `trunk build --release` |
| 2026-10-26 | 1.2.d | `dicom::object_kind` (13 jenis, aturan `DicomMediaIO.setMimeType`); RTDOSE berpiksel = gambar seperti di Java |
| 2026-10-26 | 1.3.a | `dicom::compressed_frame` (aturan fragmen seperti `DicomImageReader`, tanpa Basic Offset Table); semua frame multiframe J2K (19) & JPEG (30) ter-decode |
| 2026-10-26 | 1.3.b | `dicom::pixel` (`ImageDescriptor`, `Samples`, `decode_frame`); 40 file cocok dengan pydicom+codec C (`tests/pixel_decoding.rs`), JPEG-LS RGB → `NeedsServerDecoding` |
| 2026-10-26 | 1.3.c | `pixel::to_rgb`: planar→interleaved, YBR_FULL/422→RGB, palette (normal & tersegmentasi, 8-bit seperti Java); temuan: RLE dari dicom-pixeldata sudah interleaved, YBR_FULL_422 native & SEG 1-bit terenkapsulasi dibaca sendiri |
| 2026-10-26 | 1.3.d | `tests/codec_support.rs` kini memanggil `decode_frame` + `to_rgb`, lulus native & WASM |
| 2026-10-26 | 1.4.a | `image::{ModalityLut, LookupTable}`, `dicom::frame_macro` (functional groups); hasil = pydicom `apply_modality_lut`; padding US-sebagai-signed ditafsir ulang |
| 2026-10-26 | 1.4.b | `image::{VoiLut, VoiParams, LutShape, render_gray}`: 7 bentuk LUT (rumus Java `LookupTableUtils`), Frame VOI LUT per frame, tabel per nilai stored; cocok dengan pydicom `apply_windowing`/`apply_voi_lut` |
| 2026-10-26 | 1.5.a | `dicom::anatomy` (407 body part dari `BodyPart.java` via `tools/anatomy/generate.py`, 390 kode SRT, `regionGroups.json` di `assets/config/`); SurfacePart/OtherPart menyusul di 6.11 |
| 2026-10-26 | 1.5.b | `image::{WindowPreset, PresetLayers, preset_list}`: JSON + layer (built-in/site/user), percent image/series, anatomi, shortcut & default per layer; urutan = `PresetCollectionBuilder` |
| 2026-10-26 | 1.6.a | `image::{ColorMap, ColorSpace, Rgba}`: 40 map `colormaps.json`, domain relative/absolute/fixed/percent, RGB/HSL/LAB, linear/step/sampled, cyclic, alpha; palette DICOM PS3.6 cocok dengan pydicom. Impor Color Palette IOD menyusul di 2.4 |
| 2026-10-26 | 1.7.a | `model::{SortKey, sort_instances, slice_position}` (6 kriteria Java; nilai hilang di akhir agar urutan total); posisi/orientasi frame pertama disimpan di atribut instance (functional group untuk enhanced) |
| 2026-10-26 | 1.7.b | `model::{SplittingRules, ImagePlane}` + `DicomModel::add_split_instance`: aturan bawaan DEFAULT/CT/PT/MR (dengan kondisi PROJECTION), sub-seri `#n.<uid>` + `split_number`, concatenation tetap satu seri, video/PDF/SR/waveform satu objek per seri; layer site `seriesSplittingRules.json` menyusul di 7.6 |
| 2026-10-26 | 1.8.a | modul `geometry` (D15: `glam`): `ImagePlane` (dipindah dari `model`), `same_orientation`, `PatientAxis` + warna, `Direction` biped/quadruped, `labels_from_vectors` (rotasi & flip seperti `InfoLayer`), `labels_from_patient_orientation`; kode quadruped dua huruf dikenali (Java memotongnya) |
| 2026-10-26 | 1.8.b | `geometry::{SliceGeometry, Localizer, cross_line}`: geometri tampilan (piksel persegi), garis irisan (`IntersectSlice`) & outline slab (`IntersectVolume`), dedup titik terakhir seperti `View2d.getCrossLine`; titik potong dihitung dengan interpolasi eksak (NOTE) |
| 2026-10-26 | 1.9.a | `image::{overlay_mask, overlay_planes, embedded_overlays, presentation_overlay_planes, paint_overlay}`: 16 grup 60xx, origin & frame, overlay embedded dibaca dari bit tinggi data mentah; jumlah piksel cocok dengan pydicom (222, 2278, 680) |
| 2026-10-26 | 1.9.b | `image::{Shutter, bitmap_shutter, shutter_color}`: irisan persegi/lingkaran/poligon, warna CIELab atau P-value, shutter bitmap PR hanya dari grup `Shutter Overlay Group` (NOTE: Java memakai semua overlay PR); data uji sintetis `xa-shutter.dcm`, `pr-bitmap-shutter.dcm` |
| 2026-10-26 | 1.10.a | `dicom::{read_dicomdir, resolve_file_id, DirSeries}`: offset record dipindai dari byte mentah (dicom-rs tidak menyimpannya; cocok dengan pydicom), record nonaktif & PRIVATE dilewati, seri/instance duplikat digabung seperti Java; resolver File ID tanpa filesystem (huruf kecil, ekstensi tunggal, tolak `..`). Ikon seri disimpan, decode di 2.5 |
| 2026-10-26 | 1.10.b | `dicom::{read_zip, root_dicomdir}` dengan crate `zip` 9.1.2 (stored/deflate zlib-rs, AES & ZipCrypto, jalan di WASM); `Error::PasswordRequired` agar UI bisa meminta password seperti dialog Java; entri `..` dilewati |
| 2026-10-26 | 1.11.a1 | `image::PresentationState` (+ `VoiLut::from_item`, `Palette` publik): jenis SOP Class, referensi seri/gambar/frame, VOI per gambar, Modality & Presentation LUT, overlay aktif, shutter, layer blending & palette; 1.11.a dipecah jadi a1/a2. Data sintetis diperbaiki: PR memakai `ReferencedImageSequence`, SEG `ReferencedInstanceSequence` |
| 2026-10-26 | 1.11.a2 | `image::{Annotation, GraphicLayer, GraphicShape, TextObject, DisplayedArea, SpatialTransform, AnnotationSpace}` + `PresentationState::{annotations_for, displayed_area_for}`: titik dalam koordinat gambar; unit DISPLAY dipetakan sesuai PS3.3 (NOTE: Java mencampur urutan skala & rotasi; area 1-based standar). Model XML privat Weasis menyusul di 4.9 |
| 2026-10-26 | 1.11.b | `dicom::{KeyObject, KeyImage, ContentOrder}` (referensi hierarkis, rejection IOCM, add/remove/toggle frame; NOTE: gambar utuh tetap ditandai saat frame ditambah) + `model::HiddenObjects` (KO, rejection, PR per pasien, terbaru dulu) pengganti `HiddenSeriesManager` |
| 2026-10-26 | 1.12.a | `image::{CornerLayout, TagView, Corner, format_value}`: tata letak bawaan 4 pojok × 7 baris, atribut pertama yang ada, format `$V`/`:l$n$`/`:f$pola$` (DecimalFormat, half-even), nama PN, tanggal ISO (NOTE: format lokal di UI); layer site `attributesView.json` → 7.6, masking → 1.15 |
