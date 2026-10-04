//! The ground attacks' articles: the jab's Greenhouse sprayer
//! (itgamewatchgreenhouse.c, 802C61F4..802C65C4), the down tilt's Manhole
//! (itgamewatchmanhole.c, 802C65E4..802C6924) and the forward smash's Fire
//! torch (itgamewatchfire.c, 802C68F8..802C6C18). Each hangs on the owner's
//! left hand.
use crate::{common_control, no_collision, no_physics, owner_motion as owner};
use melee_it::{
    desc::ItemAssets, state_change::ANIM_UPDATE, ItemAnimationContext, ItemControl, ItemCore,
    ItemEvent, ItemLogic, ItemStateRow,
};
use melee_types::ItemKind;

pub struct Greenhouse;

/// it_803F7898's anim_id column: one article state per jab motion.
pub const GREENHOUSE_STATES: [i32; 4] = [0, 1, 2, 3];
/// The sprayer's motion for Attack100Loop, whose animation repeats.
const GREENHOUSE_LOOP: u16 = 2;

static GREENHOUSE_ROWS: [ItemStateRow; 4] = [
    greenhouse_row(0, greenhouse_animation),
    greenhouse_row(1, greenhouse_animation),
    greenhouse_row(2, greenhouse_loop_animation),
    greenhouse_row(3, greenhouse_animation),
];
const fn greenhouse_row(
    motion: usize,
    animation: fn(&mut ItemCore, &mut ItemAnimationContext<'_>) -> bool,
) -> ItemStateRow {
    ItemStateRow {
        animation_id: GREENHOUSE_STATES[motion],
        animation,
        physics: no_physics,
        collision: no_collision,
    }
}

impl ItemLogic for Greenhouse {
    const KIND: ItemKind = ItemKind::GameWatchGreenhouse;
    const STATES: &'static [ItemStateRow] = &GREENHOUSE_ROWS;
    const PICKUP_READS_OWNER: bool = true;
    fn pickup_possible(item: &ItemCore) -> bool {
        !item.held
    }
    /// itGamewatchGreenhouse_PickedUp (802C63B4): the command variables
    /// cleared; motion 0 while the owner is in Attack11
    /// (ftGw_Attack11_ItemGreenhouse_CheckAttack11), else motion 1; one
    /// animation step (Item_802694CC).
    fn picked_up(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
        item.command_variables[1] = 0;
        item.command_variables[0] = 0;
        let holder = ctx.owner.expect("the Greenhouse sprayer's owner");
        let motion = u16::from(holder.motion != owner::ATTACK_11);
        item.change_motion_with(
            motion,
            GREENHOUSE_STATES[usize::from(motion)],
            ANIM_UPDATE,
            ctx.assets,
        );
        item.advance_animation(ctx.assets);
    }
    /// itGamewatchGreenhouse_802C6430..802C64A8 (ftGw_Attack11_DecideAction):
    /// the sprayer follows its owner's jab motion.
    fn control(item: &mut ItemCore, control: ItemControl, assets: &ItemAssets) {
        if common_control(item, control) {
            return;
        }
        match control {
            ItemControl::Motion(motion) => item.change_motion_with(
                motion,
                GREENHOUSE_STATES[usize::from(motion)],
                ANIM_UPDATE,
                assets,
            ),
            _ => unreachable!("{control:?} sent to the Greenhouse sprayer"),
        }
    }
}

/// greenhouse_Check -> ftGw_Attack11_ItemGreenhouse_CheckAll (8014C034):
/// the owner has left its jab motions.
fn jab_over(ctx: &ItemAnimationContext<'_>) -> bool {
    ctx.owner
        .is_none_or(|o| !(owner::ATTACK_11..=owner::ATTACK_100_END).contains(&o.motion))
}

/// itGamewatchGreenhouse_Motion3_Anim (802C64D0).
fn greenhouse_animation(_item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    jab_over(ctx)
}

/// itGamewatchGreenhouse_Motion2_Anim (802C6530): the loop's animation
/// restarts when it runs out (it_80272C6C).
fn greenhouse_loop_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.article_animation_ended(ctx.assets) {
        item.change_motion_with(
            GREENHOUSE_LOOP,
            GREENHOUSE_STATES[usize::from(GREENHOUSE_LOOP)],
            ANIM_UPDATE,
            ctx.assets,
        );
    }
    jab_over(ctx)
}

pub struct Manhole;

