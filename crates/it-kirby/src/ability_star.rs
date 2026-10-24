//! The star a lost copy ability leaves as (It_Kind_Unk1), it_2ADA.c
//! (802ADA1C..802ADBE4).
//!
//! ftKb_SpecialN_800F5D04 throws it from above Kirby when a taunt or a hit
//! takes his ability. It belongs to nobody, has no hitbox and no event
//! callback: it falls under its own gravity for its lifetime (50 frames,
//! which end it 9 units above where it began) and ends early on a floor.
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::ANIM_UPDATE,
    ItemAnimationContext, ItemCollisionContext, ItemCore, ItemLogic, ItemPhysicsContext,
    ItemStateRow, SpawnItem,
};
use melee_types::ItemKind;

pub struct AbilityStar;

/// ftData.x48_items index (ftKb_Init_OnLoad registers it third).
pub const ARTICLE_INDEX: u32 = 2;
/// it_803F6C88's anim_id column: one motion state, article state 0.
pub const ARTICLE_STATES: [i32; 1] = [0];
/// itUnkAttributes: the lifetime alone.
pub const SPECIAL_ATTRIBUTES: u32 = 1;

static STATES: [ItemStateRow; 1] = [ItemStateRow {
    animation_id: ARTICLE_STATES[0],
    animation,
    physics,
    collision,
}];

/// it_3F2F.c's row has the owner-removed callback alone (it_802ADBE4).
impl ItemLogic for AbilityStar {
    const KIND: ItemKind = ItemKind::Unk1;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// it_802ADAF0 clears xDC8 x15 (it_8026B3A8).
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// it_802ADA1C once Item_80268B18 returns: both lifetimes from the
    /// special attribute (it_80275158), then it_802ADAF0: never grabbable,
    /// motion 0, and the ECB no longer turns with the model.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        _spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        let lifetime = assets.special_attributes[0];
        item.life_timer = lifetime;
        item.half_life = lifetime * common.half_life_scale;
        item.grabbable = false;
        item.change_motion_with(0, ARTICLE_STATES[0], ANIM_UPDATE, assets);
        // xDCE b7 off: the spinning star's ECB stays upright.
        item.ecb_follows_rotation = false;
    }
}

/// it_2ADA_UnkMotion0_Anim (802ADB48): the lifetime counts down.
fn animation(item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

/// it_2ADA_UnkMotion0_Phys (802ADB80): it_80272860 at ItemAttr x10 / x14.
fn physics(item: &mut ItemCore, ctx: &ItemPhysicsContext<'_>) {
    item.fall(ctx.assets.fall_acceleration, ctx.assets.fall_speed_limit);
}

/// it_2ADA_UnkMotion0_Coll (802ADBB0) -> it_8026DFB0: an airborne pass; a
/// floor, a wall (it_80276308) or a ceiling (it_802763E0) ends the star.
/// Any of them ends it (`cond & 0xF`, itgroundcoll.c:381-385): recorded on
/// a platform above Kirby and on Yoshi's Story's side wall, thrown from
/// Randall under the stage.
fn collision(item: &mut ItemCore, ctx: &mut ItemCollisionContext<'_>) -> bool {
    item.air_contact_bits(ctx.map) & 0xF != 0
}
