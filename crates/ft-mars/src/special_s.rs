//! Dancing Blade, ftmarsspecials.c (8013741C..80138208).
use crate::init::Marth;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{AnimationPhase, CollisionPhase, InputPhase},
        ActionId, Fighter,
    },
    input::Buttons,
};
#[derive(Clone, Debug, Default)]
pub struct SpecialSide {
    /// Fighter +2340, mv.ms.specials.x0, reset by the first hit's entry.
    pub reserved: i32,
}
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    if air {
        unimplemented!("ftMs_SpecialAirS_Enter");
    }
    f.physics.self_velocity.y = 0.0;
    f.commands.variables[0] = 0;
    f.commands.variables[1] = 0;
    f.character.get_mut::<Marth>().special_side = Default::default();
    f.change_motion_state(ActionId(349), a)
        .expect("Dancing Blade assets");
    f.step_animation(a);
}
pub fn anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(melee_types::CommonMotionState::Wait.into(), p.assets)?;
    }
    Ok(None)
}
pub fn input(f: &mut Fighter, p: InputPhase<'_>) {
    if !f.input.pressed.intersects(Buttons::A | Buttons::B) {
        return;
    }
    if f.commands.variables[0] == 0 {
        f.commands.variables[1] = 1;
        return;
    }
    if f.commands.variables[1] != 0 {
        return;
    }
    let y = f.input.current.stick.y;
    let threshold = p.assets.input.special_vertical_threshold;
    let action = match f.motion_state.action.0 {
        349 => {
            if y > threshold {
                350
            } else {
                351
            }
        }
        350 | 351 => {
            if y > threshold {
                352
            } else if y < -threshold {
                354
            } else {
                353
            }
        }
        352..=354 => {
            if y > threshold {
                355
            } else if y < -threshold {
                357
            } else {
                356
            }
        }
        _ => unreachable!("Dancing Blade continuation row"),
    };
    // ftMs_SpecialS_80137A68 resets the per-instance hit count and stale latch.
    f.commands.variables[0] = 0;
    f.commands.variables[1] = 0;
    f.combat.stale.new_instance();
    f.change_motion_state(ActionId(action), p.assets)
        .expect("Dancing Blade continuation");
}
pub fn collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let c = &mut f.core;
    if !matches!(
        melee_ft::collision::ground::map_escape(
            &mut c.physics,
            &mut c.collision,
            p.map,
            &mut c.skeleton,
            c.animation.root,
            c.input.current.stick.x
        ),
        melee_ft::collision::ground::WaitGroundResult::Supported
    ) {
        unimplemented!("Dancing Blade preserved ground-to-air transition");
    }
    Ok(())
}
