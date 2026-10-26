"""Generate the synthetic DICOM test files in `weasis-rs/testdata/synthetic/`.

Development tool only (not part of the application). It creates small files for the
object types that have no freely licensed public sample: CT/PET volumes, MONOCHROME1,
Key Object, Segmentation, Presentation State, display shutters, ultrasound regions,
encapsulated PDF, audio, video and ZIP.

Usage (from `weasis-rs/`):
    python3 -m pip install pydicom==3.0.1 numpy
    python3 tools/testdata/generate.py

Requires `ffmpeg` on PATH for the video files. The output is deterministic except for the
encoded video bytes, which depend on the ffmpeg version.
"""

import shutil
import subprocess
import tempfile
import zipfile
from pathlib import Path

import numpy as np
from pydicom.dataset import Dataset, FileDataset, FileMetaDataset
from pydicom.encaps import encapsulate
from pydicom.sequence import Sequence
from pydicom.uid import (
    ExplicitVRLittleEndian,
    MPEG2MPML,
    MPEG4HP41,
    PYDICOM_IMPLEMENTATION_UID,
)

OUT_DIR = Path(__file__).resolve().parents[2] / "testdata" / "synthetic"

# Fixed UID root so that the generated files are reproducible ("2.25" = UUID-derived root).
UID_ROOT = "2.25.1234567890"

# SOP Class UIDs (DICOM PS3.4 Annex B).
CT_IMAGE = "1.2.840.10008.5.1.4.1.1.2"
PET_IMAGE = "1.2.840.10008.5.1.4.1.1.128"
CR_IMAGE = "1.2.840.10008.5.1.4.1.1.1"
XA_IMAGE = "1.2.840.10008.5.1.4.1.1.12.1"
US_IMAGE = "1.2.840.10008.5.1.4.1.1.6.1"
KEY_OBJECT = "1.2.840.10008.5.1.4.1.1.88.59"
SEGMENTATION = "1.2.840.10008.5.1.4.1.1.66.4"
GRAYSCALE_PR = "1.2.840.10008.5.1.4.1.1.11.1"
ENCAPSULATED_PDF = "1.2.840.10008.5.1.4.1.1.104.1"
BASIC_VOICE_AUDIO = "1.2.840.10008.5.1.4.1.1.9.4.1"
VIDEO_PHOTOGRAPHIC = "1.2.840.10008.5.1.4.1.1.77.1.4.1"

PATIENT = {"PatientName": "WEASIS^SYNTHETIC", "PatientID": "WRS-0001",
           "PatientBirthDate": "19700101", "PatientSex": "O"}
STUDY_UID = f"{UID_ROOT}.1"
FRAME_OF_REFERENCE_UID = f"{UID_ROOT}.2"
STUDY_DATE = "20260101"

CT_SLICES = 16
CT_SIZE = 64
CT_SPACING_MM = 2.0
CT_THICKNESS_MM = 2.5
PET_SLICES = 8
PET_SIZE = 32


def uid(*parts: int) -> str:
    """Build a reproducible UID under UID_ROOT."""
    return ".".join([UID_ROOT, *map(str, parts)])


def new_dataset(sop_class: str, sop_instance: str, transfer_syntax=ExplicitVRLittleEndian):
    """Create a dataset with file meta, patient and study attributes filled in."""
    meta = FileMetaDataset()
    meta.MediaStorageSOPClassUID = sop_class
    meta.MediaStorageSOPInstanceUID = sop_instance
    meta.TransferSyntaxUID = transfer_syntax
    meta.ImplementationClassUID = PYDICOM_IMPLEMENTATION_UID

    ds = FileDataset(None, {}, file_meta=meta, preamble=b"\0" * 128)
    ds.SOPClassUID = sop_class
    ds.SOPInstanceUID = sop_instance
    ds.SpecificCharacterSet = "ISO_IR 192"
    for keyword, value in PATIENT.items():
        setattr(ds, keyword, value)
    ds.StudyInstanceUID = STUDY_UID
    ds.StudyID = "1"
    ds.StudyDate = STUDY_DATE
    ds.StudyTime = "120000"
    ds.AccessionNumber = "WRS-ACC-1"
    ds.StudyDescription = "Weasis-rs synthetic test study"
    ds.ReferringPhysicianName = ""
    return ds


