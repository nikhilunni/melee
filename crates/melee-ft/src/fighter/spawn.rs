use super::assets::{FighterAssets, Result};
use super::*;
use crate::{anim::attach::PartFlags, collision::ecb};
use gekko_math::rng::HsdRng;
use hsd_anim::jobj::JObjId;
use melee_mp::CollMap;
use melee_types::{CommonMotionState, GroundOrAir};

#[derive(Clone, Copy, Default)]
struct MotionChange<'a> {
    start: f32,
    rate: f32,
    source: Option<super::grab_throw::ThrowSource<'a>>,
    ground_air: bool,
    /// fn_800DE798: restore the release owner after reset, before initial commands.
    throw_owner: Option<u32>,
    preserve: MotionPreservation,
}

/// Retail motion-entry flags retained across a ground/air counterpart change.
#[derive(Clone, Copy, Default)]
pub struct MotionPreservation {
    pub hit_status: bool,
    pub hitboxes: bool,
    pub effects: bool,
}

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
#[derive(Debug, Clone)]
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
impl Fighter {
    /// Fighter_Create (0x80068E98), UnkInitLoad (0x80068914),
    /// UnkInitReset (0x80067C98), UnkProcessDeath (0x80068354).
    /// Costume skeleton loading and Player slot selection precede this call.
    /// Grounded spawn takes the +/-10 probe in ft_80082A68 (0x80082A68).
    pub fn spawn(
        player: PlayerSlot,
        character: CharacterState,
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
        character: CharacterState,
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
        character: CharacterState,
        assets: &FighterAssets,
        skeleton: JObjTree,
        root: JObjId,
        context: SpawnContext<'_>,
        entry_delay: Option<i32>,
    ) -> Result<Self> {
        let initial_scale = skeleton.scale(root);
        let mut fighter = Self::prepare(player, character, assets, skeleton, root, context.map);
        fighter.initialize_spawn(assets, context, entry_delay, initial_scale)?;
        Ok(fighter)
    }

    /// Reset-time services shared by creation and revival; all owners already exist.
    pub(super) fn initialize_spawn(
        &mut self,
        assets: &FighterAssets,
        context: SpawnContext<'_>,
        entry_delay: Option<i32>,
        initial_scale: Vec3,
    ) -> Result<()> {
        let SpawnContext { map, rng, counter } = context;
        let root = self.core.animation.root;
        let supported = self
            .core
            .initialize_spawn_geometry(map, counter, initial_scale);
        self.character.on_reset();
        // Fighter_UnkProcessDeath, fighter.c:561: always initialize this capsule.
        self.core.thrown_hitbox.state = 1;
        self.core
            .thrown_hitbox
            .update(&mut self.core.skeleton, root);
        self.core.cpu =
            CpuState::initialize(self.core.player.cpu_mode, self.core.player.cpu_level, rng);
        if let Some(delay) = entry_delay {
            // Fighter_ChangeMotionState sets TopN's facing rotation even
            // for SM_None. The ordinary animation-entry path does this itself.
            self.core.skeleton.set_rotation_y(
                root,
                (std::f64::consts::FRAC_PI_2 * f64::from(self.core.physics.facing)) as f32,
            );
            self.enter_match(delay, assets)?;
        } else {
            self.change_motion_state(
                (if supported {
                    CommonMotionState::Wait
                } else {
                    CommonMotionState::Fall
                })
                .into(),
                assets,
            )?;
        }
        // ftLib_800867E8 at the end of Fighter_Create: clear input and freeze
        // sampling until match setup calls ftLib_8008688C.
        self.core.input.clear_current_and_buffers();
        self.core.status.input_frozen = true;
        Ok(())
    }

    /// Allocate the typed owners without consuming RNG or entering a state.
    /// Savestate restoration replaces the supplied dynamic owners afterwards;
    /// it must never call spawn or change_motion_state.
    pub fn prepare(
        player: PlayerSlot,
        mut character: CharacterState,
        assets: &FighterAssets,
        skeleton: JObjTree,
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
        let motion_row = state::COMMON[CommonMotionState::Wait as usize];
        Self {
            core: FighterCore::prepare(
                player,
                assets,
                skeleton,
                root,
                map,
                capabilities,
                MotionState::new(motion_row),
            ),
            character,
            motion_row,
        }
    }

    /// Fighter_ChangeMotionState (0x800693AC), fighter.c:933-1391,
    /// reached through ft_8008A2BC/ft_8008A348 (Wait) or ftCo_Fall_Enter
    /// (cold airborne spawn).
    pub fn change_motion_state(&mut self, state: ActionId, assets: &FighterAssets) -> Result<()> {
        self.change_motion_state_at(state, assets, 0.0)
    }

