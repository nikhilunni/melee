//! Tumble landing and prone recovery, ftCo_DownBound.c.
use super::fly_reflect::BounceSurface;
use super::FighterCore;
use super::{
    assets::{FighterAssets, Result},
    Fighter, MotionData,
};
use hsd_types::Vec3;
use melee_gr::wind::Wind;
use melee_types::{CommonMotionState as S, FtPart};

impl super::FighterCore {
    /// ftCo_800986B0 (800986B0): the shield was pressed within the tech
    /// window, and not too soon after the previous press (the hammer never
    /// applies here).
    pub(super) fn tech_window_open(&self, assets: &FighterAssets) -> bool {
        let timers = &self.input.buttons;
        f32::from(timers.digital_shield) < assets.damage.tech_window
            && i32::from(timers.previous_digital_shield) >= assets.damage.tech_lockout
    }
}

impl Fighter {
    /// ftCo_80098400 / ftCo_800984D4: buffered AB at bounce completion,
    /// fresh AB during DownWait; an upward C-stick crossing works in both.
    fn down_attack_input(&mut self, assets: &FighterAssets, buffered: bool) -> Result<bool> {
        let input = &self.core.input;
        let pressed = if buffered {
            f32::from(input.buttons.attack) < assets.damage.down_attack_buffer
                || f32::from(input.buttons.special) < assets.damage.down_attack_buffer
        } else {
            input
                .pressed
                .intersects(crate::input::Buttons::A | crate::input::Buttons::B)
        };
        let threshold = assets.damage.down_cstick_attack_threshold;
        let cstick_attack =
            input.previous.cstick.y < threshold && input.current.cstick.y >= threshold;
        if !(pressed || cstick_attack) {
            return Ok(false);
        }
        let up = if buffered {
            S::DownBoundU
        } else {
            S::DownWaitU
        };
        let state = if self.core.motion_state.id == up {
            S::DownAttackU
        } else {
            S::DownAttackD
        };
        self.change_motion_state(state.into(), assets)?;
        self.step_animation(assets);
        Ok(true)
    }

    /// ftCo_Down_CheckInput (80098214): C-stick crossing before held main stick.
    fn down_roll_input(&mut self, assets: &FighterAssets) -> Result<bool> {
        use gekko_math::msl::fabsf;
        let input = &self.core.input;
        let threshold = assets.damage.down_roll_threshold;
        let horizontal = |stick: crate::input::Stick| {
            fabsf(stick.x) >= threshold
                && melee_lb::trigf::atan2f(stick.y, fabsf(stick.x)) < assets.input.tilt_angle
        };
        let stick =
            if fabsf(input.previous.cstick.x) < threshold && horizontal(input.current.cstick) {
                input.current.cstick.x
            } else if horizontal(input.current.stick) {
                input.current.stick.x
            } else {
                return Ok(false);
            };
        // Retail deliberately tests DownWaitU even when called by DownBound.
        let up = self.core.motion_state.id == S::DownWaitU;
        let forward = stick * self.core.physics.facing >= 0.0;
        let state = match (up, forward) {
            (true, true) => S::DownFowardU,
            (true, false) => S::DownBackU,
            (false, true) => S::DownFowardD,
            (false, false) => S::DownBackD,
        };
        // ftCo_80098324 (80098324): enter, first animation step, then
        // ftCommon_8007CCE8 projects the knockback along the floor.
        self.change_motion_state(state.into(), assets)?;
        self.step_animation(assets);
        self.core.project_ground_knockback(assets);
        Ok(true)
    }

