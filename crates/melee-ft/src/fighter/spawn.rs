use super::assets::{FighterAssets, Result};
use super::*;
use crate::{anim::attach::PartFlags, collision::ecb};
use gekko_math::rng::HsdRng;
use hsd_anim::jobj::JObjId;
use melee_mp::CollMap;
use melee_types::{CommonMotionState, GroundOrAir};

/// Spawn inputs read from Player, pl/player.c:228-240 and fighter.c:688-739.
#[derive(Clone, Debug)]
pub struct PlayerSlot {
    pub id: u8,
    pub control: PlayerKind,
    pub costume: u8,
    pub stocks: u8,
    pub position: Vec3,
    pub facing: f32,
    /// Player_GetModelScale; independent of co_attrs.model_scaling.
    pub scale: f32,
    /// Player_GetDamage, used by reset (+1830).
    pub damage: f32,
    pub cpu_mode: i32,
    pub cpu_level: i32,
}

/// Fighter_NewSpawn_80068E40: wrapping counter never produces zero after wrap.
#[derive(Debug)]
pub struct SpawnCounter(pub u32);

/// Scene-owned spawn services; the RNG and counter are shared by both players.
pub struct SpawnContext<'a> {
    pub map: &'a mut CollMap,
    pub rng: &'a mut HsdRng,
    pub counter: &'a mut SpawnCounter,
}
impl SpawnCounter {
    pub fn allocate(&mut self) -> u32 {
        let number = self.0;
        self.0 = self.0.wrapping_add(1);
        if self.0 == 0 {
            self.0 = 1;
        }
        number
    }
}
impl CpuState {
    /// ftCo_800A101C (0x800A101C), ftCo_0A01.c:674-750.
    /// Human and CPU slots both consume exactly one draw here.
    pub fn initialize(mode: i32, level: i32, rng: &mut HsdRng) -> Self {
        let behavior = match mode {
            1 | 25 => 12,
            15 => 0,
            _ => 1,
        };
        // Retail 0x800A124C fmul / 0x800A1258 fctiwz: double 10.0 * Randf then fctiwz.
        // asm.py ftCo_800A101C --fused: no multiply-add sites.
        let reaction_timer = (10.0_f64 * f64::from(rng.randf())) as i32;
        Self {
            buttons: 0,
            stick: [0; 2],
            mode,
            level,
            behavior,
            reaction_timer,
            hurtbox_extents: [1.0, 1.0, 1.0, 2.0],
        }
    }
}
impl<C: CharacterCallbacks> Fighter<C> {
    /// Fighter_Create (0x80068E98), UnkInitLoad (0x80068914),
    /// UnkInitReset (0x80067C98), UnkProcessDeath (0x80068354).
    /// Costume skeleton loading and Player slot selection precede this call.
    /// Grounded spawn takes the +/-10 probe in ft_80082A68 (0x80082A68).
    pub fn spawn(
        player: PlayerSlot,
        character: C,
        assets: &FighterAssets,
        skeleton: JObjTree,
        root: JObjId,
        context: SpawnContext<'_>,
    ) -> Result<Self> {
        let SpawnContext { map, rng, counter } = context;
        let initial_scale = skeleton.scale(root);
        let mut fighter = Self::prepare(player, character, assets, skeleton, root, map);
        // Reset probes support before Fighter_UpdateModelScale (fighter.c:543).
        fighter.skeleton.set_scale(root, &initial_scale);
        fighter.spawn_number = counter.allocate();
        let data = &mut fighter.collision.data;
        data.last_pos = fighter.physics.position;
        data.cur_pos = fighter.physics.position;
        data.last_pos.y += 10.0;
        data.cur_pos.y -= 10.0;
        let pose = ecb::EcbPose::read(&mut fighter.skeleton, root, data);
        let supported = map.air_collide_pass(data, Some(&|i| pose.position(i)));
        if supported {
            fighter.physics.position = data.cur_pos;
            fighter.physics.ground_or_air = GroundOrAir::Ground;
            fighter.physics.jumps_used = 0;
            fighter.collision.lock_frames = 0;
        } else {
            // ftCommon_8007D5D4 (0x8007D5D4), ftcommon.c:515-525.
            // A failed probe leaves Fighter.cur_pos at the Player marker.
            fighter.physics.ground_or_air = GroundOrAir::Air;
            fighter.physics.ground_velocity = 0.0;
            fighter.physics.shield_knockback_velocity.z = 0.0;
            fighter.physics.position.z = 0.0;
            fighter.physics.animation_velocity.y = 0.0;
            fighter.physics.jumps_used = 1;
            fighter.collision.lock_frames = 10;
            fighter.collision.data.x130_flags |= melee_types::mp::coll_data_x130::LOCKED;
        }
        fighter
            .skeleton
            .set_translate(root, &fighter.physics.position);
        let scale = fighter.player.scale * fighter.attributes.size.model_scaling;
        fighter
            .skeleton
            .set_scale(root, &Vec3::new(scale, scale, scale));
        fighter.character.on_reset();
        // Fighter_UnkProcessDeath, fighter.c:561: always initialize this capsule.
        fighter.thrown_hitbox.state = 1;
        fighter.thrown_hitbox.update(&mut fighter.skeleton, root);
        fighter.cpu = CpuState::initialize(fighter.player.cpu_mode, fighter.player.cpu_level, rng);
        fighter.change_motion_state(
            if supported {
                CommonMotionState::Wait
            } else {
                CommonMotionState::Fall
            },
            assets,
        )?;
        // ftLib_800867E8 at the end of Fighter_Create: clear input and freeze
        // sampling until match setup calls ftLib_8008688C.
        fighter.input.clear_current_and_buffers();
        fighter.status.input_frozen = true;
        Ok(fighter)
    }

