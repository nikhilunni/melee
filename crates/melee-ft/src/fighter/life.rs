//! Stock loss and revival, ft_0D31.c / ft_0D4D.c.
use super::FighterCore;
use super::{
    assets::{FighterAssets, Result},
    Fighter, MotionData,
};
use hsd_types::Vec3;
use melee_types::CommonMotionState as S;

#[derive(Clone, Copy, Debug)]
pub struct Arena {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
    pub camera_top: f32,
    pub revival_positions: [Vec3; 4],
    /// grLast stage initialization (801DFF18): stage_info.unk8C.b4.
    pub player_revival_markers: bool,
}
#[derive(Clone, Debug)]
pub enum LifeState {
    Dead { remaining: i32 },
    AwaitingRespawn,
    Revival { remaining: i32, target: Vec3 },
    PlatformWait { remaining: i32, target: Vec3 },
}
/// Fighter accessory JObj, loaded from PlCo ftLoadCommonData[8].
#[derive(Clone, Debug)]
pub struct RevivalPlatform {
    pub tree: hsd_anim::jobj::JObjTree,
    pub root: hsd_anim::jobj::JObjId,
}
pub struct LifeParameters {
    pub death_delay: i32,
    pub revival_duration: i32,
    pub platform_duration: i32,
    pub invincibility_duration: i32,
    pub death_effect_scale: f32,
}
impl Fighter {
    /// gm_8016719C -> Player_80032070 -> Fighter_UnkProcessDeath (80068354).
    pub fn reset_for_revival(
        &mut self,
        assets: &FighterAssets,
        arena: &Arena,
        context: super::SpawnContext<'_>,
    ) -> Result<()> {
        if !arena.player_revival_markers {
            unimplemented!("gm_80167638: shared revival marker offset allocation");
        }
        let target = arena.revival_positions[usize::from(self.core.player.id)];
        self.core.player.position = Vec3::new(target.x, arena.camera_top, 0.0);
        self.core.player.facing = if self.core.player.position.x >= 0.0 {
            -1.0
        } else {
            1.0
        };
        self.core.player.damage = 0.0;
        self.core.reset_life(assets, context.map);
        self.install_motion_row(super::state::COMMON[S::Wait as usize]);
        let scale = self.core.skeleton.scale(self.core.animation.root);
        self.initialize_spawn(assets, context, None, scale)?;
        self.enter_revival(assets, target)
    }
    /// ftCo_800D3158 / ftCo_800D3BC8 (800D3158 / 800D3BC8), after Update.
    pub fn check_blast_zone(&mut self, assets: &FighterAssets, arena: &Arena) -> Result<()> {
        if matches!(self.core.state_data, MotionData::Life(_))
            || self.core.status.disabled
            || self.core.status.ledge_grab_disabled
        {
            return Ok(());
        }
        let p = self.core.physics.position;
        if p.x < arena.left || p.x > arena.right || p.y > arena.top {
            unimplemented!("ftCo_800D3158: side/up death");
        }
        if p.y >= arena.bottom {
            return Ok(());
        }
        if self.core.combat.grab.is_some() {
            unimplemented!("ftCo_800D331C: release linked fighter on death");
        }
        // ftCommon_8007E2FC (8007E2FC): death clears every velocity owner.
        self.core.physics.self_velocity = Vec3::ZERO;
        self.core.physics.animation_velocity = Vec3::ZERO;
        self.core.physics.knockback_velocity = Vec3::ZERO;
        self.core.physics.shield_knockback_velocity = Vec3::ZERO;
        self.core.physics.ground_velocity = 0.0;
        self.core.physics.ground_knockback_velocity = 0.0;
        self.core.physics.ground_shield_knockback_velocity = 0.0;
        self.change_motion_state(S::DeadDown.into(), assets)?;
        self.core.state_data = MotionData::Life(LifeState::Dead {
            remaining: assets.life.death_delay,
        });
        self.core.player.stocks = self.core.player.stocks.saturating_sub(1);
        self.core.effect_state.invisible = true;
        self.core
            .effects
            .push(melee_ef::request::EffectRequest::Death {
                position: p,
                scale: assets.life.death_effect_scale,
            });
        Ok(())
    }
    /// ftCo_800D4FF4 (800D4FF4), after Fighter_UnkProcessDeath reset.
    pub fn enter_revival(&mut self, assets: &FighterAssets, target: Vec3) -> Result<()> {
        self.leave_ground();
        self.change_motion_state(S::Rebirth.into(), assets)?;
        self.core.state_data = MotionData::Life(LifeState::Revival {
            remaining: assets.life.revival_duration,
            target,
        });
        self.core.status.input_frozen = false;
        self.core.status.ignore_fighter_nudge = true;
        self.core.commands.hurt_status = melee_types::combat::HurtStatus::Intangible;
        let platform = &mut self.core.revival_platform;
        platform.tree.req_anim_all(platform.root, 0.0);
        // ftCoD4FF4 (800D51C0): separate model-scale product, no FMA.
        let scale = self.core.player.scale
            * self.core.attributes.size.model_scaling
            * self.core.attributes.size.respawn_platform_scale;
        platform
            .tree
            .set_scale(platform.root, &Vec3::new(scale, scale, scale));
        platform
            .tree
            .set_translate(platform.root, &self.core.physics.position);
        self.core.revival_platform_active = true;
        Ok(())
    }
    /// ftCo_RebirthWait_IASA (ft_0D4D.c): leave the revival platform on input.
    ///
    /// Priority inputs first (special, item pickup with LR+A, tether catch, air
    /// dodge, item throw, aerial jump); otherwise a held shield bit, D-pad up, the
    /// squat/turn/walk stick tests or a lost partner drop into Fall. Either way
    /// the fighter receives the revival invincibility (`ftColl_8007B7A4` with
    /// PlCo +5D8) and the intangibility colour flash (`ftCo_800BFFD0(fp, 9, 0)`).
    pub(super) fn revival_input(&mut self, assets: &FighterAssets) -> Result<()> {
        use crate::input::Buttons;
        let common = &assets.input;
        let facing = self.core.physics.facing;
        let pressed = self.core.input.pressed;
        let held = self.core.input.current.held;
        let stick = self.core.input.current.stick;
        // Retail also requires x683 >= PlCo +1C (the A-timer window); that constant is
        // not loaded yet, so this superset stops explicitly whenever LR+A is pressed.
        let lr_plus_a = held.intersects(Buttons::SHIELD) && pressed.intersects(Buttons::A);
        // No partner (x221F_b4 is the Ice Climbers' Nana flag): var_r30 stays 0.
        let priority = if pressed.intersects(Buttons::B) {
            // ftCo_SpecialAir_CheckInput
            self.enter_buffered_special(assets, true);
            true
        } else if lr_plus_a {
            // ftCo_800D7100 / ftCo_800D705C: item pickup and the x209C catch timer.
            unimplemented!("ftCo_RebirthWait_IASA: LR+A item pickup / catch timer");
        } else if pressed.intersects(Buttons::DIGITAL_SHOULDERS) {
            // ftCo_800C3B10 is tether characters only; ftCo_80099A58 -> EscapeAir.
            self.enter_air_dodge(assets)?;
            true
        } else if self.aerial_jump_requested(assets) {
            // ftCo_800CB870 -> ftCo_JumpAerial_CheckInput.
            self.enter_aerial_jump(assets)?;
            true
        } else {
            false
        };
        if !priority {
            let thresholds = &common.thresholds;
            let fall = held.intersects(Buttons::SHIELD) // ftCo_80091A2C
                || pressed.intersects(Buttons::UP) // ftCo_800DE9B8
                || stick.y < -thresholds.squat_stick_threshold // fn_800D5F84
                || stick.x * facing <= thresholds.turn_stick_threshold // ftCo_800C97A8
                || stick.x * facing >= thresholds.walk_stick_threshold; // ftWalkCommon_800DFC70
            if !fall {
                return Ok(());
            }
            self.change_motion_state(S::Fall.into(), assets)?; // ftCo_Fall_Enter
        }
        // ftColl_8007B7A4(gobj, p_ftCommonData->x5D8): x1994 = max(x1994, dur); the x198C
        // flash-type selector is renderer state and is not modelled.
        self.core.status.ledge_intangibility = self
            .core
            .status
            .ledge_intangibility
            .max(assets.life.invincibility_duration);
        self.core
            .commands
            .color_animations
            .push(melee_cmd::ColorAnimationRequest { id: 9, duration: 0 });
        // pl_80040374: stamps the stale-move table's platform-exit frame (xD60);
        // it feeds no compared key or RNG draw.
        Ok(())
    }
    /// Rebirth_Anim (800D52F8), RebirthWait_Anim (800D56EC).
    pub(super) fn revival_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        match &mut self.core.state_data {
            MotionData::Life(LifeState::Revival { remaining, target }) => {
                *remaining -= 1;
                if *remaining == 0 {
                    let target = *target;
                    melee_mp::set_position(
                        &mut self.core.collision.data,
                        &self.core.physics.position,
                    );
                    self.core.physics.self_velocity.y = 0.0;
                    self.change_motion_state(S::RebirthWait.into(), assets)?;
                    self.core.state_data = MotionData::Life(LifeState::PlatformWait {
                        remaining: assets.life.platform_duration,
                        target,
                    });
                    self.core.status.ignore_fighter_nudge = true;
                    self.core.commands.hurt_status = melee_types::combat::HurtStatus::Intangible;
                }
            }
            MotionData::Life(LifeState::PlatformWait { remaining, .. }) => {
                *remaining -= 1;
                if *remaining == 0 {
                    unimplemented!(
                        "ftCo_RebirthWait_Anim: platform timeout -> Fall with invincibility"
                    );
                }
            }
            _ => panic!("revival scratch"),
        }
        Ok(())
    }
}
impl FighterCore {
    /// Fighter_UnkInitReset (80067C98): retain loaded resources and reset live state.
    fn reset_life(&mut self, assets: &FighterAssets, map: &melee_mp::CollMap) {
        let player = &self.player;
        // Same audited coordinate calculation as initial preparation (80067CE8).
        let offset = 0.0 * player.scale;
        let position = Vec3::new(
            gekko_math::fma::fmadds(player.facing, offset, player.position.x),
            player.position.y,
            player.position.z,
        );
        self.animation.reset_for_spawn(&mut self.skeleton);
        self.skeleton.set_translate(self.animation.root, &position);
        self.physics = crate::physics::FighterPhysics::standing(position, player.facing);
        self.physics.percent = player.damage;
        self.input = crate::input::FighterInput::default();
        self.collision = super::EnvironmentCollision::new(crate::collision::ecb::initialize(
            map,
            position,
            &self.bones.ecb,
            player.scale,
            self.attributes.size.weight,
        ));
        self.state_data = MotionData::None;
        self.combat = super::damage::CombatState::default();
        self.shield = super::shield::ShieldState::default();
        self.effect_state = super::effects::FighterEffects::default();
        self.effects = melee_ef::request::EffectQueue::default();
        self.status = super::Status::reset(assets.shield_health);
        self.commands = super::commands::CommandState::default();
        self.ground_pose = crate::collision::pose::GroundPoseFlags::default();
        self.dynamics_use_floor_plane = false;
        self.player_position = position;
        self.player_facing = player.facing;
        self.joystick_count = 0;
        self.previous_collision_bounds = Vec3::ZERO;
        self.camera = super::CameraSubject::default();
        self.hurtboxes.clone_from_slice(&assets.hurtboxes);
        self.dynamic_colliders
            .clone_from_slice(&assets.dynamic_colliders);
        self.thrown_hitbox.clone_from(&assets.thrown_hitbox);
        self.revival_platform_active = false;
    }
    /// ftCo_DeadDown_Anim (800D3E00): GM respawn is performed at this callback boundary.
    pub(super) fn death_animation(&mut self) {
        let MotionData::Life(LifeState::Dead { remaining }) = &mut self.state_data else {
            panic!("death scratch");
        };
        *remaining -= 1;
        if *remaining == 0 {
            if self.player.stocks == 0 {
                unimplemented!("gm_80167320: final stock / elimination");
            }
            self.state_data = MotionData::Life(LifeState::AwaitingRespawn);
        }
    }
    /// fn_800D54A4 and Fighter_8006C80C: place then animate the accessory.
    pub fn update_revival_platform(&mut self) {
        if self.revival_platform_active {
            let platform = &mut self.revival_platform;
            platform
                .tree
                .set_translate(platform.root, &self.physics.position);
            platform.tree.anim_all::<super::RetailTrig>(platform.root);
            assert!(
                platform.tree.events.is_empty(),
                "revival accessory event routing"
            );
        }
    }
    /// Rebirth_Phys (800D535C) / RebirthWait_Phys (800D58F4).
    pub(super) fn revival_physics(&mut self, assets: &FighterAssets, wind: Vec3) {
        let (remaining, target) = match self.state_data {
            MotionData::Life(
                LifeState::Revival { remaining, target }
                | LifeState::PlatformWait { remaining, target },
            ) => (remaining, target),
            _ => panic!("revival physics scratch"),
        };
        // This scene's static marker has zero player offset. The retail FMA
        // at 800D53BC / 800D5954 is the moving-marker offset path.
        let inv = 1.0 / remaining as f32;
        self.physics.self_velocity.x = (target.x - self.physics.position.x) * inv;
        self.physics.self_velocity.y = (target.y - self.physics.position.y) * inv;
        self.decay_air_knockback(assets);
        crate::physics::integrate::integrate_velocity(&mut self.physics);
        crate::physics::integrate::integrate_environment(&mut self.physics, None, wind);
    }
}
