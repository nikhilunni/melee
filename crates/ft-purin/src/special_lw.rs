//! Rest, ftpurinspeciallw.c (8013CE8C..8013D234). The motion script owns
//! the sleep, its hitbox and effects; the move plays its animation out.
use crate::{
    common,
    init::{Accessory, Jigglypuff},
};
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        ActionId, Fighter, MotionPreservation,
    },
};

/// ftPr_MS_SpecialLwL..SpecialAirLwR (369..372).
pub const GROUND_LEFT: ActionId = ActionId(369);
pub const AIR_LEFT: ActionId = ActionId(370);
pub const GROUND_RIGHT: ActionId = ActionId(371);
pub const AIR_RIGHT: ActionId = ActionId(372);

/// Fighter_ChangeMotionState flags 0x0C4C508E of the ground/air changes:
/// ftCommon_GroundAirColl_MF with KeepColAnimHitStatus and SkipHit.
const GROUND_AIR_PRESERVATION: MotionPreservation = MotionPreservation {
    hit_status: true,
    hitboxes: true,
    effects: true,
    fast_fall: false,
};

fn is_air(action: ActionId) -> bool {
    action == AIR_LEFT || action == AIR_RIGHT
}

/// ftPr_SpecialLw_Enter (8013CE8C) / ftPr_SpecialAirLw_Enter (8013CF2C).
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    let state = if air {
        common::by_facing(f, AIR_LEFT, AIR_RIGHT)
    } else {
        common::by_facing(f, GROUND_LEFT, GROUND_RIGHT)
    };
    let retained = f.inherited_scratch_word();
    f.change_motion_state(state, a).expect("Rest assets");
    f.character.get_mut::<Jigglypuff>().retained_word = retained;
    f.step_animation(a);
    f.commands.variables[0] = 0;
    // accessory4_cb = ftPr_SpecialHi_8013CE7C, which only uninstalls itself.
    f.character.get_mut::<Jigglypuff>().accessory = Accessory::Uninstall;
    f.core.arm_accessory4();
}

/// ftPr_SpecialLw_Anim / ftPr_SpecialAirLw_Anim.
pub fn anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let air = is_air(f.motion_state.action);
        common::finish(f, p.assets, air)?;
    }
    Ok(None)
}

/// ftPr_SpecialLw_IASA / ftPr_SpecialAirLw_IASA are empty.
pub fn input(_: &mut Fighter, _: InputPhase<'_>) {}

/// ftPr_SpecialLw_Phys: ft_80084F3C.
pub fn ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
}

/// ftPr_SpecialAirLw_Phys: ft_80084EEC.
pub fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::air_friction(f, p);
}

/// ftPr_SpecialLw_Coll: off the edge, ftPr_SpecialLw_8013D104 continues in
/// the air at the current frame.
pub fn ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::stays_grounded(f, &mut p) {
        return Ok(());
    }
    f.leave_ground();
    let state = common::by_facing(f, AIR_LEFT, AIR_RIGHT);
    f.change_ground_air_motion(
        state,
        p.assets.expect("Rest collision assets"),
        GROUND_AIR_PRESERVATION,
    )
}

/// ftPr_SpecialAirLw_Coll: landing (ft_80081D0C) continues on the ground
/// (ftPr_SpecialLw_8013D19C).
pub fn air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    f.land();
    let state = common::by_facing(f, GROUND_LEFT, GROUND_RIGHT);
    f.change_ground_air_motion(
        state,
        p.assets.expect("Rest landing assets"),
        GROUND_AIR_PRESERVATION,
    )
}
