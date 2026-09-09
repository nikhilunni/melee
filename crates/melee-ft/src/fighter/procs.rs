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
    /// Timed status/hitlag/mushroom arms are explicit in Status::require_idle.
    pub fn proc_status(&mut self) {
        if self.status.disabled {
            return;
        }
        self.status.require_idle();
        // ft_800819A8 (0x800819A8), ft_0819.c:32-46: three fadds.
        let cd = &self.collision.data;
        self.previous_collision_bounds = Vec3::new(
            cd.ecb.left.x + cd.cur_pos.x,
            cd.ecb.right.x + cd.cur_pos.x,
            self.physics.position.y + self.attributes.camera.damage_camera_y_offset,
        );
    }
    pub(super) fn step_animation(&mut self, assets: &FighterAssets) {
        let commands = &mut self.commands;
        let ground_pose = &mut self.ground_pose;
        self.animation.step_with_hooks::<RetailTrig>(
            &mut self.skeleton,
            |animation, tree| commands.step(animation, tree, ground_pose, assets),
            |_, _| {},
        ); // ftCo_800DB500: no attached parasol (item-free gate).
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
        self.status.require_idle();
        match self.motion_state.callbacks.animation {
            state::AnimationCallback::Wait => {}
            state::AnimationCallback::FallUnimplemented => {
                unimplemented!("ftCo_Fall.c:106: Fall_Anim")
            }
        }
        self.physics.begin_tick();
        if self.status.name_tag_timer > 1 && !self.status.input_frozen {
            self.status.name_tag_timer -= 1;
        }
        if self.status.time_since_hit != -1 {
            self.status.time_since_hit = self.status.time_since_hit.wrapping_add(1);
        }
        if self.status.time_since_smash != -1.0 {
            self.status.time_since_smash += 1.0;
        }
        self.step_animation(assets);
        let commands = &mut self.commands;
        let ground_pose = &mut self.ground_pose;
        let result = self.animation.update_wait_with_restart(
            &mut self.skeleton,
            rng,
            Some(&assets.wait_choices),
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
        self.status.require_idle();
        match self.motion_state.callbacks.input {
            state::InputCallback::Wait => {}
            state::InputCallback::FallUnimplemented => unimplemented!("ftCo_Fall.c:198: Fall_IASA"),
        }
        let effects = update_input(
            &mut self.input,
            input_source(self.player.control, self.cpu.mode),
            sample,
            &assets.input,
            InputContext {
                save_and_clear: self.status.input_frozen,
                ..InputContext::default()
            },
        );
        self.joystick_count += u64::from(effects.joystick_count_increments);
        if effects.run_input_callback {
            let transition = wait_iasa(
                &self.input,
                &assets.input,
                &WaitContext {
                    facing: self.physics.facing,
                    specials_available: self.capabilities.specials,
                    shield_health: self.status.shield_health,
                    ..WaitContext::default()
                },
            );
            if transition != WaitTransition::None {
                unimplemented!("ftCo_Wait.c:46-66: non-idle IASA transition {transition:?}");
            }
        }
    }
    /// Fighter_procUpdate (0x8006B82C), s_link 4, fighter.c:2150-2438.
    pub fn proc_update(&mut self, assets: &FighterAssets, map: &CollMap, wind: Vec3) {
        if self.status.disabled {
            return;
        }
        self.status.require_idle();
        match self.motion_state.callbacks.physics {
            state::PhysicsCallback::Wait => {}
            state::PhysicsCallback::FallUnimplemented => {
                unimplemented!("ftCo_Fall.c:203: Fall_Phys")
            }
        }
        if self.physics.ground_or_air != melee_types::GroundOrAir::Ground {
            unimplemented!("fighter.c:2181: airborne physics");
        }
        if self.status.ledge_cooldown != 0 {
            self.status.ledge_cooldown -= 1;
        }
        step_wait(
            &mut self.physics,
            &self.collision.data,
            &GroundedParameters::from_attributes(&self.attributes, &assets.common),
            map,
            wind,
        );
        for hurt in &mut self.hurtboxes {
            hurt.cached = false;
        }
    }
    /// Fighter_procMap (0x8006C27C), s_link 6, fighter.c:2476-2516.
    pub fn proc_map(&mut self, map: &mut CollMap) {
        if self.status.disabled {
            return;
        }
        self.status.require_idle();
        match self.motion_state.callbacks.collision {
            state::CollisionCallback::Wait => {}
            state::CollisionCallback::FallUnimplemented => {
                unimplemented!("ftCo_Fall.c:208: Fall_Coll")
            }
        }
        match map_wait(
            &mut self.physics,
            &mut self.collision,
            map,
            &mut self.skeleton,
            self.animation.root,
            self.input.current.stick.x,
        ) {
            WaitGroundResult::Supported => {}
            WaitGroundResult::EnterFall => unimplemented!("ft_081B.c:1094: Wait -> Fall"),
            WaitGroundResult::EnterTeeter => unimplemented!("ft_081B.c:1092: Wait -> Ottotto"),
        }
    }
    /// Fighter_8006C5F4 (0x8006C5F4), s_link 7, fighter.c:2518-2525.
    pub fn proc_pose(&mut self, map: &CollMap) {
        if self.status.disabled {
            return;
        }
        let pose = FlatGroundPose {
            bones: self.bones.ground_pose.as_ref().expect("Fox ground pose"),
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
            self.status.require_idle();
        }
    }
    /// Fighter_8006C80C (0x8006C80C), s_link 9, fighter.c:2555-2596.
    /// Empty async queue, disabled attacks, NULL accessory on ordinary Wait.
    /// The always-enabled thrown capsule still updates (ft_07C1.c:52-73).
    pub fn proc_hitbox_positions(&mut self) {
        if !self.status.disabled {
            self.status.require_idle();
            self.thrown_hitbox
                .update(&mut self.skeleton, self.animation.root);
        }
    }
    /// Fighter_UnkProcessGrab_8006CA5C (0x8006CA5C), s_link 12.
    /// Wait has no catch hitbox (x221E_b6 is cleared by motion entry).
    pub fn proc_grab(&mut self) {
        if !self.status.disabled {
            self.status.require_idle();
        }
    }
    /// Fighter_8006CB94 (0x8006CB94), s_link 13, fighter.c:2621-2642.
    /// The scene must supply Damage/Grab before dispatch when interactions exist.
    pub fn proc_hit_detection(&mut self) {
        if !self.status.disabled {
            self.status.require_idle();
        }
    }
    /// Fighter_ProcessHit_8006D1EC (0x8006D1EC), s_link 14.
    /// Shield regeneration and all hit accumulators are inactive at reset health.
    pub fn proc_process_hit(&mut self, assets: &FighterAssets) {
        if self.status.disabled {
            return;
        }
        self.status.require_idle();
        if self.status.shield_health != assets.shield_health {
            unimplemented!("fighter.c:2813-2820: shield regeneration");
        }
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
        self.status.require_idle();
        for collider in &mut self.dynamic_colliders {
            collider.position = caches::bone_position(
                &mut self.skeleton,
                self.animation.root,
                collider.bone,
                collider.offset,
            );
        }
        for &bone in &self.dynamics_first_bone {
            if bone <= 0xFF {
                unimplemented!("lb_00F9.c:472-935: active dynamic-bone solver");
            }
        }
    }
    /// Fighter_UnkCallCameraCallback_8006D9EC (0x8006D9EC), s_link 18.
    pub fn proc_camera(&mut self, assets: &FighterAssets, fixed_zoom: f32) {
        if self.status.disabled {
            return;
        }
        self.status.require_idle();
        self.status.camera_shift = Vec2::ZERO;
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
