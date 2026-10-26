//! Softcopy Presentation States (PR): how referenced images are to be displayed (PS3.3 §A.33).
//!
//! This module reads what changes the pixels: references, Softcopy VOI LUT, Modality LUT,
//! Presentation LUT, overlays, shutters, and for pseudo-color and blending states the palette
//! and the blending layers; then where and how the image is shown (displayed area, rotation,
//! flip) and the graphic annotations drawn over it.
//!
//! Java reference: `weasis-dicom-tools/.../org/dcm4che3/img/data/PrDicomObject.java` and
//! `weasis-dicom/weasis-dicom-codec/src/main/java/org/weasis/dicom/codec/PresentationStateReader.java`
//! (`isImageApplicable`).

use dicom_dictionary_std::tags;
use dicom_object::{InMemDicomObject, Tag};

use super::{
    Annotation, AnnotationSpace, DisplayedArea, GraphicLayer, LookupTable, ModalityTransform,
    OverlayPlane, Rgba, Shutter, SpatialTransform, VoiLut, bitmap_shutter, overlay_planes,
    presentation_overlay_planes,
};
use crate::dicom::pixel::Palette;
use crate::dicom::{ContentOrder, DicomFile, first_item, item_f64, item_text};
use crate::error::{Error, Result};

/// Presentation State SOP Classes (PS3.4 Annex B).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentationKind {
    /// Grayscale Softcopy Presentation State (GSPS).
    Grayscale,
    /// Color Softcopy Presentation State.
    Color,
    /// Pseudo-Color Softcopy Presentation State: a palette applied to grayscale images.
    PseudoColor,
    /// Blending Softcopy Presentation State: two series fused (e.g. PET over CT).
    Blending,
    /// XA/XRF Grayscale Softcopy Presentation State.
    XaXrfGrayscale,
    /// Volumetric and advanced states, only partly supported (as in the Java viewer).
    Other,
}

impl PresentationKind {
    /// Kind of a SOP Class UID, `None` when it is not a Presentation State.
    pub fn from_sop_class(uid: &str) -> Option<Self> {
        let kind = match uid.strip_prefix("1.2.840.10008.5.1.4.1.1.11.")? {
            "1" => Self::Grayscale,
            "2" => Self::Color,
            "3" => Self::PseudoColor,
            "4" => Self::Blending,
            "5" => Self::XaXrfGrayscale,
            "6" | "7" | "8" | "9" | "10" | "11" | "12" => Self::Other,
            _ => return None,
        };
        Some(kind)
    }
}

/// An image referenced by a Presentation State.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageReference {
    /// Referenced SOP Instance UID.
    pub sop_instance_uid: String,
    /// Referenced Frame Numbers (1-based); empty for all frames.
    pub frames: Vec<i32>,
}

impl ImageReference {
    pub(super) fn list(parent: &InMemDicomObject) -> Vec<Self> {
        items(parent, tags::REFERENCED_IMAGE_SEQUENCE)
            .iter()
            .filter_map(|item| {
                Some(Self {
                    sop_instance_uid: item_text(item, tags::REFERENCED_SOP_INSTANCE_UID)?,
                    frames: ints(item, tags::REFERENCED_FRAME_NUMBER),
                })
            })
            .collect()
    }
}

/// Whether a list of references covers a frame (1-based) of an image. An empty list covers
/// every image when `empty_means_all`.
fn covers(
    references: &[ImageReference],
    sop_instance_uid: &str,
    frame: i32,
    empty_means_all: bool,
) -> bool {
    if references.is_empty() {
        return empty_means_all;
    }
    references
        .iter()
        .find(|r| r.sop_instance_uid == sop_instance_uid)
        .is_some_and(|r| r.frames.is_empty() || r.frames.contains(&frame))
}

/// A series referenced by a Presentation State, with its images.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeriesReference {
    /// Series Instance UID.
    pub series_instance_uid: String,
    /// Images of the series the state applies to.
    pub images: Vec<ImageReference>,
}

impl SeriesReference {
    fn list(parent: &InMemDicomObject) -> Vec<Self> {
        items(parent, tags::REFERENCED_SERIES_SEQUENCE)
            .iter()
            .filter_map(|item| {
                Some(Self {
                    series_instance_uid: item_text(item, tags::SERIES_INSTANCE_UID)?,
                    images: ImageReference::list(item),
                })
            })
            .collect()
    }
}

/// One item of the Softcopy VOI LUT Sequence: a VOI for some images, or all when none listed.
#[derive(Debug, Clone, PartialEq)]
pub struct VoiItem {
    /// Images it applies to; empty for all the images of the state.
    pub images: Vec<ImageReference>,
    /// Windows, function and tables.
    pub voi: VoiLut,
}