    /// Fighter_ChangeMotionState (0x800693AC): retain a caller-supplied walk phase.
    pub fn change_motion_state_at(
        &mut self,
        state: ActionId,
        assets: &FighterAssets,
        start: f32,
    ) -> Result<()> {
        self.change_motion_state_with_rate(state, assets, start, 1.0)
    }

    /// Fighter_ChangeMotionState (800693AC), fighter.c:1268-1296:
    /// install the caller's rate before frame-zero animation and commands.
    pub(super) fn change_motion_state_with_rate(
        &mut self,
        state: ActionId,
        assets: &FighterAssets,
        start: f32,
        rate: f32,
    ) -> Result<()> {
        self.change_motion_state_with_options(
            state,
            assets,
            MotionChange {
                start,
                rate,
                ..Default::default()
            },
        )
    }

    /// ftCo_800DE7C0 installs fn_800DE798 as a one-shot motion-entry callback.
    pub(super) fn change_damage_motion(
        &mut self,
        state: ActionId,
        assets: &FighterAssets,
        throw_owner: Option<u32>,
    ) -> Result<()> {
        self.change_motion_state_with_options(
            state,
            assets,
            MotionChange {
                rate: 1.0,
                throw_owner,
                ..Default::default()
            },
        )
    }

    /// Borrow a throw animation/script while preserving ordinary row selection.
    pub(super) fn change_motion_state_with_source(
        &mut self,
        state: ActionId,
        assets: &FighterAssets,
        start: f32,
        rate: f32,
        source: Option<super::grab_throw::ThrowSource<'_>>,
    ) -> Result<()> {
        self.change_motion_state_with_options(
            state,
            assets,
            MotionChange {
                start,
                rate,
                source,
                ..Default::default()
            },
        )
    }

    /// Fighter_ChangeMotionState with ftCommon_GroundAirColl_MF (fighter.c).
    /// Preserve visibility and advance command control flow without executing
    /// commands already applied by the outgoing ground/air motion.
    pub fn change_ground_air_motion(
        &mut self,
        state: ActionId,
        assets: &FighterAssets,
        preserve: MotionPreservation,
    ) -> Result<()> {
        self.change_motion_state_with_options(
            state,
            assets,
            MotionChange {
                start: self.animation.frame,
                rate: 1.0,
                ground_air: true,
                preserve,
                ..Default::default()
            },
        )
    }

    fn change_motion_state_with_options(
        &mut self,
        state: ActionId,
        assets: &FighterAssets,
        change: MotionChange<'_>,
    ) -> Result<()> {
        let source = change.source;
        let row = self.row(state);
        let state = row.id;
        self.core.begin_motion_change(source);
        if self.core.physics.ground_or_air == GroundOrAir::Ground {
            self.character.on_grounded_motion();
        }
        let move_id = if usize::from(row.action.0) < super::COMMON_COUNT {
            super::attack::stale::GROUND_MOVES
                .get(state as usize)
                .copied()
                .flatten()
        } else {
            self.character
                .table()
                .special_moves
                .get(usize::from(row.action.0) - super::COMMON_COUNT)
                .copied()
                .flatten()
        };
        let animate = self
            .core
            .reset_motion(MotionState::new(row), assets, move_id, change);
        self.motion_row = row;
        // Fighter_ChangeMotionState 80069BC4..BE0 invokes x21EC here, before
        // frame-zero ftAction_80073354 at8006A0BC. Ordinary entry stays cleared.
        if let Some(owner) = change.throw_owner {
            self.commands.thrown_by = Some(owner);
        }
        if !animate {
            return Ok(());
        }
        if state == CommonMotionState::Guard
            || (!self.character.animated_shield()
                && matches!(
                    state,
                    CommonMotionState::GuardOn | CommonMotionState::GuardReflect
                ))
        {
            // Ft_MF_SkipAnim, fighter.c:1349-1361: clear AObjs and script.
            self.core.clear_animation();
            return Ok(());
        }
        self.core
            .start_motion_animation(assets, row.animation, state, change)
    }
}

