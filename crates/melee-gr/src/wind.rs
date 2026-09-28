//! Stage wind devices (ftCo_800C06E8 registrations), sampled by each fighter
//! in ftColl_GetWindOffsetVec (0x8007B924) at its position after velocity
//! integration and before the moving-floor offset.
//!
//! Dream Land is the only ported stage with one: Whispy Woods' gust
//! (fn_802112F4, 0x802112F4) pushes along x inside a strict rectangle.
use hsd_types::Vec3;

/// One tick's wind device state, copied into the fighter physics phase.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Wind {
    gust: Option<Gust>,
}

/// An active rectangular gust.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gust {
    /// Added to the position each tick inside the rectangle (Dream Land's
    /// is x only).
    pub velocity: Vec3,
    /// Either order; the test is strict on both sides.
    pub x_bounds: [f32; 2],
    pub y_bounds: [f32; 2],
}

impl Wind {
    /// No device, or a registered device that is not blowing.
    pub const CALM: Wind = Wind { gust: None };

    pub const fn gust(gust: Gust) -> Self {
        Self { gust: Some(gust) }
    }

    /// fn_802112F4 (0x802112F4): the offset at `position` (the fighter's
    /// cur_pos, ftLib_80086644).
    pub fn at(&self, position: Vec3) -> Vec3 {
        match self.gust {
            Some(gust) if gust.contains(position) => gust.velocity,
            _ => Vec3::ZERO,
        }
    }
}

impl Gust {
    /// grOldPupupu_8021128C (0x8021128C): order each pair, then test
    /// strictly inside both ranges.
    fn contains(&self, position: Vec3) -> bool {
        let inside = |value: f32, [a, b]: [f32; 2]| {
            let (low, high) = if a > b { (b, a) } else { (a, b) };
            low < value && value < high
        };
        inside(position.x, self.x_bounds) && inside(position.y, self.y_bounds)
    }
}
