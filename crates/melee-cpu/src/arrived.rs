//! Behaviour 10 (ftCo_800ACD5C): the CPU reached its destination.
use crate::{
    script::{self, Command as C},
    world::Scene,
};
use melee_ft::fighter::Fighter;

/// CPU mode 11: the item-fetching CPU (TODO(meaning)).
const MODE_FETCH_ITEM: i32 = 0xB;
/// PlCo stored script 0x26: TODO(meaning) (Zelda's transformation input).
pub(crate) const SCRIPT_TRANSFORM: usize = 0x26;

/// ftCo_800ACD5C (0x800ACD5C).
pub fn arrived(fp: &mut Fighter, scene: &mut Scene) {
    let cpu = &fp.core.cpu;
    if cpu.xf9_b1 && cpu.xf8_b6 {
        if let Some(target) = cpu.target {
            fp.core.cpu.xf8_b6 = false;
            roll_from(fp, scene, target);
            return;
        }
    }
    let cpu = &fp.core.cpu;
    // gm_GetCurrentGameMode() != GM_TRAINING: the scenes are all versus.
    if cpu.x88 > 0 {
        let cpu = &mut fp.core.cpu;
        script::neutral_stick(cpu);
        script::command1(cpu, C::WaitFor, 1);
        script::command(cpu, C::PressUp);
        script::command1(cpu, C::WaitFor, 1);
        script::command(cpu, C::ReleaseUp);
        script::command(cpu, C::Done);
        cpu.x88 = 0;
        // gm_8016C75C: this player's KOs.
        cpu.x8c = scene.player_kills;
        return;
    }
    if cpu.mode == MODE_FETCH_ITEM && !crate::facts::has_item(fp) {
        script::stored(&mut fp.core.cpu, scene.data, SCRIPT_TRANSFORM);
        return;
    }
    if !crate::select::arrived(fp, scene, 1.0) {
        script::return_to_previous(&mut fp.core.cpu);
        return;
    }
    if fp.character.table().cpu_profile.transform_on_request && fp.core.cpu.xf8_b5 {
        script::stored(&mut fp.core.cpu, scene.data, SCRIPT_TRANSFORM);
        fp.core.cpu.xf8_b5 = false;
        return;
    }
    if !crate::attack::facing_target(fp, scene) {
        crate::behave::turn_around(fp);
        return;
    }
    if fp.core.cpu.level >= 5 && crate::charge::charge_on_arrival(fp) {
        return;
    }
    script::finish_with_neutral_stick(&mut fp.core.cpu);
}

/// ftCo_800BA674 (0x800BA674): roll away from `target` when the island
/// leaves more than twice the kind's reach behind the CPU (PlCo's x20
/// table times a double 2.0), else toward it.
fn roll_from(fp: &mut Fighter, scene: &Scene, target: usize) {
    let reach = 2.0 * f64::from(scene.data.reach[usize::from(fp.core.kind as u8)]);
    let d = fp.core.physics.position.x - scene.fighter(target).core.physics.position.x;
    if f64::from(d) > 0.0 {
        if f64::from(crate::roll::island_end_distance(fp, scene, false)) > reach {
            crate::roll::roll_right(fp);
        } else {
            crate::roll::roll_left(fp);
        }
    } else if f64::from(crate::roll::island_end_distance(fp, scene, true)) > reach {
        crate::roll::roll_left(fp);
    } else {
        crate::roll::roll_right(fp);
    }
}
