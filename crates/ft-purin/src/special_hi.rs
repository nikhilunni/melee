//! Sing, ftpurinspecialhi.c (8013C9C8..8013CE8C). The motion script owns
//! the song's hitboxes; the move itself only plays out its animation and
//! installs the notes effect (ftPr_Init_8013C94C) as accessory4.
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
use melee_types::FtPart;

/// ftPr_MS_SpecialHiL..SpecialAirHiR (365..368).
pub const GROUND_LEFT: ActionId = ActionId(365);
pub const AIR_LEFT: ActionId = ActionId(366);
pub const GROUND_RIGHT: ActionId = ActionId(367);
pub const AIR_RIGHT: ActionId = ActionId(368);

/// efSync_Spawn(1238, gobj, parts[FtPart_WaistN]): the song's notes.
const NOTES_EFFECT: u16 = 1238;

/// Fighter_ChangeMotionState flags 0x0C4C508A of the ground/air changes:
/// ftCommon_GroundAirColl_MF with SkipHit, keeping effects and hitboxes.
const GROUND_AIR_PRESERVATION: MotionPreservation = MotionPreservation {
    hit_status: false,
    hitboxes: true,
    effects: true,
    fast_fall: false,
};

fn is_air(action: ActionId) -> bool {
    action == AIR_LEFT || action == AIR_RIGHT
}

/// ftPr_SpecialHi_Enter (8013C9C8) / ftPr_SpecialAirHi_Enter (8013CA98).
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    let state = if air {
        common::by_facing(f, AIR_LEFT, AIR_RIGHT)
    } else {
        common::by_facing(f, GROUND_LEFT, GROUND_RIGHT)
    };
    let retained = f.inherited_scratch_word();
    f.change_motion_state(state, a).expect("Sing assets");
    f.character.get_mut::<Jigglypuff>().retained_word = retained;
    f.step_animation(a);
    // ftPurin_SpecialHi_SetVars: cmd_vars[0], accessory4 and the Pokémon
    // Stadium sleep flag (gm_8016B1D8 && grStadium_801D4FF8), which no
    // supported stage sets.
    f.commands.variables[0] = 0;
    arm_notes(f);
    f.character.get_mut::<Jigglypuff>().stadium_sleep = false;
}

fn arm_notes(f: &mut Fighter) {
    f.character.get_mut::<Jigglypuff>().accessory = Accessory::Notes;
    f.core.arm_accessory4();
}

/// ftPr_Init_8013C94C (8013C94C), accessory4: the notes effect once per
/// owned-effect lifetime (x2219_b0), then the effect hitlag callbacks.
pub fn notes(f: &mut Fighter, assets: &FighterAssets) {
    if !f.effect_state.destroy_on_state_change {
        let bone = usize::from(assets.parts.joint(FtPart::WaistN).expect("WaistN part"));
        f.effects
            .push(melee_ef::request::EffectRequest::SyncAttached {
                id: NOTES_EFFECT,
                bone,
            });
        f.effect_state.destroy_on_state_change = true;
    }
    // Fighter_SetEffectHitlagCallbacks.
    f.effect_state.hitlag_callbacks = true;
}

/// ftPr_SpecialHi_Anim / ftPr_SpecialAirHi_Anim.
pub fn anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if f.character.get::<Jigglypuff>().stadium_sleep {
        unimplemented!("ftPr_SpecialHi_Anim: Pokémon Stadium sleep element");
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        let air = is_air(f.motion_state.action);
        common::finish(f, p.assets, air)?;
    }
    Ok(None)
}

/// ftPr_SpecialHi_IASA / ftPr_SpecialAirHi_IASA are empty.
pub fn input(_: &mut Fighter, _: InputPhase<'_>) {}

/// ftPr_SpecialHi_Phys: ft_80084F3C.
pub fn ground_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
}

/// ftPr_SpecialAirHi_Phys: ft_80084EEC.
pub fn air_physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::air_friction(f, p);
}

/// ftPr_SpecialHi_Coll: off the edge, ftPr_SpecialHi_8013CD34 continues in
/// the air at the current frame.
pub fn ground_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    if common::stays_grounded(f, &mut p) {
        return Ok(());
    }
    f.leave_ground();
    let state = common::by_facing(f, AIR_LEFT, AIR_RIGHT);
    transition(f, p.assets.expect("Sing collision assets"), state)
}

/// ftPr_SpecialAirHi_Coll: landing (ft_CheckGroundAndLedge) continues on
/// the ground (ftPr_SpecialHi_8013CDD8); otherwise a ledge may be caught.
pub fn air_collision(f: &mut Fighter, mut p: CollisionPhase<'_>) -> Result<()> {
    let assets = p.assets.expect("Sing landing assets");
    if common::lands_or_catches_ledge(f, &mut p) {
        f.land();
        let state = common::by_facing(f, GROUND_LEFT, GROUND_RIGHT);
        return transition(f, assets, state);
    }
    f.try_grab_ledge(assets, p.map)?;
    Ok(())
}

/// Both ground/air changes re-install the notes accessory.
fn transition(f: &mut Fighter, a: &FighterAssets, state: ActionId) -> Result<()> {
    f.change_ground_air_motion(state, a, GROUND_AIR_PRESERVATION)?;
    arm_notes(f);
    Ok(())
}