/// it_803F78D8's anim_id column.
pub const MANHOLE_STATES: [i32; 1] = [0];
/// HSD_JObjGetChild(gobj->hsd_obj): the model root's child, which the
/// gust blows from.
const MANHOLE_COVER_BONE: usize = 1;

static MANHOLE_ROWS: [ItemStateRow; 1] = [ItemStateRow {
    animation_id: MANHOLE_STATES[0],
    animation: manhole_animation,
    physics: no_physics,
    collision: no_collision,
}];

impl ItemLogic for Manhole {
    const KIND: ItemKind = ItemKind::GameWatchManhole;
    const STATES: &'static [ItemStateRow] = &MANHOLE_ROWS;
    fn pickup_possible(item: &ItemCore) -> bool {
        !item.held
    }
    /// itGameWatchManhole_Logic72_PickedUp (802C67A4): the command variable
    /// cleared, the model hidden (it_8026BB44), motion 0 and one animation
    /// step.
    fn picked_up(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
        item.command_variables[0] = 0;
        item.hidden = true;
        item.change_motion_with(0, MANHOLE_STATES[0], ANIM_UPDATE, ctx.assets);
        item.advance_animation(ctx.assets);
    }
    fn control(item: &mut ItemCore, control: ItemControl, _assets: &ItemAssets) {
        if !common_control(item, control) {
            unreachable!("{control:?} sent to the Manhole");
        }
    }
}

/// itGamewatchmanhole_UnkMotion0_Anim (802C6808): the script's flag shows
/// the cover (it_8026BB20) and blows a gust from it
/// (lb_800119DC(&pos, 120, 3.0, 0.1, pi/3)); the Manhole goes once its
/// owner leaves the down tilt (ftGw_AttackLw3_ItemCheckManholeRemove).
fn manhole_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.command_variables[0] != 0 {
        item.command_variables[0] = 0;
        item.hidden = false;
        item.events.push(ItemEvent::BoneGust {
            bone: MANHOLE_COVER_BONE,
            frames: 120,
            strength: 3.0,
            decay: 0.1,
            phase_step: std::f32::consts::FRAC_PI_3,
        });
    }
    ctx.owner.is_none_or(|o| o.motion != owner::ATTACK_LW3)
}

pub struct Fire;

/// it_803F78E8's anim_id column.
pub const FIRE_STATES: [i32; 1] = [0];
/// The frame the torch appears at.
const FIRE_SHOWN_FRAME: f32 = 3.0;

static FIRE_ROWS: [ItemStateRow; 1] = [ItemStateRow {
    animation_id: FIRE_STATES[0],
    animation: fire_animation,
    physics: no_physics,
    collision: no_collision,
}];

impl ItemLogic for Fire {
    const KIND: ItemKind = ItemKind::GameWatchFire;
    const STATES: &'static [ItemStateRow] = &FIRE_ROWS;
    fn pickup_possible(item: &ItemCore) -> bool {
        !item.held
    }
    /// itGamewatchFire_PickedUp (802C6AB8): the command variables cleared,
    /// the model hidden (it_8026BB44), motion 0 and one animation step.
    fn picked_up(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
        item.command_variables[1] = 0;
        item.command_variables[0] = 0;
        item.hidden = true;
        item.change_motion_with(0, FIRE_STATES[0], ANIM_UPDATE, ctx.assets);
        item.advance_animation(ctx.assets);
    }
    fn control(item: &mut ItemCore, control: ItemControl, _assets: &ItemAssets) {
        if !common_control(item, control) {
            unreachable!("{control:?} sent to the Fire torch");
        }
    }
}

/// itGamewatchFire_Motion0_Anim (802C6B20): the torch shows at frame 3
/// (it_8026BB20), goes once its owner leaves the forward smash
/// (ftGw_AttackS4_ItemCheckTorchRemove), and holds its animation while the
/// owner charges the smash (ftLib_800876D4: smash_attrs.state == 2, which
/// the owner reports as `article_stage` 1).
fn fire_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    if item.animation_frame == FIRE_SHOWN_FRAME {
        item.hidden = false;
    }
    let Some(holder) = ctx.owner else {
        return true;
    };
    if holder.motion != owner::ATTACK_S4 {
        return true;
    }
    // lb_8000BA0C(jobj, rate): the joint animation's rate follows x5D0.
    item.animation_rate = if holder.article_stage == Some(1) {
        0.0
    } else {
        1.0
    };
    false
}