/// One of the two layers of a Blending Softcopy Presentation State.
#[derive(Debug, Clone, PartialEq)]
pub struct BlendingLayer {
    /// SUPERIMPOSED (drawn over, e.g. PET) rather than UNDERLYING (e.g. CT).
    pub superimposed: bool,
    /// Study Instance UID of the layer.
    pub study_instance_uid: Option<String>,
    /// Series of the layer.
    pub series: Vec<SeriesReference>,
    /// VOI of the layer.
    pub voi: Option<VoiLut>,
}

/// Presentation LUT: a shape (IDENTITY or INVERSE) or a table (PS3.3 §C.11.6).
#[derive(Debug, Clone, PartialEq)]
pub enum PresentationLut {
    /// Presentation LUT Shape.
    Shape(String),
    /// Presentation LUT Sequence, with its LUT Explanation.
    Table(LookupTable, Option<String>),
}

/// A parsed Presentation State.
#[derive(Debug, Clone, PartialEq)]
pub struct PresentationState {
    /// SOP Class.
    pub kind: PresentationKind,
    /// Content Label, else "PR" and the Instance Number.
    pub label: String,
    /// Presentation Creation Date and Time (`YYYYMMDD HHMMSS`), if any.
    pub creation: Option<String>,
    /// Sort order among the states of an image (newest first).
    pub order: ContentOrder,
    /// Referenced series (for a blending state without them, those of the underlying layer).
    pub series: Vec<SeriesReference>,
    /// Softcopy VOI LUT items.
    pub voi_items: Vec<VoiItem>,
    /// Modality LUT replacing that of the images, if the state has one.
    pub modality: Option<ModalityTransform>,
    /// Presentation LUT.
    pub presentation_lut: Option<PresentationLut>,
    /// Overlay planes activated on a graphic layer.
    pub overlays: Vec<OverlayPlane>,
    /// Geometric shutter.
    pub shutter: Option<Shutter>,
    /// Bitmap shutter: an overlay plane and its color.
    pub bitmap_shutter: Option<(OverlayPlane, Rgba)>,
    /// Blending layers (blending states only).
    pub blending_layers: Vec<BlendingLayer>,
    /// Relative Opacity of the superimposed layer.
    pub relative_opacity: Option<f64>,
    /// Palette of pseudo-color and blending states.
    pub palette: Option<Palette>,
    /// Displayed Area Selection items.
    pub displayed_areas: Vec<DisplayedArea>,
    /// Image Rotation and Image Horizontal Flip.
    pub spatial_transform: SpatialTransform,
    /// Graphic layers.
    pub layers: Vec<GraphicLayer>,
    /// The data set, read again for the annotations of each image.
    object: InMemDicomObject,
}

impl PresentationState {
    /// Reads a Presentation State file.
    pub fn from_file(file: &DicomFile) -> Result<Self> {
        let object: &InMemDicomObject = file.object();
        let sop_class = file.text(tags::SOP_CLASS_UID).unwrap_or_default();
        let kind = PresentationKind::from_sop_class(&sop_class).ok_or_else(|| {
            Error::UnreadableDicom(format!("{sop_class} is not a Presentation State"))
        })?;
        let blending_layers = if kind == PresentationKind::Blending {
            blending_layers(object)
        } else {
            Vec::new()
        };
        let mut series = SeriesReference::list(object);
        if series.is_empty()
            && let Some(underlying) = blending_layers.iter().find(|l| !l.superimposed)
        {
            series.clone_from(&underlying.series);
        }
        Ok(Self {
            kind,
            label: file.text(tags::CONTENT_LABEL).unwrap_or_else(|| {
                format!(
                    "PR {}",
                    file.text(tags::INSTANCE_NUMBER)
                        .unwrap_or_else(|| "0".to_owned())
                )
            }),
            creation: creation(file),
            order: ContentOrder::from_file(file),
            series,
            voi_items: voi_items(object),
            modality: modality(object),
            presentation_lut: presentation_lut(object),
            overlays: presentation_overlay_planes(file),
            shutter: Shutter::from_file(file),
            bitmap_shutter: bitmap_shutter(file, &overlay_planes(file)),
            relative_opacity: item_f64(object, tags::RELATIVE_OPACITY),
            palette: palette(object, kind),
            blending_layers,
            displayed_areas: DisplayedArea::list(object),
            spatial_transform: SpatialTransform::read(object),
            layers: GraphicLayer::list(object),
            object: object.clone(),
        })
    }

    /// Whether the state applies to a frame (1-based) of an image of a series.
    pub fn applies_to(
        &self,
        series_instance_uid: &str,
        sop_instance_uid: &str,
        frame: i32,
    ) -> bool {
        self.series
            .iter()
            .filter(|s| s.series_instance_uid == series_instance_uid)
            .any(|s| covers(&s.images, sop_instance_uid, frame, false))
    }

    /// The displayed area of a frame (1-based) of an image, if the state selects one.
    pub fn displayed_area_for(&self, sop_instance_uid: &str, frame: i32) -> Option<&DisplayedArea> {
        self.displayed_areas
            .iter()
            .find(|area| covers(&area.images, sop_instance_uid, frame, true))
    }

