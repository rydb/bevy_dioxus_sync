use std::fmt;

/// Fixed point denominator shared by every percent measurement.
const FRAC_ONE: u32 = 10_000;

/// Whole percent in the range 0 to 100.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Percent(u16);

impl Percent {
    /// Zero percent.
    pub const ZERO: Self = Self(0);
    /// One hundred percent.
    pub const FULL: Self = Self(FRAC_ONE as u16);

    /// Creates a percent from a whole value, capped at 100.
    pub const fn new(percent: u16) -> Self {
        let capped = if percent > 100 { 100 } else { percent };
        Self((capped * 100) as u16)
    }

    /// Creates a percent from fixed point units, capped at one hundred.
    pub const fn from_raw(raw: u32) -> Self {
        let capped = if raw > FRAC_ONE { FRAC_ONE } else { raw };
        Self(capped as u16)
    }

    /// Returns the fixed point units.
    pub const fn raw(self) -> u32 {
        self.0 as u32
    }

    /// Returns the whole percent value.
    pub const fn whole(self) -> u16 {
        (self.0 / 100) as u16
    }

    /// Converts a percent of an extent to pixels with half up rounding.
    pub fn as_px(self, extent_px: u32) -> u32 {
        ((self.raw() as u64 * extent_px as u64 + (FRAC_ONE as u64 / 2)) / FRAC_ONE as u64) as u32
    }

    /// Converts a pixel offset of an extent to a percent.
    pub fn from_px(px: i32, extent_px: u32) -> Self {
        if extent_px == 0 {
            return Self::ZERO;
        }
        let px = px.max(0) as u64;
        let raw = (px * FRAC_ONE as u64 + extent_px as u64 / 2) / extent_px as u64;
        Self::from_raw(raw.min(FRAC_ONE as u64) as u32)
    }
}

impl fmt::Debug for Percent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}%", self.whole())
    }
}

/// Width and height in whole percent of the center region, capped at 100.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PercentSize {
    pub width: u16,
    pub height: u16,
}

impl PercentSize {
    /// Creates a size, capping each axis at 100.
    pub const fn new(width: u16, height: u16) -> Self {
        Self {
            width: if width > 100 { 100 } else { width },
            height: if height > 100 { 100 } else { height },
        }
    }

    /// Width as an opaque percent.
    pub const fn width_percent(self) -> Percent {
        Percent::new(self.width)
    }

    /// Height as an opaque percent.
    pub const fn height_percent(self) -> Percent {
        Percent::new(self.height)
    }

    /// Converts the size to pixels inside a center region.
    pub fn as_px(self, center_width: u32, center_height: u32) -> (u32, u32) {
        (
            self.width_percent().as_px(center_width),
            self.height_percent().as_px(center_height),
        )
    }
}

impl Default for PercentSize {
    fn default() -> Self {
        Self::new(30, 30)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_caps_at_one_hundred() {
        assert_eq!(Percent::new(140), Percent::FULL);
        assert_eq!(PercentSize::new(140, 200), PercentSize::new(100, 100));
    }

    #[test]
    fn percent_round_trip() {
        let half = Percent::new(50);
        assert_eq!(half.as_px(1280), 640);
        assert_eq!(Percent::from_px(640, 1280).whole(), 50);
    }

    #[test]
    fn percent_size_uses_the_center_extent() {
        let size = PercentSize::new(30, 20);
        assert_eq!(size.as_px(1000, 500), (300, 100));
    }
}
