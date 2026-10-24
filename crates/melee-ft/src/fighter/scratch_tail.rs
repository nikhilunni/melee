//! The two motion-scratch words a later state can read back.
//!
//! Retail keeps every state's variables in one union at fp+0x2340, which
//! Fighter_ChangeMotionState (0x800693AC) does not clear. Yoshi's plain
//! GuardOn (ftYs_Init_8012BECC, 0x8012BECC) neither clears the reflect and
//! powershield bits (x221C_b1/b2) nor writes their countdowns (guard x14 and
//! x18, fp+0x2354 and fp+0x2358) as ftCo_800924C0 does (0x80092504..10), so
//! ftCo_80093BC0 counts down, as floats, whatever the last state to write
//! those two words left there: a launch's jump buffer, a special fall's
//! landing lag, an entry's scale.
//!
//! The port's scratch is typed per state and does not share storage, so the
//! fighter carries an image of the two words. A state whose scratch reaches
//! them names what it has written (`FighterCore::scratch_tail_writes` for
//! the common rows, `CharacterCallbacks::SCRATCH_TAIL` for a character's);
//! a motion change folds that into the image (`Fighter::keep_scratch_tail`).
//! `melee-sim scratch-diff` compares the result with a recording's fighter
//! memory on every tick.
use super::fly_reflect::BounceSurface;
use super::{Fighter, FighterCore, MotionData, COMMON_COUNT};
use melee_types::CommonMotionState as S;

/// Index of the word at mv+0x14 (fp+0x2354) in the image.
pub const WORD_14: usize = 0;
/// Index of the word at mv+0x18 (fp+0x2358) in the image.
pub const WORD_18: usize = 1;

/// What a state has written of one word: the bits it owns and their value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Written {
    mask: u32,
    bits: u32,
}
impl Written {
    const NOTHING: Self = Self { mask: 0, bits: 0 };
    const fn word(bits: u32) -> Self {
        Self { mask: !0, bits }
    }
    const fn float(value: f32) -> Self {
        Self::word(value.to_bits())
    }
    /// A C `bool` or small `int` field: a whole-word store.
    const fn flag(value: bool) -> Self {
        Self::word(value as u32)
    }
    /// One byte of the big-endian word (`index` 0 is the byte at the
    /// word's own address).
    const fn byte(self, index: u32, value: u8) -> Self {
        let shift = 24 - 8 * index;
        Self {
            mask: self.mask | (0xFF << shift),
            bits: (self.bits & !(0xFF << shift)) | ((value as u32) << shift),
        }
    }
    const fn over(self, old: u32) -> u32 {
        (old & !self.mask) | self.bits
    }
}

/// Stands for a pointer into the heap (0x80xxxxxx), whose address the port
/// does not have.
const POINTER: u32 = 0x8000_0000;

/// ftCo_Surface: mv.co.damage.x19.
const fn surface_code(surface: Option<BounceSurface>) -> u8 {
    match surface {
        None => 0,
        Some(BounceSurface::LeftWall) => 1,
        Some(BounceSurface::RightWall) => 2,
        Some(BounceSurface::Ceiling) => 3,
    }
}

