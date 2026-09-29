use super::assets::{FighterAssets, Result};
use super::*;
use crate::input::InputSource;
use crate::{
    anim::WaitChoice,
    collision::pose::GroundPose,
    input::{input_source, run_cpu_input_proc, update_input, InputContext, PadSample},
};
use gekko_math::rng::HsdRng;
use melee_gr::wind::Wind;
use melee_mp::CollMap;

/// ft_PlaySFX (80088148): the footstep sound ids whose pitch it varies.
const FOOTSTEP_PITCH_SOUNDS: std::ops::RangeInclusive<u32> = 332..=370;

impl Fighter {
    /// Fighter_8006A360 (0x8006A360), s_link 1, fighter.c:1444-1701.
    /// Main playback precedes Wait_Anim and its immediate animation restart.
    pub fn proc_anim(
        &mut self,
        assets: &FighterAssets,
        map: &mut CollMap,
        rng: &mut HsdRng,
    ) -> Result<Option<WaitChoice>> {
        let choice = if self.core.begin_animation_phase(assets) {
            // ftCo_800D71D8 follows ftAnim_8006EBA4, before the state's callback.
            self.run_catch_window(assets);
            self.core.combat.combo.grace = self.core.combat.combo.grace.saturating_sub(1);
            let choice = (self.motion_row.anim)(self, state::AnimationPhase { assets, map, rng })?;
            // ftAction_80072894 ran inside ftAnim_8006EBA4; its item work
            // needs the character.
            self.apply_parasol_commands();
            choice
        } else {
            None
        };
        if !self.core.status.disabled {
            // ftCo_800C0408: color programs run during hitlag as well.
            self.core.advance_color_overlay(assets);
        }
        Ok(choice)
    }
    /// Fighter_Spaghetti_8006AD10 (0x8006AD10), s_link 3, fighter.c:1777-2140.
    pub fn proc_input(&mut self, assets: &FighterAssets, sample: &PadSample) {
        if self.core.sample_input(assets, sample) {
            self.core.update_smash_charge_input(assets);
            (self.motion_row.iasa)(self, state::InputPhase { assets });
        }
    }