def set_series(ds, modality: str, series_uid: str, number: int, description: str):
    ds.Modality = modality
    ds.SeriesInstanceUID = series_uid
    ds.SeriesNumber = number
    ds.SeriesDescription = description
    ds.SeriesDate = STUDY_DATE
    ds.SeriesTime = "120000"


def set_monochrome_pixels(ds, pixels: np.ndarray, photometric="MONOCHROME2"):
    """Store a 2D integer array as native (uncompressed) pixel data."""
    ds.Rows, ds.Columns = pixels.shape
    ds.SamplesPerPixel = 1
    ds.PhotometricInterpretation = photometric
    ds.BitsAllocated = 16
    ds.BitsStored = 16 if pixels.dtype == np.int16 else 12
    ds.HighBit = ds.BitsStored - 1
    ds.PixelRepresentation = 1 if pixels.dtype == np.int16 else 0
    ds.PixelData = pixels.tobytes()


def set_plane(ds, slice_index: int, spacing_mm: float, thickness_mm: float, size: int):
    """Axial plane centred on the patient origin, slices stacked along +Z."""
    half_width = spacing_mm * size / 2
    z = slice_index * thickness_mm
    ds.FrameOfReferenceUID = FRAME_OF_REFERENCE_UID
    ds.PositionReferenceIndicator = ""
    ds.ImagePositionPatient = [-half_width, -half_width, z]
    ds.ImageOrientationPatient = [1, 0, 0, 0, 1, 0]
    ds.PixelSpacing = [spacing_mm, spacing_mm]
    ds.SliceThickness = thickness_mm
    ds.SliceLocation = z
    ds.InstanceNumber = slice_index + 1


def sphere_phantom(size: int, slice_index: int, slices: int) -> np.ndarray:
    """Distance (0..1) of each pixel from the centre of a sphere spanning the volume."""
    axis = np.linspace(-1.0, 1.0, size)
    x, y = np.meshgrid(axis, axis)
    z = (2.0 * slice_index / (slices - 1)) - 1.0
    return np.sqrt(x**2 + y**2 + z**2)


def ct_series() -> list[Dataset]:
    """CT volume: air background, water sphere with a bone shell (HU values)."""
    series_uid = uid(10)
    datasets = []
    for index in range(CT_SLICES):
        ds = new_dataset(CT_IMAGE, uid(10, index + 1))
        set_series(ds, "CT", series_uid, 1, "CT sphere phantom")
        set_plane(ds, index, CT_SPACING_MM, CT_THICKNESS_MM, CT_SIZE)
        distance = sphere_phantom(CT_SIZE, index, CT_SLICES)
        hounsfield = np.where(distance < 0.7, 0, -1000)
        hounsfield = np.where((distance >= 0.7) & (distance < 0.8), 1000, hounsfield)
        stored = (hounsfield + 1024).astype(np.int16)
        set_monochrome_pixels(ds, stored)
        ds.RescaleIntercept = -1024
        ds.RescaleSlope = 1
        ds.RescaleType = "HU"
        ds.WindowCenter = [40, 400]
        ds.WindowWidth = [400, 1800]
        ds.WindowCenterWidthExplanation = ["SOFT TISSUE", "BONE"]
        ds.ImageType = ["ORIGINAL", "PRIMARY", "AXIAL"]
        ds.KVP = 120
        datasets.append(ds)
    return datasets


def pet_series() -> list[Dataset]:
    """PET volume in the same frame of reference as the CT (for fusion)."""
    series_uid = uid(20)
    spacing = CT_SPACING_MM * CT_SIZE / PET_SIZE
    thickness = CT_THICKNESS_MM * (CT_SLICES - 1) / (PET_SLICES - 1)
    datasets = []
    for index in range(PET_SLICES):
        ds = new_dataset(PET_IMAGE, uid(20, index + 1))
        set_series(ds, "PT", series_uid, 2, "PET hot sphere")
        set_plane(ds, index, spacing, thickness, PET_SIZE)
        distance = sphere_phantom(PET_SIZE, index, PET_SLICES)
        activity = np.clip(1.0 - distance / 0.4, 0.0, 1.0) * 30000
        set_monochrome_pixels(ds, activity.astype(np.int16))
        ds.RescaleSlope = 0.5
        ds.RescaleIntercept = 0
        ds.Units = "BQML"
        ds.ImageType = ["ORIGINAL", "PRIMARY"]
        ds.DecayCorrection = "START"
        ds.CorrectedImage = ["ATTN", "DECY"]
        datasets.append(ds)
    return datasets


