//! Shield exhaustion, collapse, recovery and dizzy mash (ftCo_ShieldBreak*.c).
use super::{
    assets::{FighterAssets, Result},
    state::{AnimationPhase, CollisionPhase, PhysicsPhase},
    Fighter, FighterCore, MotionData,
};
use crate::{anim::WaitChoice, input::Buttons};
use hsd_types::Vec3;
use melee_types::{combat::HurtStatus, CommonMotionState as S, FtPart};

/// ftCommon_InitGrab / GrabMash scratch, without capture ownership.
#[derive(Clone, Copy, Debug, Default)]
pub struct DizzyState {
    pub remaining: f32,
    pub stick_directions: [i8; 2],
}
impl Fighter {
    /// ftCo_80098B20 (80098B20): launch, burst, rumble and intangibility.
    pub(super) fn enter_shield_break(&mut self, assets: &FighterAssets) -> Result<()> {
        self.leave_ground();
        self.change_motion_state(S::ShieldBreakFly.into(), assets)?;
        self.step_animation(assets);
        self.core.physics.self_velocity.x = 0.0;
        self.core.physics.self_velocity.y =
            self.core.attributes.shield.shield_break_initial_velocity;
        self.core.status.unconditional_top_exit = (self.character.table().descriptor)()
            .common_behavior
            .shield_break_top_exit;
        self.core.state_data = MotionData::None;
        self.core.status.interaction = super::Interaction::Idle;
        let bone = usize::from(self.core.bones.model.shield);
        let joint = self.core.animation.parts[bone].joint;
        let scale = self.core.skeleton.get(joint).scale.y;
        // ftCo_SpawnEf (efAsync kind 0, 1051) follows ftAnim_8006EBA4, so the
        // request sits behind the ShieldBreakFly script's queued graphics.
        self.core
            .push_effect_after_issued_graphics(melee_ef::request::EffectRequest::ShieldBreak {
                bone,
                scale,
            });
        self.core.shield_sound(130);
        self.core.shield_rumble(24);
        self.core.commands.hurt_status = HurtStatus::Intangible;
        self.core
            .commands
            .color_animations
            .push(melee_cmd::ColorAnimationRequest { id: 2, duration: 0 });
        Ok(())
    }
    /// ftCo_80098E3C (80098E3C), ftCo_80097570: HipN decides the down orientation.
    fn enter_shield_break_down(&mut self, assets: &FighterAssets) -> Result<()> {
        self.land();
        let hip = self.core.animation.parts
            [usize::from(assets.parts.joint(FtPart::HipN).expect("HipN"))]
        .joint;
        let state = if self.core.skeleton.get_mtx(hip).0[1][1] > 0.0 {
            S::ShieldBreakDownU
        } else {
            S::ShieldBreakDownD
        };
        self.change_shield_break_recovery(state, assets)?;
        let normal = self.core.collision.data.floor.normal;
        // ftCo_800978D4: async kind 4, then ftCo_800976A4's landing dust.
        self.core
            .effects
            .push(melee_ef::request::EffectRequest::Graphics {
                id: 0x406,
                bone: 0,
                offset: Vec3::ZERO,
                facing: self.core.physics.facing,
                floor_angle: melee_lb::trigf::atan2f(-normal.x, normal.y),
            });
        self.core
            .commands
            .graphics
            .push(melee_types::combat::GraphicsCommand {
                id: 0x407,
                bone: 0,
                common_bone: false,
                item_bone: false,
                destroy_on_state_change: false,
                parameter: 0.0,
                offset: Vec3::ZERO,
                range: Vec3::ZERO,
            });
        Ok(())
    }
    /// Ft_MF_KeepColAnimHitStatus: Down/Stand retain break intangibility.
    fn change_shield_break_recovery(&mut self, state: S, assets: &FighterAssets) -> Result<()> {
        self.change_shield_break_motion(state, assets)
    }
    /// ftCo_80099010 (80099010): health and timer reset on dizzy entry.
    fn enter_dizzy(&mut self, assets: &FighterAssets) -> Result<()> {
        self.change_motion_state(S::Furafura.into(), assets)?;
        let p = &assets.shield;
        self.core.status.shield_health = p.break_health;
        self.core.state_data = MotionData::Dizzy(DizzyState {
            remaining: (p.dizzy_base - self.core.physics.percent).max(0.0) + p.dizzy_extra,
            stick_directions: [0; 2],
        });
        self.core.shield_rumble(25);
        self.core
            .commands
            .footstep_sounds
            .push(super::commands::FootstepSound {
                channel: super::commands::SoundChannel::StatusEffect,
                id: 95,
                volume: 127,
                pan: 64,
            });
        Ok(())
    }
}
impl FighterCore {
    pub(super) fn shield_sound(&mut self, id: u32) {
        self.commands
            .footstep_sounds
            .push(super::commands::FootstepSound {
                channel: super::commands::SoundChannel::Ordinary,
                id,
                volume: 127,
                pan: 64,
            });
    }
    /// ftCommon_8007EBAC: controller output only; no random draw.
    fn shield_rumble(&mut self, id: u16) {
        self.commands
            .rumble_requests
            .push(super::commands::RumbleRequest {
                all_players: false,
                id,
                duration: 0,
            });
    }
}
/// ftCo_ShieldBreakFly_Anim (80098C14).
pub(super) fn fly_animation(
    fighter: &mut Fighter,
    phase: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    fighter.core.status.interaction = super::Interaction::Idle;
    fighter.step_animation(phase.assets);
    if !fighter
        .core
        .animation
        .frames_remaining(&fighter.core.skeleton)
    {
        fighter.change_shield_break_fall(phase.assets)?;
        // ftCommon_ClampAirDrift (8007D468), after motion entry.
        let maximum = fighter.core.attributes.air.air_drift_max;
        fighter.core.physics.self_velocity.x = fighter
            .core
            .physics
            .self_velocity
            .x
            .clamp(-maximum, maximum);
        fighter.core.shield_rumble(8);
    }
    Ok(None)
}
/// ftCo_ShieldBreakFall_Anim (80098DEC) is empty; ordinary animation still advances.
pub(super) fn fall_animation(
    fighter: &mut Fighter,
    phase: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    fighter.step_animation(phase.assets);
    Ok(None)
}
/// ftCo_ShieldBreakDown_Anim (80098EBC).
pub(super) fn down_animation(
    fighter: &mut Fighter,
    phase: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    fighter.step_animation(phase.assets);
    if !fighter
        .core
        .animation
        .frames_remaining(&fighter.core.skeleton)
    {
        let state = if fighter.core.motion_state.id == S::ShieldBreakDownU {
            S::ShieldBreakStandU
        } else {
            S::ShieldBreakStandD
        };
        fighter.change_shield_break_recovery(state, phase.assets)?;
    }
    Ok(None)
}
/// ftCo_ShieldBreakStand_Anim (80098F90).
pub(super) fn stand_animation(
    fighter: &mut Fighter,
    phase: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    fighter.step_animation(phase.assets);
    if !fighter
        .core
        .animation
        .frames_remaining(&fighter.core.skeleton)
    {
        fighter.enter_dizzy(phase.assets)?;
    }
    Ok(None)
}
/// ftCo_Furafura_Anim (800990B8), ftCommon_GrabMash (8007DC08).
pub(super) fn dizzy_animation(
    fighter: &mut Fighter,
    phase: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    fighter.step_animation(phase.assets);
    let p = &phase.assets.shield;
    let core = &mut fighter.core;
    core.status.shield_health = p.break_health;
    let MotionData::Dizzy(dizzy) = &mut core.state_data else {
        panic!("dizzy scratch missing")
    };
    dizzy.remaining -= p.dizzy_decay;
    if core
        .input
        .pressed
        .intersects(Buttons::A | Buttons::B | Buttons::X | Buttons::Y | Buttons::SHIELD)
    {
        dizzy.remaining -= p.dizzy_mash;
    }
    let previous = dizzy.stick_directions;
    for (direction, axis) in dizzy
        .stick_directions
        .iter_mut()
        .zip([core.input.current.stick.x, core.input.current.stick.y])
    {
        if axis < -p.mash_stick_threshold {
            *direction = -1;
        }
        if axis > p.mash_stick_threshold {
            *direction = 1;
        }
    }
    if previous != dizzy.stick_directions {
        dizzy.remaining -= p.dizzy_mash;
    }
    if dizzy.remaining <= 0.0 {
        fighter.change_motion_state(S::Wait.into(), phase.assets)?;
    }
    Ok(None)
}
/// ftCo_ShieldBreakFly_Phys -> ft_80084EEC: gravity and air friction, no input.
pub(super) fn fly_physics(fighter: &mut Fighter, phase: PhysicsPhase<'_>) {
    crate::fighter::state::callbacks::physics::air_friction(fighter, phase)
}
/// ftCo_ShieldBreakFly_Coll -> ft_80082C74: ordinary air collision, no ledge grab.
pub(super) fn fly_collision(fighter: &mut Fighter, phase: CollisionPhase<'_>) -> Result<()> {
    use crate::collision::air;
    let core = &mut fighter.core;
    air::begin_map(
        &core.physics,
        &mut core.collision,
        &mut core.skeleton,
        core.animation.root,
    );
    if air::collide_air_dodge(
        &mut core.physics,
        &mut core.collision,
        phase.map,
        &mut core.skeleton,
        core.animation.root,
    ) {
        fighter.enter_shield_break_down(phase.assets.expect("shield break landing assets"))?;
    }
    Ok(())
}
