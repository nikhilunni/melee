//! Vanish's smoke, itseakvanish.c (802B1C60..802B1DE0): an invisible item
//! left at Sheik's hip where she disappears, whose script owns the
//! explosion's hitbox.
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::ANIM_UPDATE,
    ItemAnimationContext, ItemCollisionContext, ItemCore, ItemLogic, ItemPhysicsContext,
    ItemStateRow, SpawnItem,
};
use melee_types::ItemKind;

pub struct SeakVanish;

/// it_803F70B8's anim_id column.
pub const ARTICLE_STATES: [i32; 1] = [0];
/// ftData.x48_items index (ftSk_Init_OnLoad registers it third).
pub const ARTICLE_INDEX: u32 = 2;

static STATES: [ItemStateRow; 1] = [ItemStateRow {
    animation_id: ARTICLE_STATES[0],
    animation: smoke,
    physics: no_physics,
    collision: no_collision,
}];

impl ItemLogic for SeakVanish {
    const KIND: ItemKind = ItemKind::SeakVanish;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// Nothing picks the smoke up (it_8026B3A8 clears xDC8 x15 at once).
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// it_802B1D40 (802B1D40), after Item_80268B18 in it_802B1C60: the owner
    /// and a 60-frame life (then it_8027518C's common one), command
    /// variable 0 cleared.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        _spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        item.command_variables[0] = 0;
        // it_8026BB44 -> it_80272A3C: the model is hidden.
        item.hidden = true;
        // it_8026B3A8: not grabbable. it_8026BD24 (xDD0 b3) has no port
        // consumer.
        item.grabbable = false;
        // it_8027518C: the common explosion lifetime and no destroy effect.
        item.life_timer = common.explosion_lifetime;
        item.destroy_effect_suppressed = true;
        item.mark_exploding();
        item.change_motion_with(0, ARTICLE_STATES[0], ANIM_UPDATE, assets);
    }
    /// itSeakVanish_Logic42_DmgDealt: the smoke stays.
    fn damage_dealt(_item: &mut ItemCore, _context: &melee_it::ItemEventContext<'_>) -> bool {
        false
    }
}

/// itSeakvanish_UnkMotion0_Anim -> it_802751D8: the smoke lasts its
/// lifetime.
fn smoke(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}
