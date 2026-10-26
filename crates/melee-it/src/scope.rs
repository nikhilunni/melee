//! The Super Scope's charge table (itSScopeAttributes x4..x30) and the
//! arithmetic its holder and the scope share (itsscope.c: it_80291CF4,
//! it_80291D38, it_80291DAC, it_80291F14).

/// itSScopeAttributes x4 / x8 / xC[10].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScopeCharges {
    /// x4: frames of charge that still fire an uncharged (level 0) shot.
    pub uncharged_frames: i32,
    /// x8: the charge's levels; x8 * 8 frames reach the full level 9.
    pub levels: i32,
    /// xC[10]: each level's cost in shots, a float the code truncates
    /// (fctiwz).
    pub costs: [f32; 10],
}

/// The highest charge level.
pub const MAX_LEVEL: i32 = 9;

impl ScopeCharges {
    /// From the scope's special attributes (words 1, 2 and floats 3..13).
    pub fn from_attributes(attributes: &[f32]) -> Self {
        let mut costs = [0.0; 10];
        costs.copy_from_slice(&attributes[3..13]);
        Self {
            uncharged_frames: attributes[1].to_bits() as i32,
            levels: attributes[2].to_bits() as i32,
            costs,
        }
    }

    /// it_80291CF4 (80291CF4): the level of `frames` charged: 0 up to x4,
    /// then a level per eight frames (srawi/addze: a signed division), 9
    /// from x8 * 8.
    pub fn level(&self, frames: i32) -> i32 {
        if frames <= self.uncharged_frames {
            0
        } else if frames < self.levels * 8 {
            frames / 8
        } else {
            MAX_LEVEL
        }
    }

    /// it_80291D38 (80291D38): a level's cost (xC[level], fctiwz).
    pub fn cost(&self, level: i32) -> i32 {
        assert!((0..=MAX_LEVEL).contains(&level), "it_80291D38: level {level}");
        gekko_math::msl::fctiwz(self.costs[level as usize])
    }

    /// it_80291DAC (80291DAC): the level a shot of `frames` charge fires
    /// with `ammo` shots left: -1 when empty; a level it cannot afford
    /// falls back to the highest that leaves none or some.
    pub fn affordable(&self, ammo: i32, frames: i32) -> i32 {
        if ammo <= 0 {
            return -1;
        }
        let mut level = self.level(frames);
        if ammo - self.cost(level) >= 0 {
            return level;
        }
        while level > 0 {
            let left = ammo - self.cost(level);
            if left == 0 {
                return level;
            }
            if left > 0 {
                return level + 1;
            }
            level -= 1;
        }
        unimplemented!("it_80291DAC (itsscope.c:117): no level affordable returns the item pointer")
    }

    /// it_80291F14 (80291F14): the level's cost off `ammo`, never below 0.
    pub fn spend(&self, ammo: i32, level: i32) -> i32 {
        (ammo - self.cost(level)).max(0)
    }
}