    /// ftCo_DownWait_IASA (8009802C): attack, roll, then stand.
    fn down_wait_input(&mut self, assets: &FighterAssets) -> Result<()> {
        if self.down_attack_input(assets, false)? || self.down_roll_input(assets)? {
            return Ok(());
        }
        let input = &self.core.input;
        let stick = input.current.stick;
        if (stick.y >= assets.damage.down_stand_threshold
            && melee_lb::trigf::atan2f(stick.y, gekko_math::msl::fabsf(stick.x))
                >= assets.input.tilt_angle)
            || input.pressed.intersects(crate::input::Buttons::SHIELD)
        {
            let state = if self.core.motion_state.id == S::DownWaitU {
                S::DownStandU
            } else {
                S::DownStandD
            };
            self.change_motion_state(state.into(), assets)?;
        }
        Ok(())
    }
    /// ftCo_800986B0 / ftCo_80098928 (800986B0 / 80098928): digital edge window.
    pub(super) fn try_tech(&mut self, assets: &FighterAssets) -> Result<bool> {
        if !self.core.tech_window_open(assets) {
            return Ok(false);
        }
        let stick = self.core.input.current.stick.x;
        let state = if gekko_math::msl::fabsf(stick) < assets.damage.tech_roll_threshold {
            S::Passive
        } else if stick * self.core.physics.facing >= 0.0 {
            S::PassiveStandF
        } else {
            S::PassiveStandB
        };
        self.land();
        self.change_motion_state(state.into(), assets)?;
        if state == S::Passive {
            self.core.project_ground_knockback(assets);
            self.commands
                .color_animations
                .push(melee_cmd::ColorAnimationRequest {
                    id: 120,
                    duration: 0,
                });
        }
        self.core
            .push_effect_after_issued_graphics(melee_ef::request::EffectRequest::CaptureFlash {
                bone: 0,
            });
        Ok(true)
    }
    /// ftCo_80097D40 (80097D40): the landing out of a tumble, which forgets
    /// the wall of the last DownReflect.
    pub(super) fn enter_down_bound(&mut self, assets: &FighterAssets) -> Result<()> {
        self.enter_down_bound_from(assets, false)
    }

    /// ftCo_8009794C (8009794C): choose face-up/down from the animated HipN,
    /// inverted for a kind with x2226_b1. `reflected` is ftCo_80097D88, the
    /// landing out of DownReflect, which leaves mv+4 alone.
    fn enter_down_bound_from(&mut self, assets: &FighterAssets, reflected: bool) -> Result<()> {
        self.land();
        let hip = self.core.animation.parts
            [usize::from(assets.parts.joint(FtPart::HipN).expect("HipN"))]
        .joint;
        let matrix = self.core.skeleton.get_mtx(hip);
        // ftCo_80097570 reads mtx[1][2] instead with x2226_b0, set only by
        // the Sandbag (ftsandbag.c:58), which is not ported.
        let face_up = (matrix.0[1][1] > 0.0) != self.character.table().down_bound_inverted;
        let state = if face_up {
            S::DownBoundU
        } else {
            S::DownBoundD
        };
        // ftCo_80097D40 stores a zero byte at +2344 after ftCo_8009794C
        // (retail 80097D70 stb): mv+4 keeps the predecessor's low three
        // bytes. ftCo_80097D88 does not.
        let inherited = self.inherited_scratch_word();
        let (retained_word, last_reflect) = if reflected {
            (inherited, self.core.last_down_reflect())
        } else {
            (
                inherited.map(|word| f32::from_bits(word.to_bits() & 0x00FF_FFFF)),
                None,
            )
        };
        self.change_motion_state(state.into(), assets)?;
        self.core.state_data = MotionData::Down {
            wait_remaining: 0.0,
            retained_word,
            last_reflect,
        };
        self.core.status.grab_exclusions = super::ledge::GrabExclusions::ALL;
        self.core.input.buttons.attack = 255;
        self.core.input.buttons.special = 255;
        let normal = self.core.collision.data.floor.normal;
        let floor_angle = melee_lb::trigf::atan2f(-normal.x, normal.y);
        // ftCo_800978D4: direct async kind 4 has no randomized offset.
        self.core
            .effects
            .push(melee_ef::request::EffectRequest::Graphics {
                id: 0x406,
                bone: 0,
                offset: Vec3::ZERO,
                facing: self.core.physics.facing,
                floor_angle,
            });
        // ftCo_800976A4 -> ftCo_8009F834: three draws, even for zero ranges.
        let effect = self.core.bound_effect();
        self.core
            .commands
            .graphics
            .push(melee_types::combat::GraphicsCommand {
                id: effect,
                bone: 0,
                common_bone: false,
                item_bone: false,
                destroy_on_state_change: false,
                parameter: 0.0,
                offset: Vec3::ZERO,
                range: Vec3::ZERO,
                issued_facing: None,
            });
        // ftCo_800976A4's tail: Camera_RequestQuake(QuakeKind_Large) at
        // cur_pos (the epicenter is unread); ftCommon_8007EBAC is rumble.
        self.core.quake_request = Some(melee_cm::QuakeKind::Large);
        self.core.project_ground_knockback(assets);
        Ok(())
    }

