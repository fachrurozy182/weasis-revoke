# Progres Rewrite Weasis

File ini diperbarui **setiap kali satu langkah selesai**. Sesi baru membaca file ini
untuk tahu harus lanjut dari mana. Rencana lengkap: [`PLAN.md`](PLAN.md).

## Status sekarang

- **Fase:** 2 — Kerangka UI & explorer (Fase 0 & 1 selesai; 0.5 ditunda, D12)
- **Langkah terakhir selesai:** 2.5.d Filter seri, merge & pemisahan fase
- **Langkah berikutnya:** 2.5.e Masking identitas pada label explorer
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
| Playwright (`@playwright/test`) | 1.63.0 (sama dengan Chromium terpasang) |

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
- [x] **1.13.a** PDF terenkapsulasi & video (keputusan transcode D14) — `DicomEncapDocElement`, `DicomVideoElement`
- [x] **1.13.b** Region ultrasound — `Ultrasound` (`SpecialElementRegion` = region SEG/RT, dipindah ke 6.5)
- [x] **1.14.a** Statistik & histogram — `ImageStatistics`, `ValueHistogram`, `ImageAnalyzer`
- [x] **1.14.b** Unit & besaran — `Unit`, `QuantityKind`
- [x] **1.15.a** Profil masking identitas — `Identity-Masking.md`, `identityMasking*.json`
- [x] **1.15.b** Redaksi piksel — `Redaction`, `KarnakMasks`, `PixelPrivacy`

### Fase 2 — Kerangka UI & explorer (`weasis-web`)

Verifikasi tiap sub-langkah UI: unit test + e2e Playwright (Chromium) dengan screenshot.

