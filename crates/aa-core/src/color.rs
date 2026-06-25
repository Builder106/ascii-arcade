//! Platform-neutral 8-bit RGB colour.
//!
//! Ported from `ColoredFrame.swift`'s `RGBColor`. Lives in `aa-core` so scene
//! generators can describe colour without a platform dependency; each shell maps
//! it to its own representation (NSColor, Direct2D `D2D1_COLOR_F`, an ARGB u32,
//! or an SGR triple).

/// An 8-bit-per-channel RGB colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub const WHITE: RgbColor = RgbColor {
        r: 255,
        g: 255,
        b: 255,
    };
    pub const BLACK: RgbColor = RgbColor { r: 0, g: 0, b: 0 };

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        RgbColor { r, g, b }
    }

    /// Scale each channel by `factor` (clamped to 0..=1). Used to fade trails.
    pub fn scaled(self, factor: f64) -> RgbColor {
        let f = factor.clamp(0.0, 1.0);
        RgbColor {
            r: (self.r as f64 * f).round() as u8,
            g: (self.g as f64 * f).round() as u8,
            b: (self.b as f64 * f).round() as u8,
        }
    }

    /// Linear interpolation toward `other` by `t` (clamped to 0..=1).
    pub fn mixed(self, other: RgbColor, t: f64) -> RgbColor {
        let u = t.clamp(0.0, 1.0);
        let lerp = |a: u8, b: u8| ((a as f64) * (1.0 - u) + (b as f64) * u).round() as u8;
        RgbColor {
            r: lerp(self.r, other.r),
            g: lerp(self.g, other.g),
            b: lerp(self.b, other.b),
        }
    }

    /// Pack as `0xAARRGGBB` (opaque) — handy for shells that blit a u32 buffer.
    pub fn to_argb(self) -> u32 {
        0xFF00_0000 | (self.r as u32) << 16 | (self.g as u32) << 8 | (self.b as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaled_clamps_and_rounds() {
        assert_eq!(
            RgbColor::new(200, 100, 50).scaled(0.5),
            RgbColor::new(100, 50, 25)
        );
        assert_eq!(RgbColor::WHITE.scaled(2.0), RgbColor::WHITE);
        assert_eq!(RgbColor::WHITE.scaled(-1.0), RgbColor::BLACK);
    }

    #[test]
    fn mixed_endpoints() {
        let a = RgbColor::BLACK;
        let b = RgbColor::WHITE;
        assert_eq!(a.mixed(b, 0.0), a);
        assert_eq!(a.mixed(b, 1.0), b);
        assert_eq!(a.mixed(b, 0.5), RgbColor::new(128, 128, 128));
    }
}
