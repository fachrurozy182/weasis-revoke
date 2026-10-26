# Daftar Fitur Weasis → `weasis-rs`

Inventaris fitur Weasis desktop (`master`, `32fef3339`) yang harus ada di versi web.
Setiap baris: fitur — kelas Java acuan — langkah di [`PLAN.md`](PLAN.md).
Centang `[x]` saat fitur selesai dan terverifikasi.

Singkatan path:
`core` = `weasis-core/src/main/java/org/weasis/core`,
`codec` = `weasis-dicom/weasis-dicom-codec/.../org/weasis/dicom/codec`,
`explorer` = `weasis-dicom/weasis-dicom-explorer/.../org/weasis/dicom/explorer`,
`v2d` = `weasis-dicom/weasis-dicom-viewer2d/.../org/weasis/dicom/viewer2d`,
`tools` = repo `weasis-dicom-tools` (`.../org/weasis/dicom` dan `.../org/dcm4che3`).

## A. Data & codec DICOM

- [ ] Model Patient/Study/Series/Instance, grup & node seri — `explorer/DicomModel`, `core/api/media/data/MediaSeries*`, `TagD`, `TagW` — 1.1
- [ ] Baca file DICOM + charset spesifik — `codec/DicomMediaIO`, `codec/display/CharsetEncoding`, `tools/dcm4che3/img/DicomImageReader` — 1.2
- [ ] Decode piksel semua transfer syntax, multiframe, YBR, palette color — `tools/dcm4che3/img/*`, `codec/TransferSyntax`, `tools/.../util/PaletteColorUtils` — 1.3
- [ ] Modality LUT, VOI LUT, W/L linear/sigmoid, pixel padding, MONOCHROME1 — `tools/.../img/lut/ModalityLutModule`, `VoiLutModule`, `codec/display/WindowAndPresetsOp` — 1.4
- [ ] Preset W/L per modalitas & anatomi — `codec/display/WindowPresetRegistry`, `WindowPreset`, `windowPresets.json`, `tools/.../lut/PresetWindowLevel` — 1.5
- [ ] Color map 2D — `core/api/image/lut/ColorMapRegistry`, `colormaps.json` — 1.6
- [ ] Urutan stack & pemecahan seri — `codec/SortSeriesStack`, `codec/utils/SplittingRules*` — 1.7
- [ ] Geometri irisan, orientasi pasien, lokalizer, perpotongan — `codec/geometry/*`, `tools/weasis/dicom/geom/*` — 1.8
- [ ] Overlay (60xx, embedded) & shutter — `codec/display/OverlayOp`, `ShutterOp`, `tools/.../img/data/OverlayData` — 1.9
- [ ] DICOMDIR & ZIP — `explorer/imp/DicomDirLoader`, `DicomZipCodec`, `DicomZipMediaIO` — 1.10
- [ ] Presentation State — `codec/PresentationStateReader`, `PRSpecialElement`, `tools/.../data/PrDicomObject` — 1.11
- [ ] Key Object, rejected KO, hidden series — `codec/KOSpecialElement`, `RejectedKOSpecialElement`, `HiddenSeriesManager` — 1.11
- [ ] Info pojok per modalitas — `codec/display/ModalityView`, `ModalityViewJson`, `CornerInfoData` — 1.12
- [ ] PDF terenkapsulasi, video, region ultrasound — `codec/DicomEncapDocElement`, `DicomVideoElement`, `codec/utils/Ultrasound` — 1.13
- [ ] Statistik, histogram, unit & besaran — `core/ui/model/utils/ImageStatistics`, `core/api/image/util/ValueHistogram`, `Unit` — 1.14
- [ ] Masking identitas & redaksi piksel — `codec/Redaction`, `KarnakMasks`, `PixelPrivacy`, `codec/display/RedactionOp`, `identityMasking*.json` — 1.15

## B. Jendela utama & explorer

