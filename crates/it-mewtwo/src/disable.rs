//! Disable's projectile (It_Kind_Mewtwo_Disable), itmewtwodisable.c
//! (802C49E0..802C4D10). Mewtwo's accessory spawns it ahead of the left
//! hand on the script's flag; it flies straight at attribute x4 until its
//! lifetime ends, a wall or ceiling ends it, its hit connects, or Mewtwo's
//! Disable animation ends (itMewtwoDisable_Logic67_Destroy).
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::ANIM_UPDATE,
    DisableState, ItemAnimationContext, ItemCollisionContext, ItemControl, ItemCore, ItemEvent,
    ItemEventContext, ItemLogic, ItemPhysicsContext, ItemScratch, ItemStateRow, SpawnItem,
};
use melee_types::ItemKind;

pub struct MewtwoDisable;

/// ftData.x48_items index (ftMt_Init_OnLoad registers it first).
pub const ARTICLE_INDEX: u32 = 0;
/// it_803F7750's anim_id column: one motion state, article state 0.
pub const ARTICLE_STATES: [i32; 1] = [0];
/// itMDisableAttributes: two words.
pub const SPECIAL_ATTRIBUTES: u32 = 2;

/// itMDisableAttributes (it/itCommonItems.h).
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// +0: the lifetime (it_80275158).
    fn lifetime(&self) -> f32 {
        self.0[0]
    }
    /// +4: the speed along the facing.
    fn speed(&self) -> f32 {
        self.0[1]
    }
}

static STATES: [ItemStateRow; 1] = [ItemStateRow {
    animation_id: ARTICLE_STATES[0],
    animation,
    physics,
    collision,
}];

fn creator(item: &ItemCore) -> Option<u8> {
    match &item.scratch {
        ItemScratch::MewtwoDisable(state) => state.owner,
        _ => None,
    }
}

/// it_3F2F.c's Logic67 row.
impl ItemLogic for MewtwoDisable {
    const KIND: ItemKind = ItemKind::MewtwoDisable;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// itMewtwoDisable_Logic67_SpawnMewtwoDisable clears xDC8 x15
    /// (it_8026B3A8).
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// itMewtwoDisable_Logic67_SpawnMewtwoDisable (802C4A40) once
    /// Item_80268B18 returns: the creator (itemVar owner), it_802C4B38 and
    /// it_8026B3A8. xDCC b3 (the blast-zone check) is already set.
    ///
    /// it_802C4B38 (802C4B38): the speed along the facing (802C4B74: fmuls)
    /// times the creator's x34_scale.y (802C4B78: fmuls; 1 for every
    /// supported fighter), the lifetime, then it_802C4BB8: motion 0 and
    /// the hitboxes' radii times that scale again (it_802755C0).
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        _spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        /// ftLib_80086A0C: scaled fighters are not supported.
        const OWNER_SCALE: f32 = 1.0;
        let a = Attributes(&assets.special_attributes);
        item.scratch = ItemScratch::MewtwoDisable(DisableState { owner: item.owner });
        item.velocity.x = (a.speed() * item.facing) * OWNER_SCALE;
        item.velocity.y = 0.0;
        item.velocity.z = 0.0;
        // it_80275158: both timers.
        let lifetime = a.lifetime();
        item.life_timer = lifetime;
        item.half_life = lifetime * common.half_life_scale;
        item.change_motion_with(0, ARTICLE_STATES[0], ANIM_UPDATE, assets);
        item.grabbable = false;
    }
    /// itMewtwoDisable_Logic67_Destroy (802C49E0), from Mewtwo: Item_8026A8EC
    /// at once.
    fn control(item: &mut ItemCore, control: ItemControl, _assets: &ItemAssets) {
        match control {
            ItemControl::Remove => {
                item.events.push(ItemEvent::DestroyEffects);
                item.effects_destroyed = true;
                item.destroyed = true;
            }
            _ => unimplemented!("Disable item control {control:?}"),
        }
    }
    /// itMewtwoDisable_Logic67_Destroyed (802C4A00): the creator, while it
    /// still is one, forgets the projectile
    /// (ftMt_SpecialLw_ClearDisableGObj).
    fn notifies_owner(item: &ItemCore) -> bool {
        creator(item).is_some()
    }
    /// itMewtwoDisable_Logic67_EvtUnk (802C4CD4): the creator goes, then
    /// it_8026B894.
    fn owner_removed(item: &mut ItemCore, owner: u8) {
        if let ItemScratch::MewtwoDisable(state) = &mut item.scratch {
            if state.owner == Some(owner) {
                state.owner = None;
            }
        }
        if item.owner == Some(owner) {
            item.owner = None;
        }
    }
    /// itMewtwoDisable_Logic67_DmgDealt.
    fn damage_dealt(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itMewtwoDisable_Logic67_Reflected -> it_80273030.
    fn reflected(item: &mut ItemCore, ctx: &ItemEventContext<'_>) -> bool {
        item.reverse_on_reflect(ctx.reflected_speed);
        false
    }
    /// itMewtwoDisable_Logic67_Clanked.
    fn clanked(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itMewtwoDisable_Logic67_HitShield.
    fn hit_shield(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itMewtwoDisable_Logic67_Absorbed.
    fn absorbed(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itMewtwoDisable_Logic67_ShieldBounced.
    fn shield_bounced(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
}

/// itMewtwodisable_UnkMotion0_Anim (802C4C1C): Item_TickLifetime.
fn animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.life_timer <= 0.0 {
        return true;
    }
    item.life_timer -= 1.0;
    false
}

/// itMewtwodisable_UnkMotion0_Phys (802C4C40) is empty: the velocity stays.
fn physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}

/// itMewtwodisable_UnkMotion0_Coll (802C4C44): it_8026D9A0's pass that
/// never lands; a wall (it_80276308) or ceiling (it_802763E0) ends it.
fn collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    item.airborne_pass(ctx.map);
    let walls = item.wall_bits();
    let ceiling = item.ceiling_bits();
    walls | ceiling != 0
}
