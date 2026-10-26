//! Length units of measurements, and the kind of quantity a measurement holds.
//!
//! Java reference: `weasis-core/src/main/java/org/weasis/core/api/image/util/Unit.java` and
//! `weasis-core/src/main/java/org/weasis/core/ui/model/utils/bean/QuantityKind.java`.

/// Family of a unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitSystem {
    /// Meters and their multiples.
    Metric,
    /// Inches, feet, yards, miles.
    Imperial,
    /// Pixels of an image without calibration.
    Digital,
}

/// A length unit. Pixels cannot be converted to the other units without a calibration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    /// Pixels.
    Pixel,
    /// Nanometers.
    Nanometer,
    /// Micrometers.
    Micrometer,
    /// Millimeters.
    Millimeter,
    /// Centimeters.
    Centimeter,
    /// Meters.
    Meter,
    /// Kilometers.
    Kilometer,
    /// Microinches.
    Microinch,
    /// Thousandths of an inch.
    Milliinch,
    /// Inches.
    Inch,
    /// Feet.
    Feet,
    /// Yards.
    Yard,
    /// Miles.
    Mile,
}

impl Unit {
    /// All units, in the order of their identifiers.
    pub const ALL: [Self; 13] = [
        Self::Pixel,
        Self::Nanometer,
        Self::Micrometer,
        Self::Millimeter,
        Self::Centimeter,
        Self::Meter,
        Self::Kilometer,
        Self::Microinch,
        Self::Milliinch,
        Self::Inch,
        Self::Feet,
        Self::Yard,
        Self::Mile,
    ];

    /// Identifier stored in preferences and in the Java graphic model; neighbours in a system
    /// differ by one.
    pub fn id(self) -> i32 {
        match self {
            Self::Pixel => -5,
            Self::Nanometer => -2,
            Self::Micrometer => -1,
            Self::Millimeter => 0,
            Self::Centimeter => 1,
            Self::Meter => 2,
            Self::Kilometer => 3,
            Self::Microinch => 9,
            Self::Milliinch => 10,
            Self::Inch => 11,
            Self::Feet => 12,
            Self::Yard => 13,
            Self::Mile => 14,
        }
    }

    /// Unit of an identifier; pixels for an unknown one.
    pub fn from_id(id: i32) -> Self {
        Self::ALL
            .into_iter()
            .find(|u| u.id() == id)
            .unwrap_or(Self::Pixel)
    }

    /// Abbreviation (the English texts of `Unit.*_s` in the Java `messages.properties`).
    pub fn abbreviation(self) -> &'static str {
        match self {
            Self::Pixel => "pix",
            Self::Nanometer => "nm",
            Self::Micrometer => "µm",
            Self::Millimeter => "mm",
            Self::Centimeter => "cm",
            Self::Meter => "m",
            Self::Kilometer => "km",
            Self::Microinch => "µin",
            Self::Milliinch => "mil",
            Self::Inch => "in",
            Self::Feet => "ft",
            Self::Yard => "yd",
            Self::Mile => "mi",
        }
    }

    /// Size of the unit in meters (1 for pixels).
    pub fn factor_to_meters(self) -> f64 {
        match self {
            Self::Pixel | Self::Meter => 1.0,
            Self::Nanometer => 1.0e-9,
            Self::Micrometer => 1.0e-6,
            Self::Millimeter => 1.0e-3,
            Self::Centimeter => 1.0e-2,
            Self::Kilometer => 1.0e3,
            Self::Microinch => 2.54e-8,
            Self::Milliinch => 2.54e-5,
            Self::Inch => 2.54e-2,
            Self::Feet => 0.3048,
            Self::Yard => 0.9144,
            Self::Mile => 1_609.344,
        }
    }

    /// System of the unit.
    pub fn system(self) -> UnitSystem {
        match self {
            Self::Pixel => UnitSystem::Digital,
            Self::Microinch
            | Self::Milliinch
            | Self::Inch
            | Self::Feet
            | Self::Yard
            | Self::Mile => UnitSystem::Imperial,
            _ => UnitSystem::Metric,
        }
    }

    /// A value in this unit expressed in `target`; `None` between pixels and physical units.
    pub fn convert(self, value: f64, target: Self) -> Option<f64> {
        if self == target {
            return Some(value);
        }
        if self == Self::Pixel || target == Self::Pixel {
            return None;
        }
        Some(value * self.factor_to_meters() / target.factor_to_meters())
    }

    /// Factor turning a pixel size given in `calibration` (e.g. mm) into this unit.
    pub fn conversion_ratio(self, calibration: Self) -> f64 {
        calibration.factor_to_meters() / self.factor_to_meters()
    }

    /// The next larger unit of the same system.
    pub fn larger(self) -> Option<Self> {
        self.neighbour(1)
    }

    /// The next smaller unit of the same system.
    pub fn smaller(self) -> Option<Self> {
        self.neighbour(-1)
    }

    fn neighbour(self, step: i32) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|u| u.system() == self.system() && u.id() == self.id() + step)
    }

    /// The unit of a system that shows a length (in meters) with the most readable number,
    /// preferably between 0.1 and 1000.
    pub fn best_for(meters: f64, system: UnitSystem) -> Self {
        let score = |unit: &Self| {
            let value = meters / unit.factor_to_meters();
            let magnitude = value.abs().log10().abs();
            if (0.1..=1000.0).contains(&value) {
                magnitude
            } else {
                magnitude + 10.0
            }
        };
        Self::ALL
            .into_iter()
            .filter(|u| u.system() == system && *u != Self::Pixel)
            .min_by(|a, b| score(a).total_cmp(&score(b)))
            .unwrap_or(Self::Meter)
    }
}

