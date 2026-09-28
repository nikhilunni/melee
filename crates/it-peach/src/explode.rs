//! Peach Bomber's explosion, itpeachexplode.c (802BD158..802BD2F8): an
//! invisible item whose script owns the blast hitbox, left where Peach's
//! hip was when her bomber connected.
use crate::{no_collision, no_physics};
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::ANIM_UPDATE,
    ItemAnimationContext, ItemCore, ItemEvent, ItemLogic, ItemStateRow, SpawnItem,
};
use melee_types::ItemKind;

pub struct PeachExplode;

/// it_803F7488's anim_id column: state 0 after an ordinary bomber, state 1
/// after a smash-input one.
pub const ARTICLE_STATES: [i32; 2] = [0, 1];
/// it_80272C08: efSync_Spawn(0x410) and Item_8026AE84(item, 0x74, 0x7F, 0x40).
const BLAST_EFFECT: u16 = 0x410;
const BLAST_SOUND: u32 = 0x74;

static STATES: [ItemStateRow; 2] = [row(0), row(1)];
const fn row(motion: usize) -> ItemStateRow {
    ItemStateRow {
        animation_id: ARTICLE_STATES[motion],
        animation: burn,
        physics: no_physics,
        collision: no_collision,
    }
}

impl ItemLogic for PeachExplode {
    const KIND: ItemKind = ItemKind::PeachExplode;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// Nothing picks the blast up (it_8026B3A8 clears xDC8 x15 at once).
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// it_802BD248 (802BD248): the spawner's set-up after it_802BD158,
    /// `spawn_argument` carrying the smash flag (mv.pe.specials.x0).
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        // ip->owner is the spawn's parent already; xD44 = 60 is overwritten
        // by it_8027518C below; xDAC (command variable 0) starts at zero.
        item.command_variables[0] = 0;
        // it_8026BB44 -> it_80272A3C: the model is hidden.
        item.hidden = true;
        // it_80272C08: the blast effect and sound, then it_80274C60 clears
        // xDC8 xC, so contacts no longer put the item into hitlag.
        item.events.push(ItemEvent::Effect {
            id: BLAST_EFFECT,
            position: item.position,
        });
        item.sound_requests.push(BLAST_SOUND);
        item.hitlag_enabled = false;
        // it_8026B3A8: not grabbable. it_8026BD24 (xDD0 b3) has no port
        // consumer.
        item.grabbable = false;
        // it_8027518C: the common explosion lifetime and no destroy effect.
        item.life_timer = common.explosion_lifetime;
        item.destroy_effect_suppressed = true;
        let motion = u16::from(spawn.spawn_argument != 0);
        item.change_motion_with(
            motion,
            ARTICLE_STATES[usize::from(motion)],
            ANIM_UPDATE,
            assets,
        );
        // it_80274574 -> it_80274594 rescales the model to its own scale.
    }
}

/// itPeachexplode_UnkMotion1_Anim -> it_802751D8: the blast lasts its
/// lifetime.
fn burn(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}
