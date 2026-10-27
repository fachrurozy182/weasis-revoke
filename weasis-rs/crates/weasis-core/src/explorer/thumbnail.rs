//! The thumbnail of a series: the middle image, or the middle frame of a single multi-frame
//! image, rendered with its default display and reduced to fit a square.
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/api/media/data/SeriesThumbnail.java`
//! (`MEDIA_POSITION.MIDDLE`) and `Thumbnail.java` (`Thumbnail.MAX_SIZE`).

use dicom_dictionary_std::tags;

use crate::dicom::{ObjectKind, read_dicom};
use crate::error::Result;
use crate::image::{RgbaImage, fit_within, render_default};
use crate::model::{SeriesNode, SortKey, sort_instances};

/// Largest width and height of a thumbnail image; it is shown smaller.
pub const THUMBNAIL_MAX_SIZE: u32 = 256;

/// Image of a series a thumbnail is made from (`MEDIA_POSITION`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MediaPosition {
    /// The first image.
    First,
    /// The middle image (default).
    #[default]
    Middle,
    /// The last image.
    Last,
}

impl MediaPosition {
    fn index(self, count: usize) -> usize {
        match self {
            Self::First => 0,
            Self::Middle => count / 2,
            Self::Last => count.saturating_sub(1),
        }
    }
}

/// SOP Instance UID and frame (0-based) of the image of a series thumbnail, or of a single
/// multi-frame image; `None` for series that are not images (documents, videos, waveforms…),
/// which show their kind.
pub fn thumbnail_source(series: &SeriesNode, position: MediaPosition) -> Option<(String, u32)> {
    let mut images: Vec<_> = series
        .instances
        .iter()
        .filter(|i| i.kind == ObjectKind::Image)
        .cloned()
        .collect();
    sort_instances(&mut images, SortKey::InstanceNumber);
    let chosen = images.get(position.index(images.len()))?;
    let frames: u32 = match images.len() {
        1 => chosen
            .attributes
            .get(tags::NUMBER_OF_FRAMES)
            .and_then(|n| n.trim().parse().ok())
            .unwrap_or(1),
        _ => 1,
    };
    let frame = u32::try_from(position.index(frames as usize)).unwrap_or(0);
    Some((chosen.sop_instance_uid.clone(), frame))
}

/// The thumbnail of `frame` of a DICOM file, at most `size` × `size`.
pub fn render_thumbnail(bytes: &[u8], frame: u32, size: u32) -> Result<RgbaImage> {
    let file = read_dicom(bytes)?;
    Ok(fit_within(&render_default(&file, frame)?, size))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Attributes, InstanceNode};

    fn instance(uid: &str, number: &str, frames: Option<&str>) -> InstanceNode {
        let mut attributes = Attributes::new();
        attributes.set(tags::INSTANCE_NUMBER, number);
        if let Some(frames) = frames {
            attributes.set(tags::NUMBER_OF_FRAMES, frames);
        }
        InstanceNode {
            sop_instance_uid: uid.to_owned(),
            kind: ObjectKind::Image,
            attributes,
        }
    }

    fn series(instances: Vec<InstanceNode>) -> SeriesNode {
        SeriesNode {
            key: "s".to_owned(),
            attributes: Attributes::new(),
            split_number: None,
            instances,
        }
    }

    #[test]
    fn the_middle_image_or_frame_is_chosen() {
        let stack = series(vec![
            instance("c", "3", None),
            instance("a", "1", None),
            instance("b", "2", None),
        ]);
        let middle = MediaPosition::Middle;
        assert_eq!(thumbnail_source(&stack, middle), Some(("b".to_owned(), 0)));
        assert_eq!(
            thumbnail_source(&stack, MediaPosition::Last),
            Some(("c".to_owned(), 0))
        );
        let cine = series(vec![instance("m", "1", Some("30"))]);
        assert_eq!(thumbnail_source(&cine, middle), Some(("m".to_owned(), 15)));
        assert_eq!(
            thumbnail_source(&cine, MediaPosition::Last),
            Some(("m".to_owned(), 29))
        );
        let mut document = instance("d", "1", None);
        document.kind = ObjectKind::EncapsulatedDocument;
        assert_eq!(thumbnail_source(&series(vec![document]), middle), None);
    }

    #[test]
    fn a_thumbnail_fits_its_square() {
        let path = format!(
            "{}/../../testdata/pydicom/CT_small.dcm",
            env!("CARGO_MANIFEST_DIR")
        );
        let image =
            render_thumbnail(&std::fs::read(path).expect("test data"), 0, 64).expect("rendered");
        assert_eq!((image.width, image.height), (64, 64));
    }
}