/// What a measurement value is, deduced from its key (e.g. `line.length`, `stats.mean`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantityKind {
    /// A number of pixels.
    Count,
    /// An angle.
    Angle,
    /// A ratio.
    Ratio,
    /// A length.
    Length,
    /// A coordinate.
    Coordinate,
    /// An area.
    Area,
    /// A pixel value (minimum, maximum, sum, value under the cursor).
    PixelValue,
    /// A statistic of pixel values (mean, median, standard deviation).
    PixelStatistic,
    /// A higher moment or the entropy.
    Moment,
    /// Anything else.
    Other,
}

impl QuantityKind {
    /// Kind of a measurement key.
    pub fn of_key(key: &str) -> Self {
        const LENGTHS: [&str; 7] = [
            "length",
            "distance",
            "perimeter",
            "width",
            "height",
            "radius",
            "diameter",
        ];
        const ANGLES: [&str; 3] = ["angle", "orientation", "azimuth"];
        match key {
            "" => return Self::Other,
            "stats.pixels" => return Self::Count,
            "stats.min" | "stats.max" | "stats.sum" => return Self::PixelValue,
            "stats.mean" | "stats.median" | "stats.stdev" => return Self::PixelStatistic,
            "stats.skew" | "stats.kurtosis" | "stats.entropy" => return Self::Moment,
            _ if key.starts_with("pixel.") => return Self::PixelValue,
            _ => {}
        }
        let first = key.split('.').next().unwrap_or(key);
        let last = key.rsplit('.').next().unwrap_or(key);
        if last == "ratio" {
            Self::Ratio
        } else if last == "area" {
            Self::Area
        } else if ANGLES.contains(&first) || ANGLES.contains(&last) {
            Self::Angle
        } else if last == "x" || last == "y" {
            Self::Coordinate
        } else if LENGTHS.contains(&first) || LENGTHS.contains(&last) {
            Self::Length
        } else {
            Self::Other
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lengths_convert_between_physical_units_only() {
        assert_eq!(Unit::Centimeter.convert(2.0, Unit::Millimeter), Some(20.0));
        let inches = Unit::Inch.convert(1.0, Unit::Millimeter).expect("physical");
        assert!((inches - 25.4).abs() < 1e-9);
        assert_eq!(Unit::Pixel.convert(3.0, Unit::Millimeter), None);
        assert!((Unit::Centimeter.conversion_ratio(Unit::Millimeter) - 0.1).abs() < 1e-12);
    }

    #[test]
    fn neighbours_stay_in_their_system() {
        assert_eq!(Unit::Millimeter.larger(), Some(Unit::Centimeter));
        assert_eq!(Unit::Kilometer.larger(), None);
        assert_eq!(Unit::Inch.smaller(), Some(Unit::Milliinch));
        assert_eq!(Unit::from_id(11), Unit::Inch);
        assert_eq!(Unit::from_id(99), Unit::Pixel);
    }

    #[test]
    fn the_best_unit_keeps_numbers_readable() {
        assert_eq!(Unit::best_for(0.002, UnitSystem::Metric), Unit::Millimeter);
        assert_eq!(Unit::best_for(0.05, UnitSystem::Metric), Unit::Centimeter);
        assert_eq!(Unit::best_for(0.5, UnitSystem::Metric), Unit::Meter);
        assert_eq!(Unit::best_for(5000.0, UnitSystem::Metric), Unit::Kilometer);
        assert_eq!(Unit::best_for(0.0254, UnitSystem::Imperial), Unit::Inch);
    }

    #[test]
    fn measurement_keys_have_a_kind() {
        assert_eq!(QuantityKind::of_key("line.length"), QuantityKind::Length);
        assert_eq!(QuantityKind::of_key("ellipse.area"), QuantityKind::Area);
        assert_eq!(QuantityKind::of_key("angle"), QuantityKind::Angle);
        assert_eq!(QuantityKind::of_key("point.x"), QuantityKind::Coordinate);
        assert_eq!(
            QuantityKind::of_key("stats.mean"),
            QuantityKind::PixelStatistic
        );
        assert_eq!(
            QuantityKind::of_key("pixel.value"),
            QuantityKind::PixelValue
        );
        assert_eq!(QuantityKind::of_key("cobb.ratio"), QuantityKind::Ratio);
        assert_eq!(QuantityKind::of_key("misc"), QuantityKind::Other);
    }
}
