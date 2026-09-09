use super::assets::{FighterAssets, Result};
use super::*;
use crate::{
    anim::WaitChoice,
    collision::pose::FlatGroundPose,
    input::{input_source, run_cpu_input_proc, update_input, InputContext, PadSample},
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
        (self.motion_state.row.anim)(self, state::AnimationPhase { assets, rng })
    }
    /// Finish ftAnim_8006E9B4 suspended in the blend-tree evaluator, or the
    /// Wait restart's ftAnim_8006EBE8 rate setup. The importer validates that
    /// the main pose has not been blended yet (return PC 8006EB18).
    pub fn resume_wait_animation(
        &mut self,
        assets: &FighterAssets,
        rng: &mut HsdRng,
        restart: bool,
        configuring: bool,
    ) -> Result<Option<WaitChoice>> {
        assert_eq!(self.motion_state.id, melee_types::CommonMotionState::Wait);
        assert!(!self
            .animation
            .flags
            .contains(crate::anim::playback::MotionFlags::ROOT_MOTION));
        assert!(self
            .animation
            .part_animations
            .iter()
            .all(|part| part.current == -1));
        let motion = &assets.motions[&self.animation.motion_id];
        // The sampled HSD evaluator only writes the secondary pose. Re-request
        // its pure archive tracks at the interrupted frame, then blend once
        // into the untouched saved main pose. No counters or RNG are replayed.
        let frame = if restart {
            0.0
        } else {
            self.animation.frame + self.animation.speed
        };
        self.animation
            .blend_tree
            .req_anim_all_by_flags(self.animation.root, 1, frame);
        if configuring {
            self.animation.blend_duration = motion.blend_frames;
            self.animation.blend_progress = 0.0;
        } else {
            // retail 8006EA70 already incremented this saved word.
            self.animation.blend_progress -= self.animation.speed;
        }
        if restart {
            self.commands
                .restart(assets.command_entries[&self.animation.motion_id]);
        }
        self.step_animation(assets);
        self.update_idle_animation(assets, rng)
    }

    pub(super) fn update_idle_animation(
        &mut self,
        assets: &FighterAssets,
        rng: &mut HsdRng,
    ) -> Result<Option<WaitChoice>> {
        let commands = &mut self.commands;
        let ground_pose = &mut self.ground_pose;
        let result = self.animation.update_wait_with_restart(
            &mut self.skeleton,
            rng,
            if self.motion_state.id == melee_types::CommonMotionState::SquatWait {
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
            (self.motion_state.row.iasa)(self, state::InputPhase { assets });
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
        (self.motion_state.row.physics)(self, state::PhysicsPhase { assets, map, wind });
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
        (self.motion_state.row.collision)(self, state::CollisionPhase { assets: None, map })
            .expect("grounded map callback");
    }
    /// State-changing map dispatch, including Landing's immediate command/RNG work.
    /// The original proc_map remains the grounded API used by melee-sim M3.
    pub fn proc_map_with_assets(
        &mut self,
        assets: &FighterAssets,
        map: &mut CollMap,
        rng: &mut HsdRng,
    ) -> Result<usize> {
        if self.status.disabled {
            return Ok(0);
        }
        self.status.require_supported();
        (self.motion_state.row.collision)(
            self,
            state::CollisionPhase {
                assets: Some(assets),
                map,
            },
        )?;
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
    /// Fighter camera procedure at 0x8006D9EC (0x8006D9EC), s_link 18.
    pub fn proc_camera(&mut self, assets: &FighterAssets, fixed_zoom: f32) {
        if self.status.disabled {
            return;
        }
        self.status.require_supported();
        self.status.camera_shift = Vec2::ZERO;
        (self.motion_state.row.camera)(
            self,
            state::CameraPhase {
                assets,
                zoom: fixed_zoom,
            },
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