impl FighterCore {
    /// What the current state's scratch holds at mv+0x14 and mv+0x18. Only
    /// a common row whose scratch the port keeps in `state_data` answers;
    /// the scratch of an earlier state left in `state_data` does not.
    fn scratch_tail_writes(&self) -> [Written; 2] {
        const NOTHING: [Written; 2] = [Written::NOTHING; 2];
        let id = self.motion_state.id;
        let guard_row = matches!(
            id,
            S::GuardOn | S::Guard | S::GuardOff | S::GuardSetOff | S::GuardReflect
        );
        if usize::from(self.motion_state.action.0) >= COMMON_COUNT && !guard_row {
            return NOTHING;
        }
        match (&self.state_data, id) {
            // mv.co.guard.x14, x18: floats.
            (MotionData::Guard(guard), _) if guard_row => [
                Written::float(guard.reflect_frames),
                Written::float(guard.powershield_frames),
            ],
            // mv.co.damage: x14 is a float; x18..x1B are bytes, of which a
            // launch stores x19 and x1A (ftCo_8008DCE0), a meteor x1B
            // (ftCo_Damage_CalcAngle) and a bounce x18 (ftCo_FlyReflect.c).
            (MotionData::Damage(damage), _) if writes_damage_scratch(id) => {
                let mut bytes = Written::NOTHING
                    .byte(1, surface_code(damage.last_bounce))
                    .byte(2, u8::from(damage.meteor_cancel.is_some()));
                if let Some(frames) = damage.meteor_cancel {
                    bytes = bytes.byte(3, frames);
                }
                if damage.last_bounce.is_some() {
                    bytes = bytes.byte(0, damage.bounce_lock);
                }
                [Written::float(damage.jump_buffer), bytes]
            }
            // mv.co.fallspecial: landing_lag, allow_interrupt
            // (ftCo_80096900, ftCo_FallSpecial.c:43-44).
            (MotionData::FallSpecial(fall), S::FallSpecial | S::FallSpecialF | S::FallSpecialB) => {
                [
                    Written::float(fall.landing_lag),
                    Written::flag(fall.allow_interrupt),
                ]
            }
            // mv.co.walk.slow_anim_rate, middle_anim_rate: ftCo_Walk_Enter
            // passes co_attrs' slow_walk_max and mid_walk_point
            // (ftWalkCommon_800DFCA4, ftwalkcommon.c:93-94).
            (MotionData::Walk(_), S::WalkSlow | S::WalkMiddle | S::WalkFast) => [
                Written::float(self.attributes.walking.slow_walk_max),
                Written::float(self.attributes.walking.mid_walk_point),
            ],
            // mv.co.turn.just_turned (ftCo_Turn.c:55).
            (MotionData::Turn(turn), S::Turn) => {
                [Written::NOTHING, Written::flag(turn.just_turned)]
            }
            // mv.co.turnrun.x14: whether the skid has paused the animation
            // (ftCo_TurnRun.c:51, 65).
            (MotionData::TurnRun(turn), S::TurnRun) => {
                [Written::flag(turn.animation_paused), Written::NOTHING]
            }
            // mv.co.unk_deadup: the camera-space position's y and z
            // (fp+0x2350, ftCo_800D4580).
            (
                MotionData::Life(super::life::LifeState::ScreenKo(ko)),
                S::DeadUpFall
                | S::DeadUpFallHitCamera
                | S::DeadUpFallHitCameraFlat
                | S::DeadUpFallIce
                | S::DeadUpFallHitCameraIce,
            ) => [Written::float(ko.position.y), Written::float(ko.position.z)],
            // mv.co.capturedamage.x18: the joint the victim is pulled to, a
            // pointer (ftCo_CatchPull.c:50-59, ftCo_CatchWait.c:26). Any
            // address reads as a negative float below every countdown.
            (MotionData::Catch { .. }, S::CatchPull | S::CatchDashPull | S::CatchWait) => {
                [Written::NOTHING, Written::word(POINTER)]
            }
            // mv.co.damageice.ice_coll (mv+0x8): its left.y and right.x
            // (ftCo_DamageIce.c:74-77).
            (MotionData::Frozen(frozen), S::DamageIce) => [
                Written::float(frozen.collision_box.left.y),
                Written::float(frozen.collision_box.right.x),
            ],
            // mv.co.bury.coll_box (mv+0x4): its right corner.
            (MotionData::Bury(bury), S::Bury | S::BuryWait) => [
                Written::float(bury.collision_box.right.x),
                Written::float(bury.collision_box.right.y),
            ],
            // mv.co.yoshiegg.x14, x18.x: the growth frames left and the
            // root's scale at the swallow (ftCo_YoshiEgg.c:123-126).
            (MotionData::YoshiEgg(egg), S::YoshiEgg | S::KirbyYoshiEgg) => [
                Written::float(egg.growth_remaining),
                Written::float(egg.root_scale.x),
            ],
            // mv.co.entry.x14: the animated scale's x and y
            // (ft_0C31.c:35-36).
            (MotionData::Entry(entry), S::Entry | S::EntryStart | S::EntryEnd) => [
                Written::float(entry.current_scale.x),
                Written::float(entry.current_scale.y),
            ],
            _ => NOTHING,
        }
    }
}

impl Fighter {
    /// The two words as retail's fighter holds them now: the image, under
    /// what the current row has written.
    pub fn scratch_tail(&self) -> [u32; 2] {
        let image = self.core.scratch_tail_image;
        if usize::from(self.core.motion_state.action.0) >= COMMON_COUNT {
            let written = self.character.scratch_tail(self.core.motion_state.action);
            if written != [None; 2] {
                return [
                    written[WORD_14].unwrap_or(image[WORD_14]),
                    written[WORD_18].unwrap_or(image[WORD_18]),
                ];
            }
        }
        let writes = self.core.scratch_tail_writes();
        [
            writes[WORD_14].over(image[WORD_14]),
            writes[WORD_18].over(image[WORD_18]),
        ]
    }

    /// Fold the current row's words into the image. A motion change does
    /// it before the scratch is replaced, and each proc as it starts, so
    /// an entry that builds the next state's scratch ahead of its motion
    /// change loses nothing an earlier proc wrote.
    pub(super) fn keep_scratch_tail(&mut self) {
        self.core.scratch_tail_image = self.scratch_tail();
    }
}

/// The rows that run on mv.co.damage.
const fn writes_damage_scratch(id: S) -> bool {
    matches!(
        id,
        S::DamageHi1
            | S::DamageHi2
            | S::DamageHi3
            | S::DamageN1
            | S::DamageN2
            | S::DamageN3
            | S::DamageLw1
            | S::DamageLw2
            | S::DamageLw3
            | S::DamageAir1
            | S::DamageAir2
            | S::DamageAir3
            | S::DamageFlyHi
            | S::DamageFlyN
            | S::DamageFlyLw
            | S::DamageFlyTop
            | S::DamageFlyRoll
            | S::FlyReflectWall
            | S::FlyReflectCeil
            | S::DownDamageU
            | S::DownDamageD
    )
}

#[cfg(test)]
mod tests {
    use super::Written;

    /// Retail memory in `slp_ps_fox_yoshi_t1300`: a store replaces the bits
    /// its field owns and leaves the rest of the word as it was.
    #[test]
    fn a_store_replaces_only_the_bits_its_field_owns() {
        let launch_bytes = Written::NOTHING.byte(1, 0).byte(2, 0);
        for (case, written, before, after) in [
            // Tick 1049: Damage's x19 and x1A clear the middle of a stale
            // powershield countdown of 1.0, leaving 0.5.
            ("launch bytes", launch_bytes, 0x3F80_0000, 0x3F00_0000),
            // Tick 1060: the jump buffer is a whole float.
            ("float", Written::float(23.0), 0x0000_0000, 0x41B8_0000),
            ("flag", Written::flag(true), 0x3F80_0000, 0x0000_0001),
            (
                "meteor frames",
                Written::NOTHING.byte(3, 8),
                0x3F00_0000,
                0x3F00_0008,
            ),
            ("nothing", Written::NOTHING, 0x41B8_0000, 0x41B8_0000),
        ] {
            assert_eq!(written.over(before), after, "{case}");
        }
    }
}
