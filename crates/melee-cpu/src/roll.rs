//! Rolls, the shield and the island ends they keep clear of: the small
//! scripts the defence, the attack choice and arrival share
//! (ftcpuattack.c's ftCo_800B9F90, 800BA080, 800BA160, 800BA224) and
//! ftCo_0A01.c's island-end distances (ftCo_800A2998, ftCo_800A2A70).
use crate::{
    script::{self, Command as C},
    world::Scene,
};
use gekko_math::{fma::fmadds, msl::sqrtf, HsdRng};
use melee_ft::fighter::Fighter;
use melee_types::{CommonMotionState as S, GroundOrAir};

/// ftCo_800A2A70's result without an island under the fighter.
const NO_ISLAND: f32 = -1.0;
/// A stick byte straight left (-0x7F).
const STICK_LEFT: u8 = (-0x7F_i8) as u8;
/// SetLstickY's byte for straight down (-0x7F).
const STICK_DOWN: u8 = (-0x7F_i8) as u8;

/// ftCo_800A2A70 (0x800A2A70): the distance from the fighter to its
/// island's left end (`left`) or right end; -1 in the air or off any
/// island. 800A2ADC/2AE0: fmuls dy*dy, fmadds dx*dx + it, then the inline
/// sqrtf.
pub(crate) fn island_end_distance(fp: &Fighter, scene: &Scene, left: bool) -> f32 {
    if fp.core.physics.ground_or_air == GroundOrAir::Air {
        return NO_ISLAND;
    }
    let Some(island) = scene.map.island_of_line(fp.core.collision.data.floor.index) else {
        return NO_ISLAND;
    };
    let island = scene.map.island(island);
    let end = if left { island.left } else { island.right };
    let position = fp.core.physics.position;
    let dy = end.y - position.y;
    let dx = end.x - position.x;
    sqrtf(fmadds(dx, dx, dy * dy))
}

/// ftCo_800A2998 (0x800A2998): the fighter stands within 5 * `scale` of
/// its island's nearer end (x distances; the right end when equal is not
/// nearer, 800A2A14: bge; the bound a double product).
pub(crate) fn near_island_end(fp: &Fighter, scene: &Scene, scale: f32) -> bool {
    if fp.core.physics.ground_or_air == GroundOrAir::Air {
        return false;
    }
    let Some(island) = scene.map.island_of_line(fp.core.collision.data.floor.index) else {
        return false;
    };
    let island = scene.map.island(island);
    let x = fp.core.physics.position.x;
    let to_right = gekko_math::msl::fabsf(island.right.x - x);
    let to_left = gekko_math::msl::fabsf(island.left.x - x);
    let bound = 5.0 * f64::from(scale);
    if to_right < to_left {
        f64::from(to_right) < bound
    } else {
        f64::from(to_left) < bound
    }
}

/// ftCo_800BA160 (0x800BA160): roll right (R with the stick right).
pub(crate) fn roll_right(fp: &mut Fighter) {
    roll(fp, 0x7F);
}

/// ftCo_800BA224 (0x800BA224): roll left.
pub(crate) fn roll_left(fp: &mut Fighter) {
    roll(fp, STICK_LEFT);
}

fn roll(fp: &mut Fighter, stick: u8) {
    let cpu = &mut fp.core.cpu;
    script::neutral_stick(cpu);
    script::command(cpu, C::ReleaseR);
    script::command1(cpu, C::WaitFor, 1);
    script::command1(cpu, C::SetLstickX, stick);
    script::command(cpu, C::PressR);
    script::command1(cpu, C::WaitFor, 2);
    script::command(cpu, C::ReleaseR);
    script::command1(cpu, C::WaitFor, 5);
    script::command1(cpu, C::SetLstickX, 0);
    script::command(cpu, C::Done);
}

/// ftCo_800B9F90 (0x800B9F90): shield (R with a neutral stick). Already
/// in Guard, below level 9 it holds the shield for (9 - level) times
/// 4 to 7 ticks (`(int)(4 * Randf()) + 4`: fmuls, fctiwz).
pub(crate) fn shield(fp: &mut Fighter, rng: &mut HsdRng) {
    let guarding = fp.core.motion_state.action.0 == S::Guard as u16;
    let cpu = &mut fp.core.cpu;
    if !guarding {
        script::neutral_stick(cpu);
        script::command(cpu, C::PressR);
        script::command(cpu, C::Done);
        return;
    }
    let scale = 9 - cpu.level;
    script::neutral_stick(cpu);
    script::command(cpu, C::PressR);
    if scale != 0 {
        let ticks = scale * (gekko_math::msl::fctiwz(4.0 * rng.randf()) + 4);
        script::command1(cpu, C::WaitFor, ticks as u8);
    }
    script::command(cpu, C::Done);
}

/// ftCo_800BA080 (0x800BA080): a down special (B with the stick down),
/// unless already in one of the kind's motions 0x16F..0x178.
pub(crate) fn down_special(fp: &mut Fighter) {
    let motion = fp.core.motion_state.action.0;
    let cpu = &mut fp.core.cpu;
    script::neutral_stick(cpu);
    if (0x16F..=0x178).contains(&motion) {
        script::command(cpu, C::Done);
        return;
    }
    script::command(cpu, C::ReleaseB);
    script::command1(cpu, C::WaitFor, 1);
    script::command1(cpu, C::SetLstickX, 0);
    script::command1(cpu, C::SetLstickY, STICK_DOWN);
    script::command(cpu, C::PressB);
    script::command(cpu, C::Done);
}
