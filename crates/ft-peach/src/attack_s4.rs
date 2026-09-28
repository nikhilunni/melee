//! Peach's forward smash, ftpeachattacks4.c (8011C07C..8011C1C0): golf
//! club, frying pan or tennis racket at random, never the one just used.
//! The swing rows reuse the common forward smash callbacks: Anim ends in
//! Wait (ft_8008A2BC), IASA is Wait's once the script allows it, Phys is
//! ft_80084FA8 and Coll ft_80084104.
use crate::init::Peach;
use melee_ft::fighter::{
    assets::{FighterAssets, Result},
    ActionId, Fighter, Interaction, MotionData,
};

/// ftPe_MS_AttackS4Club..AttackS4Racket (349..351).
pub const CLUB: ActionId = ActionId(349);
pub const RACKET: ActionId = ActionId(351);

/// ftPe_AttackS4_Enter (8011C07C), after decideFighter set the facing.
pub fn enter(f: &mut Fighter, assets: &FighterAssets, rng: &mut gekko_math::HsdRng) -> Result<()> {
    f.commands.allow_interrupt = false;
    f.commands.variables[0] = 0;
    let previous = f.character.get::<Peach>().smash_motion;
    let choices = i32::from(RACKET.0 - CLUB.0) + 1;
    let state = loop {
        let state = rng.randi(choices) + i32::from(CLUB.0);
        if state != previous {
            break state;
        }
    };
    f.character.get_mut::<Peach>().smash_motion = state;
    // The swings write no mv field, so mv+4 stays the predecessor's.
    let retained_word = f.inherited_scratch_word();
    f.change_motion_state(ActionId(state as u16), assets)?;
    f.step_animation(assets);
    f.core.state_data = MotionData::Smash { retained_word };
    f.core.status.interaction = Interaction::Attack;
    Ok(())
}
