use super::assets::{FighterAssets, Result};
use super::*;
use crate::{
    anim::WaitChoice,
    collision::{
        ground::{map_wait, WaitGroundResult},
        pose::FlatGroundPose,
    },
    input::{
        input_source, run_cpu_input_proc, update_input, wait_iasa, InputContext, PadSample,
        WaitContext, WaitTransition,
    },
    physics::grounded::{step_wait, GroundedParameters},
};
use gekko_math::rng::HsdRng;
use melee_mp::CollMap;

impl<C: CharacterCallbacks> Fighter<C> {
    /// Fighter_8006A1BC (0x8006A1BC), s_link 0, fighter.c:1393-1442.
    /// Hitlag expires here; unported interactions remain explicit guards.
    pub fn proc_status(&mut self) {
        if self.status.disabled {
            return;
        }
        self.status.require_supported();
        match self.status.interaction {
            super::Interaction::Hitlag => assert!(
                self.combat.hitlag_remaining > 0.0,
                "hitlag requires an active countdown"
            ),
            super::Interaction::Damage => assert!(
                matches!(self.state_data, super::MotionData::Damage(_)),
                "damage requires damage state"
            ),
            super::Interaction::Attack => assert!(
                matches!(
                    self.state_data,
                    super::MotionData::Jab(_) | super::MotionData::Tilt | super::MotionData::Smash
                ),
                "attack requires attack state"
            ),
            _ => {}
        }
        self.tick_hitlag();
        // ft_800819A8 (0x800819A8), ft_0819.c:32-46: three fadds.
        let cd = &self.collision.data;
        self.previous_collision_bounds = Vec3::new(
            cd.ecb.left.x + cd.cur_pos.x,
            cd.ecb.right.x + cd.cur_pos.x,
            self.physics.position.y + self.attributes.camera.damage_camera_y_offset,
        );
    }
    pub fn step_animation(&mut self, assets: &FighterAssets) {
        let first_footstep = self.commands.footstep_sounds.len();
        // ftAnim_8006EBA4: command-driven animation ownership changes must
        // finish before the independent part blends are evaluated.
        self.animation
            .advance_main::<RetailTrig>(&mut self.skeleton);
        self.commands.step(
            &mut self.animation,
            &mut self.skeleton,
            &mut self.ground_pose,
            assets,
        );
        for state in std::mem::take(&mut self.commands.airborne_changes) {
            match state {
                melee_types::GroundOrAir::Ground => self.land(),
                melee_types::GroundOrAir::Air => self.leave_ground(),
            }
        }
        self.apply_dynamic_commands(assets);
        self.animation
            .advance_parts::<RetailTrig>(&mut self.skeleton);
        // ftCo_800DB500 (800DB500): retain the animated local XRotN for release.
        if let Some(pose) = &mut self.combat.thrown_pose {
            let joint = self.animation.parts[usize::from(
                assets
                    .parts
                    .joint(melee_types::FtPart::XRotN)
                    .expect("XRotN"),
            )]
            .joint;
            if self
                .skeleton
                .get(joint)
                .aobj
                .as_ref()
                .is_some_and(|a| a.flags & hsd_anim::aobj::AOBJ_NO_ANIM == 0)
            {
                pose.saved_translation = self.skeleton.translation(joint);
            }
        }
        if self.commands.footstep_sounds.len() != first_footstep
            && self.collision.data.floor.flags & 255 != 0
        {
            unimplemented!(
                "ft_081B.c:1258-1296: non-default terrain footstep sound/effect mapping"
            );
        }
    }
    /// Fighter_8006A360 (0x8006A360), s_link 1, fighter.c:1444-1701.
    /// Main playback precedes Wait_Anim and its immediate animation restart.
    pub fn proc_anim(
        &mut self,
        assets: &FighterAssets,
        rng: &mut HsdRng,
    ) -> Result<Option<WaitChoice>> {
        if self.status.disabled {
            return Ok(None);
        }
        self.status.require_supported();
        self.physics.begin_tick();
        if self.combat.hitlag_remaining > 0.0 {
            return Ok(None);
        }
        if self.status.ledge_intangibility != 0 {
            self.status.ledge_intangibility -= 1;
        }
        if self.status.name_tag_timer > 1 && !self.status.input_frozen {
            self.status.name_tag_timer -= 1;
        }
        if self.status.time_since_hit != -1 {
            self.status.time_since_hit = self.status.time_since_hit.wrapping_add(1);
        }
        if self.status.time_since_smash != -1.0 {
            self.status.time_since_smash += 1.0;
        }
        if self.motion_state.callbacks.animation == state::AnimationCallback::Dead {
            self.death_animation();
            return Ok(None);
        }
        self.step_animation(assets);
        self.advance_smash_charge(assets);
        match self.motion_state.callbacks.animation {
            state::AnimationCallback::Dead => unreachable!(),
            state::AnimationCallback::Revival => {
                let choice = self.update_idle_animation(assets, rng)?;
                self.revival_animation(assets)?;
                return Ok(choice);
            }
            state::AnimationCallback::TechRoll => {
                if !self.animation.frames_remaining(&self.skeleton) {
                    self.change_motion_state(melee_types::CommonMotionState::Wait, assets)?;
                }
                return Ok(None);
            }
            state::AnimationCallback::DownBound | state::AnimationCallback::DownWait => {
                self.down_animation(assets)?;
                return Ok(None);
            }
            state::AnimationCallback::Thrown => {
                self.thrown_animation(assets);
                return Ok(None);
            }
            state::AnimationCallback::Throw => {
                // The linked release runs immediately after this callback in scene order.
                self.jab_animation(assets)?;
                return Ok(None);
            }
            state::AnimationCallback::Capture | state::AnimationCallback::CatchWait => {
                return Ok(None)
            }
            state::AnimationCallback::CatchPull => {
                if self.commands.grab_release || !self.animation.frames_remaining(&self.skeleton) {
                    self.enter_catch_wait(assets)?;
                }
                return Ok(None);
            }
            state::AnimationCallback::Catch => {
                self.catch_animation(assets)?;
                return Ok(None);
            }
            state::AnimationCallback::Damage => {
                self.damage_animation(assets)?;
                return Ok(None);
            }
            state::AnimationCallback::Jab => {
                self.jab_animation(assets)?;
                return Ok(None);
            }
            state::AnimationCallback::GuardOn
            | state::AnimationCallback::Guard
            | state::AnimationCallback::GuardReflect
            | state::AnimationCallback::GuardOff
            | state::AnimationCallback::GuardSetOff => {
                self.shield_animation(assets)?;
                return Ok(None);
            }
            state::AnimationCallback::CliffCatch
            | state::AnimationCallback::CliffWait
            | state::AnimationCallback::CliffJump1
            | state::AnimationCallback::CliffJump2 => {
                self.ledge_animation(assets)?;
                return Ok(None);
            }
            state::AnimationCallback::EscapeAir => {
                self.air_dodge_animation(assets)?;
                return Ok(None);
            }
            state::AnimationCallback::Escape | state::AnimationCallback::EscapeN => {
                self.escape_animation(assets)?;
                return Ok(None);
            }
            state::AnimationCallback::Squat | state::AnimationCallback::SquatRv => {
                self.squat_animation(assets)?;
                return Ok(None);
            }
            state::AnimationCallback::Turn => {
                self.turn_animation(assets)?;
                return Ok(None);
            }
            state::AnimationCallback::Dash => {
                self.dash_animation(assets)?;
                return Ok(None);
            }
            state::AnimationCallback::Run => {
                self.run_animation();
                return Ok(None);
            }
            state::AnimationCallback::TurnRun => {
                self.turn_run_animation(assets)?;
                return Ok(None);
            }
            state::AnimationCallback::CliffClimb => {
                self.cliff_climb_animation(assets)?;
                return Ok(None);
            }
            state::AnimationCallback::RunBrake => {
                self.run_brake_animation(assets)?;
                return Ok(None);
            }
            state::AnimationCallback::Walk => {
                self.walk_animation(assets);
                return Ok(None);
            }
            state::AnimationCallback::SquatWait => {}
            state::AnimationCallback::Entry
            | state::AnimationCallback::EntryStart
            | state::AnimationCallback::EntryEnd => {
                self.entry_animation(assets)?;
                return Ok(None);
            }
            state::AnimationCallback::MultiJump => {
                self.multi_jump_animation(assets)?;
                return Ok(None);
            }
            state::AnimationCallback::KneeBend => {
                self.knee_bend_animation(assets)?;
                return Ok(None);
            }
            state::AnimationCallback::Pass
            | state::AnimationCallback::Jump
            | state::AnimationCallback::JumpAerial => {
                self.jump_animation(assets)?;
                return Ok(None);
            }
            state::AnimationCallback::Fall => {
                self.fall_animation(assets)?;
                return Ok(None);
            }
            state::AnimationCallback::Landing => {
                self.landing_animation(assets)?;
                return Ok(None);
            }
            state::AnimationCallback::FallUnimplemented => {
                unimplemented!("unsupported installed Fall callback")
            }
            state::AnimationCallback::Wait => {}
        }
        self.update_idle_animation(assets, rng)
    }
    fn update_idle_animation(
        &mut self,
        assets: &FighterAssets,
        rng: &mut HsdRng,
    ) -> Result<Option<WaitChoice>> {
        let commands = &mut self.commands;
        let ground_pose = &mut self.ground_pose;
        let result = self.animation.update_wait_with_restart(
            &mut self.skeleton,
            rng,
            if self.motion_state.callbacks.animation == state::AnimationCallback::SquatWait {
                assets.squat_choices.as_deref()
            } else {
                assets.wait_choices.as_deref()
            },
            |id| &assets.motions[&id],
            |animation, tree| {
                commands.restart(assets.command_entries[&animation.motion_id]);
                animation.step_with_hooks::<RetailTrig>(
                    tree,
                    |animation, tree| commands.step(animation, tree, ground_pose, assets),
                    |_, _| {},
                );
            },
        )?;
        self.apply_dynamic_commands(assets);
        // ftCommon_8007E0E4: reset before the fighter-overlap nudge query.
        self.physics.player_nudge = Vec2::ZERO;
        Ok(result)
    }
    /// Fighter_8006ABA0 (0x8006ABA0), s_link 2, fighter.c:1703-1709.
    pub fn proc_cpu_gate(&mut self) {
        run_cpu_input_proc(
            self.status.disabled,
            input_source(self.player.control, self.cpu.mode),
        );
    }
    /// Fighter_Spaghetti_8006AD10 (0x8006AD10), s_link 3, fighter.c:1777-2140.
    pub fn proc_input(&mut self, assets: &FighterAssets, sample: &PadSample) {
        if self.status.disabled {
            return;
        }
        self.status.require_supported();
        let effects = update_input(
            &mut self.input,
            input_source(self.player.control, self.cpu.mode),
            sample,
            &assets.input,
            InputContext {
                save_and_clear: self.status.input_frozen,
                hitlag: self.combat.hitlag_remaining > 0.0,
                ..InputContext::default()
            },
        );
        if self.combat.hitlag_remaining > 0.0
            && matches!(self.state_data, super::MotionData::Damage(_))
            && (self.input.current.stick.x != 0.0
                || self.input.current.stick.y != 0.0
                || self.input.current.cstick.x != 0.0
                || self.input.current.cstick.y != 0.0)
        {
            unimplemented!("ftCo_Damage.c:624-664: SDI during hitlag");
        }
        self.joystick_count += u64::from(effects.joystick_count_increments);
        if effects.run_input_callback && self.combat.hitlag_remaining == 0.0 {
            self.update_smash_charge_input();
            if matches!(
                self.motion_state.callbacks.input,
                state::InputCallback::Entry
                    | state::InputCallback::EntryStart
                    | state::InputCallback::EntryEnd
            ) {
                return;
            }
            if matches!(
                self.motion_state.callbacks.input,
                state::InputCallback::Fall
                    | state::InputCallback::Jump
                    | state::InputCallback::JumpAerial
            ) {
                let transition = super::fall::iasa_with_jump(
                    &self.input,
                    self.aerial_jump_requested(assets),
                    |phase| {
                        let enabled = match &self.state_data {
                            MotionData::Jump(jump) => jump.physics_started,
                            MotionData::JumpAerial { .. } => {
                                phase == super::FloatInputPhase::BeforeAerialJump
                                    || self.commands.variables[0] != 0
                            }
                            _ => true,
                        };
                        if enabled {
                            self.character.check_float_input(
                                &self.input,
                                assets,
                                self.physics.self_velocity.y,
                                phase,
                            );
                        }
                    },
                );
                match transition {
                    WaitTransition::None => {}
                    WaitTransition::Jump => self.enter_aerial_jump(assets).expect("aerial jump"),
                    WaitTransition::Escape => self.enter_air_dodge(assets).expect("air dodge"),
                    _ => unimplemented!(
                        "ftCo_Fall.c:132-149 / ftCo_Jump.c:173-189: aerial {transition:?}"
                    ),
                }
                return;
            }
            if self.motion_state.callbacks.input == state::InputCallback::FallSpecial {
                // ftCo_FallSpecial_IASA (80096AF4): item/parasol predicates are
                // excluded by require_supported; air-dodge entry consumed all jumps.
                if i32::from(self.physics.jumps_used) < self.attributes.jumping.max_jumps {
                    unimplemented!(
                        "ftCo_FallSpecial.c:96-100: special fall with remaining aerial jumps"
                    );
                }
                return;
            }
            let context = WaitContext {
                facing: self.physics.facing,
                specials_available: self.capabilities.specials,
                shield_health: self.status.shield_health,
                ..WaitContext::default()
            };
            match self.motion_state.callbacks.input {
                state::InputCallback::Catch => return,
                state::InputCallback::Tilt => {
                    self.tilt_input(assets, &context).expect("tilt IASA");
                    return;
                }
                state::InputCallback::Damage => {
                    self.damage_input(assets, &context).expect("damage IASA");
                    return;
                }
                state::InputCallback::Jab => {
                    self.jab_input(assets, &context).expect("jab IASA");
                    return;
                }
                state::InputCallback::GuardOn
                | state::InputCallback::Guard
                | state::InputCallback::GuardReflect
                | state::InputCallback::GuardOff
                | state::InputCallback::GuardSetOff => {
                    self.shield_input(assets, &context).expect("shield IASA");
                    return;
                }
                state::InputCallback::TurnRun => {
                    // ftCo_TurnRun_IASA (800C9ED8), ftCo_TurnRun.c:82-85.
                    self.reject_running_jump(assets);
                    return;
                }
                // Empty CliffClimb_IASA / CliffEscape_IASA (8009ACA4 / 8009B12C).
                state::InputCallback::CliffClimb => return,
                state::InputCallback::CliffCatch
                | state::InputCallback::CliffJump1
                | state::InputCallback::CliffJump2 => return,
                state::InputCallback::CliffWait => {
                    self.ledge_input(assets).expect("ledge input");
                    return;
                }
                state::InputCallback::EscapeAir => {
                    self.air_dodge_input();
                    return;
                }
                // ftCo_8009563C is false without an item; EscapeN has no IASA.
                state::InputCallback::Escape => {
                    // ftCo_8009563C (8009563C), ftCo_ItemThrow.c:261-263.
                    if let MotionData::Escape(escape) = &mut self.state_data {
                        if escape.interrupt_frames != 0 {
                            escape.interrupt_frames -= 1;
                        }
                    }
                    return;
                }
                state::InputCallback::EscapeN => return,
                state::InputCallback::KneeBend => {
                    self.knee_bend_input(assets, &context);
                    return;
                }
                state::InputCallback::Squat
                | state::InputCallback::SquatWait
                | state::InputCallback::SquatRv => {
                    self.squat_input(assets, &context)
                        .expect("squat transition");
                    return;
                }
                state::InputCallback::Turn => {
                    self.turn_input(assets, &context).expect("turn transition");
                    return;
                }
                state::InputCallback::Dash => {
                    self.dash_input(assets, &context).expect("dash transition");
                    return;
                }
                state::InputCallback::Run => {
                    self.run_input(assets, &context).expect("run transition");
                    return;
                }
                state::InputCallback::RunBrake => {
                    self.run_brake_input(assets).expect("brake transition");
                    return;
                }
                state::InputCallback::Walk => {
                    self.walk_input(assets, &context).expect("walk transition");
                    return;
                }
                _ => {}
            }
            let transition = if self.motion_state.callbacks.input == state::InputCallback::Landing {
                let MotionData::Landing {
                    allow_interrupt, ..
                } = self.state_data
                else {
                    panic!("landing data missing")
                };
                super::landing::iasa(
                    &self.input,
                    &assets.input,
                    &context,
                    self.animation.frame,
                    self.animation.speed,
                    self.attributes.landing.normal_landing_lag,
                    allow_interrupt,
                )
            } else {
                assert_eq!(
                    self.motion_state.callbacks.input,
                    state::InputCallback::Wait,
                    "unsupported installed input callback"
                );
                wait_iasa(&self.input, &assets.input, &context)
            };
            if transition == WaitTransition::Squat
                && self.motion_state.callbacks.input == state::InputCallback::Landing
            {
                self.enter_landing_squat(assets)
                    .expect("landing squat hold");
            } else {
                self.apply_ground_transition(assets, transition)
                    .expect("grounded transition");
            }
        }
    }
    /// Fighter_procUpdate (0x8006B82C), s_link 4, fighter.c:2150-2438.
    pub fn proc_update(&mut self, assets: &FighterAssets, map: &CollMap, wind: Vec3) {
        if self.status.disabled {
            return;
        }
        self.status.require_supported();
        if self.combat.hitlag_remaining > 0.0 {
            return;
        }
        if self.status.ledge_cooldown != 0 {
            self.status.ledge_cooldown -= 1;
        }
        match self.motion_state.callbacks.physics {
            state::PhysicsCallback::Capture => {
                crate::physics::integrate::integrate_velocity(&mut self.physics);
                crate::physics::integrate::integrate_environment(&mut self.physics, None, wind);
            }
            state::PhysicsCallback::Dead => {}
            state::PhysicsCallback::Revival => self.revival_physics(assets, wind),
            state::PhysicsCallback::Down => self.down_physics(assets, map, wind),
            state::PhysicsCallback::Catch => self.catch_physics(assets, map, wind),
            state::PhysicsCallback::Damage => self.damage_physics(assets, map, wind),
            state::PhysicsCallback::Jab => self.jab_physics(assets, map, wind),
            state::PhysicsCallback::Wait | state::PhysicsCallback::SquatWait => step_wait(
                &mut self.physics,
                &self.collision.data,
                &GroundedParameters::from_attributes(&self.attributes, &assets.common),
                map,
                wind,
            ),
            state::PhysicsCallback::GuardOn
            | state::PhysicsCallback::Guard
            | state::PhysicsCallback::GuardReflect
            | state::PhysicsCallback::GuardOff
            | state::PhysicsCallback::GuardSetOff
            | state::PhysicsCallback::EscapeN
            | state::PhysicsCallback::KneeBend
            | state::PhysicsCallback::Landing
            | state::PhysicsCallback::Squat
            | state::PhysicsCallback::SquatRv
            | state::PhysicsCallback::Turn => {
                let params = GroundedParameters::from_attributes(&self.attributes, &assets.common);
                crate::physics::grounded::friction_physics(
                    &mut self.physics,
                    &params,
                    self.collision.data.floor.normal,
                    map.floor_speed_scale(&self.collision.data),
                );
                crate::physics::grounded::finish_ground_update(
                    &mut self.physics,
                    &self.collision.data,
                    &params,
                    map,
                    wind,
                );
            }
            state::PhysicsCallback::Escape => {
                self.escape_physics(map);
                crate::physics::grounded::finish_ground_update(
                    &mut self.physics,
                    &self.collision.data,
                    &GroundedParameters::from_attributes(&self.attributes, &assets.common),
                    map,
                    wind,
                );
            }
            state::PhysicsCallback::TurnRun
            | state::PhysicsCallback::Dash
            | state::PhysicsCallback::Run
            | state::PhysicsCallback::RunBrake => {
                if self.motion_state.id == melee_types::CommonMotionState::TurnRun {
                    self.turn_run_physics(assets);
                } else {
                    self.running_physics(assets);
                }
                crate::physics::grounded::apply_ground_movement(
                    &mut self.physics,
                    self.collision.data.floor.normal,
                    map.floor_speed_scale(&self.collision.data),
                );
                crate::physics::grounded::finish_ground_update(
                    &mut self.physics,
                    &self.collision.data,
                    &GroundedParameters::from_attributes(&self.attributes, &assets.common),
                    map,
                    wind,
                );
            }
            state::PhysicsCallback::Walk => {
                let MotionData::Walk(walk) = &mut self.state_data else {
                    panic!("walk data missing")
                };
                walk.slippery_animation_velocity = crate::physics::grounded::walk_physics(
                    &mut self.physics,
                    &self.attributes,
                    &assets.movement,
                    self.input.current.stick.x,
                    walk.acceleration_multiplier,
                );
                crate::physics::grounded::apply_ground_movement(
                    &mut self.physics,
                    self.collision.data.floor.normal,
                    map.floor_speed_scale(&self.collision.data),
                );
                crate::physics::grounded::finish_ground_update(
                    &mut self.physics,
                    &self.collision.data,
                    &GroundedParameters::from_attributes(&self.attributes, &assets.common),
                    map,
                    wind,
                );
            }
            state::PhysicsCallback::CliffClimb => {
                self.cliff_climb_physics(assets, map)
                    .expect("ledge option physics");
                if self.physics.ground_or_air == melee_types::GroundOrAir::Ground {
                    crate::physics::grounded::finish_ground_update(
                        &mut self.physics,
                        &self.collision.data,
                        &GroundedParameters::from_attributes(&self.attributes, &assets.common),
                        map,
                        wind,
                    );
                } else {
                    crate::physics::integrate::integrate_velocity(&mut self.physics);
                    crate::physics::integrate::integrate_environment(&mut self.physics, None, wind);
                }
            }
            state::PhysicsCallback::CliffCatch
            | state::PhysicsCallback::CliffWait
            | state::PhysicsCallback::CliffJump1 => {
                self.ledge_physics(assets, map).expect("ledge physics");
                crate::physics::integrate::integrate_velocity(&mut self.physics);
                crate::physics::integrate::integrate_environment(&mut self.physics, None, wind);
            }
            state::PhysicsCallback::CliffJump2 => {
                self.ledge_jump_physics(assets);
                crate::physics::integrate::integrate_velocity(&mut self.physics);
                crate::physics::integrate::integrate_environment(&mut self.physics, None, wind);
            }
            state::PhysicsCallback::EscapeAir => {
                self.air_dodge_physics(assets);
                crate::physics::integrate::integrate_velocity(&mut self.physics);
                crate::physics::integrate::integrate_environment(&mut self.physics, None, wind);
            }
            state::PhysicsCallback::Pass => {
                crate::physics::airborne::fall_physics(
                    &mut self.physics,
                    &self.attributes.air,
                    self.input.current.stick.x,
                );
                crate::physics::integrate::integrate_velocity(&mut self.physics);
                crate::physics::integrate::integrate_environment(&mut self.physics, None, wind);
            }
            state::PhysicsCallback::MultiJump => {
                self.multi_jump_physics(assets);
                self.decay_air_knockback(assets);
                crate::physics::integrate::integrate_velocity(&mut self.physics);
                crate::physics::integrate::integrate_environment(&mut self.physics, None, wind);
            }
            state::PhysicsCallback::FallSpecial => {
                self.special_fall_physics(assets);
                crate::physics::integrate::integrate_velocity(&mut self.physics);
                crate::physics::integrate::integrate_environment(&mut self.physics, None, wind);
            }
            state::PhysicsCallback::Fall
            | state::PhysicsCallback::Jump
            | state::PhysicsCallback::JumpAerial => {
                assert_eq!(
                    self.physics.shield_knockback_velocity,
                    Vec3::ZERO,
                    "air shield knockback decay needs damage physics"
                );
                self.airborne_physics(assets);
                self.decay_air_knockback(assets);
                crate::physics::integrate::integrate_velocity(&mut self.physics);
                crate::physics::integrate::integrate_environment(&mut self.physics, None, wind);
            }
            state::PhysicsCallback::Entry
            | state::PhysicsCallback::EntryStart
            | state::PhysicsCallback::EntryEnd => {
                self.entry_physics(assets.entry);
                crate::physics::integrate::integrate_velocity(&mut self.physics);
                crate::physics::integrate::integrate_environment(&mut self.physics, None, wind);
            }
            state::PhysicsCallback::FallUnimplemented => {
                unimplemented!("unsupported installed Fall physics")
            }
        }
        if self.shield.active {
            self.physics.shield_position_cached = false;
            self.shield.hit.position_cached = false;
            if self.motion_state.id != melee_types::CommonMotionState::GuardSetOff {
                self.shield.reflect.volume.position_cached = false;
            }
        }
        for hurt in &mut self.hurtboxes {
            hurt.cached = false;
        }
    }
    /// Fighter_procMap (0x8006C27C), s_link 6, fighter.c:2476-2516.
    pub fn proc_map(&mut self, map: &mut CollMap) {
        if self.status.disabled {
            return;
        }
        self.status.require_supported();
        assert!(
            matches!(
                self.motion_state.callbacks.collision,
                state::CollisionCallback::GuardOn
                    | state::CollisionCallback::Guard
                    | state::CollisionCallback::GuardReflect
                    | state::CollisionCallback::GuardOff
                    | state::CollisionCallback::GuardSetOff
                    | state::CollisionCallback::Escape
                    | state::CollisionCallback::EscapeN
                    | state::CollisionCallback::Wait
                    | state::CollisionCallback::Landing
                    | state::CollisionCallback::Walk
                    | state::CollisionCallback::Dash
                    | state::CollisionCallback::Run
                    | state::CollisionCallback::RunBrake
                    | state::CollisionCallback::KneeBend
                    | state::CollisionCallback::Squat
                    | state::CollisionCallback::SquatWait
                    | state::CollisionCallback::SquatRv
                    | state::CollisionCallback::Turn
            ),
            "airborne map needs proc_map_with_assets"
        );
        assert!(
            !self.map_ground(map),
            "ground departure needs proc_map_with_assets"
        );
    }
    fn map_ground(&mut self, map: &mut CollMap) -> bool {
        let simple = matches!(
            self.motion_state.callbacks.collision,
            state::CollisionCallback::GuardOn
                | state::CollisionCallback::Guard
                | state::CollisionCallback::GuardReflect
                | state::CollisionCallback::GuardOff
                | state::CollisionCallback::GuardSetOff
                | state::CollisionCallback::Dash
                | state::CollisionCallback::Run
                | state::CollisionCallback::KneeBend
                | state::CollisionCallback::Squat
                | state::CollisionCallback::SquatWait
                | state::CollisionCallback::SquatRv
                | state::CollisionCallback::Turn
        );
        let callback = if matches!(
            self.motion_state.callbacks.collision,
            state::CollisionCallback::Escape | state::CollisionCallback::EscapeN
        ) || (self.motion_state.callbacks.collision
            == state::CollisionCallback::GuardSetOff
            && self.shield.allow_sdi)
        {
            crate::collision::ground::map_escape
        } else if simple {
            crate::collision::ground::map_ground_action
        } else {
            map_wait
        };
        match callback(
            &mut self.physics,
            &mut self.collision,
            map,
            &mut self.skeleton,
            self.animation.root,
            self.input.current.stick.x,
        ) {
            WaitGroundResult::Supported => {
                if matches!(
                    self.motion_state.callbacks.collision,
                    state::CollisionCallback::Dash | state::CollisionCallback::Run
                ) {
                    // ft_800844EC -> ftCo_8009EDA4 (ftCo_StopWall.c:16-30).
                    let wall = if self.physics.facing < 0.0 {
                        melee_types::mp::collide::RIGHT_WALL_HUG
                    } else {
                        melee_types::mp::collide::LEFT_WALL_HUG
                    };
                    if self.collision.data.env_flags as u32 & wall != 0
                        && gekko_math::msl::fabsf(self.physics.ground_velocity)
                            > self.attributes.walking.walk_max_vel
                    {
                        unimplemented!("ftCo_StopWall.c:25-26: running wall impact -> StopWall");
                    }
                }
            }
            WaitGroundResult::EnterFall => return true,
            WaitGroundResult::EnterTeeter => unimplemented!("ft_081B.c:1092: Wait -> Ottotto"),
        }
        false
    }
    /// State-changing map dispatch, including Landing's immediate command/RNG work.
    /// The original proc_map remains the grounded API used by melee-sim M3.
    pub fn proc_map_with_assets(
        &mut self,
        assets: &FighterAssets,
        map: &mut CollMap,
        rng: &mut HsdRng,
    ) -> Result<usize> {
        use crate::collision::air;
        if self.status.disabled {
            return Ok(0);
        }
        self.status.require_supported();
        match self.motion_state.callbacks.collision {
            state::CollisionCallback::Revival => {
                air::begin_map(
                    &self.physics,
                    &mut self.collision,
                    &mut self.skeleton,
                    self.animation.root,
                );
                let cd = &mut self.collision.data;
                cd.last_pos = cd.cur_pos;
                cd.cur_pos = self.physics.position;
                let pose = crate::collision::ecb::EcbPose::read(
                    &mut self.skeleton,
                    self.animation.root,
                    cd,
                );
                if self.motion_state.id == melee_types::CommonMotionState::Rebirth {
                    map.air_collide_stay_ecb5(cd, Some(&|i| pose.position(i)));
                } else if map.air_collide_ecb5(cd, Some(&|i| pose.position(i))) {
                    unimplemented!("ftCoD5A30: revival platform reaches floor");
                }
                self.physics.position = cd.cur_pos;
            }
            state::CollisionCallback::Thrown => {}
            state::CollisionCallback::Capture => self.capture_collision(assets, map)?,
            state::CollisionCallback::Catch => self.catch_collision(assets, map)?,
            state::CollisionCallback::Damage => self.damage_collision(assets, map)?,
            state::CollisionCallback::GuardOn
            | state::CollisionCallback::Guard
            | state::CollisionCallback::GuardReflect
            | state::CollisionCallback::GuardOff
            | state::CollisionCallback::GuardSetOff
            | state::CollisionCallback::Escape
            | state::CollisionCallback::EscapeN
            | state::CollisionCallback::Wait
            | state::CollisionCallback::Landing
            | state::CollisionCallback::Walk
            | state::CollisionCallback::Dash
            | state::CollisionCallback::Run
            | state::CollisionCallback::RunBrake
            | state::CollisionCallback::KneeBend
            | state::CollisionCallback::Squat
            | state::CollisionCallback::SquatWait
            | state::CollisionCallback::SquatRv
            | state::CollisionCallback::Turn => {
                if self.map_ground(map) {
                    self.leave_ground();
                    self.change_motion_state(melee_types::CommonMotionState::Fall, assets)?;
                }
            }
            state::CollisionCallback::Entry
            | state::CollisionCallback::EntryStart
            | state::CollisionCallback::EntryEnd => {
                air::begin_map(
                    &self.physics,
                    &mut self.collision,
                    &mut self.skeleton,
                    self.animation.root,
                );
                if self.motion_state.callbacks.collision != state::CollisionCallback::Entry {
                    let MotionData::Entry(entry) = &self.state_data else {
                        panic!("entry data missing")
                    };
                    let was_airborne = self.physics.ground_or_air == melee_types::GroundOrAir::Air;
                    let supported = air::collide_entry(
                        &mut self.physics,
                        &mut self.collision,
                        map,
                        entry.collision_box,
                    );
                    if was_airborne && supported {
                        self.land();
                    } else if !was_airborne && !supported {
                        self.leave_ground();
                    }
                }
                self.skeleton
                    .set_translate(self.animation.root, &self.physics.position);
            }
            state::CollisionCallback::TurnRun => self.turn_run_collision(assets, map)?,
            state::CollisionCallback::CliffClimb => self.ledge_collision(assets, map)?,
            state::CollisionCallback::CliffCatch
            | state::CollisionCallback::CliffWait
            | state::CollisionCallback::CliffJump1 => {
                self.ledge_collision(assets, map)?;
            }
            state::CollisionCallback::EscapeAir => {
                air::begin_map(
                    &self.physics,
                    &mut self.collision,
                    &mut self.skeleton,
                    self.animation.root,
                );
                if air::collide_air_dodge(
                    &mut self.physics,
                    &mut self.collision,
                    map,
                    &mut self.skeleton,
                    self.animation.root,
                ) {
                    self.enter_special_landing(assets, false, assets.air_dodge.landing_lag)?;
                }
            }
            state::CollisionCallback::Pass
            | state::CollisionCallback::FallSpecial
            | state::CollisionCallback::Fall
            | state::CollisionCallback::Jump
            | state::CollisionCallback::JumpAerial
            | state::CollisionCallback::CliffJump2 => {
                air::begin_map(
                    &self.physics,
                    &mut self.collision,
                    &mut self.skeleton,
                    self.animation.root,
                );
                let collide =
                    if self.motion_state.callbacks.collision == state::CollisionCallback::Pass {
                        air::collide_pass
                    } else {
                        air::collide_fall
                    };
                if collide(
                    &mut self.physics,
                    &mut self.collision,
                    map,
                    &mut self.skeleton,
                    self.animation.root,
                    self.status.ledge_cooldown == 0,
                ) {
                    if self.motion_state.callbacks.collision
                        == state::CollisionCallback::FallSpecial
                    {
                        self.land_from_special_fall(assets)?;
                    } else if self.physics.self_velocity.y > assets.soft_landing_speed {
                        self.land();
                        self.change_motion_state(melee_types::CommonMotionState::Wait, assets)?;
                    } else {
                        self.enter_landing(assets)?;
                    }
                } else if self.try_grab_ledge(assets, map)? {
                    // ft_800835B0: grabbing precedes the ceiling check.
                } else if matches!(
                    self.motion_state.callbacks.collision,
                    state::CollisionCallback::Jump
                        | state::CollisionCallback::JumpAerial
                        | state::CollisionCallback::CliffJump2
                ) && self.collision.data.env_flags as u32
                    & melee_types::mp::collide::CEILING_HUG
                    != 0
                {
                    unimplemented!(
                        "ft_081B.c:792-803 / ftCo_StopCeil.c:16-22: jump ceiling impact"
                    );
                }
            }
            state::CollisionCallback::FallUnimplemented => {
                unimplemented!("unsupported installed Fall collision")
            }
        }
        let mut draws = 0;
        for id in self.commands.landing_effects.drain(..) {
            // ftCo_8009F834 block_70. Even a zero range consumes three draws.
            let mut offset = Vec3::ZERO;
            for component in [&mut offset.x, &mut offset.y, &mut offset.z] {
                let random = rng.randf();
                // Retail fused sites 0x8009FCF8/FD1C/FD44 (asm.py --fused).
                *component = gekko_math::fma::fmadds(0.0, random - 0.5, *component);
                draws += 1;
            }
            let normal = self.collision.data.floor.normal;
            self.effects.push(super::effects::EffectRequest::Landing {
                id,
                offset,
                floor_angle: melee_lb::trigf::atan2f(-normal.x, normal.y),
            });
        }
        Ok(draws)
    }
    /// Fighter_8006C5F4 (0x8006C5F4), s_link 7, fighter.c:2518-2525.
    pub fn proc_pose(&mut self, map: &CollMap) {
        if self.status.disabled {
            return;
        }
        let pose = FlatGroundPose {
            bones: self
                .bones
                .ground_pose
                .as_ref()
                .expect("character ground pose"),
            player_scale: self.player.scale,
            flags: self.ground_pose,
        };
        if let Err(path) = pose.update(
            &self.physics,
            &self.collision,
            map,
            &mut self.skeleton,
            self.animation.root,
        ) {
            unimplemented!("ft_0899.c:109-232: {path:?}");
        }
    }
    /// Fighter_CallAcessoryCallbacks_8006C624 (0x8006C624), s_link 8.
    /// Reset and Wait entry leave accessory1/2/3 NULL (fighter.c:456,1373).
    pub fn proc_accessories(&mut self) {
        if !self.status.disabled {
            self.status.require_supported();
        }
    }
    /// Fighter_8006C80C (0x8006C80C), s_link 9, fighter.c:2555-2596.
    /// Active attacks update their swept capsules after animation and map collision.
    /// The always-enabled thrown capsule still updates (ft_07C1.c:52-73).
    pub fn proc_hitbox_positions(&mut self) {
        if !self.status.disabled {
            self.status.require_supported();
            for hit in self.commands.hitboxes.iter_mut().flatten() {
                hit.update(&mut self.skeleton, self.animation.root, self.player.scale);
            }
            self.thrown_hitbox
                .update(&mut self.skeleton, self.animation.root);
        }
    }
    /// Fighter_UnkProcessGrab_8006CA5C (0x8006CA5C), s_link 12.
    /// Catch startup is ported; selecting/linking a victim is still a boundary.
    pub fn proc_grab(&mut self) {
        if !self.status.disabled {
            self.status.require_supported();
        }
    }
    /// Fighter_8006CB94 (0x8006CB94), s_link 13, fighter.c:2621-2642.
    /// The scene visits fighter pairs in entity order before this local proc.
    pub fn proc_hit_detection(&mut self) {
        if !self.status.disabled {
            self.status.require_supported();
        }
    }
    /// Fighter_ProcessHit_8006D1EC (0x8006D1EC), s_link 14.
    /// Apply accumulated hits and enter damage/hitlag, then update shield and caches.
    pub fn proc_process_hit(&mut self, assets: &FighterAssets) {
        if self.status.disabled {
            return;
        }
        self.status.require_supported();
        self.process_damage(assets).expect("hit response");
        self.shield_proc(assets).expect("shield response");
        self.cpu.hurtbox_extents = caches::hurtbox_extents(
            &mut self.hurtboxes,
            &mut self.skeleton,
            self.animation.root,
            self.physics.position,
            self.physics.facing,
            self.player.scale,
        );
    }
    /// Fighter_8006D9AC (0x8006D9AC), s_link 16 -> ftCo_8009DD94.
    /// Wait x594_b3 selects ftCo_8009CB40(..., false, NULL), setting bone_id
    /// to 0x100 (ftdynamics.c:55); lb_8001044C returns at lb_00F9.c:447.
    pub fn proc_dynamics(&mut self) {
        if self.status.disabled {
            return;
        }
        self.status.require_supported();
        for collider in &mut self.dynamic_colliders {
            collider.position = caches::bone_position(
                &mut self.skeleton,
                self.animation.root,
                collider.bone,
                collider.offset,
            );
        }
        self.solve_dynamics(&mut |_, _| {
            panic!("active grounded dynamics requires proc_dynamics_with_map")
        });
    }
    /// Fighter_8006D9AC with the scene's mpCheckFloor provider.
    pub fn proc_dynamics_with_map(&mut self, map: &mut CollMap) {
        if self.status.disabled {
            return;
        }
        self.status.require_supported();
        for collider in &mut self.dynamic_colliders {
            collider.position = caches::bone_position(
                &mut self.skeleton,
                self.animation.root,
                collider.bone,
                collider.offset,
            );
        }
        self.solve_dynamics(&mut |a, b| {
            map.check_floor(a.x, a.y, b.x, b.y, 0.1, -1, -1, -1, None)
                .map(|hit| hit.pos)
        });
    }
    fn solve_dynamics(&mut self, floor: &mut impl FnMut(Vec3, Vec3) -> Option<Vec3>) {
        use melee_lb::dynamics::{Collider, SolverEnvironment};
        let colliders: Vec<_> = self
            .dynamic_colliders
            .iter()
            .map(|c| Collider {
                position: c.position,
                radius: c.radius,
            })
            .collect();
        let mut environment = SolverEnvironment {
            disabled: false,
            colliders: &colliders,
            forces: &[],
            first_force_bone: 0,
            ground_check: self.player.scale == 1.0
                && self.physics.ground_or_air == melee_types::GroundOrAir::Ground,
        };
        let plane = self.dynamics_use_floor_plane;
        let height = self.physics.position.y;
        let count = self.dynamics.len();
        for (index, (set, &first)) in self
            .dynamics
            .iter_mut()
            .zip(&self.dynamics_first_bone)
            .enumerate()
        {
            environment.first_force_bone = self.character.dynamics_first_force_bone(index, count);
            set.solve(&mut self.skeleton, first, &environment, &mut |a, b| {
                if plane {
                    melee_lb::dynamics::floor_plane(a, b, height)
                } else {
                    floor(a, b)
                }
            });
        }
    }
    /// ftCo_Cliff_Cam (80081644), ftcliffcommon.c:160-168: camera box first,
    /// then notify the supporting stage joint while hanging airborne.
    pub fn proc_camera_with_map(
        &mut self,
        assets: &FighterAssets,
        fixed_zoom: f32,
        map: &mut CollMap,
    ) {
        self.proc_camera(assets, fixed_zoom);
        if !self.status.disabled && self.camera.on_ledge {
            let MotionData::Cliff(cliff) = &self.state_data else {
                panic!("cliff camera scratch missing")
            };
            map.notify_ledge_grab(&mut self.collision.data, cliff.ledge_id);
        }
    }
    /// Fighter_UnkCallCameraCallback_8006D9EC (0x8006D9EC), s_link 18.
    pub fn proc_camera(&mut self, assets: &FighterAssets, fixed_zoom: f32) {
        if self.status.disabled {
            return;
        }
        self.status.require_supported();
        self.status.camera_shift = Vec2::ZERO;
        self.camera.on_ledge = self.motion_state.callbacks.camera == state::CameraCallback::Cliff
            && self.physics.ground_or_air == melee_types::GroundOrAir::Air;
        // ftCamera_80076018 / ftCamera_UpdateCameraBox: separate fmuls/fadds,
        // no contraction (retail asm.py --fused).
        let [h, v] = assets.camera_extents;
        let scale = self.player.scale;
        let h = Vec3::new(h.x * scale, h.y * scale, h.z * scale);
        self.camera.vertical = Vec3::new(v.x * scale, v.y * scale, v.z * scale);
        self.camera.horizontal = if self.physics.facing == 1.0 {
            Vec2::new(h.z, h.y * fixed_zoom)
        } else {
            Vec2::new(-h.y * fixed_zoom, -h.z)
        };
        self.camera.facing = self.physics.facing;
        let position = self.physics.position;
        self.camera.position = Vec3::new(position.x, position.y + h.x, position.z);
        self.camera.bone_position = caches::bone_position(
            &mut self.skeleton,
            self.animation.root,
            self.attributes.camera.camera_zoom_target_bone as usize,
            self.attributes.camera.zoom_offset,
        );
    }
    /// Fighter_8006DA4C (0x8006DA4C), s_link 22, fighter.c:3073-3083.
    pub fn proc_player_mirror(&mut self) {
        if self.status.disabled {
            return;
        }
        self.player_position = self.physics.position;
        self.player_facing = self.physics.facing;
    }
}