def cr_monochrome1() -> Dataset:
    """CR image stored as MONOCHROME1 (high values displayed dark)."""
    ds = new_dataset(CR_IMAGE, uid(30, 1))
    set_series(ds, "CR", uid(30), 3, "CR MONOCHROME1 gradient")
    gradient = np.tile(np.linspace(0, 4095, 128), (128, 1)).astype(np.uint16)
    set_monochrome_pixels(ds, gradient, photometric="MONOCHROME1")
    ds.InstanceNumber = 1
    ds.ImagerPixelSpacing = [0.2, 0.2]
    ds.WindowCenter = 2048
    ds.WindowWidth = 4096
    ds.ViewPosition = "PA"
    ds.BodyPartExamined = "CHEST"
    return ds


def xa_shutter() -> Dataset:
    """XA image with a rectangular, a circular and a polygonal shutter (their intersection is
    shown) in red, given as CIE L*a*b*."""
    ds = new_dataset(XA_IMAGE, uid(80, 1))
    set_series(ds, "XA", uid(80), 8, "XA shutters")
    set_monochrome_pixels(ds, np.full((64, 64), 2048, dtype=np.uint16))
    ds.InstanceNumber = 1
    ds.ShutterShape = ["RECTANGULAR", "CIRCULAR", "POLYGONAL"]
    ds.ShutterLeftVerticalEdge = 5
    ds.ShutterRightVerticalEdge = 60
    ds.ShutterUpperHorizontalEdge = 9
    ds.ShutterLowerHorizontalEdge = 56
    ds.CenterOfCircularShutter = [33, 33]  # row, column, 1-based
    ds.RadiusOfCircularShutter = 30
    ds.VerticesOfThePolygonalShutter = [1, 33, 33, 64, 64, 33, 33, 1]  # a diamond
    # sRGB red is L* 53.24, a* 80.09, b* 67.20 (DICOM encoding, PS3.3 C.10.7.1.1).
    ds.ShutterPresentationColorCIELabValue = [
        round(53.24 * 65535 / 100), round((80.09 + 128) * 65535 / 255),
        round((67.20 + 128) * 65535 / 255)]
    return ds


def us_regions() -> Dataset:
    """Ultrasound image with a 2D tissue region calibrated in cm (top) and a PW Doppler
    spectral region in cm/s over seconds (bottom)."""
    ds = new_dataset(US_IMAGE, uid(90, 1))
    set_series(ds, "US", uid(90), 10, "US regions")
    set_monochrome_pixels(ds, np.full((120, 160), 64, dtype=np.uint16))
    ds.InstanceNumber = 1

    def region(bounds, spatial_format, data_type, units, delta):
        item = Dataset()
        (item.RegionLocationMinX0, item.RegionLocationMinY0,
         item.RegionLocationMaxX1, item.RegionLocationMaxY1) = bounds
        item.RegionSpatialFormat = spatial_format
        item.RegionDataType = data_type
        item.RegionFlags = 0
        item.PhysicalUnitsXDirection, item.PhysicalUnitsYDirection = units
        item.PhysicalDeltaX, item.PhysicalDeltaY = delta
        item.ReferencePixelX0 = 0
        item.ReferencePixelY0 = 0
        return item

    ds.SequenceOfUltrasoundRegions = Sequence([
        region((0, 0, 159, 79), 1, 1, (3, 3), (0.025, 0.025)),
        region((0, 80, 159, 119), 3, 3, (4, 7), (0.01, -2.5)),
    ])
    return ds


