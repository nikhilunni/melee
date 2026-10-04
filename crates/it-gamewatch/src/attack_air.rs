//! The aerials' articles: the neutral aerial's Parachute
//! (itgamewatchparachute.c, 802C6C38..802C6F20), the back aerial's Turtle
//! (itgamewatchturtle.c, 802C6F40..802C71EC) and the up aerial's Spitball
//! Sparky breath (itgamewatchbreath.c, 802C720C..802C74B8). Each has a
//! second motion its owner selects when the aerial lands.
use crate::{common_control, no_collision, no_physics, owner_motion as owner};
use melee_it::{
    desc::ItemAssets, state_change::ANIM_UPDATE, ItemAnimationContext, ItemControl, ItemCore,
    ItemLogic, ItemStateRow,
};
use melee_types::ItemKind;

/// The motion each article takes when its owner lands in the aerial's
/// landing row (it_802C6E50, it_802C7158, it_802C7424).
pub const LANDED: u16 = 1;

/// it_803F78F8, it_803F7918 and it_803F7938's anim_id columns.
pub const ARTICLE_STATES: [i32; 2] = [0, 1];

const fn rows(
    animation: fn(&mut ItemCore, &mut ItemAnimationContext<'_>) -> bool,
) -> [ItemStateRow; 2] {
    [
        ItemStateRow {
            animation_id: ARTICLE_STATES[0],
            animation,
            physics: no_physics,
            collision: no_collision,
        },
        ItemStateRow {
            animation_id: ARTICLE_STATES[1],
            animation,
            physics: no_physics,
            collision: no_collision,
        },
    ]
}

/// The pickup callbacks the three share (itGameWatchParachute_Logic74_
/// PickedUp 802C6DF8, itGameWatchTurtle_Logic75_PickedUp 802C7100,
/// itGameWatchBreath_Logic76_PickedUp 802C73CC): the command variable
/// cleared, motion 0 and one animation step.
fn picked_up(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
    item.command_variables[0] = 0;
    item.change_motion_with(0, ARTICLE_STATES[0], ANIM_UPDATE, ctx.assets);
    item.advance_animation(ctx.assets);
}

/// The owner is outside `motions` (or gone).
fn owner_left(ctx: &ItemAnimationContext<'_>, motions: std::ops::RangeInclusive<u16>) -> bool {
    ctx.owner.is_none_or(|o| !motions.contains(&o.motion))
}

pub struct Parachute;

static PARACHUTE_ROWS: [ItemStateRow; 2] = rows(parachute_animation);
/// The landed parachute's last frame.
const PARACHUTE_END_FRAME: f32 = 30.0;

impl ItemLogic for Parachute {
    const KIND: ItemKind = ItemKind::GameWatchParachute;
    const STATES: &'static [ItemStateRow] = &PARACHUTE_ROWS;
    fn pickup_possible(item: &ItemCore) -> bool {
        !item.held
    }
    fn picked_up(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
        picked_up(item, ctx);
    }
    fn control(item: &mut ItemCore, control: ItemControl, assets: &ItemAssets) {
        if common_control(item, control) {
            return;
        }
        match control {
            // it_802C6E50 (802C6E50): hidden (it_8026BB44), then motion 1.
            ItemControl::Motion(LANDED) => {
                item.hidden = true;
                item.change_motion_with(LANDED, ARTICLE_STATES[1], ANIM_UPDATE, assets);
            }
            _ => unreachable!("{control:?} sent to the Parachute"),
        }
    }
}

/// itGamewatchparachute_UnkMotion1_Anim (802C6E88): the parachute goes at
/// frame 30, or once its owner leaves AttackAirN..LandingAirN
/// (ftGw_AttackAirN_ItemCheckParachuteRemove, 8014B18C).
fn parachute_animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    item.animation_frame == PARACHUTE_END_FRAME
        || owner_left(ctx, owner::ATTACK_AIR_N..=owner::LANDING_AIR_N)
}

pub struct Turtle;

static TURTLE_ROWS: [ItemStateRow; 2] = rows(turtle_animation);

impl ItemLogic for Turtle {
    const KIND: ItemKind = ItemKind::GameWatchTurtle;
    const STATES: &'static [ItemStateRow] = &TURTLE_ROWS;
    fn pickup_possible(item: &ItemCore) -> bool {
        !item.held
    }
    fn picked_up(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
        picked_up(item, ctx);
    }
    fn control(item: &mut ItemCore, control: ItemControl, assets: &ItemAssets) {
        if common_control(item, control) {
            return;
        }
        match control {
            // it_802C7158 (802C7158).
            ItemControl::Motion(LANDED) => {
                item.change_motion_with(LANDED, ARTICLE_STATES[1], ANIM_UPDATE, assets);
            }
            _ => unreachable!("{control:?} sent to the Turtle"),
        }
    }
}

/// itGamewatchturtle_UnkMotion1_Anim (802C7180): the turtle goes once its
/// owner leaves AttackAirB..LandingAirB
/// (ftGw_AttackAirN_ItemCheckTurtleRemove, 8014B380).
fn turtle_animation(_item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    owner_left(ctx, owner::ATTACK_AIR_B..=owner::LANDING_AIR_B)
}

pub struct Breath;

static BREATH_ROWS: [ItemStateRow; 2] = rows(breath_animation);

impl ItemLogic for Breath {
    const KIND: ItemKind = ItemKind::GameWatchBreath;
    const STATES: &'static [ItemStateRow] = &BREATH_ROWS;
    fn pickup_possible(item: &ItemCore) -> bool {
        !item.held
    }
    fn picked_up(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
        picked_up(item, ctx);
    }
    fn control(item: &mut ItemCore, control: ItemControl, assets: &ItemAssets) {
        if common_control(item, control) {
            return;
        }
        match control {
            // it_802C7424 (802C7424).
            ItemControl::Motion(LANDED) => {
                item.change_motion_with(LANDED, ARTICLE_STATES[1], ANIM_UPDATE, assets);
            }
            _ => unreachable!("{control:?} sent to the Spitball Sparky breath"),
        }
    }
}

/// itGamewatchbreath_UnkMotion1_Anim (802C744C): the breath goes once its
/// owner leaves AttackAirHi..LandingAirHi
/// (ftGw_AttackAirN_ItemCheckSparkyRemove, 8014B624).
fn breath_animation(_item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    owner_left(ctx, owner::ATTACK_AIR_HI..=owner::LANDING_AIR_HI)
}
