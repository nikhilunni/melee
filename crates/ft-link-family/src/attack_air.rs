//! The Links' aerials, ftlinkattackair.c: the down aerial bounces off what
//! it strikes (lwOnHit, 800EB484) and, after a pause, strikes again
//! (lwOnAnim, 800EB528).
use crate::LinkFamily;
use melee_coll::hitbox::{CapsulePhase, HitCapsule};
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        attack::aerial,
        state::AnimationPhase,
        Fighter,
    },
};
use melee_types::CommonMotionState;

/// The down aerial's bounce: mv.lk.attackair.lw_frame_start and the three
/// hitboxes ftColl_8007AFF8 switched off, which lwOnAnim switches back on.
#[derive(Clone, Debug)]
pub struct DownAir {
    /// deal_dmg_cb / anim_cb are this motion's (Fighter_ChangeMotionState
    /// clears them).
    pub armed: bool,
    /// Frames until the hitboxes return.
    pub frame_start: f32,
    /// Retail disables x914[0..2] in place; the port keeps what they were
    /// (three slots allocated with the fighter, outside its inline payload).
    pub hits: Vec<Option<HitCapsule>>,
}
impl Default for DownAir {
    fn default() -> Self {
        Self {
            armed: false,
            frame_start: 0.0,
            hits: vec![None; 3],
        }
    }
}

/// ftLk_AttackAir_Enter (800EB400): the common aerial, and for the down
/// aerial its own damage and animation callbacks.
pub fn enter<C: LinkFamily>(f: &mut Fighter, assets: &FighterAssets) -> Result<()> {
    aerial::enter(f, assets)?;
    if f.motion_state.action == CommonMotionState::AttackAirLw.into() {
        let down_air = &mut f.character.get_mut::<C>().specials().down_air;
        down_air.armed = true;
        down_air.frame_start = 0.0;
        f.motion_row.anim = animation::<C>;
    }
    Ok(())
}

/// lwOnHit (800EB484): every hitbox off, the bounce's vertical speed, no
/// fast fall, the pause; past the bounce animation's length the motion
/// restarts there (its callbacks, and so this pause, are then the common
/// ones).
pub fn deal_damage<C: LinkFamily>(f: &mut Fighter, assets: &FighterAssets) {
    if !f.character.get::<C>().specials_ref().down_air.armed {
        return;
    }
    let a = f.character.get::<C>().attributes().down_air.clone();
    // retail 800EB4AC: fsubs.
    let frame_length = a.hit_frame_end - a.hit_frame_start;
    for i in 0..3 {
        let hit = f.commands.hitboxes[i].take();
        f.character.get_mut::<C>().specials().down_air.hits[i] = hit;
    }
    f.commands.hitboxes.fill(None);
    f.physics.self_velocity.y = a.hit_vertical_speed;
    f.physics.fast_fall = false;
    let down_air = &mut f.character.get_mut::<C>().specials().down_air;
    down_air.frame_start = a.hit_frame_start;
    if f.animation.frame > frame_length {
        let data = f.core.state_data.clone();
        f.change_motion_state_at(CommonMotionState::AttackAirLw.into(), assets, frame_length)
            .expect("down aerial assets");
        f.core.state_data = data;
    }
}

/// lwOnAnim (800EB528): the pause counts down by the animation speed; when
/// it runs out before the bounce animation's end, hitboxes 0..2 return
/// (ftColl_8007B064: enabled, histories cleared; ftColl_8007ABD0: the
/// attribute damage, staled). Then ftCo_AttackAir_Anim.
fn animation<C: LinkFamily>(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let start = f.character.get::<C>().specials_ref().down_air.frame_start;
    if start > 0.0 {
        let left = start - f.animation.speed;
        f.character.get_mut::<C>().specials().down_air.frame_start = left;
        let end = f.character.get::<C>().attributes().down_air.hit_frame_end;
        if left <= 0.0 && f.animation.frame < end {
            rearm::<C>(f);
        }
    }
    aerial::finish_animation(f, p.assets)?;
    Ok(None)
}

fn rearm<C: LinkFamily>(f: &mut Fighter) {
    assert!(
        f.player.scale == 1.0,
        "ftColl_8007ABD0: a scaled Link's down aerial"
    );
    assert!(
        f.commands.smash_charge.is_none(),
        "ftCo_800DEEB8: a charged down aerial"
    );
    let damages = f.character.get::<C>().attributes().down_air.hitbox_flags;
    for (i, damage) in damages.into_iter().enumerate() {
        let hit = f.character.get_mut::<C>().specials().down_air.hits[i].take();
        let Some(mut hit) = hit else {
            unimplemented!("lwOnAnim: re-enabling hitbox {i}, off before the bounce");
        };
        hit.phase = CapsulePhase::Enabled;
        hit.victims = Default::default();
        hit.phantom_victims = Default::default();
        let damage = damage as f32;
        hit.knockback_damage = gekko_math::msl::fctiwz(damage) as u32;
        hit.descriptor.damage = f.commands.stale_damage(damage);
        f.commands.hitboxes[i] = Some(hit);
    }
}