def pr_bitmap_shutter(ct: list[Dataset]) -> Dataset:
    """GSPS with a bitmap shutter (group 6000) hiding all but the central 32 x 32 pixels, and an
    annotation overlay (group 6002) that must not be taken for the shutter."""
    target = ct[8]
    ds = new_dataset(GRAYSCALE_PR, uid(61, 1))
    set_series(ds, "PR", uid(61), 9, "Bitmap shutter")
    ds.InstanceNumber = 1
    ds.ContentLabel = "SHUTTER"
    ds.ContentCreatorName = "WEASIS^RS"
    ds.PresentationCreationDate = STUDY_DATE
    ds.PresentationCreationTime = "120000"
    ds.ReferencedSeriesSequence = Sequence([series_reference([target], "ReferencedImageSequence")])
    layer = Dataset()
    layer.GraphicLayer = "LAYER1"
    layer.GraphicLayerOrder = 1
    ds.GraphicLayerSequence = Sequence([layer])
    ds.ShutterShape = "BITMAP"
    ds.ShutterOverlayGroup = 0x6000
    ds.ShutterPresentationValue = 0x8000

    shutter = np.ones((CT_SIZE, CT_SIZE), dtype=np.uint8)
    shutter[16:48, 16:48] = 0
    annotation = np.zeros((CT_SIZE, CT_SIZE), dtype=np.uint8)
    annotation[0, :] = 1
    for group, bits in ((0x6000, shutter), (0x6002, annotation)):
        ds.add_new((group, 0x0010), "US", CT_SIZE)
        ds.add_new((group, 0x0011), "US", CT_SIZE)
        ds.add_new((group, 0x0040), "CS", "G")
        ds.add_new((group, 0x0050), "SS", [1, 1])
        ds.add_new((group, 0x0100), "US", 1)
        ds.add_new((group, 0x0102), "US", 0)
        ds.add_new((group, 0x3000), "OW", np.packbits(bits.ravel(), bitorder="little").tobytes())
    ds.add_new((0x6002, 0x1001), "CS", "LAYER1")
    ds.PresentationLUTShape = "IDENTITY"
    return ds


def image_reference(image: Dataset) -> Dataset:
    item = Dataset()
    item.ReferencedSOPClassUID = image.SOPClassUID
    item.ReferencedSOPInstanceUID = image.SOPInstanceUID
    return item


def series_reference(images: list[Dataset], sequence: str = "ReferencedSOPSequence") -> Dataset:
    """Series item listing its images in `sequence`: Referenced SOP Sequence for a Key Object,
    Referenced Image Sequence for a Presentation State, Referenced Instance Sequence for a SEG."""
    item = Dataset()
    item.SeriesInstanceUID = images[0].SeriesInstanceUID
    setattr(item, sequence, Sequence([image_reference(image) for image in images]))
    return item


def coded(value: str, scheme: str, meaning: str) -> Dataset:
    item = Dataset()
    item.CodeValue = value
    item.CodingSchemeDesignator = scheme
    item.CodeMeaning = meaning
    return item


def key_object(ct: list[Dataset]) -> Dataset:
    """Key Object Selection flagging two CT slices "For Teaching" (DCM 113004)."""
    selected = [ct[5], ct[10]]
    ds = new_dataset(KEY_OBJECT, uid(40, 1))
    set_series(ds, "KO", uid(40), 4, "Key images")
    ds.InstanceNumber = 1
    ds.ContentDate = STUDY_DATE
    ds.ContentTime = "120000"
    ds.ValueType = "CONTAINER"
    ds.ContinuityOfContent = "SEPARATE"
    ds.ConceptNameCodeSequence = Sequence([coded("113004", "DCM", "For Teaching")])
    ds.ContentTemplateSequence = Sequence([Dataset()])
    ds.ContentTemplateSequence[0].MappingResource = "DCMR"
    ds.ContentTemplateSequence[0].TemplateIdentifier = "2010"

    content = []
    for image in selected:
        item = Dataset()
        item.RelationshipType = "CONTAINS"
        item.ValueType = "IMAGE"
        item.ReferencedSOPSequence = Sequence([image_reference(image)])
        content.append(item)
    ds.ContentSequence = Sequence(content)

    study = Dataset()
    study.StudyInstanceUID = STUDY_UID
    study.ReferencedSeriesSequence = Sequence([series_reference(selected)])
    ds.CurrentRequestedProcedureEvidenceSequence = Sequence([study])
    return ds