- [x] **2.1.a** Tema: `Weasis.theme.json` (format IntelliJ/FlatLaf) → variabel CSS, font & metrik FlatLaf — `FlatWeasisTheme`, `FlatLaf.properties`
- [x] **2.1.b** Komponen dasar: tombol, toggle, toolbar, dropdown, slider, panel collapsible, dialog + harness e2e
- [x] **2.2.a** Jendela utama: menu bar (File, View, Display, Tools, Help) & tata letak — `WeasisWin`
- [x] **2.2.b** Panel dockable kiri/kanan/bawah: sembunyi, geser, resize, tab (pengganti DockingFrames)
- [x] **2.2.c** Tab viewer & indikator di menu bar (Java tidak punya status bar: `getStatusBar()` tidak pernah dipakai `WeasisWin`)
- [x] **2.2.d** Grup tab viewer: split kanan/bawah, pindah ke grup lain (`TabPlacement`)
- [x] **2.3.a** Registry shortcut: `KeyBinding` (kode AWT), konteks, konflik, preferensi per OS, 77 default (`ShortcutManager`)
- [x] **2.3.b** `ShortcutTable`, konversi event keyboard browser, preferensi di browser, halaman Help ▸ Shortcuts
- [x] **2.3.c** Shortcut docking: maximize/normalize, close, tab berikut/sebelumnya, daftar panel
- [x] **2.4.a** Import file lokal: worker `import_worker`, File ▸ Open ▸ DICOM, tugas loading, laporan error/tidak didukung
- [x] **2.4.b** Dialog File ▸ Import ▸ DICOM (halaman Local Device: file, folder, rekursif) & drag and drop ke jendela
- [x] **2.4.c** DICOMDIR & ZIP (password) di worker; halaman DICOMDIR di dialog Import
- [x] **2.5.a** Explorer: pemilih pasien (combo box yang bisa dicari), panel studi & seri terurut (`DicomSorter`)
- [x] **2.5.b** Seleksi seri (klik, Ctrl, Shift), shortcut explorer, menu konteks
- [x] **2.5.c** Thumbnail seri (decode di worker), Rebuild Thumbnail; ikon DICOMDIR tidak dipakai (NOTE)
- [x] **2.5.d** Filter seri (`SeriesFilter`), merge bagian split, pemisahan fase 4D
- [ ] **2.5.e** Masking identitas pada label explorer
- [ ] **2.3.a** Model shortcut: registry, konteks, `KeyBinding` — `ShortcutManager`, `Shortcut-Management.md`
- [ ] **2.3.b** Dispatch keyboard & akselerator menu di web
- [ ] **2.4.a** Pipa impor di Web Worker: file & folder, progress — `LoadLocalDicom`, `LoadingTaskPanel`
- [ ] **2.4.b** DICOMDIR, ZIP (password), drag & drop — `DicomDirImport`, `LocalImport`
- [ ] **2.5.a** Explorer: pasien/studi/seri — `DicomExplorer`, `PatientPane`, `StudyPane`, `SeriesPane`
- [ ] **2.5.b** Thumbnail seri (termasuk ikon DICOMDIR)
- [ ] **2.5.c** Filter, split series, pilih pasien — `SeriesFilter`, `SplitSeriesManager`
- [ ] **2.6.a** Buka seri ke viewer — `PluginOpeningStrategy`
- [ ] **2.6.b** Hanging protocol — `HangingProtocols`
- [ ] **2.7.a** Tag view — `DicomFieldsView`
- [ ] **2.7.b** Pencarian tag — `TagSearchTablePanel`
- [ ] **2.8.a** Explorer non-DICOM — `weasis-base-explorer`
- [ ] **2.8.b** Viewer non-DICOM — `weasis-base-viewer2d`
- [ ] **2.9.a** i18n: `messages*.properties` + `weasis-i18n-dist`
- [ ] **2.9.b** Pemilih bahasa — `JLocaleLanguage`
- [ ] **2.10.a** About, lisensi, monitor sumber daya — `WeasisAboutBox`, `LicencesDialog`, `ResourceMonitorDialog`

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
| 2026-10-26 | 1.13.a | `dicom::{EncapsulatedDocument, VideoStream, VideoPlayback}`: dokumen (+ Encapsulated Document Length), fragmen video digabung (NOTE: Java hanya fragmen terakhir); D14 dipastikan: H.264 dalam MP4 diputar browser, MPEG-2/HEVC/H.264 mentah ditranskode server |
| 2026-10-26 | 1.13.b | `dicom::{UltrasoundRegion, ultrasound_regions, unique_spatial_calibration}`: region & kalibrasi cm tunggal seperti Java (tidak dipakai bila tidak persegi); data uji sintetis `us-regions.dcm`; `SpecialElementRegion` ternyata untuk SEG/RT → 6.5 |
| 2026-10-26 | 1.14.a | `image::{Histogram, RegionStatistics, region_statistics}`: min/max/mean/SD (populasi) langsung dari piksel, median/skew/kurtosis/entropi dari histogram nilai stored (1 bin per nilai untuk integer, 1024 untuk float), dalam unit modalitas; rebin seperti `ValueHistogram` |
| 2026-10-26 | 1.14.b | `image::{Unit, UnitSystem, QuantityKind}` + `dicom::PixelCalibration` (Pixel Spacing → Imager Pixel Spacing + magnifikasi → Nominal Scanned → kalibrasi US → aspect ratio), sama dengan `DicomImageElement.initPixelConfiguration` |
| 2026-10-26 | 1.15.a | `model::{MaskingModel, MaskingProfile, IdentityMask, MaskedValue, TagCategory, AnonymizationAction}`: `identityMasking*.json` dipakai ulang, layer + `extends` + lock, profil yang tidak menyembunyikan identitas ditolak; pseudonim SHA-256 (crate `sha2`), geser tanggal 30–329 hari dari salt (sama dengan `Arrays.hashCode`); `TagView::masked_text` (REMOVE → kandidat berikutnya, CLEAR → kosong). Salt dari pemanggil (crypto browser/server) |
| 2026-10-26 | 1.15.b | `image::{PixelPrivacy, RedactionMask, RedactionShape}` + `model::{PixelMask, DeviceKey, MaskRegion, best_pixel_mask}` (blok `masks` di dokumen masking): mask gambar > seri > pustaka perangkat (region yang kategorinya disimpan profil tidak dibakar), isi region = median cincin 4 piksel dengan kernel elips 9×9 OpenCV; konversi Karnak → 7.6. **Fase 1 selesai** |
| 2026-10-26 | 2.1.a | `weasis-web::theme::Theme`: `Weasis.theme.json` (IntelliJ/FlatLaf, dipakai ulang di `assets/config/`) → 1300+ variabel CSS (`--color-*`, `--ui-*` dari `*`, `--Komponen-kunci`, insets → padding CSS); `style/weasis.css` (font FlatLaf, metrik); dicek di Chromium: latar `#2D2A2E`, teks `#FCFCFA` |
| 2026-10-26 | 2.1.b | `weasis-web::widgets::{Button, ToggleButton, Toolbar, DropDownButton, Slider, CollapsiblePanel, Dialog}` + `style/widgets.css` (meniru `JButton`/FlatLaf, `WtoolBar`, `DropDownButton`, `JSliderW`, `CollapsiblePanel`); halaman `#gallery`; harness e2e `tests/e2e` (Playwright 1.63.0) 5 test lulus + screenshot; e2e ditambahkan ke CI |
| 2026-10-26 | 2.2.a | `weasis-web::shell::{MainWindow, MenuBar}` + `menus.rs`: menu File/View/Tools/Help dari `WeasisWin` (klik membuka, hover berpindah, Escape/klik luar menutup, submenu hover, item centang Anonymize); `i18n::tr` membaca `messages.properties` Java (base-ui, core) — bahasa lain di 2.9; Exit/cek update/folder log tidak ada di web (NOTE); e2e 4 test + screenshot. Bug backdrop menutupi menu bar ditemukan e2e & diperbaiki |
| 2026-10-26 | 2.2.b | `weasis-web::shell::{dock_layout::DockLayout, dock::{Docking, DockPanel, DockStrip, DropZones}}`: panel kiri/kanan/bawah dengan tab, minimize ke strip samping (klik → dock lagi, bukan flyout DockingFrames — NOTE), close, splitter pointer (160–900 px), tab diseret ke zona drop untuk pindah sisi; tata letak disimpan di `localStorage` (D8); View ▸ Tools / Explorer berisi item centang (`buildSubMenu`); placeholder DICOM Explorer, Image Tools, Display (isi di 2.4/3.x); bundle i18n dicom-explorer & dicom-viewer2d; `web-sys` (DataTransfer, Storage); e2e 5 test + screenshot |
| 2026-10-26 | 2.2.c | `weasis-web::shell::{Viewers, ViewerArea, Viewer}` + `tab_stack::TabStack`: tab viewer (konten tab tersembunyi dipertahankan), tutup → tab yang terakhir dipilih (riwayat fokus DockingFrames), menu konteks Close Others / Close All; `Viewers` lewat context Leptos untuk explorer (2.4/2.5); `MaskingIndicator` di ujung menu bar + judul jendela "Weasis v… — Anonymized display" seperti `updateTitle`; tombol capture → 7.2. Status bar di PLAN ternyata tidak ada di Java → diganti indikator. Demo viewer di `#gallery`; e2e 4 test baru (total 18) |
| 2026-10-26 | 2.2.d | `weasis-web::shell::working_area::WorkingArea` (pohon split dari grup `TabStack`, rasio 0.1–0.9, grup kosong digantikan saudaranya) + `viewer_menu::TabMenu`: Split Right / Split Down (hanya bila grup punya >1 tab), Move to Other Group (nama = judul viewer unik, dipotong 60 karakter), Close Others / Close All per grup seperti `CloseOthersAction`; divider split bisa digeser; tab baru dibuka di grup yang fokus. Viewer yang pindah grup dibangun ulang (NOTE). e2e 2 test baru (total 20). **Langkah 2.2 selesai** |
| 2026-10-26 | 2.3.a | `weasis_core::gui::{KeyBinding, ShortcutManager, ShortcutContext, ShortcutEntry, id}`: kode tombol & modifier AWT (extended, mask lama dinormalisasi) supaya preferensi sama dengan Java; teks "Ctrl+NumPad +" seperti `getKeyText`; `key_code_from_browser` (huruf/angka ikut layout, lainnya posisi `code`, numpad dibedakan); konteks pohon & konflik; preferensi `<id>.keyCode/.modifier` hanya yang berubah, node per OS, plugin yang mendaftar belakangan ikut preferensi; 77 default dari `registerViewerShortcuts` (modifier menu Cmd/Ctrl). Shortcut fitur ActionW → 3.7 |
| 2026-10-26 | 2.3.b–c | `weasis_core::gui::ShortcutTable` (id → nilai, yang pertama cocok menang); `weasis-web::shortcuts::{Shortcuts, event_binding}` (registry di context, preferensi node per OS di `localStorage`, deteksi macOS); Help ▸ Shortcuts = halaman HTML `generateHtmlPage` (CSS sama, bagian tetap mouse/preset) dibuka di tab baru lewat Blob URL; shortcut docking di seluruh halaman: Ctrl+M maximize (panel & grup lain disembunyikan), normalize, close, tab berikut/sebelumnya, Ctrl+Shift+E daftar panel (daftar klik, bukan siklus DockingFrames — NOTE); Ctrl+W/N/Tab dipakai browser & externalize tidak ada (NOTE); `MainWindow` dipindah ke `shell/main_window.rs` (mod.rs hanya deklarasi, CODING-GUIDE §2); helper `storage`. Akselerator menu dari registry menunggu item menu yang punya shortcut (3.x). e2e 3 test baru (total 23). **Langkah 2.3 selesai** |
| 2026-10-26 | 2.4.a | `weasis_core::explorer::{classify, read_file, ImportEvent, ImportedFile}` (DICOM dari ekstensi `dcm/dic/dicm/dicom` atau "DICM" di 128, ZIP dari ekstensi/`PK\3\4`, DICOMDIR & sidecar `.xml` dipisah; DICOMDIR/tidak dikenal = unsupported); serde untuk `NewInstance`/`Attributes` (tag `GGGGEEEE`); bin `import_worker` (Trunk `data-type="worker"`, `filehash = false` agar nama loader tetap) membaca `File` dengan `FileReaderSync` dan mengirim `Ready`/`Read`/`Finished` (JSON); `weasis-web::explorer::{Importer, DicomExplorer, ImportReport}`: satu permintaan per kali, `LoadingTaskPanel` (stop = terminate worker), worker yang mati → sisa file dihitung error, laporan akhir seperti `LoadDicom.done` (opsi "jangan tampilkan lagi" di `localStorage`); daftar pasien sementara (explorer asli di 2.5). Ditemukan e2e: pesan sebelum worker siap hilang → handshake `Ready`; byte bukan DICOM membuat dicom-rs mengalokasikan GB (detik di WASM) → `element_lengths_fit` sebelum parse tanpa preamble + test WASM `malformed_input`. Peringatan reaktif di menu View diperbaiki. e2e 2 test baru (total 25) |
| 2026-10-26 | 2.4.b | `weasis-web::explorer::{ImportDialog, dropped_files}`: File ▸ Import ▸ DICOM membuka dialog `DicomImport` (daftar halaman kiri, tombol ?/Import/Import and Close/Close) dengan halaman `LocalImport` (field file & folder + "Search recursively" disimpan `last.recursive.mode`); browser tidak memberi path → field menampilkan nama, satu folder per kali (NOTE); folder = input `webkitdirectory`, non-rekursif disaring dari `webkitRelativePath`; file/folder yang di-drop ke jendela → explorer DICOM, folder dibaca rekursif lewat `webkitGetAsEntry` (fallback `getAsFile`), drag tab dock tidak terpengaruh; `help::online_help_url` dipindah ke modul sendiri. e2e 2 test baru (total 27), termasuk laporan "50 file dilewati" + "jangan tampilkan lagi" |
| 2026-10-27 | 2.4.c | `weasis_core::explorer::{read_archive, dicomdir_files, dicomdir_selection, ImportRequest}`: ZIP dibaca di worker (lewat DICOMDIR di root bila ada, kalau tidak semua entri DICOM, ZIP di dalam ZIP ikut); arsip terenkripsi → `password_required` → dialog "DICOM Zip" (Cancel = dilewati); halaman DICOMDIR di dialog Import (folder yang berisi DICOMDIR; deteksi CD-ROM & cache tidak relevan di browser — NOTE), hanya file yang dirujuk dibaca (bukan membangun seri dari record — NOTE, hasil model sama); DICOMDIR tidak ada/kosong → tanya "import manual?" → folder diimpor langsung (NOTE). Permintaan worker = header JSON `ImportRequest` + `File`s (path dikirim eksplisit karena `webkitRelativePath` hilang di structured clone). Data uji `synthetic/ct-series-encrypted.zip` (password `weasis`, ZipCrypto, dibuat `generate.py` dengan CLI `zip`). e2e 2 test baru (total 29). **Langkah 2.4 selesai** |
| 2026-10-27 | 2.5.a | `weasis_core::model::{compare_patients, compare_studies, compare_series, SortingTime}` (port `DicomSorter`: studi terbaru dulu, seri per nomor/bagian split/tanggal/deskripsi/instance pertama, SR & dokumen di akhir; nama dibandingkan tanpa huruf besar-kecil, bukan collator bahasa — NOTE) + `{patient_label, study_label, series_label}` (TagView node `DicomModel`, "?" bila kosong; `TagView::of`); tag instance `DateOfSecondaryCapture`/`TimeOfSecondaryCapture`/`StackID` ditambahkan untuk urutan; widget `SearchableComboBox`; explorer: pemilih pasien (pasien pertama otomatis), panel studi (judul) dengan seri (kotak thumbnail 144 px + deskripsi), seri PR/KO/SEG/RT disembunyikan. e2e diperbarui + 1 test baru (total 30) |
| 2026-10-27 | 2.5.b | `weasis_core::explorer::{SeriesSelection, related_series}` (port `SeriesSelectionModel`: klik, Ctrl toggle, Shift interval dari anchor, Ctrl+Shift tambah/hapus interval menurut anchor, next/prev berputar, first/last/all; seri dengan Frame of Reference sama, opsional orientasi sama dari gambar tengah) + `DicomModel::{study_of_series, patient_of_study}`; explorer: panel seri bisa dipilih (`aria-selected`), shortcut `explorer.*` lewat `ShortcutTable` (Enter → 2.6), klik kanan menambah seri ke seleksi lalu `SeriesMenu` (Select related Series (same axis), Remove selected Series / this Study / this Patient; item lain menunggu 2.5.c/2.5.d/2.6/2.7). e2e 3 test baru (total 33) |
| 2026-10-27 | 2.5.c | `weasis_core::image::{render_default, fit_within, RgbaImage}` (frame dengan tampilan awal: warna apa adanya, grayscale lewat modality LUT + preset pertama `preset_list`, MONOCHROME1 dibalik; diperkecil rata-rata area) + `weasis_core::explorer::{thumbnail_source, MediaPosition, render_thumbnail, archive_entry, ImportedInstance}` (gambar tengah/awal/akhir, frame tengah untuk satu multi-frame; instance mencatat jalur di dalam arsip); worker: `ImportRequest::Thumbnail` → `[ImportEvent::Thumbnail, piksel RGBA]`; `explorer::thumbnails::Thumbnails` (sumber tiap instance = `File` + jalur arsip + password; thumbnail JPEG 80% sebagai data URL seperti cache JPEG Java); setelah import, thumbnail seri baru diminta satu per satu lewat antrean worker; panel seri: gambar, jumlah gambar kiri bawah, `#n` split kanan atas, jenis untuk non-gambar (PDF, Video, …); menu Rebuild Thumbnail (gambar pertama/tengah/terakhir). Bug ditemukan e2e: permintaan pertama terkirim dua kali → thumbnail bergeser satu seri; `enqueue` kini hanya mengantre. Ikon DICOMDIR tidak dipakai karena thumbnail dibuat dari file (NOTE). e2e 2 test baru (total 35) |
| 2026-10-27 | 2.5.d | `weasis_core::explorer::{SeriesFilter, FilterMode}` (port `SeriesFilter`: teks di atribut seri & studi + nomor seri + anatomi Body Part Examined & grup regionnya — nama bagian tubuh, bukan arti kode terjemahan (NOTE) —, satu studi, atau set modalitas "CT, PT"); `DicomModel::{phase_count, separate_phases, merge_series}` (port `calculateSamplingRateFor4d`/`seriesPostProcessing`/`mergeSeries`: 2–7 fase dipisah otomatis setelah import, lebih banyak lewat menu; penomoran split sudah ada dari 1.7 sehingga `SplitSeriesManager` tidak perlu); explorer: `SeriesFilterBar` (tombol mode, field + saran datalist / pilihan studi, "x/y series shown", border aksen), filter disimpan per pasien, seleksi dihapus saat filter berubah; menu seri: Merge selected Series, Split the phases into subseries; pemrosesan pasca-load dipindah ke `explorer/series_processing.rs` (importer < 400 baris). e2e 1 test baru (total 36) |
