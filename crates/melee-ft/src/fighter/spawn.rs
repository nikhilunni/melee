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
    /// Human and CPU slots both consume the reaction and attack-delay draws.
    pub fn initialize(mode: i32, level: i32, rng: &mut HsdRng) -> Self {
        let behavior = match mode {
            1 | 25 => 12,
            15 => 0,
            _ => 1,
        };
        // Retail 0x800A124C fmul / 0x800A1258 fctiwz: double 10.0 * Randf then fctiwz.
        // asm.py ftCo_800A101C --fused: no multiply-add sites.
        let reaction_timer = (10.0_f64 * f64::from(rng.randf())) as i32;
        let attack_delay = Self::initial_attack_delay(mode, level, rng);
        Self {
            buttons: 0,
            stick: [0; 2],
            mode,
            level,
            behavior,
            reaction_timer,
            attack_delay,
            hurtbox_extents: [1.0, 1.0, 1.0, 2.0],
        }
    }

    /// ftCo_800B9704 (ftcpuattack.c:2098-2106), called at 0x800A1744.
    fn initial_attack_delay(mode: i32, level: i32, rng: &mut HsdRng) -> i32 {
        // Retail 0x800B9718 draws even for humans. 0x800B9734/0x800B974C
        // are fmadds; 0x800B9750 truncates with fctiwz before mode-7 halving.
        let random_delay = gekko_math::fma::fmadds(15.0, rng.randf(), 15.0);
        let delay = gekko_math::msl::fctiwz(gekko_math::fma::fmadds(
            (10 - level) as f32,
            random_delay,
            10.0,
        ));
        if mode == 7 {
            delay / 2
        } else {
            delay
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
        Self::create(player, character, assets, skeleton, root, context, None)
    }

    /// Fighter_Create (80069324..8006933C): Player's entry flag selects
    /// Entry directly after reset, without first entering Wait or Fall.
    pub fn spawn_for_match(
        player: PlayerSlot,
        character: C,
        assets: &FighterAssets,
        skeleton: JObjTree,
        root: JObjId,
        context: SpawnContext<'_>,
        delay: i32,
    ) -> Result<Self> {
        Self::create(
            player,
            character,
            assets,
            skeleton,
            root,
            context,
            Some(delay),
        )
    }

    fn create(
        player: PlayerSlot,
        character: C,
        assets: &FighterAssets,
        skeleton: JObjTree,
        root: JObjId,
        context: SpawnContext<'_>,
        entry_delay: Option<i32>,
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
            fighter.leave_ground();
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
        if let Some(delay) = entry_delay {
            // Fighter_ChangeMotionState sets TopN's facing rotation even
            // for SM_None. The ordinary animation-entry path does this itself.
            fighter.skeleton.set_rotation_y(
                root,
                (std::f64::consts::FRAC_PI_2 * f64::from(fighter.physics.facing)) as f32,
            );
            fighter.enter_match(delay, assets)?;
        } else {
            fighter.change_motion_state(
                if supported {
                    CommonMotionState::Wait
                } else {
                    CommonMotionState::Fall
                },
                assets,
            )?;
        }
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
            assets.kind,
            "character callbacks must match their assets"
        );
        assert!(player.facing == 1.0 || player.facing == -1.0);
        if player.scale != 1.0 {
            unimplemented!("ftchangeparam.c:178-195: scaled fighter attribute modifiers");
        }
        let mut capabilities = Capabilities::default();
        character.on_load(&mut capabilities);
        character.on_resources_loaded(assets, &player);
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
        animation.root_motion = Some(crate::anim::root_motion::RootMotion {
            translation: animation.parts
                [usize::from(assets.parts.joint(melee_types::FtPart::TransN).unwrap())]
            .joint,
            secondary: animation.parts
                [usize::from(assets.parts.joint(melee_types::FtPart::TransN2).unwrap())]
            .joint,
            primary_history: Default::default(),
            secondary_history: Default::default(),
            effective_scale: model_scale,
            compensate_joint: None,
        });
        skeleton.set_scale(root, &Vec3::new(model_scale, model_scale, model_scale));
        let data = ecb::initialize(
            map,
            position,
            &assets.bones.ecb,
            player.scale,
            assets.attributes.size.weight,
        );
        let dynamics = assets
            .dynamics
            .iter()
            .map(|desc| {
                let joint = skeleton
                    .bone(root, desc.root)
                    .expect("missing dynamics root");
                melee_lb::dynamics::DynamicBoneSet::new(
                    &mut skeleton,
                    joint,
                    &desc.springs,
                    desc.multipliers,
                )
            })
            .collect();
        Self {
            dynamics,
            dynamics_use_floor_plane: false,
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
            state_data: MotionData::None,
            combat: super::damage::CombatState::default(),
            shield: super::shield::ShieldState::default(),
            effect_state: super::effects::FighterEffects::default(),
            effects: Vec::new(),
            character,
            capabilities,
            cpu: CpuState {
                buttons: 0,
                stick: [0; 2],
                mode: player.cpu_mode,
                level: player.cpu_level,
                behavior: 1,
                reaction_timer: 0,
                attack_delay: 0,
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
    /// (cold airborne spawn).
    pub fn change_motion_state(
        &mut self,
        state: CommonMotionState,
        assets: &FighterAssets,
    ) -> Result<()> {
        self.change_motion_state_at(state, assets, 0.0)
    }

    /// Fighter_ChangeMotionState (0x800693AC): retain a caller-supplied walk phase.
    pub(super) fn change_motion_state_at(
        &mut self,
        state: CommonMotionState,
        assets: &FighterAssets,
        start: f32,
    ) -> Result<()> {
        self.change_motion_state_with_rate(state, assets, start, 1.0)
    }

    /// Fighter_ChangeMotionState (800693AC), fighter.c:1268-1296:
    /// install the caller's rate before frame-zero animation and commands.
    pub(super) fn change_motion_state_with_rate(
        &mut self,
        state: CommonMotionState,
        assets: &FighterAssets,
        start: f32,
        rate: f32,
    ) -> Result<()> {
        self.status.require_supported();
        if self.physics.ground_or_air == GroundOrAir::Ground {
            self.character.on_grounded_motion();
        }
        self.apply_dynamic_commands(assets);
        self.flush_effects_on_motion_change();
        self.shield.clear_collision();
        self.status.ignore_fighter_nudge = false;
        self.status.on_ledge = false;
        self.status.grab_exclusions = ledge::GrabExclusions::NONE;
        self.commands.allow_interrupt = false;
        self.commands.hitboxes.fill(None);
        self.commands.hurt_status = super::escape::HurtStatus::Normal;
        // fighter.c:1101-1102: ordinary entries clear fast fall.
        if !matches!(
            state,
            CommonMotionState::Fall | CommonMotionState::FallSpecial
        ) {
            self.physics.fast_fall = false;
        }
        // ftCo_800D638C preserves the nametag while Squat becomes SquatWait;
        // ordinary motion entry clears it (fighter.c:1155-1157).
        let preserve_name_tag = matches!(
            state,
            CommonMotionState::JumpAerialF
                | CommonMotionState::JumpAerialB
                | CommonMotionState::CliffWait
        ) || (state == CommonMotionState::SquatWait
            && self.motion_state.id == CommonMotionState::Squat);
        if !preserve_name_tag {
            self.status.name_tag_timer = 0;
        }
        if self.effect_state.destroy_on_state_change {
            self.effect_state.destroy_on_state_change = false;
            self.effects
                .push(super::effects::EffectRequest::DestroyOwned);
        }
        super::commands::reset_parts(&mut self.animation, &mut self.skeleton, assets);
        if matches!(
            state,
            CommonMotionState::Entry | CommonMotionState::EntryEnd
        ) {
            self.motion_state = if state == CommonMotionState::Entry {
                MotionState::ENTRY
            } else {
                MotionState::ENTRY_END
            };
            self.animation.clear_motion(&mut self.skeleton);
            self.commands.instruction = None;
            return Ok(());
        }
        let (motion_state, animation_id) = match state {
            CommonMotionState::Wait if self.physics.ground_or_air == GroundOrAir::Ground => {
                (MotionState::WAIT, 2)
            }
            CommonMotionState::DamageN2 => (MotionState::DAMAGE_N2, 169),
            CommonMotionState::Attack11 => (MotionState::JAB, 46),
            CommonMotionState::Squat => (MotionState::SQUAT, 30),
            CommonMotionState::SquatWait => (MotionState::SQUAT_WAIT, 31),
            CommonMotionState::SquatRv => (MotionState::SQUAT_RV, 34),
            CommonMotionState::GuardOn => (MotionState::GUARD_ON, 37),
            CommonMotionState::Guard => (MotionState::GUARD, 38),
            CommonMotionState::GuardReflect => (MotionState::GUARD_REFLECT, 37),
            CommonMotionState::GuardOff => (MotionState::GUARD_OFF, 39),
            CommonMotionState::GuardSetOff => (MotionState::GUARD_SET_OFF, 40),
            CommonMotionState::EscapeF => (MotionState::ESCAPE_F, 42),
            CommonMotionState::EscapeB => (MotionState::ESCAPE_B, 43),
            CommonMotionState::EscapeN => (MotionState::ESCAPE_N, 41),
            CommonMotionState::EscapeAir => (MotionState::ESCAPE_AIR, 44),
            CommonMotionState::CliffCatch => (MotionState::CLIFF_CATCH, 216),
            CommonMotionState::CliffWait => (MotionState::CLIFF_WAIT, 217),
            CommonMotionState::CliffJumpQuick1 => (MotionState::CLIFF_JUMP_1, 227),
            CommonMotionState::CliffJumpQuick2 => (MotionState::CLIFF_JUMP_2, 228),
            CommonMotionState::CliffJumpSlow1 => (MotionState::CLIFF_JUMP_SLOW_1, 225),
            CommonMotionState::CliffJumpSlow2 => (MotionState::CLIFF_JUMP_SLOW_2, 226),
            CommonMotionState::Dash => (MotionState::DASH, 12),
            CommonMotionState::Run => (MotionState::RUN, 13),
            CommonMotionState::RunBrake => (MotionState::RUN_BRAKE, 14),
            CommonMotionState::Turn => (MotionState::TURN, 10),
            CommonMotionState::WalkSlow => (MotionState::WALK_SLOW, 7),
            CommonMotionState::WalkMiddle => (MotionState::WALK_MIDDLE, 8),
            CommonMotionState::WalkFast => (MotionState::WALK_FAST, 9),
            CommonMotionState::TurnRun => (MotionState::TURN_RUN, 11),
            CommonMotionState::CliffClimbQuick => (MotionState::CLIFF_CLIMB, 220),
            CommonMotionState::CliffEscapeQuick => (MotionState::CLIFF_ESCAPE, 224),
            CommonMotionState::KneeBend => (MotionState::KNEE_BEND, 15),
            CommonMotionState::JumpF => (MotionState::JUMP, 16),
            CommonMotionState::JumpB => (MotionState::JUMP_BACK, 17),
            CommonMotionState::JumpAerialB => (MotionState::JUMP_AERIAL_BACK, 19),
            CommonMotionState::JumpAerialF => (MotionState::JUMP_AERIAL, 18),
            CommonMotionState::Fall => (MotionState::FALL, 20),
            CommonMotionState::Pass => (MotionState::PASS, 209),
            CommonMotionState::FallAerial => (MotionState::FALL_AERIAL, 23),
            CommonMotionState::FallSpecial => (MotionState::FALL_SPECIAL, 26),
            CommonMotionState::EntryStart => (MotionState::ENTRY_START, 238),
            CommonMotionState::Landing => (MotionState::LANDING, 35),
            CommonMotionState::LandingFallSpecial => (MotionState::LANDING_FALL_SPECIAL, 36),
            _ => unimplemented!("fighter.c:1190-1194: unsupported motion entry {state:?}"),
        };
        self.motion_state = motion_state;
        for (i, set) in self.dynamics.iter_mut().enumerate() {
            let first = assets.dynamics_motion_starts[&animation_id][i];
            let dynamic = first != 0x100;
            self.dynamics_first_bone[i] = first;
            crate::dynamics::select(
                set,
                &mut self.skeleton,
                &mut self.animation.parts,
                dynamic,
                if dynamic { first as usize } else { 0 },
            );
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
        if matches!(
            state,
            CommonMotionState::GuardOn | CommonMotionState::Guard | CommonMotionState::GuardReflect
        ) {
            // Ft_MF_SkipAnim, fighter.c:1349-1361: clear AObjs and script.
            self.animation.clear_motion(&mut self.skeleton);
            self.commands.instruction = None;
            return Ok(());
        }
        self.animation.set_animation(
            &mut self.skeleton,
            &assets.motions[&animation_id],
            start,
            rate,
        )?;
        self.animation.set_rate(&mut self.skeleton, rate, false);
        self.animation.frame = start - rate;
        self.animation.remainder = 0.0;
        self.commands.restart(assets.command_entries[&animation_id]);
        // Fighter_ChangeMotionState (0x800693AC), fighter.c:1298,1342-1347:
        // main animation then commands. Part blends run only in the ordinary
        // ftAnim_8006EBA4 tick (ftanim.c:380-385), not again on motion entry.
        self.animation
            .advance_main::<RetailTrig>(&mut self.skeleton);
        // fighter.c:1309-1317: discard frame-zero extracted velocity.
        if start == 0.0
            && self
                .animation
                .flags
                .contains(crate::anim::MotionFlags::ROOT_MOTION)
        {
            if let Some(root) = &mut self.animation.root_motion {
                root.primary_history.previous = root.primary_history.position;
                root.primary_history.offset = Vec3::ZERO;
                root.primary_history.previous_offset = Vec3::ZERO;
            }
        }
        if start != 0.0 {
            self.commands.seek(
                &mut self.animation,
                &mut self.skeleton,
                &mut self.ground_pose,
                assets,
            );
        } else {
            self.commands.step(
                &mut self.animation,
                &mut self.skeleton,
                &mut self.ground_pose,
                assets,
            );
        }
        self.apply_dynamic_commands(assets);
        if state == CommonMotionState::Fall {
            if self.physics.ground_or_air == GroundOrAir::Ground {
                self.leave_ground();
            }
            self.state_data = MotionData::Fall(super::fall::FallState::new(
                super::fall::FallFamily::Ordinary,
            ));
            let max = self.attributes.air.air_drift_max;
            self.physics.self_velocity.x = self.physics.self_velocity.x.clamp(-max, max);
        }
        if state == CommonMotionState::FallAerial {
            // ftCo_FallAerial_Enter (800CCDA8): no drift clamp or ground conversion.
            self.state_data =
                MotionData::Fall(super::fall::FallState::new(super::fall::FallFamily::Aerial));
        }
        if state == CommonMotionState::Wait {
            self.status.interaction = Interaction::Idle;
            // ft_8008A348, ft_08A1.c:98 -> ftCommon_8007EFC0.
            self.status.name_tag_timer = assets.name_tag_duration;
        }
        Ok(())
    }
}
