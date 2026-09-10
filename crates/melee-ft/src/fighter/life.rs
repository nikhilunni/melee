//! Stock loss and revival, ft_0D31.c / ft_0D4D.c.
use super::FighterCore;
use super::{
    assets::{FighterAssets, Result},
    Fighter, MotionData,
};
use gekko_math::{fma::fmsubs, HsdRng};
use hsd_types::Vec3;
use melee_ef::request::EffectRequest;
use melee_types::{CommonMotionState as S, GroundOrAir};

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
    /// DeadDown / DeadLeft / DeadRight (and a star KO once it vanishes): the x40
    /// countdown to the GM respawn request.
    Dead {
        remaining: i32,
    },
    /// DeadUpStar before the vanish: x40 countdown, x44 phase 0 (hold) / 1 (flight).
    /// `camera_top` is Stage_GetCamBoundsTopOffset, sampled at entry for the flight aim.
    StarKo {
        remaining: i32,
        flying: bool,
        camera_top: f32,
    },
    /// DeadUpFall phase 0: the x524 hold before the camera-space approach.
    ScreenKo {
        remaining: i32,
    },
    AwaitingRespawn,
    Revival {
        remaining: i32,
        target: Vec3,
    },
    PlatformWait {
        remaining: i32,
        target: Vec3,
    },
}
/// Fighter accessory JObj, loaded from PlCo ftLoadCommonData[8].
#[derive(Clone, Debug)]
pub struct RevivalPlatform {
    pub tree: hsd_anim::jobj::JObjTree,
    pub root: hsd_anim::jobj::JObjId,
}
pub struct LifeParameters {
    /// +500: frames from a side/bottom death entry to the respawn request.
    pub death_delay: i32,
    pub revival_duration: i32,
    pub platform_duration: i32,
    pub invincibility_duration: i32,
    /// +4F4: uniform scale of the blast-zone explosion (efSync 0x42B).
    pub death_effect_scale: f32,
    /// +4F0: a top exit only counts with upward knockback above this (or grounded).
    pub top_knockback_threshold: f32,
    pub star: StarKoParameters,
    /// +520: `HSD_Randi(100) + 1 <= threshold` picks the screen KO over the star KO.
    pub screen_ko_threshold: i32,
    /// +524: frames a screen KO holds at the exit position before the approach.
    pub screen_ko_hold: i32,
    pub death_sounds: DeathSounds,
}

/// PlCo +504..+514: the star KO (ftCo_DeadUpStar_Anim).
#[derive(Clone, Copy, Debug)]
pub struct StarKoParameters {
    /// +504: frames held at the exit position.
    pub hold: i32,
    /// +508: frames of flight toward the background.
    pub flight: i32,
    /// +50C: frames from the vanish to the respawn request.
    pub vanish_delay: i32,
    /// +510: total z travel over the flight.
    pub depth: f32,
    /// +514: the flight aims at this fraction of the camera-bounds top.
    pub height_ratio: f32,
}

/// ft_data->x4C_sfx +4 / +8 / +C: the fighter's death voice ids.
#[derive(Clone, Copy, Debug)]
pub struct DeathSounds {
    /// ftCo_800D38B8 plays both on the voice channel at every death entry.
    pub cries: [u32; 2],
    /// ftCo_800D40B8: the star KO cry.
    pub star: u32,
}

