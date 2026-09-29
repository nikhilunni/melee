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

/// The virtual buttons HSD synthesizes from the main and C-sticks
/// (HSD_PadADConvert, controller.c:279-291): up, down, left, right.
pub const STICK_DIRECTIONS: u32 = 0x00ff_0000;
const MAIN_STICK_DIRECTIONS: [u32; 4] = [0x1_0000, 0x2_0000, 0x4_0000, 0x8_0000];
const C_STICK_DIRECTIONS: [u32; 4] = [0x10_0000, 0x20_0000, 0x40_0000, 0x80_0000];
/// default_libinfo_data: adc_th 30 and adc_angle 0, which Melee keeps.
const ADC_THRESHOLD: f32 = 30.0;
const ADC_ANGLE: f32 = 0.0;

/// HSD_PadADConvertCheck1 (803771D4): one stick's direction bits from its
/// clamped signed bytes. The length is sqrtf(fmadds(y, y, x * x)) (retail
/// 80377264); the sector bounds are double constants widened by half of
/// adc_angle, compared in double.
fn stick_direction(x: i8, y: i8, [up, down, left, right]: [u32; 4]) -> u32 {
    use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};
    let (fx, fy) = (f32::from(x), f32::from(y));
    let length = sqrtf(gekko_math::fma::fmadds(fy, fy, fx * fx));
    let angle = if x == 0 {
        (if y >= 0 { FRAC_PI_2 } else { -FRAC_PI_2 }) as f32
    } else {
        melee_lb::trigf::atan2f(fy, fx)
    };
    // retail 80377354: fmuls.
    let half = f64::from(0.5 * ADC_ANGLE);
    if length < ADC_THRESHOLD {
        return 0;
    }
    let angle = f64::from(angle);
    let three_quarters = 3.0 * FRAC_PI_4;
    let mut bits = 0;
    if angle < -three_quarters + half {
        bits |= left;
    }
    if angle >= -three_quarters - half && angle <= -FRAC_PI_4 + half {
        bits |= down;
    }
    if angle > -FRAC_PI_4 - half && angle < FRAC_PI_4 + half {
        bits |= right;
    }
    if angle >= FRAC_PI_4 - half && angle <= three_quarters + half {
        bits |= up;
    }
    if angle > three_quarters - half {
        bits |= left;
    }
    bits
}

/// HSD_PadADConvert: both sticks' direction bits from clamped signed bytes.
pub fn stick_directions(stick: [i8; 2], cstick: [i8; 2]) -> Buttons {
    Buttons(
        stick_direction(stick[0], stick[1], MAIN_STICK_DIRECTIONS)
            | stick_direction(cstick[0], cstick[1], C_STICK_DIRECTIONS),
    )
}

/// The clamped signed byte behind a normalized axis (HSD_PadScale divides
/// by 80, so the product rounds back to it).
fn stick_byte(axis: f32) -> i8 {
    let scaled = axis * STICK_MAX;
    if scaled >= 0.0 {
        fctiwz(scaled + 0.5) as i8
    } else {
        -(fctiwz(-scaled + 0.5) as i8)
    }
}

impl PadSample {
    /// The signed `PADStatus` stick bytes behind the normalized sticks, as
    /// the pad queue held them. Exact when a stick lay inside HSD's 80-unit
    /// circle; a clamped stick gives the clamped bytes.
    pub fn raw_sticks(&self) -> super::RawSticks {
        super::RawSticks {
            stick: [stick_byte(self.stick.x), stick_byte(self.stick.y)],
            cstick: [stick_byte(self.cstick.x), stick_byte(self.cstick.y)],
        }
    }
    /// The sample as HSD's game status carries it: the virtual stick
    /// direction bits follow the sticks (HSD_PadRenewMasterStatus runs
    /// HSD_PadADConvert on every read), whatever the caller supplied.
    pub fn with_stick_directions(mut self) -> Self {
        let directions = stick_directions(
            [stick_byte(self.stick.x), stick_byte(self.stick.y)],
            [stick_byte(self.cstick.x), stick_byte(self.cstick.y)],
        );
        self.buttons = Buttons((self.buttons.0 & !STICK_DIRECTIONS) | directions.0);
        self
    }

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
