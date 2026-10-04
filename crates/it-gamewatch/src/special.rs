//! The specials' articles other than Chef's food: Judgment's numbered sign
//! (itgamewatchjudge.c, 802C7774..802C7BD4), Oil Panic's oil
//! (itgamewatchpanic.c, 802C7D60..802C8018) and Fire's rescue trampoline
//! (itgamewatchrescue.c, 802C8038..802C835C).
use crate::{common_control, no_collision, no_physics, owner_motion as owner};
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::ANIM_UPDATE,
    ItemAnimationContext, ItemControl, ItemCore, ItemLogic, ItemScratch, ItemStateRow, JudgeState,
    RescueState, SpawnItem,
};
use melee_types::ItemKind;

pub struct Judge;

/// it_803F7968's anim_id column.
pub const JUDGE_STATES: [i32; 1] = [0];

static JUDGE_ROWS: [ItemStateRow; 1] = [ItemStateRow {
    animation_id: JUDGE_STATES[0],
    animation: judge_animation,
    physics: no_physics,
    collision: no_collision,
}];

impl ItemLogic for Judge {
    const KIND: ItemKind = ItemKind::GameWatchJudge;
    const STATES: &'static [ItemStateRow] = &JUDGE_ROWS;
    fn pickup_possible(item: &ItemCore) -> bool {
        !item.held
    }
    /// it_802C7774's face (`spawn_argument`: x222C_judgeVar1), kept for the
    /// pose after the pickup.
    fn launched(
        item: &mut ItemCore,
        _assets: &ItemAssets,
        _common: &ItemCommonData,
        spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        item.scratch = ItemScratch::Judge(JudgeState {
            face: spawn.spawn_argument,
        });
    }
    /// it_2725_Logic77_PickedUp (802C7B10): the command variables cleared
    /// and motion 0; then it_802C7774 poses the sign at the face's frame
    /// (it_80273670(gobj, 0, face + 1), which removes the animation and
    /// script). it_802C78B8 turns the number to the owner's facing, which
    /// only draws.
    fn picked_up(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
        item.command_variables[1] = 0;
        item.command_variables[0] = 0;
        item.change_motion_with(0, JUDGE_STATES[0], ANIM_UPDATE, ctx.assets);
        let ItemScratch::Judge(state) = item.scratch else {
            panic!("Judgment sign scratch")
        };
        item.pose_article_frame((state.face + 1) as f32);
    }
    fn control(item: &mut ItemCore, control: ItemControl, _assets: &ItemAssets) {
        if !common_control(item, control) {
            unreachable!("{control:?} sent to the Judgment sign");
        }
    }
}

/// itGamewatchjudge_UnkMotion0_Anim (802C7B54): the sign goes once its
/// owner leaves Judgment's rows (ftGw_SpecialS_ItemCheckJudgementRemove,
/// 8014C68C).
fn judge_animation(_item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    ctx.owner
        .is_none_or(|o| !(owner::SPECIAL_S1..=owner::SPECIAL_AIR_S9).contains(&o.motion))
}

pub struct Panic;

/// it_803F79A0's anim_id column: the grounded and the aerial spill.
pub const PANIC_STATES: [i32; 2] = [0, 1];

static PANIC_ROWS: [ItemStateRow; 2] = [
    ItemStateRow {
        animation_id: PANIC_STATES[0],
        animation: panic_animation,
        physics: no_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: PANIC_STATES[1],
        animation: panic_animation,
        physics: no_physics,
        collision: no_collision,
    },
];

