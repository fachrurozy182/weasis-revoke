use super::*;
use crate::dicom::read_dicom;

fn state(path: &str) -> PresentationState {
    let full = format!("{}/../../testdata/{path}", env!("CARGO_MANIFEST_DIR"));
    let bytes = std::fs::read(&full).unwrap_or_else(|e| panic!("cannot read {full}: {e}"));
    PresentationState::from_file(&read_dicom(&bytes).expect("readable")).expect("PR")
}

#[test]
fn luts_of_a_grayscale_state_are_read() {
    let pr = state("weasis-dicom-tools/prLUTs.dcm");
    assert_eq!(pr.kind, PresentationKind::Grayscale);
    assert_eq!(pr.label, "XLUT_P03");
    assert_eq!(pr.creation.as_deref(), Some("20000704 1200"));
    assert!(matches!(pr.modality, Some(ModalityTransform::Table(_))));
    let Some(PresentationLut::Table(table, explanation)) = &pr.presentation_lut else {
        panic!("presentation LUT table expected");
    };
    assert_eq!(explanation.as_deref(), Some("scrambled presentation lut"));
    assert!(table.output_range().1 <= 255, "8-bit table");

    let image = "1.2.276.0.7230010.3.200.6.3.1";
    assert!(pr.applies_to("1.2.276.0.7230010.3.200.6.3", image, 1));
    assert!(!pr.applies_to("1.2.276.0.7230010.3.200.6.3", "1.2.3", 1));
    let voi = pr.voi_for(image, 1).expect("VOI");
    assert_eq!((voi.windows.len(), voi.tables.len()), (0, 1));
}

#[test]
fn overlays_and_lut_shape_of_an_overlay_state() {
    let pr = state("weasis-dicom-tools/prOverlay.dcm");
    assert_eq!(pr.overlays.len(), 2);
    assert_eq!(
        pr.presentation_lut,
        Some(PresentationLut::Shape("IDENTITY".to_owned()))
    );
    assert_eq!(pr.shutter, None);
    assert_eq!(pr.bitmap_shutter, None);
}

#[test]
fn a_voi_item_only_applies_to_its_images() {
    let pr = state("synthetic/pr-annotation.dcm");
    let (series, slice) = ("2.25.1234567890.10", "2.25.1234567890.10.9");
    assert!(pr.applies_to(series, slice, 1));
    assert!(!pr.applies_to(series, "2.25.1234567890.10.1", 1));
    let voi = pr.voi_for(slice, 1).expect("VOI of the slice");
    assert_eq!((voi.windows[0].center, voi.windows[0].width), (40.0, 400.0));
    assert_eq!(pr.voi_for("2.25.1234567890.10.1", 1), None);
}

#[test]
fn layers_and_displayed_area_are_read() {
    let pr = state("weasis-dicom-tools/prOverlay.dcm");
    assert_eq!(pr.layers.len(), 6);
    let white = pr.layers[0].color.map(Rgba::to_bytes);
    assert_eq!(white, Some([255, 255, 255, 255]));
    let area = pr
        .displayed_area_for("1.2.276.0.7230010.3.200.12.1.1", 1)
        .expect("area");
    assert_eq!(
        area.bounds(),
        (glam::DVec2::ZERO, glam::DVec2::splat(512.0))
    );
    assert_eq!(area.aspect_ratio, Some((1, 1)));
    assert_eq!(pr.spatial_transform, SpatialTransform::default());
}

#[test]
fn annotations_are_given_for_their_images() {
    use super::super::{GraphicShape, TextObject};
    use glam::DVec2;
    let pr = state("synthetic/pr-annotation.dcm");
    let annotations = pr.annotations_for("2.25.1234567890.10.9", 1, 64, 64);
    assert_eq!(annotations.len(), 1);
    let annotation = &annotations[0];
    assert_eq!(annotation.layer, "LAYER1");
    assert_eq!(
        annotation.graphics[0].shape,
        GraphicShape::Polyline(vec![DVec2::new(10.0, 32.0), DVec2::new(54.0, 32.0)])
    );
    let TextObject {
        lines,
        anchor,
        bounding_box,
        ..
    } = &annotation.texts[0];
    assert_eq!(lines, &["Sphere"]);
    assert_eq!(*anchor, Some(DVec2::new(32.0, 8.0)));
    assert_eq!(
        *bounding_box,
        Some((DVec2::new(24.0, 2.0), DVec2::new(40.0, 8.0)))
    );
    assert!(
        pr.annotations_for("2.25.1234567890.10.1", 1, 64, 64)
            .is_empty()
    );
}

#[test]
fn frames_restrict_a_reference() {
    let references = [ImageReference {
        sop_instance_uid: "1.2".to_owned(),
        frames: vec![2, 3],
    }];
    assert!(covers(&references, "1.2", 3, false));
    assert!(!covers(&references, "1.2", 1, false));
    assert!(!covers(&[], "1.2", 1, false));
    assert!(covers(&[], "1.2", 1, true));
}

#[test]
fn presentation_state_classes_are_recognized() {
    let kind = PresentationKind::from_sop_class;
    assert_eq!(
        kind("1.2.840.10008.5.1.4.1.1.11.4"),
        Some(PresentationKind::Blending)
    );
    assert_eq!(
        kind("1.2.840.10008.5.1.4.1.1.11.9"),
        Some(PresentationKind::Other)
    );
    assert_eq!(kind("1.2.840.10008.5.1.4.1.1.2"), None);
}
