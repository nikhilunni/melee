//! Behaviour 10 (ftCo_800ACD5C): the CPU reached its destination.
use crate::{
    script::{self, Command as C},
    world::Scene,
};
use melee_ft::fighter::Fighter;
use melee_types::FighterKind as K;

/// CPU mode 11: the item-fetching CPU (TODO(meaning)).
const MODE_FETCH_ITEM: i32 = 0xB;
/// PlCo stored script 0x26: TODO(meaning) (Zelda's transformation input).
const SCRIPT_TRANSFORM: usize = 0x26;

/// ftCo_800ACD5C (0x800ACD5C).
pub fn arrived(fp: &mut Fighter, scene: &mut Scene) {
    let cpu = &fp.core.cpu;
    if cpu.xf9_b1 && cpu.target.is_some() && cpu.xf8_b6 {
        unimplemented!("ftCo_800ACD5C: ftCo_800BA674 against the target");
    }
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
    if matches!(fp.core.kind, K::Zelda | K::Seak) && fp.core.cpu.xf8_b5 {
        script::stored(&mut fp.core.cpu, scene.data, SCRIPT_TRANSFORM);
        fp.core.cpu.xf8_b5 = false;
        return;
    }
    if !crate::attack::facing_target(fp, scene) {
        crate::behave::turn_around(fp);
        return;
    }
    if fp.core.cpu.level >= 5 && matches!(fp.core.kind, K::Donkey | K::Samus | K::Mewtwo | K::Kirby)
    {
        unimplemented!("ftCo_800ACD5C: {:?}'s neutral-special charge", fp.core.kind);
    }
    script::finish_with_neutral_stick(&mut fp.core.cpu);
}
