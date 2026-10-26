//! Blending of two colors in RGB, HSB or CIE L*a*b* (D65, sRGB).
//!
//! Java reference: `weasis-core-img/.../opencv/op/lut/colormap/InterpolationSpace.java`

/// A color with components and alpha in `0.0..=1.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    /// Red.
    pub red: f32,
    /// Green.
    pub green: f32,
    /// Blue.
    pub blue: f32,
    /// Opacity.
    pub alpha: f32,
}

impl Rgba {
    /// Opaque white, the color of a map without color stops.
    pub const WHITE: Self = Self::opaque(1.0, 1.0, 1.0);
    /// Fully transparent black.
    pub const TRANSPARENT: Self = Self {
        red: 0.0,
        green: 0.0,
        blue: 0.0,
        alpha: 0.0,
    };

    /// Opaque color.
    pub const fn opaque(red: f32, green: f32, blue: f32) -> Self {
        Self {
            red,
            green,
            blue,
            alpha: 1.0,
        }
    }

    /// Same color with another alpha.
    pub fn with_alpha(self, alpha: f32) -> Self {
        Self { alpha, ..self }
    }

    /// Components as bytes `[r, g, b, a]`, rounded like `Rgba.red8()` in Java.
    pub fn to_bytes(self) -> [u8; 4] {
        [self.red, self.green, self.blue, self.alpha].map(to_byte)
    }