def segmentation(ct: list[Dataset]) -> Dataset:
    """Binary SEG with one segment (the CT sphere interior) on four CT slices."""
    frames = ct[6:10]
    ds = new_dataset(SEGMENTATION, uid(50, 1))
    set_series(ds, "SEG", uid(50), 5, "Sphere segmentation")
    ds.InstanceNumber = 1
    ds.ContentLabel = "SPHERE"
    ds.ContentDescription = "Water sphere"
    ds.ContentCreatorName = "WEASIS^RS"
    ds.ContentDate = STUDY_DATE
    ds.ContentTime = "120000"
    ds.SegmentationType = "BINARY"
    ds.ImageType = ["DERIVED", "PRIMARY"]
    ds.FrameOfReferenceUID = FRAME_OF_REFERENCE_UID
    ds.ReferencedSeriesSequence = Sequence([series_reference(frames, "ReferencedInstanceSequence")])

    segment = Dataset()
    segment.SegmentNumber = 1
    segment.SegmentLabel = "Sphere"
    segment.SegmentAlgorithmType = "MANUAL"
    segment.SegmentedPropertyCategoryCodeSequence = Sequence(
        [coded("91723000", "SCT", "Anatomical Structure")])
    segment.SegmentedPropertyTypeCodeSequence = Sequence([coded("39607008", "SCT", "Lung")])
    segment.RecommendedDisplayCIELabValue = [53890, 41179, 7710]
    ds.SegmentSequence = Sequence([segment])

    masks = []
    per_frame = []
    for image in frames:
        index = int(image.InstanceNumber) - 1
        masks.append(sphere_phantom(CT_SIZE, index, CT_SLICES) < 0.7)
        frame = Dataset()
        source = Dataset()
        source.ReferencedSOPClassUID = image.SOPClassUID
        source.ReferencedSOPInstanceUID = image.SOPInstanceUID
        derivation = Dataset()
        derivation.SourceImageSequence = Sequence([source])
        frame.DerivationImageSequence = Sequence([derivation])
        identification = Dataset()
        identification.ReferencedSegmentNumber = 1
        frame.SegmentIdentificationSequence = Sequence([identification])
        position = Dataset()
        position.ImagePositionPatient = image.ImagePositionPatient
        frame.PlanePositionSequence = Sequence([position])
        per_frame.append(frame)
    ds.PerFrameFunctionalGroupsSequence = Sequence(per_frame)

    shared = Dataset()
    orientation = Dataset()
    orientation.ImageOrientationPatient = [1, 0, 0, 0, 1, 0]
    shared.PlaneOrientationSequence = Sequence([orientation])
    measures = Dataset()
    measures.PixelSpacing = [CT_SPACING_MM, CT_SPACING_MM]
    measures.SliceThickness = CT_THICKNESS_MM
    shared.PixelMeasuresSequence = Sequence([measures])
    ds.SharedFunctionalGroupsSequence = Sequence([shared])

    ds.Rows = ds.Columns = CT_SIZE
    ds.NumberOfFrames = len(frames)
    ds.SamplesPerPixel = 1
    ds.PhotometricInterpretation = "MONOCHROME2"
    ds.BitsAllocated = ds.BitsStored = 1
    ds.HighBit = 0
    ds.PixelRepresentation = 0
    ds.LossyImageCompression = "00"
    # Binary SEG pixels are packed 8 per byte, least significant bit first (PS3.5 8.1.1).
    ds.PixelData = np.packbits(np.stack(masks).ravel(), bitorder="little").tobytes()
    return ds


