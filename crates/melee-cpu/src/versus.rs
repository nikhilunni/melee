//! CPU mode 4 (and 11, 12 and every unlisted mode): a Versus CPU, the
//! CpuKind the CSS gives a door switched to CPU (gm_SetupPlayerDefaults,
//! gm_1601.c:3509). ftCo_800B24B8 escapes stage dangers, otherwise picks
//! the fighter to go after and heads for it, then chooses a behaviour.
use crate::{danger, route, select, targets, world::Scene};
use gekko_math::HsdRng;
use melee_ft::fighter::Fighter;

/// ftCo_800B24B8 (0x800B24B8), and ftCo_800AEFB8 (0x800AEFB8), the same
/// for a partner whose player's fighter is gone; `frame` is its r1.
pub fn decide(fp: &mut Fighter, scene: &mut Scene, rng: &mut HsdRng, frame: u32) {
    scene.stale.think_frame.set(Some(frame));
    if let Some(escape) = danger::escape_needed(fp, scene) {
        danger::escape(fp, scene, rng, escape);
        return;
    }
    fp.core.cpu.target = targets::star_carrier(fp, scene);
    if fp.core.cpu.target.is_some() {
        // ftCo_CpuSetTargetFlags, then ftCo_CpuTargetAct: only an item (the
        // Starman or the Hammer) makes a star carrier.
        unreachable!("ftCo_800A5CE0: a star or hammer carrier needs items");
    }
    init_no_target(fp, scene, rng, true);
    targets::update_common_item(fp, scene);
    targets::update_special_item(fp, scene);
    act_on_no_target(fp, scene, rng);
}

/// ftCo_CpuInitNoTarget (inlined into ftCo_800B24B8 and its siblings): no
/// one to flee, so go after the nearest opponent; `fear` sets xF9_b3 and
/// xF9_b6.
pub(crate) fn init_no_target(fp: &mut Fighter, scene: &Scene, rng: &mut HsdRng, fear: bool) {
    let cpu = &mut fp.core.cpu;
    cpu.xf8_b0 = false;
    cpu.xf9_b2 = true;
    cpu.xf9_b4 = true;
    cpu.xf9_b3 = fear;
    cpu.xf9_b5 = true;
    cpu.xf9_b6 = fear;
    cpu.xf9_b7 = true;
    cpu.xf9_b1 = false;
    fp.core.cpu.target = targets::nearest_opponent(fp, scene, rng);
}

/// ftCo_CpuActOnNoTarget: head for the targeted item, or for the opponent
/// ftCo_800A53DC prefers (else the target), then choose a behaviour.
fn act_on_no_target(fp: &mut Fighter, scene: &mut Scene, rng: &mut HsdRng) {
    if crate::should_act(&mut fp.core.cpu) {
        if fp.core.cpu.item_target.is_some() {
            unimplemented!("ftCo_800A866C: heading for the targeted item");
        }
        let chosen = targets::attack_target(fp, scene).or(fp.core.cpu.target);
        if let Some(target) = chosen {
            route::toward_fighter(fp, scene, target);
        }
    }
    scene.stale.caller_frame.set(scene.stale.think_frame.get());
    select::choose_behavior(fp, scene, rng);
}
