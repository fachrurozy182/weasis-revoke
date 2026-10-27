//! What follows the loading of series: separating the phases of 4D series, and rendering the
//! thumbnails of new series or again from another image.
//!
//! Java reference: `weasis-dicom/weasis-dicom-explorer/src/main/java/org/weasis/dicom/explorer/LoadLocalDicom.java`
//! (`seriesPostProcessing`) and `SeriesThumbnail.reBuildThumbnail`.

use std::collections::VecDeque;

use leptos::prelude::*;
use weasis_core::explorer::MediaPosition;

use super::Importer;

impl Importer {
    /// Renders again the thumbnail of a series, from the image at `position`.
    pub fn rebuild_thumbnail(&self, series: &str, position: MediaPosition) {
        let request = self.model.with_untracked(|m| {
            m.series(series)
                .and_then(|s| self.thumbnails.request_for(s, position))
        });
        let Some(request) = request else {
            return;
        };
        let idle = self.jobs.with_value(VecDeque::is_empty);
        self.request_thumbnail(request);
        if idle {
            self.send_next();
        }
    }

    /// Separates the phases of the series a request has made, of 2 to 7 phases.
    pub(super) fn separate_new_phases(&self) {
        let keys: Vec<String> = self.model.with_untracked(|m| {
            m.patients()
                .iter()
                .flat_map(|p| &p.studies)
                .flat_map(|s| &s.series)
                .map(|s| s.key.clone())
                .filter(|key| self.post_processed.with_value(|done| !done.contains(key)))
                .collect()
        });
        for key in keys {
            self.model.update(|m| _ = m.separate_phases(&key, false));
            self.post_processed
                .update_value(|done| _ = done.insert(key));
        }
    }

    pub(super) fn queue_missing_thumbnails(&self) {
        let thumbnails = self.model.with_untracked(|m| self.thumbnails.missing(m));
        thumbnails
            .into_iter()
            .for_each(|t| self.request_thumbnail(t));
    }

    /// Separates the phases of a series, whatever their number, and renders the thumbnails of
    /// the new series.
    pub fn separate_phases(&self, series: &str) {
        self.model.update(|m| _ = m.separate_phases(series, true));
        let idle = self.jobs.with_value(VecDeque::is_empty);
        self.queue_missing_thumbnails();
        if idle {
            self.send_next();
        }
    }
}
