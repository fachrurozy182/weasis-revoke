//! Thumbnails of the series: where each imported instance is stored, which series still need a
//! thumbnail, and the thumbnails rendered by the import worker, kept as JPEG data URLs like the
//! JPEG files the desktop application writes in its cache.
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/api/media/data/Thumbnail.java`
//! (`loadThumbnail`: JPEG, quality 80).

use std::collections::{HashMap, HashSet};

use leptos::prelude::*;
use wasm_bindgen::{Clamped, JsCast};
use weasis_core::explorer::{MediaPosition, thumbnail_source};
use weasis_core::model::{DicomModel, SeriesNode};
use web_sys::{CanvasRenderingContext2d, File, HtmlCanvasElement, ImageData};

/// Quality of the JPEG thumbnails.
const JPEG_QUALITY: f64 = 0.8;

/// Where an instance is stored.
#[derive(Clone)]
pub struct Source {
    /// The file chosen by the user.
    pub file: File,
    /// Path through the archives of the file (`ImportedInstance::entry`).
    pub entry: Vec<String>,
    /// Password of the archives.
    pub password: Option<String>,
}

/// A thumbnail to render: the series, the instance source and the frame.
pub struct ThumbnailRequest {
    /// Key of the series.
    pub series: String,
    /// Where the instance is stored.
    pub source: Source,
    /// Frame, 0-based.
    pub frame: u32,
}

/// The thumbnails, shared through the importer.
#[derive(Clone, Copy)]
pub struct Thumbnails {
    /// Data URL of the thumbnail of each series.
    pub images: RwSignal<HashMap<String, String>>,
    sources: StoredValue<HashMap<String, Source>, LocalStorage>,
    requested: StoredValue<HashSet<String>>,
}

impl Thumbnails {
    /// No thumbnail.
    pub fn new() -> Self {
        Self {
            images: RwSignal::new(HashMap::new()),
            sources: StoredValue::new_local(HashMap::new()),
            requested: StoredValue::new(HashSet::new()),
        }
    }

    /// Remembers where an instance is stored.
    // TODO(3.1): the viewers read the pixels from these sources too.
    pub fn record(&self, sop_instance_uid: String, source: Source) {
        self.sources
            .update_value(|s| _ = s.insert(sop_instance_uid, source));
    }

    /// Series of the model without a thumbnail yet, each asked once.
    pub fn missing(&self, model: &DicomModel) -> Vec<ThumbnailRequest> {
        let mut requests = Vec::new();
        let series = model
            .patients()
            .iter()
            .flat_map(|p| &p.studies)
            .flat_map(|s| &s.series);
        for series in series {
            if self.requested.with_value(|r| r.contains(&series.key)) {
                continue;
            }
            if let Some(request) = self.request_for(series, MediaPosition::Middle) {
                self.requested
                    .update_value(|r| _ = r.insert(series.key.clone()));
                requests.push(request);
            }
        }
        requests
    }

    /// The request for the thumbnail of a series made from the image at `position`.
    pub fn request_for(
        &self,
        series: &SeriesNode,
        position: MediaPosition,
    ) -> Option<ThumbnailRequest> {
        let (sop_instance_uid, frame) = thumbnail_source(series, position)?;
        let source = self
            .sources
            .with_value(|s| s.get(&sop_instance_uid).cloned())?;
        Some(ThumbnailRequest {
            series: series.key.clone(),
            source,
            frame,
        })
    }

    /// Keeps the thumbnail of a series from its RGBA pixels.
    pub fn store(&self, series: String, width: u32, height: u32, rgba: &[u8]) {
        match jpeg_data_url(width, height, rgba) {
            Some(url) => self.images.update(|i| _ = i.insert(series, url)),
            None => leptos::logging::warn!("cannot encode the thumbnail of {series}"),
        }
    }
}

impl Default for Thumbnails {
    fn default() -> Self {
        Self::new()
    }
}

/// The pixels drawn on a canvas and encoded as a JPEG data URL.
fn jpeg_data_url(width: u32, height: u32, rgba: &[u8]) -> Option<String> {
    let canvas: HtmlCanvasElement = document().create_element("canvas").ok()?.dyn_into().ok()?;
    canvas.set_width(width);
    canvas.set_height(height);
    let context: CanvasRenderingContext2d = canvas.get_context("2d").ok()??.dyn_into().ok()?;
    let image = ImageData::new_with_u8_clamped_array_and_sh(Clamped(rgba), width, height).ok()?;
    context.put_image_data(&image, 0.0, 0.0).ok()?;
    canvas
        .to_data_url_with_type_and_encoder_options("image/jpeg", &JPEG_QUALITY.into())
        .ok()
}
