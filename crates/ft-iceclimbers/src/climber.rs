//! What Popo and Nana share: the special code (ftPp_*) runs on either
//! climber's fighter, reading its own ftPopo_FighterVars and attributes.
use crate::{
    attributes::IceClimberAttributes,
    init::{Climber, ClimberVars, IceClimber},
};
use melee_ft::fighter::{assets::FighterAssets, Fighter, MotionRow};

pub fn vars(f: &mut Fighter) -> &mut ClimberVars {
    &mut f.character.get_mut::<IceClimber>().vars
}

pub fn payload(f: &mut Fighter) -> &mut IceClimber {
    f.character.get_mut::<IceClimber>()
}

pub fn attributes(f: &Fighter) -> &IceClimberAttributes {
    &f.character.get::<IceClimber>().attributes
}

/// Which climber `f` is (FTKIND_POPO or FTKIND_NANA).
pub fn climber(f: &Fighter) -> Climber {
    f.character.get::<IceClimber>().climber
}

/// The accessory4 callback a special installed; a motion change removes
/// it (Fighter_ChangeMotionState).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Accessory {
    #[default]
    None,
    /// ftPp_SpecialN_8011F500: the Ice Shot's script commands.
    IceShot,
    /// fn_80123218: Nana's hand for Popo's rope (her Belay rows).
    RopeAnchor,
    /// fn_80122D2C: the Blizzard's puffs and script commands.
    Blizzard,
}

/// ftPp_SM_SpecialN = ftCo_SM_Count: row 341 plays submotion 295.
const FIRST_SPECIAL_ACTION: u16 = 341;
const FIRST_SPECIAL_ANIMATION: i32 = 295;

/// A ported row of ftPp_Init_MotionStateTable (341..366).
pub const fn row(
    action: u16,
    anim: melee_ft::fighter::state::AnimFn,
    iasa: melee_ft::fighter::state::InputFn,
    physics: melee_ft::fighter::state::PhysicsFn,
    collision: melee_ft::fighter::state::CollisionFn,
) -> MotionRow {
    MotionRow {
        action: melee_ft::fighter::ActionId(action),
        id: melee_types::CommonMotionState::None,
        animation: FIRST_SPECIAL_ANIMATION + (action - FIRST_SPECIAL_ACTION) as i32,
        anim,
        iasa,
        physics,
        collision,
        camera: melee_ft::fighter::state::callbacks::camera::follow_fighter,
        implemented: true,
    }
}

/// No IASA (ftPp_SpecialN_IASA and siblings are empty).
pub fn no_input(_: &mut Fighter, _: melee_ft::fighter::state::InputPhase<'_>) {}

/// ft_PlaySFX(fp, id, 127, 64).
pub fn play_sound(f: &mut Fighter, id: u32) {
    use melee_ft::fighter::commands::{FootstepSound, SoundChannel};
    f.commands.footstep_sounds.push(FootstepSound {
        channel: SoundChannel::Ordinary,
        id,
        volume: 127,
        pan: 64,
    });
}

/// ft_800881D8(fp, id, 127, 64): the climber's voice.
pub fn play_voice(f: &mut Fighter, id: u32) {
    use melee_ft::fighter::commands::{FootstepSound, SoundChannel};
    f.commands.footstep_sounds.push(FootstepSound {
        channel: SoundChannel::FighterVoice,
        id,
        volume: 127,
        pan: 64,
    });
}

/// ft_8008A2BC on the ground, ftCo_Fall_Enter in the air.
pub fn finish(
    f: &mut Fighter,
    assets: &FighterAssets,
    air: bool,
) -> melee_ft::fighter::assets::Result<()> {
    let state = if air {
        melee_types::CommonMotionState::Fall
    } else {
        melee_types::CommonMotionState::Wait
    };
    f.change_motion_state(state.into(), assets)
}

/// ft_80082708 (80082708): ordinary ground collision that lets the fighter
/// walk off the floor's edge. True while it is still supported.
pub fn stays_grounded(
    f: &mut Fighter,
    p: &mut melee_ft::fighter::state::CollisionPhase<'_>,
) -> bool {
    use melee_ft::collision::ground;
    let c = &mut f.core;
    matches!(
        ground::map_ground_action(
            &mut c.physics,
            &mut c.collision,
            p.map,
            &mut c.skeleton,
            c.animation.root,
            c.input.current.stick.x,
        ),
        ground::WaitGroundResult::Supported
    )
}

/// ft_80081D0C (80081D0C): ordinary airborne collision; true on landing.
pub fn lands(f: &mut Fighter, p: &mut melee_ft::fighter::state::CollisionPhase<'_>) -> bool {
    use melee_ft::collision::air;
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    air::collide_air_dodge(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
    )
}
