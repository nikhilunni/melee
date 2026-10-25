//! Behaviour 17 (ftCo_800AC434): in Kongo Jungle's barrel, fire when it
//! points up and slightly right; shot out of it, tap A now and then.
use crate::script::{self, Command as C};
use crate::world::Scene;
use gekko_math::HsdRng;
use melee_ft::fighter::Fighter;
use melee_types::CommonMotionState as S;

/// @350: the lowest aim that fires, -15 degrees (a double).
const LOWEST_AIM: f64 = -0.261_799_385_771_155_36;
/// @303.
const TAP_CHANCE: f32 = 0.5;

/// ftCo_800AC434 (0x800AC434).
pub fn ride(fp: &mut Fighter, scene: &Scene, rng: &mut HsdRng) {
    let motion = fp.core.motion_state.action.0;
    if motion == S::BarrelWait as u16 {
        // Ground_801C4DA0: stage_info.x73C, the barrel's aim; fcmpo against
        // doubles, `>=` and `<=` (cror).
        let aim = f64::from(scene.barrel_aim);
        if (LOWEST_AIM..=0.0).contains(&aim) {
            tap_a(fp);
        }
        script::command(&mut fp.core.cpu, C::Done);
    } else if motion == S::Barrel as u16 {
        // 800AC4AC..800AC4C4: x7C % ((10 - level) * 5), divw.
        let cpu = &fp.core.cpu;
        let period = (10 - cpu.level) * 5;
        if period == 0 {
            unimplemented!("ftCo_800AC434: divw by zero at level 10");
        }
        if cpu.reaction_timer.wrapping_rem(period) == 0 && rng.randf() < TAP_CHANCE {
            tap_a(fp);
        }
        script::command(&mut fp.core.cpu, C::Done);
    } else {
        script::return_to_previous(&mut fp.core.cpu);
    }
}

/// ftCo_CpuTapA: press A a tick.
fn tap_a(fp: &mut Fighter) {
    let cpu = &mut fp.core.cpu;
    script::command(cpu, C::PressA);
    script::command1(cpu, C::WaitFor, 1);
    script::command(cpu, C::ReleaseA);
}
