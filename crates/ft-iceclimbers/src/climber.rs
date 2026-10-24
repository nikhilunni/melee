//! What Popo and Nana share: the special code (ftPp_*) runs on either
//! climber's fighter, reading its own ftPopo_FighterVars and attributes.
use crate::{
    attributes::IceClimberAttributes,
    init::{Climber, ClimberVars, IceClimber},
};
use melee_ft::fighter::{Fighter, MotionRow};

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

pub use ft_iceclimbers_family::common::{
    finish, lands, no_input, play_sound, play_voice, stays_grounded,
};