    /// Allocate the typed owners without consuming RNG or entering a state.
    /// Savestate restoration replaces the supplied dynamic owners afterwards;
    /// it must never call spawn or change_motion_state.
    pub fn prepare(
        player: PlayerSlot,
        mut character: C,
        assets: &FighterAssets,
        mut skeleton: JObjTree,
        root: JObjId,
        map: &CollMap,
    ) -> Self {
        assert_eq!(
            character.kind(),
            FighterKind::Fox,
            "only Fox data is loaded"
        );
        assert!(player.facing == 1.0 || player.facing == -1.0);
        if player.scale != 1.0 {
            unimplemented!("ftchangeparam.c:178-195: scaled fighter attribute modifiers");
        }
        let mut capabilities = Capabilities::default();
        character.on_load(&mut capabilities);
        // fighter.c:241, retail 0x80067CE8: fmadds. x40 was reset to +0.
        let offset = 0.0 * player.scale; // ftCommon_800804EC: separate fmuls.
        let position = Vec3::new(
            gekko_math::fma::fmadds(player.facing, offset, player.position.x),
            player.position.y,
            player.position.z,
        );
        let mut physics = FighterPhysics::standing(position, player.facing);
        physics.percent = player.damage;
        let mut animation = FighterAnimation::new(&skeleton, root);
        animation.translation_joint = Some(usize::from(assets.bones.model.animation_translation));
        animation.model_scale = assets.attributes.size.model_scaling;
        let bone = assets
            .parts
            .joint(melee_types::FtPart::TransN)
            .expect("missing TransN");
        animation.parts[usize::from(bone)].flags.0 |= PartFlags::COPY;
        // ftParts_80074E58: translation-preserving semantic parts.
        for part in [
            melee_types::FtPart::TopN,
            melee_types::FtPart::TransN,
            melee_types::FtPart::XRotN,
            melee_types::FtPart::YRotN,
            melee_types::FtPart::HipN,
            melee_types::FtPart::TransN2,
        ] {
            let index = assets.parts.joint(part).expect("missing translation part");
            animation.parts[usize::from(index)].flags.0 |= PartFlags::TRANSLATION;
        }
        // ftParts_80074E58 (0x80074E58), ftparts.c:692: semantic part 53.
        let last = assets.parts.part_to_joint[53].expect("missing part 53");
        animation.parts[usize::from(last)].flags.0 |= PartFlags::COPY;
        skeleton.set_translate(root, &position);
        // Fighter_UpdateModelScale / ftCommon_GetModelScale:
        // retail 0x8007F69C fmuls, no contraction.
        let model_scale = player.scale * assets.attributes.size.model_scaling;
        skeleton.set_scale(root, &Vec3::new(model_scale, model_scale, model_scale));
        let data = ecb::initialize(
            map,
            position,
            &assets.bones.ecb,
            player.scale,
            assets.attributes.size.weight,
        );
        Self {
            kind: character.kind(),
            spawn_number: 0,
            physics,
            animation,
            input: FighterInput::default(),
            collision: EnvironmentCollision::new(data),
            attributes: assets.attributes.clone(),
            bones: assets.bones.clone(),
            skeleton,
            motion_state: MotionState::WAIT,
            character,
            capabilities,
            cpu: CpuState {
                buttons: 0,
                stick: [0; 2],
                mode: player.cpu_mode,
                level: player.cpu_level,
                behavior: 1,
                reaction_timer: 0,
                hurtbox_extents: [1.0, 1.0, 1.0, 2.0],
            },
            status: Status::reset(assets.shield_health),
            commands: commands::CommandState::default(),
            ground_pose: GroundPoseFlags::default(),
            dynamics_first_bone: vec![0x100; assets.bones.dynamics_roots.len()],
            player_position: position,
            player_facing: player.facing,
            joystick_count: 0,
            previous_collision_bounds: Vec3::ZERO,
            camera: CameraSubject::default(),
            hurtboxes: assets.hurtboxes.clone(),
            dynamic_colliders: assets.dynamic_colliders.clone(),
            thrown_hitbox: assets.thrown_hitbox.clone(),
            player,
        }
    }

