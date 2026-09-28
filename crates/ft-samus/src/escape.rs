//! The morph-ball roll, ftCo_Escape.c's FTKIND_SAMUS arm (ftCo_80099390,
//! ftCo_80099564, ftCo_80099754).
//!
//! A roll clears cmd_vars[0] before its motion change, then replaces the
//! row's animation and collision callbacks: while the script holds
//! cmd_vars[0] Samus is a ball (every capsule intangible but one on XRotN)
//! and collides with the ball's fixed box.
use crate::init::Samus;
use melee_ft::{
    anim::WaitChoice,
    collision::{air, ground},
    fighter::{
        assets::Result,
        state::{callbacks, AnimationPhase, CollisionPhase},
        Fighter, MotionData,
    },
};
use melee_types::{combat::HurtStatus, CommonMotionState as S, FtPart};

/// ftSs_SpecialLw_8012AEBC's capsule: `hurt.scale = 3`.
const BALL_CAPSULE_RADIUS: f32 = 3.0;

/// ftCo_80099390 (80099390), before ftCo_80099314: cmd_vars[0] = 0.
pub fn prepare_roll(f: &mut Fighter) {
    f.commands.variables[0] = 0;
}

/// ftCo_80099390 after ftCo_80099314: mv.co.escape.x4 is false and the
/// roll's callbacks are Samus's.
pub fn roll_entered(f: &mut Fighter) {
    f.character.get_mut::<Samus>().ball = false;
    set_ball_word(f, false);
    f.motion_row.anim = roll_animation;
    f.motion_row.collision = roll_collision;
}

/// mv.co.escape.x4, which a later state may inherit as mv+4.
fn set_ball_word(f: &mut Fighter, ball: bool) {
    if let MotionData::Escape(escape) = &mut f.core.state_data {
        escape.retained_word = Some(f32::from_bits(u32::from(ball)));
    }
}

/// ftSs_SpecialLw_8012AEBC (8012AEBC): every capsule intangible, capsule 0
/// the ball on XRotN (middle height, not grabbable, radius 3).
pub fn become_ball(f: &mut Fighter) {
    f.core.set_hurt_capsules(HurtStatus::Intangible);
    let bone = crate::common::part(FtPart::XRotN);
    f.core.replace_hurt_capsule(
        0,
        melee_coll::hurtbox::HurtCapsule {
            height: melee_coll::hurtbox::HurtHeight::Middle,
            grabbable: false,
            bone,
            offsets: [hsd_types::Vec3::ZERO; 2],
            radius: BALL_CAPSULE_RADIUS,
            positions: [hsd_types::Vec3::ZERO; 2],
            cached: false,
        },
    );
}

/// ftSs_SpecialLw_8012AF38 (8012AF38): every capsule enabled again.
pub fn leave_ball(f: &mut Fighter) {
    f.core.set_hurt_capsules(HurtStatus::Normal);
}

/// ftCo_80099564 (80099564): the ball follows cmd_vars[0], then
/// ftCo_Escape_Anim.
fn roll_animation(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    let assets = p.assets;
    f.step_animation(assets);
    f.advance_smash_charge(assets);
    let script = f.commands.variables[0] != 0;
    let ball = f.character.get::<Samus>().ball;
    if script && !ball {
        become_ball(f);
        f.character.get_mut::<Samus>().ball = true;
        set_ball_word(f, true);
    }
    if !script && f.character.get::<Samus>().ball {
        leave_ball(f);
        f.character.get_mut::<Samus>().ball = false;
        set_ball_word(f, false);
    }
    f.escape_animation(assets)?;
    Ok(None)
}

/// ftCo_80099754 (80099754): the ball's box stopping at the floor's edge
/// (ft_800847D0), otherwise ft_80084104; leaving the floor falls.
fn roll_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    if f.commands.variables[0] == 0 {
        return callbacks::collision::escape(f, p);
    }
    let ecb = f.character.get::<Samus>().attributes.morph_ball_box;
    let c = &mut f.core;
    air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    let supported = ground::collide_box(
        &mut c.physics,
        &mut c.collision,
        p.map,
        &mut c.skeleton,
        c.animation.root,
        ecb,
        true,
    );
    if !supported {
        let assets = p.assets.expect("morph-ball roll fall assets");
        f.leave_ground();
        f.change_motion_state(S::Fall.into(), assets)?;
    }
    Ok(())
}
