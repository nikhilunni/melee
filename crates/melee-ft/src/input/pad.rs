//! The fighter consumes HSD's normalized GameStatus, not the controller's
//! unsigned wire coordinates. HSD_PadStatus is 0x44 bytes (controller.h),
//! despite the older 0x2C stride in docs/DOLPHIN_RUN.md.

use gekko_math::msl::{fctiwz, sqrtf};

/// `HSD_Pad` bits (sysdolphin/baselib/controller.h).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Buttons(pub u32);
impl Buttons {
    pub const DOWN: Self = Self(1 << 2);
    pub const UP: Self = Self(1 << 3);
    pub const Z: Self = Self(1 << 4);
    pub const R: Self = Self(1 << 5);
    pub const L: Self = Self(1 << 6);
    pub const A: Self = Self(1 << 8);
    pub const B: Self = Self(1 << 9);
    pub const X: Self = Self(1 << 10);
    pub const Y: Self = Self(1 << 11);
    pub const XY: Self = Self(Self::X.0 | Self::Y.0);
    /// Fighter-generated analog-or-digital shield bit, HSD_PAD_LR.
    pub const SHIELD: Self = Self(1 << 31);
    pub const DIGITAL_SHOULDERS: Self = Self(Self::L.0 | Self::R.0);
    pub fn intersects(self, other: Self) -> bool {
        self.0 & other.0 != 0
    }
}
impl std::ops::BitOr for Buttons {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}
impl std::ops::BitOrAssign for Buttons {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Stick {
    pub x: f32,
    pub y: f32,
}

/// Fields read from `HSD_PadGameStatus[fp->x618_player_id]` by
/// Fighter_Spaghetti_8006AD10 (0x8006AD10). These are *after* HSD clamping
/// and scaling: neutral is +0.0, the signed sticks range -1..1, triggers 0..1.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct PadSample {
    /// HSD_PadStatus.button (+0).
    pub buttons: Buttons,
    /// nml_stickX/Y (+20/+24).
    pub stick: Stick,
    /// nml_subStickX/Y (+28/+2C).
    pub cstick: Stick,
    /// nml_analogL/R (+30/+34).
    pub left_trigger: f32,
    pub right_trigger: f32,
}

/// Melee's HSD configuration, gm/gmmain.c:47-55.
const STICK_MAX: f32 = 80.0;
const TRIGGER_MAX: u8 = 140;

/// HSD_PadClampCheck3 (0x80376E90), specialized to Melee's min=0,
/// max=80, shift=1, followed by HSD_PadScale (inlined in 0x8037750C).
/// Input is PADRead's signed, origin-calibrated bytes, *not* wire 128-centering.
/// Every signed-byte pair is covered by the native C oracle.
pub fn normalize_stick(x: i8, y: i8) -> Stick {
    let (mut x, mut y) = (x, y);
    let length = |x: i8, y: i8| {
        let (x, y) = (f32::from(x), f32::from(y));
        // HSD_PadClampCheck3 --fused: only sqrt's double Newton steps fuse.
        // retail 0x80376EF0/0x80376EFC/0x80376F00: separate fmuls/fadds.
        sqrtf(x * x + y * y)
    };
    let mut radius = length(x, y);
    if radius > STICK_MAX {
        x = fctiwz(f32::from(x) * STICK_MAX / radius) as i8;
        y = fctiwz(f32::from(y) * STICK_MAX / radius) as i8;
        radius = length(x, y);
    }
    // Preserve the shift branch even with min=0 (controller.c:193-195).
    // retail 0x80377150/60/6C and 0x803771A8/B4/B8: fmuls, fdivs, fsubs.
    if radius > 1.0e-10_f32 {
        x = fctiwz(f32::from(x) - f32::from(x) * 0.0 / radius) as i8;
        y = fctiwz(f32::from(y) - f32::from(y) * 0.0 / radius) as i8;
    }
    Stick {
        // retail 0x80377718/0x80377758: fdivs; no reciprocal multiplication.
        x: f32::from(x) / STICK_MAX,
        y: f32::from(y) / STICK_MAX,
    }
}

impl PadSample {
    /// Convenience adapter for SDK PADRead samples. HSD's button synthesis
    /// is supplied by the caller; Fighter only interprets the low button bits.
    pub fn from_origin_adjusted(
        buttons: Buttons,
        stick: [i8; 2],
        cstick: [i8; 2],
        triggers: [u8; 2],
    ) -> Self {
        Self {
            buttons,
            stick: normalize_stick(stick[0], stick[1]),
            cstick: normalize_stick(cstick[0], cstick[1]),
            // retail 0x80377808/0x80377838: fdivs.
            left_trigger: f32::from(triggers[0].min(TRIGGER_MAX)) / f32::from(TRIGGER_MAX),
            right_trigger: f32::from(triggers[1].min(TRIGGER_MAX)) / f32::from(TRIGGER_MAX),
        }
    }
}