    /// DownBound_Anim (80097DE8), DownWait_Anim (80097FD0).
    pub(super) fn down_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if matches!(self.core.motion_state.id, S::DownBoundU | S::DownBoundD) {
            if !self.core.animation.frames_remaining(&self.core.skeleton) {
                if self.down_attack_input(assets, true)? || self.down_roll_input(assets)? {
                    return Ok(());
                }
                if self.core.physics.ground_or_air == melee_types::GroundOrAir::Air {
                    self.land();
                }
                // ftCo_80097AF4 writes only mv.co.downwait.x0.
                let retained_word = self.inherited_scratch_word();
                let last_reflect = self.core.last_down_reflect();
                self.change_motion_state(
                    (if self.core.motion_state.id == S::DownBoundU {
                        S::DownWaitU
                    } else {
                        S::DownWaitD
                    })
                    .into(),
                    assets,
                )?;
                self.core.state_data = MotionData::Down {
                    wait_remaining: assets.damage.down_wait_frames,
                    retained_word,
                    last_reflect,
                };
                self.step_animation(assets);
                self.core.status.grab_exclusions = super::ledge::GrabExclusions(1);
            }
        } else {
            let MotionData::Down { wait_remaining, .. } = &mut self.core.state_data else {
                panic!("down scratch");
            };
            *wait_remaining -= 1.0;
            if *wait_remaining <= 0.0 {
                let state = if self.core.motion_state.id == S::DownWaitU {
                    S::DownStandU
                } else {
                    S::DownStandD
                };
                self.change_motion_state(state.into(), assets)?;
            }
        }
        Ok(())
    }
}

impl FighterCore {
    /// ftCo_8009F0F0 (8009F0F0): a prone fighter whose damage this frame
    /// (dmg.x1838) stays below PlCo +428 takes the hit in DownDamage instead
    /// of launching. Retail picks DownDamageU only from DownWaitU, so a
    /// face-up bounce takes DownDamageD.
    pub(super) fn down_damage_state(&self, frame_damage: f32, assets: &FighterAssets) -> Option<S> {
        let state = self.motion_state.id;
        if !matches!(
            state,
            S::DownBoundU
                | S::DownWaitU
                | S::DownDamageU
                | S::DownBoundD
                | S::DownWaitD
                | S::DownDamageD
        ) {
            return None;
        }
        // x2224_b2 (the input-replay mode) also selects this; never set in versus.
        if frame_damage >= assets.damage.down_damage_limit as f32 {
            return None;
        }
        Some(if state == S::DownWaitU {
            S::DownDamageU
        } else {
            S::DownDamageD
        })
    }
}

impl Fighter {
    /// ftCo_DownDamage_Anim (8009F1F4): count the reaction's hitstun down,
    /// then fall when airborne, or lie back down (DownWait keeps the same
    /// scratch word as its timer) or stand once it has run out.
    pub(super) fn down_damage_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        let MotionData::Damage(damage) = &mut self.core.state_data else {
            panic!("down damage scratch");
        };
        damage.hitstun -= 1.0;
        let remaining = damage.hitstun;
        if self.core.animation.frames_remaining(&self.core.skeleton) {
            return Ok(());
        }
        let up = self.core.motion_state.id == S::DownDamageU;
        if self.core.physics.ground_or_air == melee_types::GroundOrAir::Air {
            return self.change_motion_state(S::Fall.into(), assets);
        }
        if remaining <= 0.0 {
            let state = if up { S::DownStandU } else { S::DownStandD };
            return self.change_motion_state(state.into(), assets);
        }
        // ftCo_80097F38: SkipModel | SkipMatAnim | SkipNametagVis |
        // KeepColAnimPartHitStatus, then ftAnim_8006EBA4.
        let state = if up { S::DownWaitU } else { S::DownWaitD };
        // ftCo_80097F38 writes only mv.co.downwait.x0; x4 is DownDamage's.
        let retained_word = self.inherited_scratch_word();
        self.change_motion_state(state.into(), assets)?;
        // mv+4 was DownDamage's mv.co.damage.x4, a flag: its top byte is zero.
        self.core.state_data = MotionData::Down {
            wait_remaining: remaining,
            retained_word,
            last_reflect: None,
        };
        self.step_animation(assets);
        self.core.status.grab_exclusions = super::ledge::GrabExclusions(1);
        Ok(())
    }

    /// ftCo_DownDamage_Coll (8009F284): grounded, losing the floor leaves the
    /// ground with one jump spent (ftCo_8008FC94); airborne, a landing only
    /// lands (ftCommon_8007D7FC) and the reaction keeps playing.
    pub(super) fn down_damage_collision(
        &mut self,
        assets: &FighterAssets,
        map: &mut melee_mp::CollMap,
    ) -> Result<()> {
        use melee_types::mp::collide::{LEFT_WALL_HUG, RIGHT_WALL_HUG};
        let walls = |fighter: &Self| {
            fighter.core.collision.data.env_flags as u32 & (LEFT_WALL_HUG | RIGHT_WALL_HUG) != 0
        };
        if self.core.physics.ground_or_air == melee_types::GroundOrAir::Ground {
            let result = crate::collision::ground::map_ground_action(
                &mut self.core.physics,
                &mut self.core.collision,
                map,
                &mut self.core.skeleton,
                self.core.animation.root,
                self.core.input.current.stick.x,
            );
            if result == crate::collision::ground::WaitGroundResult::EnterFall {
                self.leave_ground();
            } else if walls(self) {
                unimplemented!("ftCo_800C7CA0: DownReflect wall bounce");
            }
            return Ok(());
        }
        let Some(landed) = self.land_from_damage_air(assets, map)? else {
            return Ok(());
        };
        if landed {
            self.land();
        } else if walls(self) {
            // ftCo_DownDamage_Coll: a wall tech (ftCo_800C1D38), else
            // ftCo_800C17CC's wall bounce, then its ceiling bounce.
            if !self.try_wall_tech(assets, map)? && !self.try_wall_bounce(assets, map)? {
                self.try_ceiling_bounce(assets, map)?;
            }
        }
        Ok(())
    }
}

