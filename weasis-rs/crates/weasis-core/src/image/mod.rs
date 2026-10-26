//! Image processing for display: modality and VOI lookup tables, presets and color maps.

mod color_map;
mod color_space;
mod lookup_table;
mod modality;
mod overlay;
mod presentation_state;
mod preset_list;
mod render;
mod shutter;
mod voi;
mod window_preset;

pub use color_map::{
    BUILT_IN_COLOR_MAPS_JSON, ColorMap, ColorMapDomain, ColorStop,
    DomainKind as ColorMapDomainKind, Interpolation, parse_color_maps,
};
pub use color_space::{ColorSpace, Rgba};
pub use lookup_table::LookupTable;
pub use modality::{ModalityLut, ModalityTransform};
pub use overlay::{
    EmbeddedOverlay, OverlayPlane, embedded_overlays, overlay_mask, overlay_planes, paint_overlay,
    presentation_overlay_planes,
};
pub use presentation_state::{
    BlendingLayer, ImageReference, PresentationKind, PresentationLut, PresentationState,
    SeriesReference, VoiItem,
};
pub use preset_list::{PresetContext, PresetWindowLevel, preset_list};
pub use render::render_gray;
pub use shutter::{Shutter, bitmap_shutter, shutter_color};
pub use voi::{LutShape, VoiLut, VoiParams, VoiWindow, sequence_window};
pub use window_preset::{
    BUILT_IN_PRESETS_JSON, Domain, PercentOf, PresetLayers, When, WindowPreset, parse_presets,
};
