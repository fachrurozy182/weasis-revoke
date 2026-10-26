//! Filling redaction regions with the median of the pixels around them.
//!
//! Java reference: `weasis-dicom/weasis-dicom-codec/src/main/java/org/weasis/dicom/codec/display/RedactionRenderer.java`.

use super::RedactionShape;
use crate::dicom::pixel::Samples;

/// Width of the ring of pixels whose median fills a region.
const RING_WIDTH: i64 = 4;

/// Fills each shape of a frame given as (width, height, interleaved channels).
pub(super) fn burn_shapes(
    shapes: &[RedactionShape],
    samples: &mut Samples,
    (width, height, channels): (usize, usize, usize),
) {
    let frame = Frame {
        width: i64::try_from(width).unwrap_or(0),
        height: i64::try_from(height).unwrap_or(0),
        channels,
    };
    match samples {
        Samples::U8(v) => shapes.iter().for_each(|s| burn_region(v, &frame, s)),
        Samples::U16(v) => shapes.iter().for_each(|s| burn_region(v, &frame, s)),
        Samples::I16(v) => shapes.iter().for_each(|s| burn_region(v, &frame, s)),
        Samples::U32(v) => shapes.iter().for_each(|s| burn_region(v, &frame, s)),
        Samples::I32(v) => shapes.iter().for_each(|s| burn_region(v, &frame, s)),
        Samples::F32(v) => shapes.iter().for_each(|s| burn_region(v, &frame, s)),
    }
}

struct Frame {
    width: i64,
    height: i64,
    channels: usize,
}

impl Frame {
    fn index(&self, x: i64, y: i64) -> Option<usize> {
        ((0..self.width).contains(&x) && (0..self.height).contains(&y))
            .then(|| usize::try_from(y * self.width + x).ok())
            .flatten()
    }
}

/// A sample type that can be set from a median.
trait Sample: Copy {
    fn to_f64(self) -> f64;
    fn from_f64(value: f64) -> Self;
}

macro_rules! integer_sample {
    ($($t:ty),*) => {$(
        impl Sample for $t {
            fn to_f64(self) -> f64 {
                f64::from(self)
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            fn from_f64(value: f64) -> Self {
                // Rounded half to even and saturated, like OpenCV.
                value.round_ties_even().clamp(<$t>::MIN.into(), <$t>::MAX.into()) as $t
            }
        }
    )*};
}
integer_sample!(u8, u16, i16, u32, i32);

impl Sample for f32 {
    fn to_f64(self) -> f64 {
        f64::from(self)
    }
    #[allow(clippy::cast_possible_truncation)]
    fn from_f64(value: f64) -> Self {
        value as f32
    }
}

fn burn_region<T: Sample>(values: &mut [T], frame: &Frame, shape: &RedactionShape) {
    let [x0, y0, x1, y1] = shape.bounds();
    let inside: Vec<(i64, i64)> = (y0..y1)
        .flat_map(|y| (x0..x1).map(move |x| (x, y)))
        .filter(|&(x, y)| frame.index(x, y).is_some() && shape.covers(x, y))
        .collect();
    if inside.is_empty() {
        return;
    }
    let ring = ring_around(&inside, frame);
    let samples = if ring.is_empty() { &inside } else { &ring };
    for channel in 0..frame.channels {
        let mut channel_values: Vec<f64> = samples
            .iter()
            .filter_map(|&(x, y)| values.get(frame.index(x, y)? * frame.channels + channel))
            .map(|v| v.to_f64())
            .collect();
        let fill = T::from_f64(median(&mut channel_values));
        for &(x, y) in &inside {
            if let Some(value) = frame
                .index(x, y)
                .and_then(|i| values.get_mut(i * frame.channels + channel))
            {
                *value = fill;
            }
        }
    }
}

/// Pixels within the ring width of the region, outside it: a dilation by the 9 × 9 elliptical
/// kernel of OpenCV, minus the region.
fn ring_around(inside: &[(i64, i64)], frame: &Frame) -> Vec<(i64, i64)> {
    use std::collections::HashSet;
    let region: HashSet<(i64, i64)> = inside.iter().copied().collect();
    let mut ring = HashSet::new();
    for &(x, y) in inside {
        for dy in -RING_WIDTH..=RING_WIDTH {
            let reach = kernel_half_width(dy);
            for dx in -reach..=reach {
                let point = (x + dx, y + dy);
                if frame.index(point.0, point.1).is_some() && !region.contains(&point) {
                    ring.insert(point);
                }
            }
        }
    }
    let mut ring: Vec<_> = ring.into_iter().collect();
    ring.sort_unstable();
    ring
}

/// Half width of a row of `getStructuringElement(MORPH_ELLIPSE, 9 × 9)`.
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
pub(super) fn kernel_half_width(dy: i64) -> i64 {
    let r = RING_WIDTH as f64;
    let dy = dy as f64;
    (r * ((r * r - dy * dy) / (r * r)).sqrt()).round_ties_even() as i64
}

/// Median; the mean of the two middle values for an even count.
fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    if values.len() % 2 == 1 {
        values[middle]
    } else {
        (values[middle - 1] + values[middle]) / 2.0
    }
}