def presentation_state(ct: list[Dataset]) -> Dataset:
    """GSPS on one CT slice: soft-tissue window, a text label and a polyline."""
    target = ct[8]
    ds = new_dataset(GRAYSCALE_PR, uid(60, 1))
    set_series(ds, "PR", uid(60), 6, "Annotations")
    ds.InstanceNumber = 1
    ds.ContentLabel = "ANNOTATION"
    ds.ContentDescription = "Sphere annotation"
    ds.ContentCreatorName = "WEASIS^RS"
    ds.PresentationCreationDate = STUDY_DATE
    ds.PresentationCreationTime = "120000"
    ds.ReferencedSeriesSequence = Sequence([series_reference([target], "ReferencedImageSequence")])

    layer = Dataset()
    layer.GraphicLayer = "LAYER1"
    layer.GraphicLayerOrder = 1
    ds.GraphicLayerSequence = Sequence([layer])

    text = Dataset()
    text.UnformattedTextValue = "Sphere"
    text.AnchorPointAnnotationUnits = "PIXEL"
    text.AnchorPoint = [32.0, 8.0]
    text.AnchorPointVisibility = "Y"
    text.BoundingBoxAnnotationUnits = "PIXEL"
    text.BoundingBoxTopLeftHandCorner = [24.0, 2.0]
    text.BoundingBoxBottomRightHandCorner = [40.0, 8.0]
    text.BoundingBoxTextHorizontalJustification = "CENTER"

    line = Dataset()
    line.GraphicAnnotationUnits = "PIXEL"
    line.GraphicDimensions = 2
    line.NumberOfGraphicPoints = 2
    line.GraphicData = [10.0, 32.0, 54.0, 32.0]
    line.GraphicType = "POLYLINE"
    line.GraphicFilled = "N"

    annotation = Dataset()
    annotation.ReferencedImageSequence = Sequence([image_reference(target)])
    annotation.GraphicLayer = "LAYER1"
    annotation.TextObjectSequence = Sequence([text])
    annotation.GraphicObjectSequence = Sequence([line])
    ds.GraphicAnnotationSequence = Sequence([annotation])

    voi = Dataset()
    voi.ReferencedImageSequence = Sequence([image_reference(target)])
    voi.WindowCenter = 40
    voi.WindowWidth = 400
    ds.SoftcopyVOILUTSequence = Sequence([voi])

    area = Dataset()
    area.ReferencedImageSequence = Sequence([image_reference(target)])
    area.PixelOriginInterpretation = "FRAME"
    area.DisplayedAreaTopLeftHandCorner = [1, 1]
    area.DisplayedAreaBottomRightHandCorner = [CT_SIZE, CT_SIZE]
    area.PresentationSizeMode = "SCALE TO FIT"
    area.PresentationPixelSpacing = [CT_SPACING_MM, CT_SPACING_MM]
    ds.DisplayedAreaSelectionSequence = Sequence([area])
    ds.PresentationLUTShape = "IDENTITY"
    return ds


MINIMAL_PDF = b"""%PDF-1.4
1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj
2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj
3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 300 100]/Contents 4 0 R/Resources<</Font<</F1 5 0 R>>>>>>endobj
4 0 obj<</Length 44>>stream
BT /F1 18 Tf 20 50 Td (Weasis-rs report) Tj ET
endstream endobj
5 0 obj<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>endobj
trailer<</Root 1 0 R>>
%%EOF
"""


def encapsulated_pdf() -> Dataset:
    ds = new_dataset(ENCAPSULATED_PDF, uid(70, 1))
    set_series(ds, "DOC", uid(70), 7, "PDF report")
    ds.InstanceNumber = 1
    ds.ContentDate = STUDY_DATE
    ds.ContentTime = "120000"
    ds.AcquisitionDateTime = STUDY_DATE + "120000"
    ds.BurnedInAnnotation = "YES"
    ds.DocumentTitle = "Weasis-rs report"
    ds.ConceptNameCodeSequence = Sequence([coded("18748-4", "LN", "Diagnostic imaging report")])
    ds.MIMETypeOfEncapsulatedDocument = "application/pdf"
    # Encapsulated Document must have an even length (PS3.5 7.1.1).
    document = MINIMAL_PDF if len(MINIMAL_PDF) % 2 == 0 else MINIMAL_PDF + b"\0"
    ds.EncapsulatedDocument = document
    return ds


def audio() -> Dataset:
    """One second of a 440 Hz tone as a Basic Voice Audio Waveform."""
    sample_rate = 8000
    time = np.arange(sample_rate) / sample_rate
    samples = (np.sin(2 * np.pi * 440 * time) * 8000).astype(np.int16)

    ds = new_dataset(BASIC_VOICE_AUDIO, uid(80, 1))
    set_series(ds, "AU", uid(80), 8, "Voice note")
    ds.InstanceNumber = 1
    ds.ContentDate = STUDY_DATE
    ds.ContentTime = "120000"
    ds.AcquisitionDateTime = STUDY_DATE + "120000"

    channel = Dataset()
    channel.ChannelSourceSequence = Sequence([coded("110001", "DCM", "Audio channel")])
    channel.WaveformBitsStored = 16

    waveform = Dataset()
    waveform.WaveformOriginality = "ORIGINAL"
    waveform.NumberOfWaveformChannels = 1
    waveform.NumberOfWaveformSamples = len(samples)
    waveform.SamplingFrequency = sample_rate
    waveform.ChannelDefinitionSequence = Sequence([channel])
    waveform.WaveformBitsAllocated = 16
    waveform.WaveformSampleInterpretation = "SS"
    waveform.WaveformData = samples.tobytes()
    ds.WaveformSequence = Sequence([waveform])
    return ds