/// ftCo_DownDamage_Anim (8009F1F4).
pub(super) fn down_damage_animation(
    fighter: &mut Fighter,
    phase: super::state::AnimationPhase<'_>,
) -> Result<Option<crate::anim::WaitChoice>> {
    fighter.step_animation(phase.assets);
    fighter.advance_smash_charge(phase.assets);
    fighter.down_damage_animation(phase.assets)?;
    Ok(None)
}

/// ftCo_DownDamage_Coll (8009F284).
pub(super) fn down_damage_collision(
    fighter: &mut Fighter,
    phase: super::state::CollisionPhase<'_>,
) -> Result<()> {
    let assets = phase.assets.expect("down damage collision needs assets");
    fighter.down_damage_collision(assets, phase.map)
}

/// ftCo_DownWait_IASA (8009802C), installed only on prone wait rows.
pub(super) fn wait_input(fighter: &mut Fighter, phase: super::state::InputPhase<'_>) {
    fighter
        .down_wait_input(phase.assets)
        .expect("down wait input");
}

/// DownStand/DownAttack/Down_Anim: finish the getup at Wait.
pub(super) fn recovery_animation(
    fighter: &mut Fighter,
    phase: super::state::AnimationPhase<'_>,
) -> Result<Option<crate::anim::WaitChoice>> {
    fighter.step_animation(phase.assets);
    if !fighter
        .core
        .animation
        .frames_remaining(&fighter.core.skeleton)
    {
        fighter.change_motion_state(S::Wait.into(), phase.assets)?;
    }
    Ok(None)
}
impl FighterCore {
    /// ftCommon_8007CCE8 (8007CCE8): clamp and project residual ground knockback.
    fn project_ground_knockback(&mut self, assets: &FighterAssets) {
        if self.physics.ground_or_air == melee_types::GroundOrAir::Ground
            && self.physics.ground_knockback_velocity == 0.0
        {
            let normal = self.collision.data.floor.normal;
            let limit = assets.damage.ground_knockback_limit;
            let speed = self.physics.knockback_velocity.x.clamp(-limit, limit);
            self.physics.ground_knockback_velocity = speed;
            self.physics.knockback_velocity.x = normal.y * speed;
            self.physics.knockback_velocity.y = -normal.x * speed;
        }
    }
    /// ftCo_DownBound_Phys -> ft_80084F3C, even while the bounce script marks air.
    pub(super) fn down_physics(
        &mut self,
        assets: &FighterAssets,
        map: &melee_mp::CollMap,
        wind: Wind,
    ) {
        use crate::physics::{
            friction::{friction_acceleration, wait_friction},
            grounded::{self, GroundedParameters},
            integrate,
        };
        let params = GroundedParameters::from_attributes(&self.attributes, &assets.common);
        let friction = wait_friction(
            self.physics.ground_velocity,
            params.friction,
            params.walk_max_velocity,
            params.above_walk_multiplier,
        );
        self.physics.ground_acceleration =
            friction_acceleration(self.physics.ground_velocity, friction);
        grounded::apply_ground_movement(
            &mut self.physics,
            self.collision.data.floor.normal,
            map.floor_speed_scale(&self.collision.data),
        );
        if self.physics.ground_or_air == melee_types::GroundOrAir::Ground {
            grounded::finish_ground_update(
                &mut self.physics,
                &self.collision.data,
                &params,
                map,
                wind,
            );
        } else {
            self.decay_air_knockback(assets);
            integrate::integrate_velocity(&mut self.physics);
            integrate::integrate_environment(&mut self.physics, None, wind);
        }
    }
}

