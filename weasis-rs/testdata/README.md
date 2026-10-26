# Data uji DICOM

File di folder ini dipakai oleh test otomatis `weasis-rs`. Semua file berukuran kecil dan
berlisensi bebas. Total ±9 MB.

| Folder | Sumber | Lisensi |
|---|---|---|
| `pydicom/` | [pydicom](https://github.com/pydicom/pydicom) `src/pydicom/data/test_files` dan [pydicom-data](https://github.com/pydicom/pydicom-data) `data_store/data` | MIT (`LICENSE-pydicom.txt`, `LICENSE-pydicom-data.txt`) |
| `weasis-dicom-tools/` | repo [`fachrurozy182/weasis-dicom-tools`](https://github.com/fachrurozy182/weasis-dicom-tools) `src/test/resources` | EPL-2.0 OR Apache-2.0 |
| `synthetic/` | dibuat oleh `tools/testdata/generate.py` (lihat bawah) | sama dengan proyek ini |

## Cakupan jenis objek

| Kebutuhan | File |
|---|---|
| CT | `pydicom/CT_small.dcm`, `weasis-dicom-tools/mono2-CT-16bit.dcm`, `synthetic/ct-series/` (16 irisan, volume) |
| MR | `pydicom/MR_small*.dcm`, `pydicom/MR-SIEMENS-DICOM-WithOverlays.dcm` |
| CR/DX MONOCHROME1 | `synthetic/cr-monochrome1.dcm` |
| US RGB / YBR / palette | `pydicom/US1_UNCR.dcm`, `weasis-dicom-tools/ybr422-raw.dcm`, `pydicom/examples_palette.dcm` |
| Multiframe | `pydicom/emri_small.dcm` (10), `pydicom/eCT_Supplemental.dcm` (enhanced CT), `pydicom/examples_ybr_color.dcm` (30), `weasis-dicom-tools/jpeg2000-multiframe-multifragments.dcm` (19) |
| PET + CT (fusion) | `synthetic/pet-series/` + `synthetic/ct-series/` (frame of reference sama) |
| Presentation State | `synthetic/pr-annotation.dcm`, `weasis-dicom-tools/prLUTs.dcm` (+ `imageForPrLUTs.dcm`), `weasis-dicom-tools/prOverlay.dcm` (+ `overlay.dcm`) |
| Key Object | `synthetic/ko-for-teaching.dcm` |
| Modality LUT Sequence / VOI LUT Sequence | `pydicom/mlut_18.dcm` / `pydicom/vlut_04.dcm` |
| Float Pixel Data | `pydicom/parametric_map_float.dcm` |
| Segmentasi (SEG) | `synthetic/seg-sphere.dcm`, `pydicom/liver_deflate.dcm` |
| Structured Report | `pydicom/test-SR.dcm`, `pydicom/reportsi.dcm` |
| RT | `pydicom/rtstruct.dcm`, `pydicom/rtdose.dcm`, `pydicom/rtplan.dcm` |
| ECG | `pydicom/waveform_ecg.dcm` |
| Audio | `synthetic/audio-440hz.dcm` |
| PDF terenkapsulasi | `synthetic/encapsulated-pdf.dcm` |
| Video | `synthetic/video-mpeg2.dcm`, `synthetic/video-mpeg4-h264.dcm` |
| DICOMDIR | `pydicom/dicomdir-tiny-alpha/` |
| ZIP | `synthetic/ct-series.zip` |
| Overlay | `pydicom/examples_overlay.dcm`, `pydicom/MR-SIEMENS-DICOM-WithOverlays.dcm`, `weasis-dicom-tools/overlay.dcm` |
| Hanging protocol | `weasis-dicom-tools/NeurosurgeryPlan.dcm` |
| File tanpa file meta | `pydicom/rtstruct.dcm` |
| Charset (Latin, Yunani, Rusia, Arab, Ibrani, UTF-8, GB18030, ISO 2022 Jepang/Korea) | `pydicom/charset/` (daftar di `FileInfo.txt`) |

## Cakupan transfer syntax

| Transfer syntax | UID | File |
|---|---|---|
| Implicit VR Little Endian | 1.2.840.10008.1.2 | `pydicom/MR_small_implicit.dcm`, `pydicom/rtdose.dcm` |
| Explicit VR Little Endian | 1.2.840.10008.1.2.1 | `pydicom/MR_small.dcm` dan banyak lainnya |
| Explicit VR Big Endian | 1.2.840.10008.1.2.2 | `pydicom/MR_small_bigendian.dcm`, `weasis-dicom-tools/US-RGB-8-epicard-Banded-bigendian.dcm` |
| Deflated Explicit VR LE | 1.2.840.10008.1.2.1.99 | `pydicom/image_dfl.dcm` |
| Deflated Image Frame | 1.2.840.10008.1.2.8.1 | `pydicom/liver_deflate.dcm` |
| RLE Lossless | 1.2.840.10008.1.2.5 | `pydicom/MR_small_RLE.dcm`, `pydicom/SC_rgb_rle.dcm`, `weasis-dicom-tools/ybrFull-RLE.dcm` |
| JPEG Baseline 8-bit | 1.2.840.10008.1.2.4.50 | `pydicom/SC_rgb_jpeg_dcmtk.dcm`, `pydicom/examples_ybr_color.dcm` |
| JPEG Extended 12-bit | 1.2.840.10008.1.2.4.51 | `pydicom/JPEG-lossy.dcm`, `pydicom/JPGExtended.dcm` |
| JPEG Lossless SV1 | 1.2.840.10008.1.2.4.70 | `pydicom/JPEG-LL.dcm`, `pydicom/JPGLosslessP14SV1_1s_1f_8b.dcm`, `weasis-dicom-tools/CT-JPEGLosslessSV1.dcm` |
| JPEG Lossless (proses 14) | 1.2.840.10008.1.2.4.57 | **belum ada** |
| JPEG-LS Lossless | 1.2.840.10008.1.2.4.80 | `pydicom/MR_small_jpeg_ls_lossless.dcm`, `pydicom/JLSL_16_15_1_1F.dcm`, `weasis-dicom-tools/palette-multiframe-jpeg-ls.dcm` |
| JPEG-LS Near-Lossless | 1.2.840.10008.1.2.4.81 | `pydicom/SC_rgb_jls_lossy_line.dcm` |
| JPEG 2000 Lossless | 1.2.840.10008.1.2.4.90 | `pydicom/MR_small_jp2klossless.dcm` |
| JPEG 2000 | 1.2.840.10008.1.2.4.91 | `pydicom/JPEG2000.dcm`, `pydicom/US1_J2KI.dcm`, `weasis-dicom-tools/jpeg2000-multiframe-multifragments.dcm` |
| HTJ2K Lossless | 1.2.840.10008.1.2.4.201 | `pydicom/HTJ2KLossless_08_RGB.dcm` |
| HTJ2K | 1.2.840.10008.1.2.4.203 | `pydicom/HTJ2K_08_RGB.dcm` |
| MPEG-2 Main Profile | 1.2.840.10008.1.2.4.100 | `synthetic/video-mpeg2.dcm` |
| MPEG-4 AVC/H.264 High 4.1 | 1.2.840.10008.1.2.4.102 | `synthetic/video-mpeg4-h264.dcm` |
| HEVC/H.265 | 1.2.840.10008.1.2.4.107 | **belum ada** |

## Membuat ulang data sintetis

```bash
cd weasis-rs
python3 -m pip install pydicom==3.0.1 numpy   # sekali saja; butuh juga ffmpeg
python3 tools/testdata/generate.py
```

Script ini hanya alat pengembang (bukan bagian aplikasi). Isinya: volume CT bola 16 irisan
64×64 (udara, air, cangkang tulang), volume PET 8 irisan dengan frame of reference yang sama,
CR MONOCHROME1, KO, SEG biner, GSPS (teks + garis + window), PDF, audio 440 Hz, video
MPEG-2 & H.264 (pola uji ffmpeg 176×144, 25 frame), dan ZIP berisi seri CT.
