//! Float aerials, ftpeachfloatattack.c (8011BE80..8011C0E0): the common
//! aerials entered from Float, which keep the float and return to it.
use crate::{float, init::Peach};
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        attack::aerial,
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, PhysicsPhase},
        ActionId, Fighter,
    },
    input::Buttons,
};
use melee_types::CommonMotionState;

/// ftPe_MS_FloatAttackAirN (344); F, B, Hi and Lw follow in common order.
const FIRST_FLOAT_AERIAL: u16 = 344;

/// fp->mv.pe.floatattack (fp+2340).
#[derive(Clone, Copy, Debug, Default)]
pub struct FloatAttack {
    /// Set once the float input is released: the aerial falls normally and
    /// ends in Fall instead of returning to Float.
    pub released: bool,
}

/// ftPe_8011BE80 (8011BE80): A or a C-stick edge while float time remains.
pub fn try_enter(f: &mut Fighter, assets: &FighterAssets) -> Result<bool> {
    let requested =
        f.input.pressed.intersects(Buttons::A) || aerial::cstick_edge(&f.input, &assets.input);
    if !requested || f.character.get::<Peach>().float_remaining <= 0.0 {
        return Ok(false);
    }
    if f.held_item.is_some() {
        unimplemented!("ftpeachfloatattack.c:31-35: float aerial with a held item (ftCo_800CDDA0)");
    }
    enter(f, assets)?;
    Ok(true)
}

/// ftPe_8011BF34 (8011BF34): the stick's aerial, offset into Peach's rows.
fn enter(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    let state = aerial::select(&f.input, &assets.input, f.physics.facing);
    let offset = state as u16 - CommonMotionState::AttackAirN as u16;
    f.character.get_mut::<Peach>().float_attack = FloatAttack::default();
    aerial::enter_action(f, assets, ActionId(FIRST_FLOAT_AERIAL + offset))
}

/// ftPe_FloatAttackAir_Anim (8011BF8C).
pub fn anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let peach = f.character.get_mut::<Peach>();
    if peach.float_remaining > 0.0 {
        peach.float_remaining -= 1.0;
    }
    // ftCheckThrowB3: the script's reverse flag turns once.
    if std::mem::take(&mut f.commands.grab_release) {
        f.physics.facing = -f.physics.facing;
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        if f.character.get::<Peach>().float_attack.released {
            f.change_motion_state(CommonMotionState::Fall.into(), p.assets)?;
        } else {
            float::enter(f, p.assets, false)?;
        }
    }
    Ok(None)
}

/// ftPe_FloatAttackAir_IASA (8011C030).
pub fn input(f: &mut Fighter, p: InputPhase<'_>) {
    if !float::continue_input(&f.input, p.assets) {
        f.character.get_mut::<Peach>().float_attack.released = true;
    }
    if !f.commands.allow_interrupt {
        return;
    }
    if f.try_air_item_throw(p.assets)
        .expect("float aerial item throw")
    {
        return;
    }
    if try_enter(f, p.assets).expect("float aerial interrupt") {
        return;
    }
    f.try_aerial_jump(p.assets).expect("float aerial jump");
}

/// ftPe_FloatAttackAir_Phys (8011C0B0): ft_80084DB0 once released, else
/// the float's drift.
pub fn physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    if f.character.get::<Peach>().float_attack.released {
        callbacks::physics::fall(f, p);
    } else {
        float::float_physics(f, p);
    }
}

/// ftPe_FloatAttackAir_Coll -> ft_80082C74(ftCo_LandingAir_EnterWithLag).
/// The motion is not a common aerial, so landing takes no aerial lag.
pub fn collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    aerial::collision(f, p)
}