impl super::FighterCore {
    /// ftCo_800976A4 (0x800976A4)'s effect: 0x407 unless the floor's terrain
    /// replaces it (ft_80084C74).
    pub(super) fn bound_effect(&self) -> u16 {
        self.collision
            .floor_terrain_effects(self.physics.ground_or_air)
            .bound
            .map_or(0x407, |effect| effect as u16)
    }
}

impl FighterCore {
    /// mv.co.downreflect.x4: the wall of the last DownReflect.
    fn last_down_reflect(&self) -> Option<BounceSurface> {
        match &self.state_data {
            MotionData::Down { last_reflect, .. } => *last_reflect,
            _ => None,
        }
    }
}

impl Fighter {
    /// ftCo_800C7CA0 (800C7CA0): ground knockback driving a prone fighter
    /// into a wall faster than PlCo +1B0 bounces off it, unless that wall
    /// was the last bounce.
    fn try_down_reflect(
        &mut self,
        assets: &FighterAssets,
        map: &mut melee_mp::CollMap,
    ) -> Result<bool> {
        use melee_types::mp::collide::{LEFT_WALL_HUG, RIGHT_WALL_HUG};
        let cd = &self.core.collision.data;
        let env = cd.env_flags as u32;
        let speed = assets.damage.fly_reflect_speed;
        let knockback = self.core.physics.knockback_velocity.x;
        let last = self.core.last_down_reflect();
        let (corner, normal, surface) = if knockback < -speed
            && env & RIGHT_WALL_HUG != 0
            && last != Some(BounceSurface::LeftWall)
        {
            (
                cd.ecb.left,
                cd.right_facing_wall.normal,
                BounceSurface::LeftWall,
            )
        } else if knockback > speed
            && env & LEFT_WALL_HUG != 0
            && last != Some(BounceSurface::RightWall)
        {
            (
                cd.ecb.right,
                cd.left_facing_wall.normal,
                BounceSurface::RightWall,
            )
        } else {
            return Ok(false);
        };
        // ftKb_SpecialN_800F1F1C is Kirby's.
        let offset = Vec3::new(corner.x, corner.y, 0.0);
        self.enter_down_reflect(normal, offset, surface, assets, map)?;
        Ok(true)
    }

