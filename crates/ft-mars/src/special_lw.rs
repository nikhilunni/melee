//! Counter, ftmarsspeciallw.c (801389CC..80139344).
use crate::init::Marth;
use melee_coll::defense::AbsorbDescriptor;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, PhysicsPhase},
        ActionId, Fighter,
    },
};
#[derive(Clone, Debug, Default)]
pub struct SpecialLw {
    /// Fighter +2340, mv.ms.speciallw.x0: scaled damage (used by future Roy).
    pub damage: i32,
    /// ftColl_8007B1B8: owned Counter shield volume, using the shared defense layout.
    pub volume: Option<AbsorbDescriptor>,
    /// Fighter shield_unk0 / shield_unk1, both assigned attribute +60.
    pub collision_multiplier: f32,
}
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    if air {
        unimplemented!("ftMs_SpecialAirLw_Enter");
    }
    f.physics.self_velocity.y = 0.0;
    f.change_motion_state(ActionId(369), a)
        .expect("Counter assets");
    f.step_animation(a);
    f.commands.variables[1] = 0;
    f.character.get_mut::<Marth>().special_lw = Default::default();
}
pub fn anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    match f.commands.variables[1] {
        1 => {
            f.commands.variables[1] = 2;
            let m = f.character.get_mut::<Marth>();
            let v = &m.attributes.counter_volume;
            m.special_lw.volume = Some(AbsorbDescriptor {
                bone: v.bone as usize,
                offset: v.offset,
                radius: v.radius,
            });
            m.special_lw.collision_multiplier = m.attributes.counter.collision_multiplier;
        }
        0 => f.character.get_mut::<Marth>().special_lw.volume = None,
        _ => {}
    }
    if !f.animation.frames_remaining(&f.skeleton) {
        f.character.get_mut::<Marth>().special_lw.volume = None;
        f.change_motion_state(melee_types::CommonMotionState::Wait.into(), p.assets)?;
    }
    Ok(None)
}
pub fn hit_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    // ftMs_SpecialLwHit_Anim: only Roy scales its hitboxes; Marth uses script damage.
    if !f.animation.frames_remaining(&f.skeleton) {
        f.change_motion_state(melee_types::CommonMotionState::Wait.into(), p.assets)?;
    }
    Ok(None)
}
pub fn physics(f: &mut Fighter, p: PhysicsPhase<'_>) {
    callbacks::physics::guard_on(f, p);
    // ftColl_8007AEE0's sampled shield position belongs to the incoming-hit hook.
}