- [ ] Tema FlatLaf Weasis — `weasis-launcher/.../theme/Weasis.theme.json`, `FlatWeasisTheme` — 2.1
- [ ] Jendela utama: menu File/View/Display/Tools/Help, toolbar, tab viewer — `weasis-base-ui/.../WeasisWin` — 2.2
- [ ] Panel dockable (tampil/sembunyi, geser, ubah ukuran) — `core/ui/docking/*` — 2.2
- [ ] Shortcut keyboard & tombol mouse — `core/api/gui/util/ShortcutManager`, `KeyBinding` — 2.3
- [ ] Import lokal: file, folder, drag & drop, DICOMDIR, ZIP + progress — `explorer/imp/*`, `explorer/LoadLocalDicom`, `explorer/main/LoadingTaskPanel` — 2.4
- [ ] Explorer pasien/studi/seri + thumbnail — `explorer/main/DicomExplorer`, `PatientPane`, `StudyPane`, `SeriesPane` — 2.5
- [ ] Filter & split seri, pemilihan pasien — `explorer/main/SeriesFilter`, `SplitSeriesManager`, `PatientSelectionManager` — 2.5
- [ ] Buka seri ke viewer + hanging protocol — `explorer/PluginOpeningStrategy`, `HangingProtocols`, `tools/weasis/dicom/hp/*` — 2.6
- [ ] Tag view + pencarian tag — `explorer/tag/*` — 2.7
- [ ] Explorer & viewer gambar non-DICOM — `weasis-base-explorer`, `weasis-base-viewer2d` — 2.8
- [ ] i18n & pemilih bahasa — `messages*.properties`, `weasis-i18n-dist`, `core/ui/pref/JLocaleLanguage` — 2.9
- [ ] About, lisensi, monitor sumber daya — `WeasisAboutBox`, `LicencesDialog`, `ResourceMonitorDialog` — 2.10

## C. Viewer 2D

- [ ] Render gambar di view — `core/ui/editor/image/DefaultView2d`, `core/ui/model/layer/imp/RenderedImageLayer` — 3.1
- [ ] Layout grid & sel gabung — `core/api/gui/layout/MigLayoutModel`, `MergedCellsBuilder`, `v2d/View2dContainer` — 3.2
- [ ] Scroll stack, inverse stack, offset — `ActionW.SCROLL_SERIES`, `INVERSE_STACK`, `STACK_OFFSET` — 3.3
- [ ] W/L mouse, preset, bentuk LUT, invert — `v2d/LutToolBar`, `WindowPresetActions`, `dockable/WindowBoundsPanel` — 3.4
- [ ] Color map & editornya — `core/ui/editor/image/lut/*` — 3.4
- [ ] Zoom, pan, lensa/kaca pembesar, rotasi, flip — `ZoomToolBar`, `RotationToolBar`, `ZoomWin`, `ActionW.LENS*` — 3.5
- [ ] Auto-level, brightness/contrast — `core/api/image/AutoLevelsOp`, `BrightnessOp` — 3.5
- [ ] Info layer, skala, label orientasi, toggle anotasi — `v2d/InfoLayer`, `core/ui/model/layer/LayerAnnotation` — 3.6
- [ ] Aksi mouse & menu konteks — `GridMouseHandler`, `ShowPopup`, `ActionW.CONTEXTMENU` — 3.7
- [ ] Reset — `v2d/ResetTools` — 3.8
- [ ] Cine (play, stop, kecepatan, sweep) — `v2d/CineToolBar`, `ActionW.CINE*` — 3.9
- [ ] Sinkronisasi view & link seri — `SynchView`, `v2d/DicomSynchManager`, `ActionW.SYNCH*` — 3.10
- [ ] Garis lokalizer / cross-line — `ActionW.SYNCH_CROSSLINE`, `codec/geometry/LocalizerPoster` — 3.11
- [ ] Filter kernel, crop, overlay, shutter, pixel padding — `core/api/image/FilterOp`, `CropOp`, `ActionW.IMAGE_*` — 3.12
- [ ] Terapkan PR, KO toggle/selection/filter — `v2d/PRManager`, `KOManager`, `KeyObjectToolBar` — 3.13
- [ ] Panel Display, Image, Mini tool, histogram — `v2d/dockable/DisplayTool`, `ImageTool`, `MiniTool`, `HistogramView` — 3.14
- [ ] Header DICOM toolbar, redaksi, device mask — `v2d/DcmHeaderToolBar`, `RedactionToolBar`, `DeviceMaskDialog` — 3.15

## D. Grafik & pengukuran