def encode_video(ffmpeg_args: list[str], suffix: str) -> bytes:
    """Encode 1 second of an ffmpeg test pattern and return the file bytes."""
    with tempfile.TemporaryDirectory() as tmp:
        output = Path(tmp) / f"video{suffix}"
        command = ["ffmpeg", "-loglevel", "error", "-f", "lavfi",
                   "-i", "testsrc=size=176x144:rate=25:duration=1", *ffmpeg_args, str(output)]
        subprocess.run(command, check=True)
        return output.read_bytes()


def video(transfer_syntax: str, number: int, description: str, stream: bytes) -> Dataset:
    ds = new_dataset(VIDEO_PHOTOGRAPHIC, uid(90 + number, 1), transfer_syntax)
    set_series(ds, "XC", uid(90 + number), 9 + number, description)
    ds.InstanceNumber = 1
    ds.ContentDate = STUDY_DATE
    ds.ContentTime = "120000"
    ds.Rows, ds.Columns = 144, 176
    ds.SamplesPerPixel = 3
    ds.PhotometricInterpretation = "YBR_PARTIAL_420"
    ds.PlanarConfiguration = 0
    ds.BitsAllocated = ds.BitsStored = 8
    ds.HighBit = 7
    ds.PixelRepresentation = 0
    ds.NumberOfFrames = 25
    ds.FrameTime = 40.0
    ds.CineRate = 25
    ds.FrameIncrementPointer = 0x00181063  # Frame Time
    ds.LossyImageCompression = "01"
    ds.PixelData = encapsulate([stream])
    ds["PixelData"].is_undefined_length = True
    return ds


def save(ds: Dataset, relative_path: str) -> Path:
    path = OUT_DIR / relative_path
    path.parent.mkdir(parents=True, exist_ok=True)
    ds.save_as(path, enforce_file_format=True)
    return path


def main() -> None:
    if OUT_DIR.exists():
        shutil.rmtree(OUT_DIR)

    ct = ct_series()
    ct_paths = [save(ds, f"ct-series/CT{ds.InstanceNumber:03d}.dcm") for ds in ct]
    for ds in pet_series():
        save(ds, f"pet-series/PT{ds.InstanceNumber:03d}.dcm")
    save(cr_monochrome1(), "cr-monochrome1.dcm")
    save(key_object(ct), "ko-for-teaching.dcm")
    save(segmentation(ct), "seg-sphere.dcm")
    save(presentation_state(ct), "pr-annotation.dcm")
    save(xa_shutter(), "xa-shutter.dcm")
    save(pr_bitmap_shutter(ct), "pr-bitmap-shutter.dcm")
    save(us_regions(), "us-regions.dcm")
    save(encapsulated_pdf(), "encapsulated-pdf.dcm")
    save(audio(), "audio-440hz.dcm")

    mpeg4 = encode_video(["-c:v", "libx264", "-profile:v", "high", "-level", "4.1",
                          "-pix_fmt", "yuv420p", "-movflags", "+faststart"], ".mp4")
    save(video(MPEG4HP41, 1, "Video MPEG-4 AVC", mpeg4), "video-mpeg4-h264.dcm")
    mpeg2 = encode_video(["-c:v", "mpeg2video", "-pix_fmt", "yuv420p", "-f", "mpeg"], ".mpg")
    save(video(MPEG2MPML, 2, "Video MPEG-2", mpeg2), "video-mpeg2.dcm")

    with zipfile.ZipFile(OUT_DIR / "ct-series.zip", "w", zipfile.ZIP_DEFLATED) as archive:
        for path in ct_paths:
            archive.write(path, f"ct-series/{path.name}")

    print(f"Synthetic test data written to {OUT_DIR}")


if __name__ == "__main__":
    main()