    /// Fighter_ChangeMotionState (0x800693AC), fighter.c:933-1391,
    /// reached through ft_8008A2BC/ft_8008A348 (Wait) or ftCo_Fall_Enter
    /// (cold airborne spawn). Fall ticking remains explicitly unsupported.
    pub fn change_motion_state(
        &mut self,
        state: CommonMotionState,
        assets: &FighterAssets,
    ) -> Result<()> {
        self.status.require_idle();
        let (motion_state, animation_id) = match state {
            CommonMotionState::Wait if self.physics.ground_or_air == GroundOrAir::Ground => {
                (MotionState::WAIT, 2)
            }
            CommonMotionState::Fall if self.physics.ground_or_air == GroundOrAir::Air => {
                (MotionState::FALL, 20)
            }
            _ => unimplemented!("fighter.c:1190-1194: unsupported motion entry {state:?}"),
        };
        self.motion_state = motion_state;
        let dynamic = assets.motions[&animation_id].flags.0 & 0x1000_0000 == 0;
        for (i, &bone) in self.bones.dynamics_roots.iter().enumerate() {
            self.dynamics_first_bone[i] = if dynamic { 0 } else { 0x100 };
            let mut joint = self.skeleton.bone(self.animation.root, bone as usize);
            while let Some(id) = joint {
                let part = self
                    .animation
                    .parts
                    .iter_mut()
                    .find(|p| p.joint == id)
                    .unwrap();
                if dynamic {
                    part.flags.0 |= PartFlags::LOCKED;
                } else {
                    part.flags.0 &= !PartFlags::LOCKED;
                }
                joint = self.skeleton.child(id);
            }
        }
        self.ground_pose = GroundPoseFlags::default();
        self.status.sword_trail = -1;
        self.status.camera_shift = Vec2::ZERO;
        self.collision.data.floor_skip = -1;
        let root = self.animation.root;
        self.skeleton.set_translate(root, &self.physics.position);
        self.skeleton.set_rotation_x(root, 0.0);
        // fighter.c:1183; retail uses double M_PI_2 multiplication, no FMA.
        self.skeleton.set_rotation_y(
            root,
            (std::f64::consts::FRAC_PI_2 * f64::from(self.physics.facing)) as f32,
        );
        self.skeleton.set_rotation_z(root, 0.0);
        self.animation.set_animation(
            &mut self.skeleton,
            &assets.motions[&animation_id],
            0.0,
            1.0,
        )?;
        self.commands.restart(assets.command_entries[&animation_id]);
        self.step_animation(assets);
        if state == CommonMotionState::Wait {
            // ft_8008A348, ft_08A1.c:98 -> ftCommon_8007EFC0.
            self.status.name_tag_timer = assets.name_tag_duration;
        }
        Ok(())
    }
}