- [ ] Model grafik & layer, pilih, seret, handle, salin/tempel — `core/ui/model/graphic/*`, `GraphicClipboard` — 4.1
- [ ] Line, ruler, polyline, freehand, arrow, line-with-gap — `imp/line/*` — 4.2
- [ ] Parallel, perpendicular, bidirectional — `imp/line/*` — 4.2
- [ ] Angle, open angle, four-points angle, Cobb angle — `imp/angle/*` — 4.3
- [ ] Rectangle, oblique rectangle, ellipse, circle, 3-point circle, polygon — `imp/area/*` — 4.4
- [ ] Point, pixel info, anotasi teks — `imp/PointGraphic`, `PixelInfoGraphic`, `AnnotationGraphic` — 4.5
- [ ] Kalibrasi & profil pengukuran — `graphic/profile/*`, `measurementProfiles.json` — 4.6
- [ ] Statistik ROI & tabel ukur — `dockable/MeasureTool`, `ImageRegionStatistics` — 4.7
- [ ] Profil intensitas & cardiothoracic ratio — `IntensityProfilePanel`, `CardiothoracicRatioPanel` — 4.7
- [ ] Dialog ukur & properti — `core/ui/dialog/MeasureDialog`, `PropertiesDialog` — 4.8
- [ ] Simpan grafik XML & export sebagai PR — `core/ui/model/imp/XmlGraphicModel`, `explorer/pr/*` — 4.9

## E. Jaringan

- [ ] Node DICOM/DICOMweb, header HTTP, autentikasi basic & OAuth2 — `explorer/pref/node/*`, `core/api/net/auth/*` — 5.1
- [ ] WADO-URI & manifest XML/JSON — `explorer/wado/*`, `tools/weasis/dicom/mf/*` — 5.2
- [ ] Download manager: prioritas, progress, thumbnail, batal — `explorer/wado/DownloadManager`, `SeriesDownloadManager`, `ThumbnailManager` — 5.3
- [ ] DICOMweb QIDO-RS & WADO-RS — `explorer/rs/*`, `weasis-dicom-qr/.../RsQuery`, `tools/weasis/dicom/web/*` — 5.4
- [ ] DIMSE C-ECHO, C-FIND, C-MOVE, C-GET — `weasis-dicom-qr/*`, `tools/weasis/dicom/op/*` — 5.5
- [ ] Kirim STOW-RS & C-STORE — `weasis-dicom-send/*`, `tools/weasis/dicom/op/CStore` — 5.6
- [ ] Buka viewer dari aplikasi lain (IHE IID) — `weasis-launcher`, `tools/weasis/dicom/web/InvokeImageDisplay` — 5.7

## F. Modul lanjutan

- [ ] MPR ortogonal & oblique, axis, crosshair — `v2d/mpr/*` — 6.1
- [ ] Curved MPR & cross-section — `v2d/mpr/cmpr/*` — 6.2
- [ ] MIP — `v2d/mip/*` — 6.3
- [ ] Fusion PET/CT — `v2d/fusion/*` — 6.4
- [ ] Segmentasi DICOM SEG (kontur, label map, visibilitas) — `codec/seg/*`, `core/ui/model/graphic/imp/seg/*`, `v2d/dockable/SegmentationTool` — 6.5
- [ ] SR viewer — `weasis-dicom-sr/*` — 6.6
- [ ] RT: struktur, dosis, isodosis, DVH — `weasis-dicom-rt/*` — 6.7
- [ ] Waveform ECG + pengukuran & marker — `weasis-dicom-wave/*` — 6.8
- [ ] Audio — `weasis-dicom-au/*` — 6.9
- [ ] 3D volume rendering, preset, shading, crosshair, segmentasi 3D — `weasis-dicom-3d/viewer3d/*`, `volumeColorMaps.json` — 6.10
- [ ] Dicomizer: import, edit (crop, rotasi, kontras, kalibrasi, anotasi), metadata, publish — `weasis-acquire/*`, `tools/weasis/dicom/tool/Dicomizer` — 6.11

## G. Export, cetak, preferensi

- [ ] Export gambar view & layout — `ActionW.EXPORT_VIEW`, `core/ui/editor/image/CaptureAction` — 7.1
- [ ] Export animasi APNG/GIF/DICOM SC — `core/ui/editor/image/export/*`, `codec/export/*` — 7.2
- [ ] Export DICOM: folder/ZIP, DICOMDIR, JPEG, anonimisasi — `explorer/exp/*`, `AnonymizationAction` — 7.3
- [ ] Export ISO — `weasis-dicom-isowriter/*` — 7.4
- [ ] Cetak lewat browser & DICOM Print — `core/ui/util/ImagePrint`, `explorer/print/*` — 7.5
- [ ] Preferensi umum, bahasa, tema, layar, proxy, logging, shortcut, launcher — `core/ui/pref/*` — 7.6
- [ ] Preferensi viewer, label, grafik, gambar, masking, profil — `core/ui/pref/*` — 7.6
- [ ] Preferensi DICOM, preset W/L, device mask, segmentasi, MPR, 3D, explorer — `*PrefView` di modul DICOM — 7.6
