# Rencana Rewrite Weasis ke Rust (Web)

Dokumen ini adalah **sumber kebenaran** untuk proyek rewrite. Isinya tetap (scope,
keputusan, langkah). Status harian dicatat di [`PROGRESS.md`](PROGRESS.md).

- Dibuat: 2026-10-26
- Branch kerja: `claude/vibrant-bardeen-k1240f` (berbasis `master`)
- Kode acuan: Weasis **4.8.0-SNAPSHOT**, branch `master` commit `32fef3339` (2026-10-16),
  ±233 ribu baris Java dalam 1.147 file. Semua rujukan file di dokumen ini relatif ke root repo.
- Acuan tambahan: repo [`fachrurozy182/weasis-dicom-tools`](https://github.com/fachrurozy182/weasis-dicom-tools)
  branch `master` (commit `73a359e`, 218 file, ±45 ribu baris). Clone baca-saja:
  `git clone --depth 1 https://github.com/fachrurozy182/weasis-dicom-tools ../fachrurozy182/weasis-dicom-tools`
- Acuan tambahan: repo [`fachrurozy182/weasis-core-img`](https://github.com/fachrurozy182/weasis-core-img)
  branch `master` (commit `10f88e4`, 54 file, ±10 ribu baris). Clone baca-saja:
  `git clone --depth 1 https://github.com/fachrurozy182/weasis-core-img ../fachrurozy182/weasis-core-img`
- Dokumen pendukung: [`FEATURES.md`](FEATURES.md) (daftar fitur), [`CODING-GUIDE.md`](CODING-GUIDE.md)
  (standar kode), [`PROGRESS.md`](PROGRESS.md) (status).

---

## 1. Tujuan & scope

**Tujuan:** membuat ulang Weasis sebagai **aplikasi web** dengan Rust (backend + frontend),
dengan **fitur sama** seperti Weasis desktop di `master` dan **tampilan mengikuti Weasis
desktop** (dibaca langsung dari source code Swing-nya).

| Termasuk | Tidak termasuk |
|---|---|
| Semua modul di `weasis-core`, `weasis-base/*`, `weasis-dicom/*`, `weasis-acquire/*` | HL7 |
| Membuka file lokal: file, folder, DICOMDIR, ZIP, drag & drop | weasis-pacs-connector (proyek terpisah, bukan bagian repo ini) |
| WADO-URI + manifest XML/JSON, DICOMweb (QIDO/WADO-RS/STOW), DICOM Q/R & send | Installer native (`weasis-distributions`, jpackage) |
| MPR, fusion PET/CT, segmentasi, SR, RT, waveform, audio, 3D volume rendering | Launcher OSGi/Felix dan sistem plugin runtime |
| Dicomizer (`weasis-acquire`), export ISO, cetak | |

## 2. Keputusan (decision log)

| # | Keputusan | Alasan |
|---|---|---|
| D1 | Kode Rust di folder `weasis-rs/` di repo ini | Kode Java tetap di sebelahnya sebagai acuan; satu branch |
| D2 | Frontend: **Leptos** (CSR, WASM), build dengan **Trunk** | Rust penuh di browser |
| D3 | File lokal di-parse & di-decode **di browser** (WASM, Web Worker) | Tidak perlu upload; privasi data pasien |
| D4 | Library DICOM: **dicom-rs** (`dicom-object`, `dicom-pixeldata`, `dicom-ul`, `dicom-dictionary-std`) | Toolkit DICOM Rust yang paling matang |
| D5 | Backend: **axum**, satu binary yang menyematkan aset web | Deploy sederhana |
| D6 | Render 2D: **Canvas 2D** + LUT di WASM. 3D: **WebGL2**. WebGPU hanya jika WebGL2 tidak cukup | Paling sederhana dulu |
| D7 | Konfigurasi asli dipakai ulang apa adanya (§4.2) | Hasil identik dengan desktop |
| D8 | Preferensi pengguna di `localStorage`; konfigurasi node DICOM & kredensial di backend | Kredensial tidak boleh tersimpan di browser |
| D9 | Semua akses jaringan DICOM lewat **backend**: DIMSE (C-FIND/C-MOVE/C-GET/C-STORE) karena browser tidak bisa TCP mentah; DICOMweb/WADO di-proxy agar bebas CORS & kredensial aman | Batasan browser |
| D10 | Tampilan meniru desktop dari source: `WeasisWin` (menu & tata letak), `Weasis.theme.json` (warna FlatLaf), toolbar & panel dockable masing-masing modul | Permintaan pengguna |
| D11 | Acuan tetap commit `32fef3339`. Perubahan master setelahnya tidak dikejar kecuali diputuskan | Target tidak bergerak |

| D12 | Uji coba terhadap aplikasi Java (golden) dan server PACS **ditunda sampai pemilik repo menginstruksikan**. Selama itu, verifikasi langkah = build, lint, dan test otomatis milik `weasis-rs` (unit test & e2e) | Permintaan pengguna; JDK 25+ belum tersedia |
| D13 | Semua kode mengikuti [`CODING-GUIDE.md`](CODING-GUIDE.md): rapi, mudah dibaca, setiap file menyebut kelas Java acuannya | Pemilik repo akan memperbaiki kode sendiri |
| D14 | Codec di browser, semuanya **murni Rust**: `dicom-pixeldata` (native, deflate, RLE), `oxiarc-jpeg` (JPEG baseline/extended/lossless), `pure_jpegls` (JPEG-LS 1 komponen), `oxideav-jpeg2000` (JPEG 2000 & HTJ2K). JPEG-LS multi-komponen (RGB) di-decode **server**. Video: H.264 diputar langsung oleh browser; MPEG-2 & HEVC di-transcode server (dipastikan di langkah 1.13) | Hasil uji 0.4: semua lulus di WASM dan cocok dengan codec C (lossless identik, lossy selisih ≤ 1). `openjp2` ditolak (memicu *undefined behavior*), `hayro-jpeg2000` ditolak (tanpa HTJ2K, tanpa clamp) |

Keputusan baru ditambahkan di tabel ini (D15, D16, …) beserta alasannya.

## 3. Arsitektur

```
weasis-rs/
├── Cargo.toml              workspace
├── crates/
│   ├── weasis-core/        lib murni Rust (tanpa UI, tanpa jaringan): model, codec,
│   │                       LUT, geometri, MPR, ukur, SR/RT/wave parsing
│   ├── weasis-net/         klien DICOMweb, WADO, manifest, DIMSE (dipakai server)
│   ├── weasis-web/         Leptos CSR → WASM: UI, viewer, explorer, tools
│   └── weasis-server/      axum: aset web, proxy DICOMweb, gateway DIMSE, konfigurasi node
├── assets/config/          salinan konfigurasi asli (§4.2)
├── testdata/               file DICOM uji (kecil, berlisensi bebas)
├── tests/e2e/              Playwright
└── docs/                   PLAN.md, PROGRESS.md, FEATURES.md
```

Alur gambar: `sumber (file lokal / backend) → Worker: parse + decode → buffer piksel mentah
→ Modality LUT → VOI (W/L, bentuk LUT) → color map → ImageData → canvas`, lalu lapisan
grafik/anotasi digambar di atas canvas.

`weasis-core` tidak boleh bergantung pada web API atau jaringan, supaya bisa dites dengan
`cargo test` biasa dan dipakai oleh browser maupun server.

## 4. Peta acuan

### 4.1 Modul Java → Rust

| Java (baris) | Rust | Isi |
|---|---|---|
| `weasis-core` (97 rb) | `weasis-core::{model,image,graphic}` + `weasis-web` | model data, operasi gambar, alat ukur, layer, preferensi, export animasi, shortcut, masking identitas |
| `weasis-dicom-codec` (22 rb) | `weasis-core::dicom` | baca DICOM, PR, KO, SEG, overlay, shutter, geometri, preset, redaksi |
| `weasis-dicom-explorer` (27 rb) | `weasis-web::explorer` + `weasis-net` | explorer, import/export, tag view, WADO/manifest, node DICOM, cetak |
| `weasis-dicom-viewer2d` (31 rb) | `weasis-web::viewer` + `weasis-core::mpr` | viewer 2D, MPR, curved MPR, MIP, fusion, segmentasi, KO, redaksi |
| `weasis-dicom-3d/viewer3d` (15 rb) | `weasis-web::viewer3d` | volume rendering |
| `weasis-dicom-qr` (4 rb), `-send` (0,5 rb) | `weasis-net` + `weasis-web::qr` | Q/R (DIMSE & DICOMweb), STOW/C-STORE |
| `weasis-dicom-sr`, `-rt`, `-wave`, `-au` (13 rb) | `weasis-core` + `weasis-web` | SR, RT/DVH, ECG, audio |
| `weasis-dicom-isowriter` | `weasis-core::iso` | export ISO (unduh file) |
| `weasis-acquire/*` (10 rb) | `weasis-web::acquire` | Dicomizer |
| `weasis-base/*` (8 rb) | `weasis-web::{shell,files}` | jendela utama, explorer non-DICOM |
| `weasis-launcher` (5 rb) | parameter URL + `weasis-server` | membuka viewer dari aplikasi lain (IHE IID) |
| `weasis-imageio`, `weasis-opencv`, library `weasis-dicom-tools` (eksternal) | `dicom-pixeldata` + crate Rust | codec & operasi gambar |

Library eksternal yang dipakai Weasis:

| Library | Versi dipakai master | Isi penting | Source |
|---|---|---|---|
| `weasis-dicom-tools` | 5.35.1 | decode & render (`org.dcm4che3.img`), LUT/preset, overlay, PR, geometri (`geom`), hanging protocol (`hp`), manifest (`mf`), DIMSE (`op`), DICOMweb (`web`), Dicomizer, anatomi (`ref`) | repo pengguna, `master` = 5.35.2 (beda patch; perbedaan perilaku dicatat jika ditemukan) |
| `weasis-core-img` | 5.0.0.1 | operasi gambar OpenCV (`opencv.op`), W/L & LUT (`opencv.op.lut`), color map (`opencv.op.lut.colormap`), kontur segmentasi (`opencv.seg`), utilitas (`core.util`); ±840 import | repo pengguna, `master` = 5.0.0.1 (sama) |

### 4.2 Data konfigurasi yang dipakai ulang (D7)

| File | Untuk |
|---|---|
| `weasis-dicom/weasis-dicom-codec/src/main/resources/windowPresets.json` | preset W/L |
| `weasis-core/src/main/resources/colormaps.json` | color map 2D |
| `weasis-dicom/weasis-dicom-3d/viewer3d/src/main/resources/volumeColorMaps.json` | preset 3D |
| `weasis-core/src/main/resources/measurementProfiles.json` | profil pengukuran |
| `weasis-core/.../identityMasking.json`, `weasis-dicom-codec/.../identityMasking-dicom.json` | masking identitas |
| `weasis-dicom-codec/.../config/KeyObjectSelectionCodes.xml` | kode KO |
| `weasis-dicom-rt/.../config/layoutModel.xml` | layout RT |
| `weasis-launcher/src/main/resources/org/weasis/theme/Weasis.theme.json` | warna tema |
| `weasis-launcher/conf/base.json` | preferensi bawaan |
| `messages*.properties` + artefak eksternal `weasis-i18n-dist` | teks & terjemahan |

Dokumen konsep yang wajib dibaca sebelum langkah terkait: `weasis-core/docs/*.md`,
`weasis-dicom/weasis-dicom-codec/docs/Window-Presets.md`,
`weasis-dicom/weasis-dicom-explorer/docs/Retrieve-Paths.md`,
`weasis-dicom/weasis-dicom-viewer2d/docs/{mpr-*,fusion-architecture,synch-architecture}.md`.

## 5. Langkah kerja

Aturan setiap langkah:
1. Kerjakan **satu** langkah saja. Baca dulu file Java acuan yang disebut.
2. Jalankan **Verifikasi** langkah itu. Kalau gagal, perbaiki dulu; jangan pindah langkah.
3. Centang `[x]` di sini, perbarui `PROGRESS.md`, commit `weasis-rs: [x.y] <ringkasan>`, push.
4. **Sebelum memulai satu fase**, pecah setiap langkahnya menjadi sub-langkah `x.y.a`, `x.y.b`
   di `PROGRESS.md` (berdasarkan kelas Java acuannya). Langkah yang terlalu besar juga dipecah.

Setiap langkah fitur menulis test: unit test di crate Rust, atau e2e Playwright untuk UI.
Kode ditulis sesuai [`CODING-GUIDE.md`](CODING-GUIDE.md) (D13).
Verifikasi standar sebuah langkah (kecuali disebut lain): `cargo fmt --check`,
`cargo clippy --workspace -- -D warnings`, `cargo test --workspace` lulus, plus test baru langkah itu.

### Fase 0 — Persiapan & uji kelayakan
- [x] **0.1** Workspace `weasis-rs/` (4 crate kosong, `rust-toolchain.toml`, `.gitignore`).
  Verifikasi: `cargo check --workspace`.
- [x] **0.2** Target `wasm32-unknown-unknown` + Trunk; catat versi di `PROGRESS.md`.
  Verifikasi: `trunk --version`, `rustup target list --installed`.
- [x] **0.3** Data uji di `testdata/` + `testdata/README.md` (sumber & lisensi per file).
  Wajib: CT, MR, CR/DX (MONOCHROME1), US RGB, multiframe, PET+CT (fusion), PR, KO, SEG,
  SR, RTSTRUCT/RTDOSE/RTPLAN, ECG, audio, PDF terenkapsulasi, video, DICOMDIR, ZIP,
  overlay, dan setiap transfer syntax di 0.4. Verifikasi: tabel README lengkap, total < 100 MB.
- [x] **0.4** Uji codec di WASM: Implicit/Explicit LE, Explicit BE, Deflate, RLE,
  JPEG baseline 8/12-bit, JPEG lossless, JPEG-LS, JPEG 2000, HTJ2K, video (MPEG-2/4, HEVC).
  Catat hasil per syntax di `PROGRESS.md`. Yang tidak bisa di WASM → keputusan baru:
  decode/transcode di server. Verifikasi: `wasm-bindgen-test` lulus untuk syntax yang didukung.
- [ ] **0.5** *(Ditunda sampai instruksi pemilik, D12.)* Nilai acuan (golden): build Weasis Java (butuh JDK 25+, container saat ini
  JDK 21) dan ambil keluaran; jika tidak bisa, pakai rumus standar DICOM (PS3.3 C.11) +
  dcmtk/pydicom. Catat cara yang dipakai. Verifikasi: ≥ 1 file acuan di `testdata/golden/`.
- [x] **0.6** Halaman Leptos "hello" disajikan oleh axum. Verifikasi: screenshot Playwright.
- [ ] **0.7** CI GitHub Actions khusus `weasis-rs/**`: fmt, clippy, test, `trunk build`.
  Verifikasi: CI hijau.
- [x] **0.8** `docs/FEATURES.md`: daftar fitur rinci per kelas Java → langkah di rencana ini.
  Verifikasi: setiap modul §4.1 dan setiap kelas `*Factory`, `*Tool`, `*ToolBar`, `*PrefView`
  tercantum.

### Fase 1 — Inti DICOM (`weasis-core`, tanpa UI)
- [x] **1.1** Model Patient/Study/Series/Instance + tag (`DicomModel`, `TagD`, `TagW`, `MediaSeries`).
- [ ] **1.2** Baca file → metadata, charset (`DicomMediaIO`, `CharsetEncoding`); file rusak → error jelas.
- [ ] **1.3** Decode piksel: u8/u16/i16/float/RGB, multiframe, planar, YBR → RGB.
- [ ] **1.4** Modality LUT, pixel padding, VOI W/L linear/sigmoid, VOI LUT, MONOCHROME1, invert.
- [ ] **1.5** Preset W/L dari `windowPresets.json` (`WindowPresetRegistry`, `Window-Presets.md`).
- [ ] **1.6** Color map dari `colormaps.json` (`ColorMapRegistry`, `ColorMap-Design.md`).
- [ ] **1.7** Urutan & pemecahan seri (`SortSeriesStack`, `SplittingRules`, `SplittingModalityRules`).
- [ ] **1.8** Geometri: orientasi, `GeometryOfSlice`, `SlicePlaneGeometry`, lokalizer, intersect.
- [ ] **1.9** Overlay (`OverlayOp`) & shutter (`ShutterOp`).
- [ ] **1.10** DICOMDIR (`DicomDirLoader`) & ZIP (`DicomZipCodec`).
- [ ] **1.11** PR (`PresentationStateReader`), KO & rejected KO, hidden series.
- [ ] **1.12** Info pojok per modalitas (`ModalityView`, `ModalityViewJson`, `CornerInfoData`).
- [ ] **1.13** PDF terenkapsulasi, video, US region (`Ultrasound`, `SpecialElementRegion`).
- [ ] **1.14** Statistik & histogram (`ImageStatistics`, `ValueHistogram`), unit (`Unit`, `QuantityKind`).
- [ ] **1.15** Masking identitas & redaksi piksel (`Identity-Masking.md`, `Redaction`, `KarnakMasks`).

Verifikasi Fase 1: unit test dengan `testdata/` dan nilai yang dihitung dari rumus standar DICOM;
perbandingan dengan `testdata/golden/` ditambahkan setelah langkah 0.5 dijalankan (D12).

### Fase 2 — Kerangka UI & explorer
- [ ] **2.1** Tema dari `Weasis.theme.json`; komponen dasar (tombol, toolbar, dropdown,
  slider, collapsible panel, dialog) meniru FlatLaf.
- [ ] **2.2** Jendela utama meniru `WeasisWin`: menu (File, View, Display, Tools, Help),
  toolbar, tab viewer, panel dockable kiri/kanan yang bisa disembunyikan, digeser, di-resize
  (pengganti DockingFrames), status bar.
- [ ] **2.3** Shortcut (`ShortcutManager`, `KeyBinding`, `Shortcut-Management.md`).
- [ ] **2.4** Import lokal: file, folder, drag & drop, DICOMDIR, ZIP; parse di Worker + progress
  (`LocalImport`, `DicomDirImport`, `LoadLocalDicom`, `LoadingTaskPanel`).
- [ ] **2.5** DICOM explorer: pasien/studi/seri, thumbnail, filter, split series, pilih pasien
  (`DicomExplorer`, `PatientPane`, `StudyPane`, `SeriesPane`, `SeriesFilter`, `SplitSeriesManager`).
- [ ] **2.6** Buka seri ke viewer & hanging protocol (`PluginOpeningStrategy`, `HangingProtocols`).
- [ ] **2.7** Tag view + pencarian tag (`DicomFieldsView`, `TagSearchTablePanel`).
- [ ] **2.8** Explorer & viewer non-DICOM (`weasis-base-explorer`, `weasis-base-viewer2d`).
- [ ] **2.9** i18n: `messages*.properties` + `weasis-i18n-dist`, pemilih bahasa (`JLocaleLanguage`).
- [ ] **2.10** About, lisensi, monitor sumber daya (`WeasisAboutBox`, `LicencesDialog`, `ResourceMonitorDialog`).

Verifikasi Fase 2: e2e per langkah + screenshot.

### Fase 3 — Viewer 2D
- [ ] **3.1** Pipeline render ke canvas, satu view (`DefaultView2d`, `RenderedImageLayer`).
- [ ] **3.2** Layout grid & sel gabung (`MigLayoutModel`, `MergedCellsBuilder`, `View2dContainer`).
- [ ] **3.3** Scroll stack (roda, slider, keyboard).
- [ ] **3.4** W/L mouse, preset, bentuk LUT, invert, color map + editor (`LutToolBar`,
  `WindowPresetActions`, `WindowBoundsPanel`, `ColorMapEditorDialog`).
- [ ] **3.5** Zoom, pan, kaca pembesar (`ZoomWin`), rotasi, flip, auto-level, brightness/contrast.
- [ ] **3.6** Info layer, skala, label orientasi, anotasi (`InfoLayer`, `LayerAnnotation`).
- [ ] **3.7** Aksi mouse & menu konteks (`GridMouseHandler`, `ShowPopup`).
- [ ] **3.8** Reset (`ResetTools`).
- [ ] **3.9** Cine (`CineToolBar`).
- [ ] **3.10** Sinkronisasi antar view (`SynchView`, `DicomSynchManager`, `synch-architecture.md`).
- [ ] **3.11** Garis lokalizer / cross-reference.
- [ ] **3.12** Filter, crop, overlay/shutter, pixel padding (`FilterOp`, `CropOp`).
- [ ] **3.13** PR & KO: terapkan, toolbar, filter seri (`PRManager`, `KOManager`, `KeyObjectToolBar`).
- [ ] **3.14** Panel tool: Display, Image, Mini tool, histogram (`DisplayTool`, `ImageTool`,
  `MiniTool`, `HistogramView`).
- [ ] **3.15** Header DICOM toolbar & redaksi (`DcmHeaderToolBar`, `RedactionToolBar`, `DeviceMaskDialog`).

### Fase 4 — Grafik & pengukuran
- [ ] **4.1** Model grafik & layer: pilih, seret, handle, hapus, salin/tempel
  (`AbstractDragGraphic`, `GraphicLayer`, `GraphicClipboard`, `Measurement-Tools.md`).
- [ ] **4.2** Garis: line, ruler, polyline, freehand, arrow, line-with-gap, parallel,
  perpendicular, bidirectional (`imp/line/*`).
- [ ] **4.3** Sudut: angle, open, four-points, Cobb (`imp/angle/*`).
- [ ] **4.4** Area: rectangle, oblique rectangle, ellipse, circle, 3-point circle, polygon (`imp/area/*`).
- [ ] **4.5** Point, pixel info, anotasi teks (`PointGraphic`, `PixelInfoGraphic`, `AnnotationGraphic`).
- [ ] **4.6** Kalibrasi & profil pengukuran (`measurementProfiles.json`, `MeasurementProfiles`).
- [ ] **4.7** Statistik ROI, tabel ukur, profil intensitas, cardiothoracic ratio
  (`MeasureTool`, `ImageRegionStatistics`, `IntensityProfilePanel`, `CardiothoracicRatioPanel`).
- [ ] **4.8** Dialog ukur & properti grafik (`MeasureDialog`, `PropertiesDialog`).
- [ ] **4.9** Simpan grafik ke XML & export PR (`XmlGraphicModel`, `DicomPrSerializer`, `DicomExportPR`).

### Fase 5 — Jaringan (backend + UI)
- [ ] **5.1** Konfigurasi node DICOM & DICOMweb + autentikasi (basic, OAuth2) di backend
  (`DicomNodeListView`, `DicomWebNode`, `AuthenticationEditor`, `OAuth2Service`).
- [ ] **5.2** WADO-URI & manifest XML/JSON (`LoadRemoteDicomManifest`, `XmlManifestParser`,
  `JsonManifestParser`, `Retrieve-Paths.md`).
- [ ] **5.3** Download manager: prioritas, progress, thumbnail, batal (`DownloadManager`,
  `SeriesDownloadManager`, `ThumbnailManager`).
- [ ] **5.4** DICOMweb QIDO-RS & WADO-RS (`RsQuery`, `RsQueryParams`).
- [ ] **5.5** DIMSE Q/R: C-FIND, C-MOVE, C-GET lewat backend (`DicomQrView`, `RetrieveTask`).
- [ ] **5.6** Kirim: STOW-RS & C-STORE (`SendDicomView`, `StowRS`).
- [ ] **5.7** Membuka viewer dari aplikasi lain lewat URL (IHE IID, `weasis-launcher`).

Verifikasi Fase 5: unit test dengan server tiruan. Test integrasi terhadap dcm4chee/Orthanc
di Docker hanya atas instruksi pemilik (D12) (catat di `PROGRESS.md`
jika Docker tidak tersedia; pakai server tiruan).

### Fase 6 — Modul lanjutan
Setiap langkah di fase ini besar; wajib dipecah jadi sub-langkah sebelum dikerjakan.
- [ ] **6.1** MPR ortogonal & oblique (`mpr/*`, `mpr-architecture.md`, `mpr-coordinate-systems.md`, `mpr-transformations.md`).
- [ ] **6.2** Curved MPR & cross-section (`mpr/cmpr/*`, `mpr-curved.md`).
- [ ] **6.3** MIP (`mip/*`).
- [ ] **6.4** Fusion PET/CT (`fusion/*`, `fusion-architecture.md`).
- [ ] **6.5** Segmentasi DICOM SEG (`codec/seg/*`, `SegmentationTool`, `imp/seg/*`).
- [ ] **6.6** SR viewer (`weasis-dicom-sr`).
- [ ] **6.7** RT: struktur, dosis, isodosis, DVH (`weasis-dicom-rt`).
- [ ] **6.8** Waveform ECG + pengukuran (`weasis-dicom-wave`).
- [ ] **6.9** Audio (`weasis-dicom-au`).
- [ ] **6.10** 3D volume rendering WebGL2: preset, shading, crosshair, segmentasi 3D (`viewer3d`).
- [ ] **6.11** Dicomizer: import gambar, edit (crop, rotasi, kontras, kalibrasi, anotasi),
  metadata, publish (`weasis-acquire/*`).

### Fase 7 — Export, cetak, preferensi
- [ ] **7.1** Export gambar view & layout (PNG/JPEG, dengan/tanpa anotasi).
- [ ] **7.2** Export animasi APNG/GIF/DICOM SC (`editor/image/export/*`, `Animation-Export.md`).
- [ ] **7.3** Export DICOM: ZIP, DICOMDIR, anonimisasi (`LocalExport`, `ExportDicom`, `AnonymizationAction`).
- [ ] **7.4** Export ISO (`IsoImageExport`).
- [ ] **7.5** Cetak: lewat browser (`ImagePrint`, `PrintOptions`) & DICOM Print lewat backend (`DicomPrint`).
- [ ] **7.6** Dialog preferensi: semua `*PrefView` (umum, tema, view, zoom, label, grafik,
  masking, proxy, layar, viewer, preset, MPR, segmentasi, explorer, node).

### Fase 8 — Rilis
- [ ] **8.1** axum: aset tersemat, kompresi, header keamanan & cache.
- [ ] **8.2** Uji performa (§6); kalau gagal → render 2D WebGL2 (perbarui D6).
- [ ] **8.3** Dockerfile + `weasis-rs/README.md` (cara menjalankan).
- [ ] **8.4** Semua baris `FEATURES.md` tercentang; e2e penuh hijau di CI.

## 6. Target kualitas

- Seri CT 500 irisan (512×512) dari folder lokal: irisan pertama tampil < 1 detik, seluruh seri siap < 10 detik.
- Scroll stack & W/L dengan mouse ≥ 30 fps pada 512×512.
- MPR ortogonal pada volume 512×512×500 ≥ 15 fps saat menggeser irisan.
- Hasil W/L: selisih ≤ 1 nilai abu-abu dibanding acuan.

## 7. Pertanyaan terbuka

| # | Pertanyaan | Status |
|---|---|---|
| Q1 | Server PACS untuk uji integrasi (dcm4chee di Docker, atau ada server sendiri)? | Ditunda bersama uji coba (D12) |
| Q2 | Source `weasis-core-img` 5.0.0.1 tersedia? | Selesai: repo `fachrurozy182/weasis-core-img` (`master`) |

## 8. Risiko

| Risiko | Mitigasi |
|---|---|
| Crate codec masih muda (`oxideav-jpeg2000` 0.0.x, `pure_jpegls`) | Test `codec_support` dijalankan di CI; versi dikunci di `Cargo.lock`; jika bermasalah, decode di server (D14) |
| Volume besar (MPR, 3D, fusion) melebihi memori browser | Buffer per volume di Worker, tipe data asli (u16), batas memori seperti `Memory-Management.md` |
| Swing → web: perilaku docking & dialog berbeda | Ikuti tata letak & isi panel, bukan mekanisme Swing-nya |
| Scope sangat besar (±233 rb baris) | Fase 0–4 dulu agar viewer inti cepat terpakai; fase lanjut dipecah saat dimulai |