/// Which side blast zone a fighter left through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Left,
    Right,
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
    /// ftCo_800D3158 (800D3158), after Update: blast-zone exits in retail order
    /// (right, left, top, bottom).
    pub fn check_blast_zone(
        &mut self,
        assets: &FighterAssets,
        arena: &Arena,
        rng: &mut HsdRng,
    ) -> Result<()> {
        if matches!(self.core.state_data, MotionData::Life(_))
            || self.core.status.disabled
            || self.core.status.ledge_grab_disabled
        {
            return Ok(());
        }
        let p = self.core.physics.position;
        if p.x > arena.right {
            return self.enter_side_death(assets, arena, Side::Right);
        }
        if p.x < arena.left {
            return self.enter_side_death(assets, arena, Side::Left);
        }
        if p.y > arena.top {
            let counts = self.core.physics.ground_or_air == GroundOrAir::Ground
                || self.core.status.unconditional_top_exit
                || self.core.physics.knockback_velocity.y > assets.life.top_knockback_threshold;
            if counts {
                // Player_GetMoreFlagsBit5 (plain DeadUp) and Camera_8003010C (the fixed
                // camera) are both off in a Vs match; DamageIce victims stop in damage.rs.
                let roll = rng.randi(100) + 1;
                return if assets.life.screen_ko_threshold >= roll {
                    self.enter_screen_ko(assets)
                } else {
                    self.enter_star_ko(assets, arena)
                };
            }
        }
        if p.y < arena.bottom {
            return self.enter_bottom_death(assets, arena);
        }
        Ok(())
    }
    /// ftCo_800D3BC8 (800D3BC8): the bottom exit, DeadDown. The explosion is
    /// clamped to the side blast zones and drawn upright.
    fn enter_bottom_death(&mut self, assets: &FighterAssets, arena: &Arena) -> Result<()> {
        let mut effect = self.core.physics.position;
        if effect.x > arena.right {
            effect.x = arena.right;
        }
        if effect.x < arena.left {
            effect.x = arena.left;
        }
        self.enter_death(assets, S::DeadDown, 0x61, effect, 0.0)
    }
    /// ftCo_800D3680 / ftCo_800D3950 (800D3680 / 800D3950): the side exits, DeadLeft /
    /// DeadRight. The explosion is clamped to the top/bottom blast zones and rotated
    /// a quarter turn toward the exit.
    fn enter_side_death(
        &mut self,
        assets: &FighterAssets,
        arena: &Arena,
        side: Side,
    ) -> Result<()> {
        let mut effect = self.core.physics.position;
        if effect.y > arena.top {
            effect.y = arena.top;
        }
        if effect.y < arena.bottom {
            effect.y = arena.bottom;
        }
        let (state, sound, angle) = match side {
            Side::Left => (S::DeadLeft, 0x88, -std::f32::consts::FRAC_PI_2),
            Side::Right => (S::DeadRight, 0x89, std::f32::consts::FRAC_PI_2),
        };
        self.enter_death(assets, state, sound, effect, angle)
    }
    /// Shared body of the DeadDown / DeadLeft / DeadRight entries: the fighter
    /// vanishes, loses the stock immediately and counts down PlCo +500.
    fn enter_death(
        &mut self,
        assets: &FighterAssets,
        state: S,
        exit_sound: u32,
        effect_position: Vec3,
        effect_angle: f32,
    ) -> Result<()> {
        self.release_for_death();
        self.change_motion_state(state.into(), assets)?;
        self.core.state_data = MotionData::Life(LifeState::Dead {
            remaining: assets.life.death_delay,
        });
        // x2219_b1 / x221E_b1 / x221E_b2 / x221F_b1 are the dead flags MotionData::Life
        // stands for; pl_8003DF44 stamps the killer's stale-move table (no compared key).
        self.core.effect_state.invisible = true;
        // Camera_RequestQuake(QuakeKind_Large) and the ftCo_800D35FC rumble have no
        // simulated observer.
        self.core.lose_stock();
        self.core.play_death_sounds(assets, exit_sound);
        self.core.effects.push(EffectRequest::Death {
            position: effect_position,
            angle: effect_angle,
            scale: assets.life.death_effect_scale,
        });
        // ftCo_800D4E50: the coin-mode payout only.
        Ok(())
    }
    /// ftCo_800D40B8 (800D40B8): the star KO entry, DeadUpStar. The stock is only
    /// lost when the star vanishes (ftCo_DeadUpStar_Anim phase 1).
    fn enter_star_ko(&mut self, assets: &FighterAssets, arena: &Arena) -> Result<()> {
        self.release_for_death();
        self.change_motion_state(S::DeadUpStar.into(), assets)?;
        self.core.state_data = MotionData::Life(LifeState::StarKo {
            remaining: assets.life.star.hold,
            flying: false,
            camera_top: arena.camera_top,
        });
        // ftCo_800D40B8_inline: dead flags and ft_80088C5C (stop the fighter's channels).
        // ftCommon_8007EFC0(fp, true): the nametag countdown.
        self.core.status.name_tag_timer = 1;
        // ft_800881D8(x4C_sfx->xC, 127, 64) on the voice channel.
        self.core.push_voice(assets.life.death_sounds.star);
        // pl_8003DF44 as above; x68 = 0 marks the non-ice variant.
        Ok(())
    }
    /// ftCo_DeadUpStar_Anim (800D42E4): hold for PlCo +504, then fly toward the
    /// background for PlCo +508 frames aiming at `height_ratio` of the camera top.
    pub(super) fn star_ko_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        self.step_animation(assets);
        let star = &assets.life.star;
        let position_y = self.core.physics.position.y;
        let MotionData::Life(LifeState::StarKo {
            remaining,
            flying,
            camera_top,
        }) = &mut self.core.state_data
        else {
            // Phase 2, after the vanish: the PlCo +50C countdown to the respawn request.
            self.core.death_animation();
            return Ok(());
        };
        // The ice variant (x68) also spins the XRotN bone here.
        if *remaining != 0 {
            *remaining -= 1;
        }
        if *remaining != 0 {
            return Ok(());
        }
        if *flying {
            return self.vanish_star_ko(assets);
        }
        // retail 800D4438: fmsubs, then fdivs by the int-converted frame count.
        let frames = star.flight as f32;
        let velocity_y = fmsubs(star.height_ratio, *camera_top, position_y) / frames;
        let velocity_z = star.depth / frames;
        *remaining = star.flight;
        *flying = true;
        self.core.physics.self_velocity.y = velocity_y;
        self.core.physics.self_velocity.z = velocity_z;
        Ok(())
    }
    /// ftCo_DeadUpStar_Anim phase 1 -> 2 (800D4484..800D4530): the star vanishes.
    fn vanish_star_ko(&mut self, assets: &FighterAssets) -> Result<()> {
        // ftCommon_8007E2FC; ftCommon_8007DB24 is the ice variant only.
        self.core.clear_velocities();
        // efAsync_Spawn(gobj, &x60C, 2, 0x42D, NULL, &cur_pos): efLib_CreateGenerator
        // 0x121 at the fighter position (root-relative, zero offset).
        self.core.effects.push(EffectRequest::Landing {
            id: 0x42D,
            offset: Vec3::ZERO,
            floor_angle: 0.0,
        });
        // ftCo_800D4E50: the coin-mode payout only. x221F_b1, the vanish, ft_80088C5C:
        self.core.effect_state.invisible = true;
        self.core.play_death_sounds(assets, 0x83);
        // ftCo_800D34E0: the stock is lost here, which also fires the HUD explosion.
        self.core.lose_stock();
        self.core.state_data = MotionData::Life(LifeState::Dead {
            remaining: assets.life.star.vanish_delay,
        });
        Ok(())
    }
    /// ftCo_800D4780 -> ftCo_800D4580 (800D4580): the screen KO entry, DeadUpFall.
    fn enter_screen_ko(&mut self, assets: &FighterAssets) -> Result<()> {
        self.release_for_death();
        self.change_motion_state(S::DeadUpFall.into(), assets)?;
        self.core.state_data = MotionData::Life(LifeState::ScreenKo {
            remaining: assets.life.screen_ko_hold,
        });
        // x2220_b7: from here the render callback (ftDrawCommon_80080E18_inline2) places
        // the fighter through the camera's inverse view matrix; the lerp endpoints
        // (PlCo +538 / +544) and the lbl_803B7500 rotation reset are camera-space state.
        // Dead flags, ft_80088C5C, ftCommon_8007EFC0(fp, true):
        self.core.status.name_tag_timer = 1;
        // ftCo_800BFFD0(fp, 0x2B, 0): the screen-KO colour animation.
        self.core
            .commands
            .color_animations
            .push(melee_cmd::ColorAnimationRequest {
                id: 0x2B,
                duration: 0,
            });
        // pl_8003DF44 as above; x68 = 0 marks the non-ice variant.
        Ok(())
    }
    /// ftCo_DeadUpFall_Anim (800D4854), phase 0: the PlCo +524 hold. The approach
    /// that follows is camera-space.
    pub(super) fn screen_ko_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        self.step_animation(assets);
        let MotionData::Life(LifeState::ScreenKo { remaining }) = &mut self.core.state_data else {
            panic!("screen KO scratch");
        };
        if *remaining != 0 {
            *remaining -= 1;
        }
        if *remaining == 0 {
            unimplemented!(
                "ftDrawCommon_80080E18_inline2: the screen-KO approach (x2220_b7) positions \
                 the fighter through the camera inverse view matrix; needs the camera port"
            );
        }
        Ok(())
    }
    /// ftCo_800D331C (800D331C): detach everything the fighter owns before a death
    /// entry. Fox and Marth have no death1/2/3_cb hooks; held items (item_gobj,
    /// x197C, x1980), metal and the x2226_b4 hat are not part of the port yet.
    fn release_for_death(&mut self) {
        if self.core.combat.grab.is_some() {
            unimplemented!("ftCo_800D331C: release linked fighter on death");
        }
        self.core.clear_velocities();
        // ftCommon_8007DB24: x2219_b0 = 0, then efLib_DestroyAll.
        self.core.effect_state.destroy_on_state_change = false;
        self.core.effects.push(EffectRequest::DestroyOwned);
        // x6C / x70 keep the fatal motion id for the stale-move stats.
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
    /// ftCommon_8007E2FC (8007E2FC): a death clears every velocity owner.
    pub(super) fn clear_velocities(&mut self) {
        self.physics.self_velocity = Vec3::ZERO;
        self.physics.animation_velocity = Vec3::ZERO;
        self.physics.knockback_velocity = Vec3::ZERO;
        self.physics.shield_knockback_velocity = Vec3::ZERO;
        self.physics.ground_velocity = 0.0;
        self.physics.ground_knockback_velocity = 0.0;
        self.physics.ground_shield_knockback_velocity = 0.0;
    }
    /// ftCo_800D34E0 (800D34E0): the stock-loss bookkeeping. Falls, KO/suicide
    /// counts, the match frame count and Player_SetHPByIndex(0) feed no compared
    /// key; the stock count drives the stock display.
    fn lose_stock(&mut self) {
        self.player.stocks = self.player.stocks.saturating_sub(1);
    }
    /// ft_80088C5C stops the fighter's channels, ftCo_800D38B8 plays x4C_sfx +4 and +8
    /// on the voice channel, then ft_PlaySFX(exit, 127, 64).
    fn play_death_sounds(&mut self, assets: &FighterAssets, exit_sound: u32) {
        use super::commands::{FootstepSound, SoundChannel};
        for id in assets.life.death_sounds.cries {
            self.push_voice(id);
        }
        self.commands.footstep_sounds.push(FootstepSound {
            channel: SoundChannel::Ordinary,
            id: exit_sound,
            volume: 127,
            pan: 64,
        });
    }
    /// ft_800881D8(fp, id, 127, 64): one request on the fighter's voice channel.
    fn push_voice(&mut self, id: u32) {
        use super::commands::{FootstepSound, SoundChannel};
        self.commands.footstep_sounds.push(FootstepSound {
            channel: SoundChannel::FighterVoice,
            id,
            volume: 127,
            pan: 64,
        });
    }
    /// ftCo_DeadDown_Anim (800D3E00) and the DeadLeft / DeadRight twins: GM respawn is
    /// performed at this callback boundary.
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
        self.free_flight_physics(assets, wind);
    }
    /// Fighter_procUpdate's tail for a state without its own physics: knockback
    /// decay, then the velocity and environment integration.
    pub(super) fn free_flight_physics(&mut self, assets: &FighterAssets, wind: Vec3) {
        self.decay_air_knockback(assets);
        crate::physics::integrate::integrate_velocity(&mut self.physics);
        crate::physics::integrate::integrate_environment(&mut self.physics, None, wind);
    }
}
