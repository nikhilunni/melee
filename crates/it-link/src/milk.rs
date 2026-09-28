//! Young Link's taunt milk (It_Kind_CLink_Milk), itclinkmilk.c
//! (802C8B28..802C8F20).
//!
//! It hangs from Young Link's left thumb, facing his way, until he leaves
//! the side taunt (ftCl_Init_8014920C), when it tells him (u.lk.x18 clears)
//! and goes; he may put it away first (it_802C8C34). The bottle's
//! visibility (cmd_vars[1] through xDAC) and its scale are drawing only.
use melee_it::{desc::ItemAssets, state_change::ANIM_UPDATE, *};
use melee_types::ItemKind;

/// it_803F7A28's anim_id column: held facing right, facing left.
pub const ARTICLE_STATES: [i32; 2] = [0, 1];
/// No special attributes.
pub const SPECIAL_ATTRIBUTES: u32 = 0;
/// ftData.x48_items index (Young Link's sixth article).
pub const ARTICLE_INDEX: u32 = 5;

/// ftLk_MS_AppealSR / ftLk_MS_AppealSL, the taunt the milk lasts through.
const TAUNT_ROWS: [u16; 2] = [342, 343];

fn state(item: &mut ItemCore) -> &mut MilkState {
    match &mut item.scratch {
        ItemScratch::Milk(state) => state,
        _ => unreachable!("milk without its state"),
    }
}

static STATES: [ItemStateRow; 2] = [row(0), row(1)];

const fn row(motion: usize) -> ItemStateRow {
    ItemStateRow {
        animation_id: ARTICLE_STATES[motion],
        animation: held,
        physics: no_physics,
        collision: no_collision,
    }
}

fn no_physics(_item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {}
fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}

/// The milk.
pub struct Milk;

impl ItemLogic for Milk {
    const KIND: ItemKind = ItemKind::CLinkMilk;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// it_802C8B28 (802C8B28) once Item_80268B18 returns: command
    /// variables clear, xDCC b3 (the blast-zone test) off, its fighter.
    fn launched(
        item: &mut ItemCore,
        _assets: &ItemAssets,
        _common: &melee_it::desc::ItemCommonData,
        spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        item.command_variables = [0; 4];
        item.blast_zone_checked = false;
        item.scratch = ItemScratch::Milk(MilkState {
            parent: spawn.owner,
        });
    }
    /// it_2725_Logic80_PickedUp (802C8C74): the row facing its way, then
    /// its first frame (Item_802694CC).
    fn picked_up(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
        let motion = if item.facing == 1.0 { 0 } else { 1 };
        item.change_motion_with(motion, ARTICLE_STATES[usize::from(motion)], ANIM_UPDATE, ctx.assets);
        item.advance_animation(ctx.assets);
    }
    /// it_802C8C34 (802C8C34) from Young Link: it forgets him and goes.
    fn control(item: &mut ItemCore, control: ItemControl, _assets: &ItemAssets) {
        match control {
            ItemControl::Remove => {
                state(item).parent = None;
                item.owner = None;
                item.destroyed = true;
            }
            _ => unimplemented!("milk item control {control:?}"),
        }
    }
    /// itCLinkMilk_NotifyParent: its fighter hears of it only while he
    /// still owns it.
    fn notifies_owner(item: &ItemCore) -> bool {
        match &item.scratch {
            ItemScratch::Milk(state) => state.parent.is_some() && item.owner == state.parent,
            _ => false,
        }
    }
}

/// itClinkmilk_UnkMotion1_Anim (802C8CDC): once its fighter has left the
/// side taunt (ftCl_Init_8014920C) or is gone, it goes.
fn held(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if state(item).parent.is_none() {
        return true;
    }
    let in_taunt = ctx
        .owner
        .is_some_and(|owner| TAUNT_ROWS.contains(&owner.motion));
    !in_taunt
}