impl ItemLogic for Panic {
    const KIND: ItemKind = ItemKind::GameWatchPanic;
    const STATES: &'static [ItemStateRow] = &PANIC_ROWS;
    const PICKUP_READS_OWNER: bool = true;
    fn pickup_possible(item: &ItemCore) -> bool {
        !item.held
    }
    /// itGameWatchPanic_Logic78_PickedUp (802C7F20): the command variables
    /// cleared; the oil takes its owner's ground state (ftLib_800865CC:
    /// grounded in SpecialLwShoot, airborne in SpecialAirLwShoot) and with
    /// it motion 0 or 1; one animation step.
    fn picked_up(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) {
        item.command_variables[1] = 0;
        item.command_variables[0] = 0;
        let holder = ctx.owner.expect("the oil's owner");
        let airborne = holder.motion != owner::SPECIAL_LW_SHOOT;
        item.ground_or_air = if airborne {
            melee_types::GroundOrAir::Air
        } else {
            melee_types::GroundOrAir::Ground
        };
        let motion = u16::from(airborne);
        item.change_motion_with(
            motion,
            PANIC_STATES[usize::from(motion)],
            ANIM_UPDATE,
            ctx.assets,
        );
        item.advance_animation(ctx.assets);
    }
    fn control(item: &mut ItemCore, control: ItemControl, _assets: &ItemAssets) {
        if !common_control(item, control) {
            unreachable!("{control:?} sent to the oil");
        }
    }
}

/// itGamewatchpanic_UnkMotion1_Anim (802C7FAC): the oil goes once its
/// owner leaves the spill's rows (ftGw_SpecialLw_ItemCheckPanicRemove:
/// SpecialLwShoot..SpecialAirLwShoot).
fn panic_animation(_item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    ctx.owner.is_none_or(|o| {
        !(owner::SPECIAL_LW_SHOOT..=owner::SPECIAL_AIR_LW_SHOOT).contains(&o.motion)
    })
}

pub struct Rescue;

/// it_803F79C0's anim_id column: both motions play article state 0.
pub const RESCUE_STATES: [i32; 2] = [0, 0];

static RESCUE_ROWS: [ItemStateRow; 2] = [
    ItemStateRow {
        animation_id: RESCUE_STATES[0],
        animation: rescue_animation,
        physics: no_physics,
        collision: no_collision,
    },
    ItemStateRow {
        animation_id: RESCUE_STATES[1],
        animation: rescue_animation,
        physics: no_physics,
        collision: no_collision,
    },
];

impl ItemLogic for Rescue {
    const KIND: ItemKind = ItemKind::GameWatchRescue;
    const STATES: &'static [ItemStateRow] = &RESCUE_ROWS;
    /// The logic row has no pickup callback.
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// it_802C8038 (802C8038) after Item_80268B18: the command variables
    /// cleared, no blast-zone removal (xDCC b3), the fighter it was made
    /// for (xDD8), then it_802C8208: the motion of its owner's row
    /// (`spawn_argument`: 0 grounded, 1 aerial) and one animation step.
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        _common: &ItemCommonData,
        spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        item.command_variables = [0; 4];
        item.blast_zone_checked = false;
        item.scratch = ItemScratch::Rescue(RescueState {
            fighter: spawn.owner,
        });
        let motion = spawn.spawn_argument as u16;
        item.change_motion_with(
            motion,
            RESCUE_STATES[usize::from(motion)],
            ANIM_UPDATE,
            assets,
        );
        item.advance_animation(assets);
    }
    fn control(item: &mut ItemCore, control: ItemControl, _assets: &ItemAssets) {
        match control {
            // it_802C8158 (802C8158): the owner's reset already done.
            ItemControl::Remove => {
                item.scratch = ItemScratch::Rescue(RescueState { fighter: None });
                item.owner = None;
                item.destroyed = true;
            }
            // it_802C81C8 / it_802C81E8.
            ItemControl::OwnerHitlag(frozen) => item.frozen = frozen,
            _ => unreachable!("{control:?} sent to the rescue trampoline"),
        }
    }
    /// The owner hears of the trampoline's end only while it is still the
    /// fighter it was made for (ftGw_SpecialHi_ItemRescueSetNULL through
    /// it_802C8158's test).
    fn notifies_owner(item: &ItemCore) -> bool {
        matches!(item.scratch, ItemScratch::Rescue(state) if state.fighter == item.owner)
    }
}

/// itGamewatchrescue_UnkMotion1_Anim (802C8234): the trampoline goes once
/// the fighter it was made for leaves Fire's rows
/// (ftGw_SpecialHi_ItemCheckRescueRemove, 8014DFB8), or has none.
fn rescue_animation(_item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    ctx.owner
        .is_none_or(|o| !(owner::SPECIAL_HI..=owner::SPECIAL_AIR_HI).contains(&o.motion))
}