    /// Fighter_procUpdate (0x8006B82C), s_link 4, fighter.c:2150-2438.
    pub fn proc_update(&mut self, assets: &FighterAssets, map: &CollMap, wind: Wind) {
        if self.core.status.disabled {
            return;
        }
        let physics = self.core.begin_physics_phase();
        if physics {
            (self.motion_row.physics)(self, state::PhysicsPhase { assets, map, wind });
        }
        self.core.apply_combo_push(&assets.combo);
        if self.core.combat.hitlag_remaining > 0.0 {
            match self.core.combat.hitlag_callbacks {
                super::damage::HitlagCallbacks::Damage => self.core.damage_hitlag_input(),
                super::damage::HitlagCallbacks::Guard => self.core.guard_hitlag_input(),
                super::damage::HitlagCallbacks::None => {}
            }
        }
        if !physics {
            // fighter.c:2380-2390 (0x8006BE48): a grounded fighter rides a
            // moving floor (mpGetSpeed) even in hitlag, where the state's
            // physics (which applies it otherwise) is skipped; the wind
            // offset is zero there.
            self.core.ride_floor_in_hitlag(map);
        }
        self.core.invalidate_collision_positions();
    }
    /// Fighter_procMap (0x8006C27C), s_link 6, fighter.c:2476-2516.
    pub fn proc_map(&mut self, map: &mut CollMap) {
        if self.core.status.disabled {
            return;
        }
        self.core.status.require_supported();
        (self.motion_row.collision)(self, state::CollisionPhase { assets: None, map })
            .expect("grounded map callback");
        self.core
            .skeleton
            .set_translate(self.core.animation.root, &self.core.physics.position);
    }
    /// State-changing map dispatch. Landing's dust draws with the proc's
    /// graphics (`resolve_graphics_commands`), in script order.
    /// The original proc_map remains the grounded API used by melee-sim M3.
    pub fn proc_map_with_assets(
        &mut self,
        assets: &FighterAssets,
        map: &mut CollMap,
    ) -> Result<()> {
        if self.core.status.disabled {
            return Ok(());
        }
        self.core.status.require_supported();
        (self.motion_row.collision)(
            self,
            state::CollisionPhase {
                assets: Some(assets),
                map,
            },
        )?;
        // Fighter_procMap publishes the final position after the collision callback.
        self.core
            .skeleton
            .set_translate(self.core.animation.root, &self.core.physics.position);
        Ok(())
    }
    /// Fighter_ProcessHit_8006D1EC (0x8006D1EC), s_link 14.
    /// Apply accumulated hits and enter damage/hitlag, then update shield and caches.
    pub fn proc_process_hit(&mut self, assets: &FighterAssets, rng: &mut HsdRng) {
        if self.core.status.disabled {
            return;
        }
        self.core.status.require_supported();
        if let Some(callback) = self.character.table().process_defense_hit {
            callback(self, assets);
        }
        // Fighter_ProcessHit updates health before any response can clear Guard.
        let exhausted = self.core.update_shield_health(assets);
        self.process_damage(assets, rng).expect("hit response");
        self.shield_proc(assets, exhausted)
            .expect("shield response");
        self.core.update_hurtbox_extents();
    }
    /// Rendered joints feed the next ftCo_8009CB40 ownership change through
    /// their cached matrices. Publish the dynamic chain even while locked:
    /// a same-tick Wait -> Walk can release and reclaim ownership before the
    /// next display. The solver updates SRT/springs, not these JObj caches.
    pub fn prepare_dynamic_display_caches(&mut self) {
        if self.core.status.disabled
            || self.core.effect_state.invisible
            || self.core.commands.fighter_hidden
        {
            return;
        }
        for set in &self.core.dynamics {
            for bone in &set.bones {
                self.core.skeleton.setup_matrix(bone.joint);
            }
        }
    }
    /// Fighter_8006D9AC (0x8006D9AC), s_link 16 -> ftCo_8009DD94.
    /// Retail skips this entire proc while the hitlag latch is set.
    /// Wait x594_b3 selects ftCo_8009CB40(..., false, NULL), setting bone_id
    /// to 0x100 (ftdynamics.c:55); lb_8001044C returns at lb_00F9.c:447.
    pub fn proc_dynamics(&mut self) {
        if self.core.status.disabled || self.core.in_hitlag() {
            return;
        }
        self.core.status.require_supported();
        self.core.update_dynamic_colliders();
        self.solve_dynamics(None, &[]);
    }
    /// Fighter_8006D9AC with the scene's mpCheckFloor provider.
    pub fn proc_dynamics_with_map(&mut self, map: &mut CollMap) {
        if self.core.status.disabled || self.core.in_hitlag() {
            return;
        }
        self.core.status.require_supported();
        self.core.update_dynamic_colliders();
        self.solve_dynamics(Some(map), &[]);
    }
    /// Fighter_8006D9AC with the scene's force-field pool; Marth and Roy
    /// respond to its wind state first (ftCo_8009E614).
    pub fn proc_dynamics_with_forces(
        &mut self,
        assets: &FighterAssets,
        map: &mut CollMap,
        forces: &[melee_lb::dynamics::ForceField],
    ) {
        if self.core.status.disabled || self.core.in_hitlag() {
            return;
        }
        self.core.status.require_supported();
        self.core.update_dynamic_colliders();
        self.core.respond_to_stage_wind(assets);
        self.solve_dynamics(Some(map), forces);
    }
    fn solve_dynamics(
        &mut self,
        mut map: Option<&mut CollMap>,
        forces: &[melee_lb::dynamics::ForceField],
    ) {
        let frame = self.core.dynamics_frame();
        let count = self.core.dynamics.len();
        for index in 0..count.min(self.core.dynamics_first_bone.len()) {
            let first_force_bone = self.character.dynamics_first_force_bone(index, count);
            self.core.solve_dynamic_set(
                index,
                first_force_bone,
                &frame,
                map.as_deref_mut(),
                forces,
            );
        }
    }
    /// ftCo_Cliff_Cam (80081644), ftcliffcommon.c:160-168: camera box first,
    /// then notify the supporting stage joint while hanging airborne.
    pub fn proc_camera_with_map(
        &mut self,
        assets: &FighterAssets,
        stage: &melee_cm::StageCamera,
        map: &mut CollMap,
    ) {
        self.proc_camera(assets, stage);
        if !self.core.status.disabled && self.core.camera.on_ledge {
            let MotionData::Cliff(cliff) = &self.core.state_data else {
                panic!("cliff camera scratch missing")
            };
            map.notify_ledge_grab(&mut self.core.collision.data, cliff.ledge_id);
        }
    }
    /// Fighter camera procedure at 0x8006D9EC (0x8006D9EC), s_link 18.
    pub fn proc_camera(&mut self, assets: &FighterAssets, stage: &melee_cm::StageCamera) {
        if self.core.status.disabled {
            return;
        }
        self.core.status.require_supported();
        self.core.status.camera_shift = Vec2::ZERO;
        (self.motion_row.camera)(self, state::CameraPhase { assets, stage });
    }
}
impl FighterCore {
    /// The port's attack-proc guard: motion scratch of an attack, a rebound,
    /// a ledge attack, or a character special with a staled move.
    pub(super) fn in_attack_state(&self) -> bool {
        matches!(
            self.state_data,
            super::MotionData::Jab(_)
                | super::MotionData::RapidJab(_)
                | super::MotionData::Aerial { .. }
                | super::MotionData::Tilt { .. }
                | super::MotionData::DashAttack { .. }
                | super::MotionData::Smash { .. }
                | super::MotionData::DownTilt { .. }
                | super::MotionData::Down { .. }
                | super::MotionData::Swing(_)
        ) || matches!(
            (&self.state_data, self.motion_state.id),
            (
                super::MotionData::Rebound(_),
                melee_types::CommonMotionState::ReboundStop
                    | melee_types::CommonMotionState::Rebound
            )
        ) || (matches!(self.state_data, super::MotionData::Cliff(_))
            && matches!(
                self.motion_state.id,
                melee_types::CommonMotionState::CliffAttackQuick
                    | melee_types::CommonMotionState::CliffAttackSlow
            ))
            || (usize::from(self.motion_state.action.0) >= super::COMMON_COUNT
                && self.combat.stale.current_move().is_some())
    }
    /// Fighter_8006A1BC (0x8006A1BC), s_link 0, fighter.c:1393-1442.
    /// Hitlag expires here; unported interactions remain explicit guards.
    /// Returns whether a cape turn ended with a character's x21F8 to run
    /// (`Fighter::proc_status` runs it).
    pub fn proc_status(&mut self) -> bool {
        if self.status.disabled {
            return false;
        }
        self.status.require_supported();
        match self.status.interaction {
            super::Interaction::Hitlag => assert!(
                self.in_hitlag(),
                "hitlag requires x2219_b5 (a countdown or a partner hold)"
            ),
            super::Interaction::Damage => assert!(
                matches!(self.state_data, super::MotionData::Damage(_)),
                "damage requires damage state"
            ),
            super::Interaction::Attack => {
                assert!(self.in_attack_state(), "attack requires attack state")
            }
            _ => {}
        }
        self.release_separated_hold();
        self.tick_hitlag();
        let character_turn_end = self.step_cape_turn();
        // ft_800819A8 (0x800819A8), ft_0819.c:32-46: three fadds.
        let cd = &self.collision.data;
        self.previous_collision_bounds = Vec3::new(
            cd.ecb.left.x + cd.cur_pos.x,
            cd.ecb.right.x + cd.cur_pos.x,
            self.physics.position.y + self.attributes.camera.damage_camera_y_offset,
        );
        character_turn_end
    }
    /// Command 25 (ftAction set ground/air), applied as the script runs:
    /// in the ordinary animation step and in Fighter_ChangeMotionState's
    /// frame-zero run (ftAction_80073240).
    pub(super) fn apply_airborne_commands(&mut self) {
        for state in self.commands.airborne_changes.take_all() {
            match state {
                melee_cmd::AirborneMode::Ground => self.land(),
                melee_cmd::AirborneMode::Air => self.leave_ground(),
                melee_cmd::AirborneMode::AirUseAllJumps => {
                    // ftCommon_8007D60C: five ECB ticks and all jumps consumed.
                    self.physics.ground_or_air = melee_types::GroundOrAir::Air;
                    self.physics.ground_velocity = 0.0;
                    self.physics.animation_velocity.y = 0.0;
                    self.physics.jumps_used = self.attributes.jumping.max_jumps as u8;
                    self.collision.lock_frames = 5;
                    self.collision.data.x130_flags |= melee_types::mp::coll_data_x130::LOCKED;
                }
            }
        }
    }
    pub fn step_animation(&mut self, assets: &FighterAssets) {
        // ftAnim_8006EBA4: command-driven animation ownership changes must
        // finish before the independent part blends are evaluated.
        self.animation
            .advance_main::<RetailTrig>(&mut self.skeleton);
        let hand = self.held_item_hand(assets);
        self.commands.step(
            &mut self.animation,
            &mut self.skeleton,
            &mut self.ground_pose,
            assets,
            hand,
            self.physics.facing,
        );
        self.apply_script_damage();
        self.apply_airborne_commands();
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
    }
    /// ftAction_80072CD8 (80072CD8): a footstep on the ground asks the
    /// floor's terrain (ft_80084BFC) for a sound to play instead, whether
    /// the command's own sound still plays, and a foot effect. In the air
    /// only the command's sound plays. Each sound goes through
    /// ftAction_80071B50, whose ft_PlaySFX draws a pitch for footstep
    /// sounds. The scene resolves the step's requests after the proc, as it
    /// does random sounds (a script step mixing both is not ordered).
    ///
    /// A terrain effect (ftaction.c:1164-1177) is ftCo_8009F834 at a foot
    /// with zero offset and range; it joins the graphics commands at the
    /// footstep's place in the script.
    pub fn resolve_terrain_footsteps(&mut self, rng: &mut HsdRng) {
        let terrain = self
            .collision
            .floor_terrain_flags(self.physics.ground_or_air)
            .map(|flags| melee_mp::terrain_footstep(self.collision.stage, flags));
        let mut inserted = 0;
        for step in self.commands.terrain_footsteps.take_all() {
            let Some(terrain) = terrain else {
                self.play_command_sound(step.sound, rng);
                continue;
            };
            if let Some(id) = terrain.sound {
                let sound = super::commands::FootstepSound {
                    id,
                    ..step.sound.clone()
                };
                self.play_command_sound(sound, rng);
            }
            if let Some(effect) = terrain.effect {
                let feet = &self.bones.model;
                let foot = if step.alt_foot {
                    feet.right_foot
                } else {
                    feet.left_foot
                };
                self.commands.graphics.insert(
                    step.graphics_before + inserted,
                    melee_types::combat::GraphicsCommand {
                        id: effect as u16,
                        bone: usize::from(foot),
                        common_bone: false,
                        item_bone: false,
                        destroy_on_state_change: false,
                        parameter: 0.0,
                        offset: hsd_types::Vec3::ZERO,
                        range: hsd_types::Vec3::ZERO,
                        issued_facing: None,
                    },
                );
                inserted += 1;
            }
            if terrain.keeps_sound {
                self.play_command_sound(step.sound, rng);
            }
        }
    }
    /// ftAction_80071B50 for a resolved footstep: ft_PlaySFX (behavior 0)
    /// varies a footstep sound's pitch (ft_0877.c:460-461, HSD_Randi(200)
    /// for ids 332..=370 after ft_80087D0C, which keeps footstep ids there).
    fn play_command_sound(&mut self, sound: super::commands::FootstepSound, rng: &mut HsdRng) {
        if matches!(sound.channel, super::commands::SoundChannel::Ordinary)
            && FOOTSTEP_PITCH_SOUNDS.contains(&sound.id)
        {
            let _pitch = rng.randi(200) - 100;
        }
        self.commands.footstep_sounds.push(sound);
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

    /// ftCo_8008A6D8 (8008A6D8): play idle animation `motion` from its
    /// first frame with its own script, as a finished idle loop does.
    pub(super) fn play_idle_animation(
        &mut self,
        assets: &FighterAssets,
        motion: i32,
    ) -> Result<()> {
        self.animation
            .set_animation(&mut self.skeleton, &assets.motions[&motion], 0.0, 1.0)?;
        let hand = self.held_item_hand(assets);
        restart_idle_step(
            &mut self.animation,
            &mut self.skeleton,
            &mut self.commands,
            &mut self.ground_pose,
            assets,
            (hand, self.physics.facing),
        );
        self.apply_dynamic_commands(assets);
        Ok(())
    }

    pub(super) fn update_idle_animation(
        &mut self,
        assets: &FighterAssets,
        rng: &mut HsdRng,
    ) -> Result<Option<WaitChoice>> {
        let hand = self.held_item_hand(assets);
        let facing = self.physics.facing;
        let commands = &mut self.commands;
        let ground_pose = &mut self.ground_pose;
        // ftCo_8008A7A8: holding an item, most kinds replay the current idle.
        let holding = self.held_item.is_some() && !assets.idle_variants_while_holding;
        let result = self.animation.update_wait_with_restart(
            &mut self.skeleton,
            rng,
            if holding {
                None
            } else if self.motion_state.id == melee_types::CommonMotionState::SquatWait {
                assets.squat_choices.as_deref()
            } else {
                assets.wait_choices.as_deref()
            },
            |id| &assets.motions[&id],
            |animation, tree| {
                restart_idle_step(
                    animation,
                    tree,
                    commands,
                    ground_pose,
                    assets,
                    (hand, facing),
                )
            },
        )?;
        self.apply_dynamic_commands(assets);
        // ftCommon_8007E0E4: reset before the fighter-overlap nudge query.
        self.physics.player_nudge = Vec2::ZERO;
        Ok(result)
    }
    /// Fighter_8006ABA0 (0x8006ABA0), s_link 2, fighter.c:1703-1709.
    pub fn proc_cpu_gate(&mut self) {
        run_cpu_input_proc(self.status.disabled, self.input_source());
    }
    /// ftCo_800A2040 (0x800A2040): Player_8003248C resolves the slot's kind
    /// for this fighter. PdPmdat's entry z is zero for the Ice Climbers, the
    /// only character whose second fighter is a partner rather than a
    /// transformation, so Nana is a CPU in a human slot.
    pub fn input_source(&self) -> InputSource {
        let kind =
            crate::input::resolve_player_kind(self.player.control, self.player.secondary, true);
        input_source(kind, self.cpu.mode)
    }
    /// Fighter_8006ABA0's gate: ftCo_800A2040 and x221F_b3 clear. The scene
    /// then runs melee-cpu's ftCo_800B3900 for this fighter.
    pub fn cpu_driven(&self) -> bool {
        !self.status.disabled && self.input_source() == InputSource::Cpu
    }
    /// Fighter_8006C5F4 (0x8006C5F4), s_link 7, fighter.c:2518-2525.
    pub fn proc_pose(&mut self, assets: &FighterAssets, map: &CollMap) {
        if self.status.disabled {
            return;
        }
        let pose = GroundPose {
            bones: self
                .bones
                .ground_pose
                .as_ref()
                .expect("character ground pose"),
            player_scale: self.player.scale,
            flags: self.ground_pose,
            max_tilt_degrees: assets.common.ground_pose_max_angle_degrees,
        };
        pose.update(
            &self.physics,
            &self.collision,
            map,
            &mut self.skeleton,
            self.animation.root,
        );
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
                super::hitbox::update(
                    hit,
                    &mut self.skeleton,
                    &self.animation,
                    self.player.scale,
                    self.grafted_part,
                );
            }
            self.thrown_hitbox
                .update(&mut self.skeleton, &self.animation);
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
    /// Fighter_8006DA4C (0x8006DA4C), s_link 22, fighter.c:3073-3083.
    pub fn proc_player_mirror(&mut self) {
        if self.status.disabled {
            return;
        }
        self.player_position = self.physics.position;
        self.player_facing = self.physics.facing;
    }
}

impl FighterCore {
    // Scheduler bookkeeping keeps the original order around typed row dispatch.
    fn begin_animation_phase(&mut self, assets: &FighterAssets) -> bool {
        if self.status.disabled {
            return false;
        }
        self.status.require_supported();
        self.physics.begin_tick();
        // Fighter_8006A360, fighter.c:1464-1484: a protection timer running
        // out clears the flash (color animation 9) if it still owns the slot.
        // These and the magnifier run during hitlag too.
        if self.status.ledge_intangibility != 0 {
            self.status.ledge_intangibility -= 1;
            if self.status.ledge_intangibility == 0 {
                self.combat.color_overlay.flash_expired = true;
            }
        }
        if self.status.revival_invincibility != 0 {
            self.status.revival_invincibility -= 1;
            if self.status.revival_invincibility == 0 {
                self.combat.color_overlay.flash_expired = true;
            }
        }
        self.apply_magnifier_damage(&assets.magnifier);
        // fighter.c:1658: x2219_b5, the hitlag flag, gates the rest.
        if self.in_hitlag() {
            return false;
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
        true
    }
    fn sample_input(&mut self, assets: &FighterAssets, sample: &PadSample) -> bool {
        if self.status.disabled {
            return false;
        }
        self.status.require_supported();
        let hitlag = self.in_hitlag();
        // Fighter_Spaghetti_8006AD10 (fighter.c:1802-1869): ftCo_800A2040
        // selects the CPU's getters over HSD_PadGameStatus.
        let cpu_sample;
        let sample = if self.input_source() == InputSource::Cpu {
            cpu_sample = self.cpu.pad_sample();
            &cpu_sample
        } else {
            sample
        };
        let effects = update_input(
            &mut self.input,
            InputSource::Pad,
            sample,
            &assets.input,
            InputContext {
                save_and_clear: self.status.input_frozen,
                hitlag,
                ..InputContext::default()
            },
        );
        self.joystick_count += u64::from(effects.joystick_count_increments);
        effects.run_input_callback && !self.in_hitlag()
    }
    /// Fighter_procUpdate 0x8006BE48 for a fighter whose physics hitlag
    /// skipped: the moving-floor offset (mpGetSpeed) at the current position.
    fn ride_floor_in_hitlag(&mut self, map: &CollMap) {
        if self.physics.ground_or_air != melee_types::GroundOrAir::Ground {
            return;
        }
        let speed = map.line_speed(self.collision.data.floor.index, &self.physics.position);
        crate::physics::integrate::integrate_environment(&mut self.physics, speed, Wind::CALM);
    }
    fn begin_physics_phase(&mut self) -> bool {
        if self.status.disabled {
            return false;
        }
        self.status.require_supported();
        if self.in_hitlag() {
            return false;
        }
        if self.status.ledge_cooldown != 0 {
            self.status.ledge_cooldown -= 1;
        }
        true
    }
    fn invalidate_collision_positions(&mut self) {
        if self.combat.reflector_enabled {
            self.shield.reflect.volume.position_cached = false;
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
    /// ftAction_80072E4C -> ftCo_8009F834 for the landing effects queued
    /// before graphics command `before` (all of them with `usize::MAX`).
    /// The floor's terrain may replace the command's effect (ft_80084C38).
    pub(super) fn resolve_landing_effects(&mut self, rng: &mut HsdRng, before: usize) -> usize {
        let mut draws = 0;
        let terrain = self
            .collision
            .floor_terrain_effects(self.physics.ground_or_air)
            .landing;
        loop {
            let Some(&(id, position)) = self.commands.landing_effects.iter().next() else {
                break;
            };
            if position > before {
                break;
            }
            self.commands.landing_effects.remove(0);
            let id = terrain.map_or(id, |effect| effect as u16);
            if matches!(id, 0x423 | 0x424) {
                // ftCo_8009F834 block_12 -> block_67: no offset draws; the
                // floor's angle on the ground, efAsync kind 3 on
                // parts[FtPart_TopN], the root joint (Mario's landing flash).
                let normal = self.collision.data.floor.normal;
                let floor_angle = if self.physics.ground_or_air == melee_types::GroundOrAir::Ground
                {
                    melee_lb::trigf::atan2f(-normal.x, normal.y)
                } else {
                    0.0
                };
                self.effects
                    .push_graphics(melee_ef::request::EffectRequest::Graphics {
                        id,
                        bone: 0,
                        offset: Vec3::ZERO,
                        facing: self.physics.facing,
                        floor_angle,
                    });
                continue;
            }
            // ftCo_8009F834 block_70. Even a zero range consumes three draws.
            let mut offset = Vec3::ZERO;
            for component in [&mut offset.x, &mut offset.y, &mut offset.z] {
                let random = rng.randf();
                // Retail fused sites 0x8009FCF8/FD1C/FD44 (asm.py --fused).
                *component = gekko_math::fma::fmadds(0.0, random - 0.5, *component);
                draws += 1;
            }
            let normal = self.collision.data.floor.normal;
            self.effects
                .push_graphics(melee_ef::request::EffectRequest::Landing {
                    id,
                    offset,
                    floor_angle: melee_lb::trigf::atan2f(-normal.x, normal.y),
                });
        }
        draws
    }
    fn update_hurtbox_extents(&mut self) {
        self.cpu.hurtbox_extents = caches::hurtbox_extents(
            &mut self.hurtboxes,
            &mut self.skeleton,
            &self.animation,
            self.physics.position,
            self.physics.facing,
            self.player.scale,
        );
    }
}

/// Dynamics inputs sampled once before the per-set character hooks.
struct DynamicsFrame {
    // Fighter +0x1670..0x1828 holds eleven 0x28-byte collider records
    // (ft/types.h); assets::read_dynamic_colliders already enforces this bound.
    colliders: [melee_lb::dynamics::Collider; 11],
    collider_count: usize,
    ground_check: bool,
    plane: bool,
    height: f32,
}
impl FighterCore {
    /// ftCo_8009DD94: collider positions before the first dynamics set.
    fn update_dynamic_colliders(&mut self) {
        for collider in &mut self.dynamic_colliders {
            collider.position = caches::part_position(
                &mut self.skeleton,
                &self.animation,
                collider.bone,
                collider.offset,
            );
        }
    }
    fn dynamics_frame(&self) -> DynamicsFrame {
        let mut colliders = std::array::from_fn(|_| melee_lb::dynamics::Collider {
            position: hsd_types::Vec3::ZERO,
            radius: 0.0,
        });
        assert!(self.dynamic_colliders.len() <= colliders.len());
        for (source, destination) in self.dynamic_colliders.iter().zip(&mut colliders) {
            *destination = melee_lb::dynamics::Collider {
                position: source.position,
                radius: source.radius,
            };
        }
        DynamicsFrame {
            colliders,
            collider_count: self.dynamic_colliders.len(),
            ground_check: self.player.scale == 1.0
                && self.physics.ground_or_air == melee_types::GroundOrAir::Ground,
            plane: self.dynamics_use_floor_plane,
            height: self.physics.position.y,
        }
    }
    /// ftCo_8009DD94 (ftdynamics.c:397-419): solve after the set's force-bone hook.
    fn solve_dynamic_set(
        &mut self,
        index: usize,
        first_force_bone: usize,
        frame: &DynamicsFrame,
        mut map: Option<&mut CollMap>,
        forces: &[melee_lb::dynamics::ForceField],
    ) {
        let environment = melee_lb::dynamics::SolverEnvironment {
            disabled: false,
            colliders: &frame.colliders[..frame.collider_count],
            forces,
            first_force_bone,
            ground_check: frame.ground_check,
        };
        self.dynamics[index].solve(
            &mut self.skeleton,
            self.dynamics_first_bone[index],
            &environment,
            &mut |a, b| {
                if frame.plane {
                    melee_lb::dynamics::floor_plane(a, b, frame.height)
                } else {
                    map.as_deref_mut()
                        .expect("active grounded dynamics requires proc_dynamics_with_map")
                        .check_floor(a.x, a.y, b.x, b.y, 0.1, -1, -1, -1, None)
                        .map(|hit| hit.pos)
                }
            },
        );
    }
}

/// ftCo_8008A6D8's restart after the idle motion is attached: its script from
/// the top, then the first step (ftAnim_8006EBA4).
fn restart_idle_step(
    animation: &mut crate::anim::playback::FighterAnimation,
    tree: &mut hsd_anim::jobj::JObjTree,
    commands: &mut super::commands::CommandState,
    ground_pose: &mut GroundPoseFlags,
    assets: &FighterAssets,
    (hand, facing): (super::commands::HeldItemHand, f32),
) {
    commands.restart(assets.command_entries[&animation.motion_id]);
    animation.step_with_hooks::<RetailTrig>(
        tree,
        |animation, tree| commands.step(animation, tree, ground_pose, assets, hand, facing),
        |_, _| {},
    );
}
