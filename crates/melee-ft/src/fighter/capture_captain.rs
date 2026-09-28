//! A Falcon Dive victim, ftCo_CaptureCaptain.c, and the captor-side
//! attachment and release it shares with the throw code (ftCo_Throw.c).
//!
//! A grounded victim stays free: the captor's XRotN follows the victim's
//! TransN2 (ftCo_800DB368 with the roles swapped) and the captor copies the
//! victim's position each tick. An airborne victim hangs from the captor
//! instead, placed by accessory1 (ftCo_800DB464) like a thrown fighter.
use super::{
    assets::{FighterAssets, Result},
    grab::GrabLink,
    grab_throw::{self, ThrowSource},
    Fighter, FighterCore,
};
use hsd_types::Vec3;
use melee_types::{CommonMotionState as S, FtPart, GroundOrAir};

/// ftCo_SM_CaptureCaptain: the captor's animation the victim plays.
pub const VICTIM_MOTION: i32 = 276;

/// ftCo_800DB368 (800DB368) with `anchor` holding and `attached` hanging:
/// zero XRotN's rotation, keep its translation (x2174) and constrain it to
/// the anchor's TransN2 (lb_8000C1C0).
pub fn attach(
    attached: &mut FighterCore,
    anchor: &mut FighterCore,
    attached_assets: &FighterAssets,
    anchor_assets: &FighterAssets,
) {
    let xrot = attached.animation.parts
        [usize::from(attached_assets.parts.joint(FtPart::XRotN).expect("XRotN"))]
    .joint;
    let saved_translation = attached.skeleton.translation(xrot);
    attached.skeleton.set_rotation(
        xrot,
        &hsd_anim::quat::Quaternion {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 0.0,
        },
    );
    let hip = attached.animation.parts
        [usize::from(attached_assets.parts.joint(FtPart::HipN).expect("HipN"))]
    .joint;
    attached.combat.thrown_pose = Some(grab_throw::ThrownPose {
        saved_translation,
        hip_translation: attached.skeleton.translation(hip),
    });
    grab_throw::update_constraint(attached, anchor, attached_assets, anchor_assets);
}

/// ftCo_8009CA0C (8009CA0C), Falcon Dive's grabbed_cb: the victim plays the
/// captor's CaptureCaptain animation in place.
pub fn enter(
    victim: &mut Fighter,
    captor: &mut Fighter,
    victim_assets: &FighterAssets,
    captor_assets: &FighterAssets,
) -> Result<()> {
    // ftCommon_8007DB58.
    victim.interrupt_actions();
    // ftCo_8009750C drops a heavy item; ftCo_800DD168 releases the victim's
    // own victim.
    assert!(
        victim.core.held_item.is_none(),
        "ftCo_8009750C: a captured fighter holding an item"
    );
    assert!(
        victim.core.combat.grab.is_none(),
        "ftCo_800DD168: a captured fighter holding another"
    );
    victim.core.physics.facing = -captor.core.physics.facing;
    if victim.core.physics.ground_or_air == GroundOrAir::Air {
        // ftCo_800DB368(captor, victim); accessory1 = ftCo_800DB464, which
        // the scene runs for a captured fighter with a thrown pose.
        attach(
            &mut victim.core,
            &mut captor.core,
            victim_assets,
            captor_assets,
        );
    }
    let motion = &captor_assets.motions[&VICTIM_MOTION];
    let remap = motion.remap.as_ref().expect("prepared capture skeleton");
    let source = ThrowSource {
        assets: captor_assets,
        animation: Some((
            motion,
            crate::anim::attach::MotionRemapView {
                source: &remap.source,
                destination: &victim_assets.parts,
                source_masks: &remap.source_masks,
            },
        )),
        flags: motion.flags,
        blend_frames: motion.blend_frames,
    };
    victim.change_motion_state_with_source(
        S::CaptureCaptain.into(),
        victim_assets,
        0.0,
        1.0,
        Some(source),
    )?;
    // ftCommon_8007E2F4(fp, 0x1FF).
    victim.core.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
    victim.step_animation(victim_assets);
    // ftCommon_8007E2FC.
    victim.core.clear_movement();
    victim.core.combat.grab = Some(GrabLink::Captured {
        captor: captor.core.spawn_number,
    });
    Ok(())
}