impl FighterCore {
    /// Reset dynamics, support probe and model placement before the death hook.
    fn initialize_spawn_geometry(
        &mut self,
        map: &mut CollMap,
        counter: &mut SpawnCounter,
        initial_scale: Vec3,
    ) -> bool {
        let root = self.animation.root;
        // ftCo_8009CF84 enables each chain before reset. prepare only allocates
        // owners: savestate import must attach animations before restoring locks.
        for set in &mut self.dynamics {
            crate::dynamics::select(set, &mut self.skeleton, &mut self.animation.parts, true, 0);
        }
        self.dynamics_first_bone.fill(0);
        // Reset probes support before Fighter_UpdateModelScale (fighter.c:543).
        self.skeleton.set_scale(root, &initial_scale);
        self.spawn_number = counter.allocate();
        let data = &mut self.collision.data;
        data.last_pos = self.physics.position;
        data.cur_pos = self.physics.position;
        data.last_pos.y += 10.0;
        data.cur_pos.y -= 10.0;
        let pose = ecb::EcbPose::read(&mut self.skeleton, root, data);
        let supported = map.air_collide_pass(data, Some(&|i| pose.position(i)));
        if supported {
            self.physics.position = data.cur_pos;
            self.physics.ground_or_air = GroundOrAir::Ground;
            self.physics.jumps_used = 0;
            self.collision.lock_frames = 0;
        } else {
            // ftCommon_8007D5D4 (0x8007D5D4), ftcommon.c:515-525.
            // A failed probe leaves Fighter.cur_pos at the Player marker.
            self.leave_ground();
        }
        self.skeleton.set_translate(root, &self.physics.position);
        let scale = self.player.scale * self.attributes.size.model_scaling;
        self.skeleton
            .set_scale(root, &Vec3::new(scale, scale, scale));
        supported
    }

