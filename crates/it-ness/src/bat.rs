//! The baseball bat, itnessbat.c (802AD478..802AD8D0): a model in Ness's
//! hand for the forward smash. The smash's hitboxes and reflect bubble are
//! the fighter's.
use crate::{no_collision, no_physics};
use melee_it::{
    desc::ItemAssets, state_change::ANIM_UPDATE, ItemAnimationContext, ItemControl, ItemCore,
    ItemLogic, ItemStateRow,
};
use melee_types::ItemKind;

pub struct NessBat;

/// ftData.x48_items index (ftNs_Init_OnLoad).
pub const ARTICLE_INDEX: u32 = 9;
/// it_803F6C68's anim_id column: the swing, and the pose of the results
/// screen's bat (it_802AD590).
pub const ARTICLE_STATES: [i32; 2] = [0, -1];
/// it_802AD478: it_80275158(bat, 1200.0f).
pub const LIFETIME: f32 = 1200.0;
/// ftNs_MS_AttackS4, the only motion the bat outlives
/// (ftNs_AttackS4_CheckNessBatRemove).
const OWNER_MOTION: u16 = 341;

static STATES: [ItemStateRow; 2] = [
    ItemStateRow {
        animation_id: ARTICLE_STATES[0],
        animation: swing,
        physics: no_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: ARTICLE_STATES[1],
        animation: posed,
        physics: no_physics,
        collision: no_collision,
    },
];

impl ItemLogic for NessBat {
    const KIND: ItemKind = ItemKind::NessBat;
    const STATES: &'static [ItemStateRow] = &STATES;
    fn pickup_possible(item: &ItemCore) -> bool {
        !item.held
    }
    /// it_802AD478 once Item_80268B18 returns: the four command variables
    /// cleared, xDCC b3 off and both lifetimes (it_80275158).
    fn launched(
        item: &mut ItemCore,
        _assets: &ItemAssets,
        common: &melee_it::desc::ItemCommonData,
        _spawn: &melee_it::SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        item.command_variables = [0; 4];
        item.life_timer = LIFETIME;
        item.half_life = LIFETIME * common.half_life_scale;
    }
    /// it_2725_Logic58_PickedUp (802AD6F8): the swing while the command
    /// variables are clear, then Item_802694CC.
    fn picked_up(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
        let state = usize::from(item.command_variables.iter().any(|&v| v != 0));
        item.change_motion_with(state as u16, ARTICLE_STATES[state], ANIM_UPDATE, ctx.assets);
        item.advance_animation(ctx.assets);
    }
    /// it_802AD6B8: the owner puts the bat away.
    fn control(item: &mut ItemCore, control: ItemControl, _assets: &ItemAssets) {
        match control {
            ItemControl::Remove => item.destroyed = true,
            _ => unreachable!("{control:?} sent to the bat"),
        }
    }
}

/// itNessbat_UnkMotion0_Anim (802AD76C): the script's first command
/// variable hides the model; the bat goes once Ness has left the forward
/// smash (ftNs_AttackS4_CheckNessBatRemove).
fn swing(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.hidden = item.command_variables[0] != 0;
    ctx.owner.is_none_or(|owner| owner.motion != OWNER_MOTION)
}

/// itNessbat_UnkMotion1_Anim (802AD880): only the owner's scale.
fn posed(_item: &mut ItemCore, _ctx: &mut ItemAnimationContext<'_>) -> bool {
    false
}