/// ftCo_CaptureCaptain_Anim (empty): the animation still advances.
pub fn animation(
    fighter: &mut Fighter,
    phase: super::state::AnimationPhase<'_>,
) -> Result<Option<crate::anim::WaitChoice>> {
    fighter.step_animation(phase.assets);
    Ok(None)
}

/// ftCo_CaptureCaptain_Phys (empty): Fighter_procUpdate's tail only.
pub fn physics(fighter: &mut Fighter, phase: super::state::PhysicsPhase<'_>) {
    let super::state::PhysicsPhase { assets, map, wind } = phase;
    if fighter.core.physics.ground_or_air == GroundOrAir::Ground {
        crate::physics::grounded::finish_ground_update(
            &mut fighter.core.physics,
            &fighter.core.collision.data,
            &crate::physics::grounded::GroundedParameters::from_attributes(
                &fighter.core.attributes,
                &assets.common,
            ),
            map,
            wind,
        );
    } else {
        fighter.core.finish_air_update(assets, wind);
    }
}

/// ftCo_CaptureCaptain_Coll: unless hanging from the captor (x2226_b2),
/// ft_80083B68's airborne collision that never lands.
pub fn collision(fighter: &mut Fighter, phase: super::state::CollisionPhase<'_>) -> Result<()> {
    if fighter.core.combat.thrown_pose.is_some() {
        return Ok(());
    }
    let c = &mut fighter.core;
    crate::collision::air::begin_map(
        &c.physics,
        &mut c.collision,
        &mut c.skeleton,
        c.animation.root,
    );
    let cd = &mut c.collision.data;
    cd.last_pos = cd.cur_pos;
    cd.cur_pos = c.physics.position;
    let pose = crate::collision::ecb::EcbPose::read(&mut c.skeleton, c.animation.root, cd);
    phase.map.air_collide_stay(cd, Some(&|i| pose.position(i)));
    c.physics.position = cd.cur_pos;
    c.skeleton
        .set_translate(c.animation.root, &c.physics.position);
    Ok(())
}

/// ftCo_800DDDE4(captor, victim, true) (800DDDE4), then
/// ftCo_800DE7C0(victim, NULL, false): the victim takes the captor's throw
/// record 0 without a throw owner. Whichever fighter hangs (the captor from
/// a grounded victim, x221B_b7, or an airborne victim from the captor)
/// leaves the ground, detached at the other's TransN2.
pub fn release(
    victim: &mut Fighter,
    captor: &mut Fighter,
    victim_assets: &FighterAssets,
    captor_assets: &FighterAssets,
    map: &mut melee_mp::CollMap,
    rng: &mut gekko_math::HsdRng,
) -> Result<()> {
    let hit = grab_throw::record_throw_hit(
        &mut victim.core,
        &mut captor.core,
        victim_assets,
        captor_assets,
    );
    if captor.core.combat.thrown_pose.is_some() {
        captor.core.leave_ground();
        grab_throw::detach(
            &mut captor.core,
            &mut victim.core,
            captor_assets,
            victim_assets,
            map,
            true,
        );
    } else {
        victim.core.leave_ground();
        grab_throw::detach(
            &mut victim.core,
            &mut captor.core,
            victim_assets,
            captor_assets,
            map,
            true,
        );
    }
    victim.core.combat.grab = None;
    captor.core.combat.grab = None;
    // ftCo_800DE7C0(victim, NULL, false): no fn_800DE798 throw owner.
    grab_throw::launch_thrown(victim, hit, None, None, victim_assets, rng)
}

impl FighterCore {
    /// Whether this fighter is the grounded victim of a Falcon Dive.
    pub fn in_capture_captain(&self) -> bool {
        self.motion_state.id == S::CaptureCaptain
    }
    /// A position copied from the fighter this one hangs from.
    pub fn follow(&mut self, position: Vec3) {
        self.physics.position = position;
    }
}