    /// Allocate decoded owners after the character's OnLoad/resource hooks.
    fn prepare(
        player: PlayerSlot,
        assets: &FighterAssets,
        mut skeleton: JObjTree,
        root: JObjId,
        map: &CollMap,
        capabilities: Capabilities,
        motion_state: MotionState,
    ) -> Self {
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
        // lbanim.h FigaTree::nodes is s8: reserve the full per-joint domain
        // once for this FighterPartsTable-sized skeleton.
        skeleton.reserve_animation_tracks(i8::MAX as usize);
        // Throws constrain XRotN to the captor's joint. Keep its slot across releases.
        let xrot = assets.parts.joint(melee_types::FtPart::XRotN).unwrap();
        skeleton.reserve_position_constraint(animation.parts[usize::from(xrot)].joint);
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
        // ftCo_8009CF84 builds spring rest lengths in the unscaled costume pose.
        let dynamics: Vec<_> = assets
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
        let capture_geometry =
            super::grab_throw::CaptureGeometry::from_skeleton(&mut skeleton, root, assets);
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
        Self {
            dynamics,
            dynamics_use_floor_plane: false,
            kind: assets.kind,
            revival_platform: assets.revival_platform.clone(),
            revival_platform_active: false,
            spawn_number: 0,
            physics,
            animation,
            input: FighterInput::default(),
            collision: EnvironmentCollision::new(data),
            attributes: assets.attributes.clone(),
            bones: assets.bones.clone(),
            skeleton,
            motion_state,
            state_data: MotionData::None,
            combat: super::damage::CombatState {
                capture_geometry,
                ..Default::default()
            },
            shield: super::shield::ShieldState::default(),
            effect_state: super::effects::FighterEffects::default(),
            effects: melee_ef::request::EffectQueue::default(),
            item_requests: Default::default(),
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
            grab_handicap: 9, // gm default handicap, before any saved-player override.
            player,
        }
    }
    /// Fighter_ChangeMotionState (fighter.c:933-949), before OnGroundedMotion.
    fn begin_motion_change(&mut self, source: Option<super::grab_throw::ThrowSource<'_>>) {
        self.status.require_supported();
        self.commands.smash_charge = None;
        self.commands.borrowed_script = source.map(|source| source.assets.commands.clone());
        // Fighter_ChangeMotionState (800693AC): clear the throw exception on
        // ordinary entry. Borrowed thrown scripts still need their source owner.
        if source.is_none() {
            self.commands.thrown_by = None;
        }
        self.status.interaction = Interaction::Idle;
    }
    /// Fighter_ChangeMotionState (fighter.c:950-1189): outgoing effects, scalar
    /// resets, row installation and pose setup, before the animated-shield hook.
    fn reset_motion(
        &mut self,
        row: MotionState,
        assets: &FighterAssets,
        move_id: Option<melee_types::combat::StaleMove>,
        change: MotionChange<'_>,
    ) -> bool {
        let state = row.id;
        self.apply_dynamic_commands(assets);
        self.flush_effects_on_motion_change();
        self.shield.clear_collision();
        self.status.unconditional_top_exit = false; // fighter.c:1075
        self.combat.armor = 0.0;
        self.status.ignore_fighter_nudge = false;
        self.combat.combo.grace = assets.combo.grace_frames;
        self.status.on_ledge = false;
        self.status.grab_exclusions = ledge::GrabExclusions::NONE;
        if !change.ground_air {
            self.commands.articles_visible = true;
            self.commands.fighter_hidden = false;
        }
        self.commands.allow_interrupt = false;
        if !change.preserve.hitboxes {
            self.commands.hitboxes.fill(None);
        }
        self.commands.first_hit_stale_penalty = None;
        self.combat.stale.enter(move_id);
        self.commands.stale_multiplier =
            move_id.map(|_| self.combat.stale.multiplier(&assets.stale_weights));
        if !change.preserve.hit_status {
            self.commands.hurt_status = melee_types::combat::HurtStatus::Normal;
            self.commands.capsule_status = melee_types::combat::HurtStatus::Normal;
        }
        self.commands.capsule_overrides.clear();
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
        if self.effect_state.destroy_on_state_change && !change.preserve.effects {
            self.effect_state.destroy_on_state_change = false;
            self.effects
                .push(melee_ef::request::EffectRequest::DestroyOwned);
        }
        super::commands::reset_parts(&mut self.animation, &mut self.skeleton, assets);
        if matches!(
            state,
            CommonMotionState::Entry | CommonMotionState::EntryEnd
        ) {
            self.motion_state = row;
            self.animation.clear_motion(&mut self.skeleton);
            self.commands.instruction = None;
            return false;
        }
        if matches!(
            state,
            CommonMotionState::DeadDown
                | CommonMotionState::DeadLeft
                | CommonMotionState::DeadRight
                | CommonMotionState::DeadUp
        ) {
            self.motion_state = row;
            self.animation.clear_motion(&mut self.skeleton);
            self.commands.instruction = None;
            self.animation.frame = -1.0;
            return false;
        }
        if !row.implemented {
            state::unsupported_action(row.action);
        }
        if state == CommonMotionState::Wait && self.physics.ground_or_air != GroundOrAir::Ground {
            unimplemented!("fighter.c:1190-1194: unsupported motion entry {state:?}");
        }
        let animation_id = row.animation;
        self.motion_state = row;
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
        true
    }
    /// Ft_MF_SkipAnim, fighter.c:1349-1361.
    fn clear_animation(&mut self) {
        self.animation.clear_motion(&mut self.skeleton);
        self.commands.instruction = None;
    }
    /// Fighter_ChangeMotionState (fighter.c:1268-1347): attach, evaluate main
    /// tracks, run commands, then finish common entry bookkeeping.
    fn start_motion_animation(
        &mut self,
        assets: &FighterAssets,
        animation_id: i32,
        state: CommonMotionState,
        change: MotionChange<'_>,
    ) -> Result<()> {
        let MotionChange {
            start,
            rate,
            source,
            ..
        } = change;
        let had_root_motion = self.animation.flags.contains(
            crate::anim::MotionFlags::ROOT_MOTION | crate::anim::MotionFlags::SECOND_ROOT,
        );
        // fighter.c:1274-1296: a ground/air switch loads the preceding frame first.
        let animation_start = if change.ground_air && start != 0.0 {
            start - rate
        } else {
            start
        };
        if let Some(source) = source {
            self.animation.set_animation_remapped(
                &mut self.skeleton,
                source.motion,
                animation_start,
                rate,
                Some(source.remap),
            )?;
        } else {
            self.animation.set_animation(
                &mut self.skeleton,
                &assets.motions[&animation_id],
                animation_start,
                rate,
            )?;
        }
        self.animation.set_rate(&mut self.skeleton, rate, false);
        self.animation.frame = start - rate;
        self.animation.remainder = 0.0;
        self.commands.restart(
            source
                .map_or(assets, |source| source.assets)
                .command_entries[&animation_id],
        );
        if change.ground_air && start != 0.0 {
            // fighter.c:1274-1296: evaluate the preceding pose, clear the
            // extracted delta, then evaluate the resumed frame. This supplies
            // the correct TransN velocity on the next physics pass.
            self.animation.frame -= rate;
            self.animation
                .advance_main::<RetailTrig>(&mut self.skeleton);
            if let Some(root) = &mut self.animation.root_motion {
                for history in [&mut root.primary_history, &mut root.secondary_history] {
                    history.previous = history.position;
                    history.offset = Vec3::ZERO;
                    history.previous_offset = Vec3::ZERO;
                }
            }
        }
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
        if change.ground_air {
            // fighter.c:1295, before ftAction_8007349C decrements by speed.
            self.commands.timer = -start;
            self.commands.advance_control(&self.animation, assets);
        } else if start != 0.0 {
            self.commands.seek(
                &mut self.animation,
                &mut self.skeleton,
                &mut self.ground_pose,
                assets,
            );
        } else {
            self.advance_damage_overlay(assets);
            self.commands.step(
                &mut self.animation,
                &mut self.skeleton,
                &mut self.ground_pose,
                assets,
            );
        }
        self.apply_dynamic_commands(assets);
        // Fighter_ChangeMotionState, fighter.c:1363-1368: leaving root motion
        // clamps gr_vel to dash speed. Retail tests the new b0 twice.
        if had_root_motion
            && !self
                .animation
                .flags
                .contains(crate::anim::MotionFlags::ROOT_MOTION)
        {
            let max = self.attributes.running.dash_max_velocity;
            self.physics.ground_velocity = self.physics.ground_velocity.clamp(-max, max);
        }
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