    /// Parses `transparent`, `#rgb`, `#rrggbb` or `#rrggbbaa`.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        if text.eq_ignore_ascii_case("transparent") {
            return Some(Self::TRANSPARENT);
        }
        let digits = text.strip_prefix('#').unwrap_or(text);
        let byte = |i: usize| u8::from_str_radix(digits.get(i..i + 2)?, 16).ok();
        let channel = |value: u8| f32::from(value) / 255.0;
        match digits.len() {
            3 => {
                let nibble = |i: usize| {
                    u8::from_str_radix(digits.get(i..=i)?, 16)
                        .ok()
                        .map(|n| n * 17)
                };
                Some(Self::opaque(
                    channel(nibble(0)?),
                    channel(nibble(1)?),
                    channel(nibble(2)?),
                ))
            }
            6 => Some(Self::opaque(
                channel(byte(0)?),
                channel(byte(2)?),
                channel(byte(4)?),
            )),
            8 => Some(
                Self::opaque(channel(byte(0)?), channel(byte(2)?), channel(byte(4)?))
                    .with_alpha(channel(byte(6)?)),
            ),
            _ => None,
        }
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn to_byte(component: f32) -> u8 {
    // Clamped to 0..=1 first, so the cast cannot truncate.
    (component.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// Space in which two stop colors are blended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorSpace {
    /// Component-wise sRGB blend; cheapest, may dip through gray.
    #[default]
    Rgb,
    /// Hue (shortest arc), saturation and brightness; keeps ramps vivid.
    Hsl,
    /// CIE L*a*b*; perceptually even ramps.
    Lab,
}

impl ColorSpace {
    /// Opaque color at fraction `t` between `from` (0) and `to` (1).
    pub fn mix(self, from: Rgba, to: Rgba, t: f64) -> Rgba {
        #[allow(clippy::cast_possible_truncation)]
        let t = t.clamp(0.0, 1.0) as f32;
        match self {
            Self::Rgb => Rgba::opaque(
                lerp(from.red, to.red, t),
                lerp(from.green, to.green, t),
                lerp(from.blue, to.blue, t),
            ),
            Self::Hsl => mix_hsb(from, to, t),
            Self::Lab => mix_lab(from, to, t),
        }
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn mix_hsb(from: Rgba, to: Rgba, t: f32) -> Rgba {
    let [mut h1, s1, v1] = to_hsb(from);
    let [mut h2, s2, v2] = to_hsb(to);
    // A gray has no hue: borrow the other end's so the ramp does not sweep the wheel.
    if s1 == 0.0 {
        h1 = h2;
    } else if s2 == 0.0 {
        h2 = h1;
    }
    let mut dh = h2 - h1;
    if dh > 0.5 {
        dh -= 1.0;
    } else if dh < -0.5 {
        dh += 1.0;
    }
    let mut hue = h1 + dh * t;
    hue -= hue.floor();
    from_hsb(hue, lerp(s1, s2, t), lerp(v1, v2, t))
}

fn to_hsb(c: Rgba) -> [f32; 3] {
    let max = c.red.max(c.green).max(c.blue);
    let min = c.red.min(c.green).min(c.blue);
    let delta = max - min;
    let saturation = if max == 0.0 { 0.0 } else { delta / max };
    let mut hue = 0.0;
    if delta > 0.0 {
        hue = if max == c.red {
            (c.green - c.blue) / delta
        } else if max == c.green {
            2.0 + (c.blue - c.red) / delta
        } else {
            4.0 + (c.red - c.green) / delta
        } / 6.0;
        if hue < 0.0 {
            hue += 1.0;
        }
    }
    [hue, saturation, max]
}

fn from_hsb(hue: f32, saturation: f32, value: f32) -> Rgba {
    let sector = (hue - hue.floor()) * 6.0;
    let f = sector - sector.floor();
    let p = value * (1.0 - saturation);
    let q = value * (1.0 - saturation * f);
    let u = value * (1.0 - saturation * (1.0 - f));
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    match sector as u8 {
        0 => Rgba::opaque(value, u, p),
        1 => Rgba::opaque(q, value, p),
        2 => Rgba::opaque(p, value, u),
        3 => Rgba::opaque(p, q, value),
        4 => Rgba::opaque(u, p, value),
        _ => Rgba::opaque(value, p, q),
    }
}

const D65: [f64; 3] = [0.950_456, 1.0, 1.088_754];
const LAB_THRESHOLD: f64 = 8.856_451_679_035_63e-3;
const LAB_INVERSE_THRESHOLD: f64 = 0.206_896_551_724_137_93;
const LAB_ALPHA: f64 = 841.0 / 108.0;
const LAB_BETA: f64 = 4.0 / 29.0;
const GAMMA_THRESHOLD: f64 = 0.003_130_668_442_500_588_3;
const GAMMA_INVERSE_THRESHOLD: f64 = 0.040_448_236_277_107_6;

#[allow(clippy::cast_possible_truncation)]
fn mix_lab(from: Rgba, to: Rgba, t: f32) -> Rgba {
    let a = rgb_to_lab(from);
    let b = rgb_to_lab(to);
    let t = f64::from(t);
    let [r, g, bl] = lab_to_rgb([0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t));
    Rgba::opaque(r as f32, g as f32, bl as f32)
}

fn rgb_to_lab(c: Rgba) -> [f64; 3] {
    let [r, g, b] = [c.red, c.green, c.blue].map(|v| inverse_gamma(f64::from(v)));
    let x =
        0.412_395_588_967_414_2 * r + 0.357_583_430_763_714_8 * g + 0.180_492_647_381_701_57 * b;
    let y =
        0.212_586_230_785_595_55 * r + 0.715_170_303_703_410_8 * g + 0.072_200_498_643_336_23 * b;
    let z =
        0.019_297_215_491_746_945 * r + 0.119_183_864_580_848_53 * g + 0.950_497_125_131_579_8 * b;
    let [fx, fy, fz] = [x / D65[0], y / D65[1], z / D65[2]].map(lab_f);
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

fn lab_to_rgb([l, a, b]: [f64; 3]) -> [f64; 3] {
    let fy = (l + 16.0) / 116.0;
    let fx = fy + a / 500.0;
    let fz = fy - b / 200.0;
    let [x, y, z] = [
        D65[0] * lab_inverse(fx),
        D65[1] * lab_inverse(fy),
        D65[2] * lab_inverse(fz),
    ];
    let mut rgb = [
        3.2406 * x - 1.5372 * y - 0.4986 * z,
        -0.9689 * x + 1.8758 * y + 0.0415 * z,
        0.0557 * x - 0.2040 * y + 1.0570 * z,
    ];
    // Out-of-gamut colors are shifted up rather than clipped, as in the Java code.
    let min = rgb.iter().copied().fold(f64::INFINITY, f64::min);
    if min < 0.0 {
        rgb = rgb.map(|v| v - min);
    }
    rgb.map(|v| gamma(v).clamp(0.0, 1.0))
}

fn lab_f(t: f64) -> f64 {
    if t > LAB_THRESHOLD {
        t.cbrt()
    } else {
        LAB_ALPHA * t + LAB_BETA
    }
}

fn lab_inverse(t: f64) -> f64 {
    if t > LAB_INVERSE_THRESHOLD {
        t * t * t
    } else {
        (t - LAB_BETA) / LAB_ALPHA
    }
}

fn gamma(value: f64) -> f64 {
    if value <= GAMMA_THRESHOLD {
        12.92 * value
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    }
}

fn inverse_gamma(value: f64) -> f64 {
    if value <= GAMMA_INVERSE_THRESHOLD {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_colors() {
        assert_eq!(Rgba::parse("#ff0000"), Some(Rgba::opaque(1.0, 0.0, 0.0)));
        assert_eq!(Rgba::parse("#f00"), Some(Rgba::opaque(1.0, 0.0, 0.0)));
        assert_eq!(Rgba::parse("transparent"), Some(Rgba::TRANSPARENT));
        assert_eq!(
            Rgba::parse("#ff000080").map(|c| c.to_bytes()),
            Some([255, 0, 0, 128])
        );
        assert_eq!(Rgba::parse("red"), None);
    }

    #[test]
    fn every_space_returns_the_ends_at_0_and_1() {
        let from = Rgba::opaque(1.0, 0.0, 0.0);
        let to = Rgba::opaque(0.0, 0.0, 1.0);
        for space in [ColorSpace::Rgb, ColorSpace::Hsl, ColorSpace::Lab] {
            assert_eq!(
                space.mix(from, to, 0.0).to_bytes(),
                [255, 0, 0, 255],
                "{space:?}"
            );
            assert_eq!(
                space.mix(from, to, 1.0).to_bytes(),
                [0, 0, 255, 255],
                "{space:?}"
            );
        }
    }

    #[test]
    fn hsl_keeps_red_to_blue_vivid_through_magenta() {
        let mid = ColorSpace::Hsl.mix(
            Rgba::opaque(1.0, 0.0, 0.0),
            Rgba::opaque(0.0, 0.0, 1.0),
            0.5,
        );
        assert_eq!(mid.to_bytes(), [255, 0, 255, 255]);
    }
}