    /// The annotations shown on a frame (1-based) of an image of `columns` × `rows` pixels, in
    /// image coordinates.
    pub fn annotations_for(
        &self,
        sop_instance_uid: &str,
        frame: i32,
        columns: u32,
        rows: u32,
    ) -> Vec<Annotation> {
        let area = self.displayed_area_for(sop_instance_uid, frame);
        let space = AnnotationSpace::new(columns, rows, area, self.spatial_transform);
        Annotation::list(&self.object, &space)
            .into_iter()
            .filter(|annotation| covers(&annotation.images, sop_instance_uid, frame, true))
            .collect()
    }

    /// The VOI of a frame (1-based) of an image: the first Softcopy VOI LUT item that covers it,
    /// or for a blending state the VOI of the underlying layer.
    pub fn voi_for(&self, sop_instance_uid: &str, frame: i32) -> Option<&VoiLut> {
        if self.kind == PresentationKind::Blending {
            return self
                .blending_layers
                .iter()
                .find(|l| !l.superimposed)?
                .voi
                .as_ref();
        }
        self.voi_items
            .iter()
            .find(|item| covers(&item.images, sop_instance_uid, frame, true))
            .map(|item| &item.voi)
    }
}

fn creation(file: &DicomFile) -> Option<String> {
    let date = file.text(tags::PRESENTATION_CREATION_DATE)?;
    Some(match file.text(tags::PRESENTATION_CREATION_TIME) {
        Some(time) => format!("{date} {time}"),
        None => date,
    })
}

fn voi_items(object: &InMemDicomObject) -> Vec<VoiItem> {
    items(object, tags::SOFTCOPY_VOILUT_SEQUENCE)
        .iter()
        .map(|item| VoiItem {
            images: ImageReference::list(item),
            // As in the Java viewer, the first mapped value of a table is read as unsigned: the
            // state does not know the Pixel Representation of its images.
            voi: VoiLut::from_item(item, false),
        })
        .collect()
}

fn blending_layers(object: &InMemDicomObject) -> Vec<BlendingLayer> {
    items(object, tags::BLENDING_SEQUENCE)
        .iter()
        .map(|item| BlendingLayer {
            superimposed: item_text(item, tags::BLENDING_POSITION).as_deref()
                == Some("SUPERIMPOSED"),
            study_instance_uid: item_text(item, tags::STUDY_INSTANCE_UID),
            series: SeriesReference::list(item),
            voi: first_item(item, tags::SOFTCOPY_VOILUT_SEQUENCE)
                .map(|voi| VoiLut::from_item(voi, false)),
        })
        .collect()
}

/// Modality LUT Sequence or Rescale Slope/Intercept of the state (PS3.3 §C.11.1).
fn modality(object: &InMemDicomObject) -> Option<ModalityTransform> {
    if let Some(item) = first_item(object, tags::MODALITY_LUT_SEQUENCE) {
        return LookupTable::from_item(item, false)
            .ok()
            .map(ModalityTransform::Table);
    }
    let slope = item_f64(object, tags::RESCALE_SLOPE)?;
    Some(ModalityTransform::Rescale {
        slope: if slope == 0.0 { 1.0 } else { slope },
        intercept: item_f64(object, tags::RESCALE_INTERCEPT).unwrap_or(0.0),
    })
}

fn presentation_lut(object: &InMemDicomObject) -> Option<PresentationLut> {
    match first_item(object, tags::PRESENTATION_LUT_SEQUENCE) {
        Some(item) => {
            let table = LookupTable::from_item(item, false).ok()?;
            Some(PresentationLut::Table(
                table,
                item_text(item, tags::LUT_EXPLANATION),
            ))
        }
        None => item_text(object, tags::PRESENTATION_LUT_SHAPE).map(PresentationLut::Shape),
    }
}

/// The palette of a pseudo-color state, or of a blending state.
///
/// NOTE: a blending state should carry it at top level (PS3.3 §A.33.4); some writers put it in
/// the superimposed item, which is accepted as in the Java viewer.
fn palette(object: &InMemDicomObject, kind: PresentationKind) -> Option<Palette> {
    match kind {
        PresentationKind::PseudoColor => Palette::from_item(object).ok(),
        PresentationKind::Blending => Palette::from_item(object).ok().or_else(|| {
            items(object, tags::BLENDING_SEQUENCE)
                .iter()
                .find(|item| {
                    item_text(item, tags::BLENDING_POSITION).as_deref() == Some("SUPERIMPOSED")
                })
                .and_then(|item| Palette::from_item(item).ok())
        }),
        _ => None,
    }
}

fn items(parent: &InMemDicomObject, sequence: Tag) -> &[InMemDicomObject] {
    parent
        .element(sequence)
        .ok()
        .and_then(|element| element.items())
        .unwrap_or_default()
}

fn ints(item: &InMemDicomObject, tag: Tag) -> Vec<i32> {
    item.element(tag)
        .ok()
        .and_then(|element| element.to_multi_int::<i32>().ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
