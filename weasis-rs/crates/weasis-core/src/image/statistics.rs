//! Statistics of the pixels of a region (or of a whole image), and value histograms.
//!
//! As in the Java viewer, the pixel count, minimum, maximum, mean and standard deviation are
//! computed on the pixels themselves; the median, skewness, kurtosis and entropy come from a
//! histogram of the stored values. Values are reported in modality units (e.g. HU).
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/ui/editor/image/ImageRegionStatistics.java`
//! (`getStatistics`, `medianBin`), `weasis-core/.../ui/model/utils/ImageStatistics.java` and
//! `weasis-core/.../api/image/util/ValueHistogram.java`.

/// Number of bins of the histogram of non-integer images.
const FLOAT_BINS: usize = 1024;

/// Counts of values in equal bins over `min..=max`.
#[derive(Debug, Clone, PartialEq)]
pub struct Histogram {
    /// Count of each bin.
    pub counts: Vec<f64>,
    /// Lower bound of the first bin.
    pub min: f64,
    /// Upper bound of the last bin (included in it).
    pub max: f64,
}

impl Histogram {
    /// Histogram of values over `min..=max` in `bins` bins; other values are ignored.
    pub fn of(values: impl IntoIterator<Item = f64>, min: f64, max: f64, bins: usize) -> Self {
        let mut counts = vec![0.0; bins.max(1)];
        let width = (max - min) / counts.len() as f64;
        let last = counts.len() - 1;
        for value in values {
            if !(min..=max).contains(&value) {
                continue;
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let index = if width > 0.0 {
                ((value - min) / width) as usize
            } else {
                0
            };
            counts[index.min(last)] += 1.0;
        }
        Self { counts, min, max }
    }

    /// Number of counted values.
    pub fn total(&self) -> f64 {
        self.counts.iter().sum()
    }

    /// Value at the center of a bin.
    pub fn center(&self, bin: usize) -> f64 {
        self.min + (bin as f64 + 0.5) * (self.max - self.min) / self.counts.len() as f64
    }

    /// The same counts spread over other bins, in proportion to the overlap of the bins.
    pub fn rebin(&self, min: f64, max: f64, bins: usize) -> Self {
        let mut out = vec![0.0; bins.max(1)];
        let source_width = (self.max - self.min) / self.counts.len() as f64;
        let target_width = (max - min) / out.len() as f64;
        let last = out.len() - 1;
        for (index, &count) in self.counts.iter().enumerate().filter(|(_, c)| **c != 0.0) {
            let low = self.min + index as f64 * source_width;
            let high = low + source_width;
            let first = bin_index((low - min) / target_width).min(last);
            let end = bin_index((high - min) / target_width).min(last);
            for (target, slot) in out.iter_mut().enumerate().take(end + 1).skip(first) {
                let start = min + target as f64 * target_width;
                let overlap = high.min(start + target_width) - low.max(start);
                if overlap > 0.0 {
                    *slot += count * overlap / source_width;
                }
            }
        }
        Self {
            counts: out,
            min,
            max,
        }
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn bin_index(position: f64) -> usize {
    position.floor().max(0.0) as usize
}

/// Statistics of a set of pixels, in modality units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegionStatistics {
    /// Number of pixels.
    pub pixels: usize,
    /// Smallest value.
    pub min: f64,
    /// Largest value.
    pub max: f64,
    /// Median (interpolated in the histogram).
    pub median: f64,
    /// Mean.
    pub mean: f64,
    /// Sum (mean × pixels).
    pub sum: f64,
    /// Standard deviation of the pixels (population).
    pub std_dev: f64,
    /// Skewness (sample estimator, from the histogram).
    pub skewness: f64,
    /// Kurtosis excess (sample estimator, from the histogram).
    pub kurtosis: f64,
    /// Entropy in bits (from the histogram).
    pub entropy: f64,
}

/// Statistics of the stored values of the selected pixels (padding and pixels outside the
/// region already removed). `range` is the stored value range of the image, `integer` whether
/// the samples are integers, and `to_modality` the Modality LUT. `None` without pixels.
pub fn region_statistics(
    stored: &[f64],
    range: (f64, f64),
    integer: bool,
    to_modality: impl Fn(f64) -> f64,
) -> Option<RegionStatistics> {
    let direct = DirectStatistics::of(stored)?;
    let (low, high) = range;
    // Integer images get one bin per stored value; the range ends one past the maximum.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let bins = if integer {
        (high.trunc() - low.trunc()) as usize + 1
    } else {
        FLOAT_BINS
    };
    let histogram = Histogram::of(stored.iter().copied(), low, high + 1.0, bins);
    let shape = HistogramShape::of(&histogram.counts, low, high, &to_modality);

    let (real_a, real_b) = (to_modality(direct.min), to_modality(direct.max));
    // The stored standard deviation is scaled by the slope of the Modality LUT.
    let scale = if direct.max > direct.min {
        (real_b - real_a).abs() / (direct.max - direct.min)
    } else {
        1.0
    };
    let mean = to_modality(direct.mean);
    Some(RegionStatistics {
        pixels: stored.len(),
        min: real_a.min(real_b),
        max: real_a.max(real_b),
        median: shape.median,
        mean,
        sum: mean * stored.len() as f64,
        std_dev: direct.std_dev * scale,
        skewness: shape.skewness,
        kurtosis: shape.kurtosis,
        entropy: shape.entropy,
    })
}

struct DirectStatistics {
    min: f64,
    max: f64,
    mean: f64,
    std_dev: f64,
}

impl DirectStatistics {
    fn of(values: &[f64]) -> Option<Self> {
        if values.is_empty() {
            return None;
        }
        let count = values.len() as f64;
        let mean = values.iter().sum::<f64>() / count;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / count;
        Some(Self {
            min: values.iter().copied().fold(f64::INFINITY, f64::min),
            max: values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            mean,
            std_dev: variance.sqrt(),
        })
    }
}

struct HistogramShape {
    median: f64,
    skewness: f64,
    kurtosis: f64,
    entropy: f64,
}

impl HistogramShape {
    fn of(bins: &[f64], low: f64, high: f64, to_modality: &impl Fn(f64) -> f64) -> Self {
        // A single bin is a uniform region: every level maps to its value.
        let factor = if bins.len() > 1 {
            (high - low) / (bins.len() - 1) as f64
        } else {
            0.0
        };
        let level = |k: usize| to_modality(k as f64 * factor + low);
        let total: f64 = bins.iter().sum();
        let mean = bins
            .iter()
            .enumerate()
            .map(|(k, &c)| c * level(k))
            .sum::<f64>()
            / total;
        let (mut m2, mut m3, mut m4, mut entropy) = (0.0, 0.0, 0.0, 0.0);
        for (k, &count) in bins.iter().enumerate() {
            let deviation = level(k) - mean;
            m2 += count * deviation.powi(2);
            m3 += count * deviation.powi(3);
            m4 += count * deviation.powi(4);
            let p = count / total;
            if p > 0.0 {
                entropy -= p * p.log2();
            }
        }
        let variance = m2 / (total - 1.0);
        let (skewness, kurtosis) = if bins.len() > 3 && variance > 1e-9 {
            let n = total;
            (
                n * m3 / ((n - 1.0) * (n - 2.0) * variance.sqrt() * variance),
                (n * (n + 1.0) * m4 - 3.0 * m2 * m2 * (n - 1.0))
                    / ((n - 1.0) * (n - 2.0) * (n - 3.0) * variance * variance),
            )
        } else {
            (0.0, 0.0)
        };
        Self {
            median: to_modality(median_bin(bins, total / 2.0) * factor + low),
            skewness,
            kurtosis,
            entropy,
        }
    }
}

/// Fractional bin where the cumulative count reaches `half`, interpolated inside the bin.
fn median_bin(bins: &[f64], half: f64) -> f64 {
    let mut cumulative = 0.0;
    for (index, &count) in bins.iter().enumerate() {
        if cumulative + count >= half {
            let fraction = if count > 0.0 {
                (half - cumulative) / count
            } else {
                0.0
            };
            return index as f64 + fraction;
        }
        cumulative += count;
    }
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn the_maximum_belongs_to_the_last_bin() {
        let histogram = Histogram::of([0.0, 5.0, 9.9, 10.0, 11.0], 0.0, 10.0, 10);
        assert_eq!(histogram.counts[0], 1.0);
        assert_eq!(histogram.counts[9], 2.0);
        assert_eq!(histogram.total(), 4.0);
        assert!(near(histogram.center(0), 0.5));
    }

    #[test]
    fn rebinning_keeps_the_total() {
        let histogram = Histogram::of((0..100).map(f64::from), 0.0, 100.0, 10);
        let coarse = histogram.rebin(0.0, 100.0, 3);
        assert!(near(coarse.total(), 100.0));
        assert!(near(coarse.counts[0], 100.0 / 3.0));
    }

    #[test]
    fn statistics_of_a_ct_region_in_hounsfield_units() {
        // Stored 0..=9, rescale slope 2 and intercept -1024.
        let stored: Vec<f64> = (0..10).map(f64::from).collect();
        let stats =
            region_statistics(&stored, (0.0, 9.0), true, |v| v * 2.0 - 1024.0).expect("stats");
        assert_eq!(stats.pixels, 10);
        assert!(near(stats.min, -1024.0) && near(stats.max, -1006.0));
        assert!(near(stats.mean, -1015.0));
        assert!(near(stats.sum, -10150.0));
        // Population standard deviation of 0..=9 is 2.8723 in stored values, doubled by the slope.
        assert!((stats.std_dev - 2.0 * 8.25f64.sqrt()).abs() < 1e-9);
        assert!(near(stats.median, -1014.0));
        assert!(near(stats.skewness, 0.0));
        assert!(near(stats.entropy, 10f64.log2()));
    }

    #[test]
    fn a_uniform_region_has_no_spread() {
        let stats = region_statistics(&[7.0; 4], (7.0, 7.0), true, |v| v).expect("stats");
        assert_eq!(
            (stats.min, stats.max, stats.median, stats.std_dev),
            (7.0, 7.0, 7.0, 0.0)
        );
        assert_eq!(region_statistics(&[], (0.0, 1.0), true, |v| v), None);
    }
}