    /// fn_800C7DC4 (800C7DC4): off the floor, spark and small quake at the
    /// contact, the ground knockback speed sent along the wall's normal and
    /// damped, then DownReflect facing along it.
    fn enter_down_reflect(
        &mut self,
        normal: Vec3,
        offset: Vec3,
        surface: BounceSurface,
        assets: &FighterAssets,
        map: &mut melee_mp::CollMap,
    ) -> Result<()> {
        let MotionData::Down {
            wait_remaining,
            retained_word,
            ..
        } = self.core.state_data
        else {
            panic!("down scratch");
        };
        self.leave_ground();
        let p = self.core.physics.position;
        let contact = Vec3::new(p.x + offset.x, p.y + offset.y, p.z + offset.z);
        let angle = melee_lb::trigf::atan2f(-normal.x, normal.y);
        self.core
            .effects
            .push(melee_ef::request::EffectRequest::SurfaceRebound {
                position: contact,
                angle,
            });
        self.core.quake_request = Some(melee_cm::QuakeKind::Small);
        // retail 800C7E90..800C7EC8: fmuls by xF0, then x and y by PlCo +1BC.
        let speed = self.core.physics.ground_knockback_velocity;
        let damping = assets.fly_reflect.damping;
        self.core.physics.knockback_velocity = Vec3::new(
            normal.x * speed * damping,
            normal.y * speed * damping,
            normal.z * speed,
        );
        self.core.physics.self_velocity = Vec3::ZERO;
        self.core.physics.facing = if self.core.physics.knockback_velocity.x < 0.0 {
            -1.0
        } else {
            1.0
        };
        self.change_down_reflect_motion(assets)?;
        let trans = self
            .core
            .animation
            .root_motion
            .as_ref()
            .expect("DownReflect TransN")
            .primary_history
            .position;
        // retail 800C7F38..800C7F44 tests only Collide_RightWallHug: off a
        // wall on the right the fighter is placed in y, as off a ceiling.
        if self.core.collision.data.env_flags as u32 & melee_types::mp::collide::RIGHT_WALL_HUG != 0
        {
            // retail 800C7F5C / 800C7F60: fadds, then fnmsubs with the negated facing.
            self.core.physics.position.x =
                gekko_math::fma::fnmsubs(trans.z, -self.core.physics.facing, p.x + offset.x);
        } else {
            // retail 800C7F78 / 800C7F7C: two fadds.
            self.core.physics.position.y = trans.y + (p.y + offset.y);
        }
        // ftCo_80090574 -> ft_80081DD4, result unused.
        self.damage_air_pass(assets, map);
        // dmg.x18A8 = the speed: armour's reference knockback (ftCo_8008E984),
        // which the port does not model.
        self.core
            .commands
            .rumble_requests
            .push(super::commands::RumbleRequest {
                all_players: false,
                id: 7,
                duration: 0,
            });
        // ftColl_8007B760 with PlCo +1B8.
        self.core.status.ledge_intangibility = self
            .core
            .status
            .ledge_intangibility
            .max(assets.fly_reflect.intangible_frames);
        // The caller's stb at +2344 (retail 800C7D30 / 800C7D98).
        let side = match surface {
            BounceSurface::LeftWall => 1,
            _ => 2,
        };
        self.core.state_data = MotionData::Down {
            wait_remaining,
            retained_word: retained_word
                .map(|word| f32::from_bits(word.to_bits() & 0x00FF_FFFF | side << 24)),
            last_reflect: Some(surface),
        };
        Ok(())
    }
}

/// ftCo_DownBound_Coll (80097E40): off the floor the fighter falls;
/// otherwise a wall may bounce it (ftCo_800C7CA0).
pub(super) fn bound_collision(
    fighter: &mut Fighter,
    phase: super::state::CollisionPhase<'_>,
) -> Result<()> {
    use crate::collision::ground::{map_ground_action, WaitGroundResult};
    let assets = phase.assets.expect("down bound collision needs assets");
    let result = map_ground_action(
        &mut fighter.core.physics,
        &mut fighter.core.collision,
        phase.map,
        &mut fighter.core.skeleton,
        fighter.core.animation.root,
        fighter.core.input.current.stick.x,
    );
    if result == WaitGroundResult::EnterFall {
        // ftCo_Fall_Enter converts only a grounded source; DownBound may
        // already be airborne, with an expired ECB lock.
        fighter.change_motion_state(S::Fall.into(), assets)?;
    } else {
        fighter.try_down_reflect(assets, phase.map)?;
    }
    Ok(())
}

/// ftCo_DownReflect_Anim (800C7FC8): the bounce ends in DamageFall.
pub(super) fn reflect_animation(
    fighter: &mut Fighter,
    phase: super::state::AnimationPhase<'_>,
) -> Result<Option<crate::anim::WaitChoice>> {
    fighter.step_animation(phase.assets);
    if !fighter
        .core
        .animation
        .frames_remaining(&fighter.core.skeleton)
    {
        fighter.enter_damage_fall(phase.assets)?;
        fighter.core.state_data = MotionData::Damage(super::damage::DamageState {
            hitstun: 0.0,
            jump_buffer: 0.0,
            trail_timer: 0,
            influence: phase.assets.damage.influence,
            last_bounce: None,
            bounce_lock: 0,
            meteor_cancel: None,
        });
    }
    Ok(None)
}

/// ftCo_DownReflect_Coll (800C8028): a landing (ft_80081DD4) bounces again
/// (ftCo_80097D88), keeping the wall of this reflect.
pub(super) fn reflect_collision(
    fighter: &mut Fighter,
    phase: super::state::CollisionPhase<'_>,
) -> Result<()> {
    let assets = phase.assets.expect("down reflect collision needs assets");
    if fighter.land_from_damage_air(assets, phase.map)? == Some(true) {
        fighter.enter_down_bound_from(assets, true)?;
    }
    Ok(())
}
