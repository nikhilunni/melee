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
    /// Fighter_ChangeMotionState's explicit interpolation override; None uses asset data.
    blend_frames: Option<f32>,
    source: Option<super::grab_throw::ThrowSource<'a>>,
    /// Ft_MF_SkipItemVis (bit18), part of ftCommon_GroundAirColl_MF.
    ground_air: bool,
    /// Ft_MF_SkipModelPartVis (bit22), also part of ftCommon_GroundAirColl_MF.
    skip_model_part_visibility: bool,
    /// Ft_MF_SkipModel (bit4): the model selections survive (fighter.c:979).
    skip_model: bool,
    update_commands: bool,
    preserve_name_tag: bool,
    skip_animation: bool,
    /// Fighter_ChangeMotionState Ft_MF_SkipMatAnim (bit7).
    preserve_material_animation: bool,
    /// fn_800DE798: restore the release owner after reset, before initial commands.
    throw_owner: Option<u32>,
    preserve: MotionPreservation,
    /// Ft_MF_SkipAnimVel (bit5): keep velocity when a grounded root-motion
    /// animation starts mid-way (fighter.c:1317-1336).
    skip_animation_velocity: bool,
    /// Ft_MF_SkipHitStun (bit28): hitstun ownership survives the entry.
    keep_hitstun: bool,
    /// ftCo_Fall_Enter_YoshiEgg: Fall without ftCo_Fall_Enter's drift clamp.
    unclamped_fall: bool,
    /// Ft_MF_SkipColAnim (bit12): the secondary color slot (x488) survives.
    keep_secondary_color: bool,
    /// Ft_MF_SkipParasol (bit10): the parasol step (fighter.c:986-993) is skipped.
    skip_parasol: bool,
    /// Ft_MF_Unk06 (bit6): the entry does not start the KO credit's
    /// countdown (fighter.c:1184).
    skip_ko_credit_countdown: bool,
}

/// Fighter_ChangeMotionState's `flags` argument; bit names from ft/forward.h.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MotionEntryFlags(pub u32);
impl MotionEntryFlags {
    pub const KEEP_FAST_FALL: Self = Self(1 << 0);
    pub const KEEP_GFX: Self = Self(1 << 1);
    pub const KEEP_COL_ANIM_HIT_STATUS: Self = Self(1 << 2);
    pub const SKIP_HIT: Self = Self(1 << 3);
    pub const SKIP_MODEL: Self = Self(1 << 4);
    pub const SKIP_ANIM_VEL: Self = Self(1 << 5);
    pub const UNK06: Self = Self(1 << 6);
    pub const SKIP_MAT_ANIM: Self = Self(1 << 7);
    pub const SKIP_THROW_EXCEPTION: Self = Self(1 << 8);
    pub const SKIP_PARASOL: Self = Self(1 << 10);
    pub const SKIP_COL_ANIM: Self = Self(1 << 12);
    pub const KEEP_ACCESSORY: Self = Self(1 << 13);
    pub const UPDATE_CMD: Self = Self(1 << 14);
    pub const SKIP_NAMETAG_VIS: Self = Self(1 << 15);
    pub const KEEP_COL_ANIM_PART_HIT_STATUS: Self = Self(1 << 16);
    pub const KEEP_SWORD_TRAIL: Self = Self(1 << 17);
    pub const SKIP_ITEM_VIS: Self = Self(1 << 18);
    pub const FREEZE_STATE: Self = Self(1 << 21);
    pub const SKIP_MODEL_PART_VIS: Self = Self(1 << 22);
    pub const SKIP_HITSTUN: Self = Self(1 << 28);
    pub const SKIP_ANIM: Self = Self(1 << 29);
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

/// Retail motion-entry flags retained across a ground/air counterpart change.
#[derive(Clone, Copy, Default)]
pub struct MotionPreservation {
    pub hit_status: bool,
    pub hitboxes: bool,
    pub effects: bool,
    /// Ft_MF_KeepFastFall (bit 0).
    pub fast_fall: bool,
}

/// Spawn inputs read from Player, pl/player.c:228-240 and fighter.c:688-739.
#[derive(Clone, Debug)]
pub struct PlayerSlot {
    pub id: u8,
    /// x221F_b4 (plAllocInfo.b0): the player's second fighter, such as
    /// Nana beside Popo (Player_80031AD0's `player_entity[1]`).
    pub secondary: bool,
    pub control: PlayerKind,
    pub costume: u8,
    pub stocks: u8,
    /// Player_GetFallsByIndex (StaticPlayer +68, one count per fighter):
    /// the stocks this fighter lost (ftCo_800D34E0).
    pub falls: u32,
    pub position: Vec3,
    pub facing: f32,
    /// Player_GetModelScale; independent of co_attrs.model_scaling.
    pub scale: f32,
    /// Player_GetDamage, used by reset (+1830).
    pub damage: f32,
    pub cpu_mode: i32,
    pub cpu_level: i32,
}

/// Fighter_Create's first motion (fighter.c:914-930).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FirstMotion {
    /// ftCommon_8007D92C: Wait on a floor, Fall otherwise.
    Ordinary,
    /// Player's entry flag: Entry after `delay` frames (ftCo_800C61B0).
    Entry(i32),
    /// A transformation partner (ftCo_800BFD04).
    Sleep,
}

/// Fighter_NewSpawn_80068E40: wrapping counter never produces zero after wrap.
#[derive(Debug, Clone)]
pub struct SpawnCounter(pub u32);

/// Scene-owned spawn services; the RNG and counter are shared by both players.
pub struct SpawnContext<'a> {
    pub map: &'a mut CollMap,
    pub stage_camera: &'a melee_cm::StageCamera,
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
        Self::create(
            player,
            character,
            assets,
            skeleton,
            root,
            context,
            FirstMotion::Ordinary,
        )
    }

    /// Fighter_Create for Player_80031AD0's second fighter with
    /// `has_transformation` (fighter.c:918-919): after reset, the other form
    /// of a transforming character sleeps (ftCo_800BFD04) instead of
    /// choosing Wait, Fall or Entry.
    pub fn spawn_asleep(
        player: PlayerSlot,
        character: CharacterState,
        assets: &FighterAssets,
        skeleton: JObjTree,
        root: JObjId,
        context: SpawnContext<'_>,
    ) -> Result<Self> {
        Self::create(
            player,
            character,
            assets,
            skeleton,
            root,
            context,
            FirstMotion::Sleep,
        )
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
            FirstMotion::Entry(delay),
        )
    }

    fn create(
        player: PlayerSlot,
        character: CharacterState,
        assets: &FighterAssets,
        skeleton: JObjTree,
        root: JObjId,
        context: SpawnContext<'_>,
        first: FirstMotion,
    ) -> Result<Self> {
        let initial_scale = skeleton.scale(root);
        let mut fighter = Self::prepare(player, character, assets, skeleton, root, context.map);
        fighter.initialize_spawn(assets, context, first, initial_scale)?;
        Ok(fighter)
    }

    /// Reset-time services shared by creation and revival; all owners already exist.
    pub(super) fn initialize_spawn(
        &mut self,
        assets: &FighterAssets,
        context: SpawnContext<'_>,
        first: FirstMotion,
        initial_scale: Vec3,
    ) -> Result<()> {
        // Fighter_Create (80069018): initialize chains once, before the shared
        // Fighter_UnkProcessDeath services. Revival retains the live locks/springs.
        for set in &mut self.core.dynamics {
            crate::dynamics::select(
                set,
                &mut self.core.skeleton,
                &mut self.core.animation.parts,
                true,
                0,
            );
        }
        self.core.dynamics_first_bone.fill(0);
        let root = self.core.animation.root;
        let supported = self.reset_spawn_services(assets, context, initial_scale);
        match first {
            FirstMotion::Entry(delay) => {
                // Fighter_ChangeMotionState sets TopN's facing rotation even
                // for SM_None. The ordinary animation-entry path does this itself.
                self.core.skeleton.set_rotation_y(
                    root,
                    (std::f64::consts::FRAC_PI_2 * f64::from(self.core.physics.facing)) as f32,
                );
                self.enter_match(delay, assets)?;
            }
            FirstMotion::Sleep => self.enter_sleep(assets)?,
            FirstMotion::Ordinary => {
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
        }
        // ftLib_800867E8 at the end of Fighter_Create: clear input and freeze
        // sampling until match setup calls ftLib_8008688C.
        self.core.input.clear_current_and_buffers();
        self.core.status.input_frozen = true;
        Ok(())
    }

    /// Fighter_UnkProcessDeath (80068354): retain animation parts and spring
    /// state. Fighter_Create alone chooses an initial motion after this call;
    /// ftCo_800D4FF4 enters Rebirth directly.
    pub(super) fn reset_spawn_services(
        &mut self,
        assets: &FighterAssets,
        context: SpawnContext<'_>,
        initial_scale: Vec3,
    ) -> bool {
        let SpawnContext {
            map,
            stage_camera,
            rng,
            counter,
        } = context;
        let supported = self
            .core
            .initialize_spawn_geometry(map, counter, initial_scale);
        self.core.reset_camera_subject(assets, stage_camera);
        self.character.on_reset();
        if let Some(color) = (self.character.table().color_fallback_after_reset)(&self.character) {
            self.core.combat.secondary_color_fallback = Some(color);
        }
        // Fighter_UnkProcessDeath, fighter.c:561: always initialize this capsule.
        self.core.thrown_hitbox.state = 1;
        self.core
            .thrown_hitbox
            .update(&mut self.core.skeleton, &self.core.animation);
        let position = self.core.physics.position;
        // ftCo_800A0FB0 (0x800A0FB0): no stage the port runs ignores a floor
        // line (ftCo_800A1B38 names Big Blue, Mushroom Kingdom, Corneria
        // and Venom).
        let floor_below = map
            .check_floor(
                position.x,
                10.0 + position.y,
                position.x,
                position.y - 1000.0,
                0.0,
                -1,
                -1,
                -1,
                None,
            )
            .map(|hit| hit.pos);
        let attributes = &self.core.attributes;
        self.core.cpu = CpuState::initialize(
            &super::cpu::CpuSetup {
                mode: self.core.player.cpu_mode,
                level: self.core.player.cpu_level,
                partner: self.core.capabilities.cpu_partner,
                position,
                gravity: attributes.air.gravity,
                jump_velocity: attributes.jumping.jump_v_initial_velocity,
                air_jump_multiplier: attributes.jumping.air_jump_v_multiplier,
                floor_below,
            },
            rng,
        );
        supported
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
    /// Fighter_ChangeMotionState (0x800693AC) at frame zero. Entering Wait
    /// is ft_8008A348's: with an item in hand the kind's item idle follows.
    pub fn change_motion_state(&mut self, state: ActionId, assets: &FighterAssets) -> Result<()> {
        if state == CommonMotionState::Wait.into() {
            // ft_8008A348, ft_08A1.c:85-91: the kind's article leaves the
            // hand before the motion change (Peach's parasol, it_802BDB94).
            (self.character.table().wait_articles)(self);
        }
        self.change_motion_state_at(state, assets, 0.0)?;
        if state == CommonMotionState::Wait.into() {
            self.core.play_wait_holding_idle(assets)?;
        }
        if state == CommonMotionState::Wait.into() || state == CommonMotionState::SquatWait.into() {
            (self.character.table().wait_entered)(self);
        }
        Ok(())
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

    /// Fighter_ChangeMotionState from frame zero with an explicit blend
    /// (ftCo_800CEFE0's 10-frame parasol opening).
    pub fn change_motion_state_blended(
        &mut self,
        state: ActionId,
        assets: &FighterAssets,
        blend_frames: f32,
    ) -> Result<()> {
        self.change_motion_state_with_options(
            state,
            assets,
            MotionChange {
                rate: 1.0,
                blend_frames: Some(blend_frames),
                ..Default::default()
            },
        )
    }

    /// ftCo_800CF3C8 / ftCo_800CF280: Ft_MF_SkipHit | Ft_MF_SkipParasol.
    pub(super) fn change_parasol_fall_motion(
        &mut self,
        state: ActionId,
        assets: &FighterAssets,
    ) -> Result<()> {
        self.change_motion_state_with_options(
            state,
            assets,
            MotionChange {
                rate: 1.0,
                skip_parasol: true,
                preserve: MotionPreservation {
                    hitboxes: true,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
    }

    /// Fighter_ChangeMotionState with Ft_MF_KeepGfx only: owned effects
    /// survive the entry (ftPe_UpdateFloatDir, 8011BD6C).
    pub fn change_motion_state_keeping_effects(
        &mut self,
        state: ActionId,
        assets: &FighterAssets,
        start: f32,
    ) -> Result<()> {
        self.change_motion_state_with_options(
            state,
            assets,
            MotionChange {
                start,
                rate: 1.0,
                preserve: MotionPreservation {
                    effects: true,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
    }

    /// Fighter_ChangeMotionState with Ft_MF_KeepGfx (owned effects survive)
    /// and, when `keep_material`, Ft_MF_SkipMatAnim; SkipModel/SkipColAnim
    /// have no further port owner. Egg Lay's tongue and swallow entries.
    pub fn change_motion_state_keeping_graphics(
        &mut self,
        state: ActionId,
        assets: &FighterAssets,
        start: f32,
        keep_material: bool,
    ) -> Result<()> {
        self.change_motion_state_with_options(
            state,
            assets,
            MotionChange {
                start,
                rate: 1.0,
                preserve_material_animation: keep_material,
                preserve: MotionPreservation {
                    effects: true,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
    }

    /// ftCo_800BBED4's Fighter_ChangeMotionState(YoshiEgg, Ft_MF_Unk06, 0,
    /// 1, 0, yoshi): the egg plays Yoshi's animation and script, remapped
    /// like a thrown fighter's, with Yoshi as the throw owner.
    pub(super) fn enter_borrowed_egg_motion(
        &mut self,
        yoshi: u32,
        assets: &FighterAssets,
        yoshi_assets: &FighterAssets,
    ) -> Result<()> {
        let motion = &yoshi_assets.motions[&super::capture_yoshi::EGG_MOTION];
        let remap = motion.remap.as_ref().expect("prepared egg skeleton");
        let source = super::grab_throw::ThrowSource {
            assets: yoshi_assets,
            animation: Some((
                motion,
                crate::anim::attach::MotionRemapView {
                    source: &remap.source,
                    destination: &assets.parts,
                    source_masks: &remap.source_masks,
                },
            )),
            flags: motion.flags,
            blend_frames: motion.blend_frames,
        };
        self.commands.thrown_by = Some(yoshi);
        self.change_motion_state_with_source(
            CommonMotionState::YoshiEgg.into(),
            assets,
            0.0,
            1.0,
            Some(source),
        )
    }

    /// ftCo_Fall_Enter_YoshiEgg (800CC830): Fall with Ft_MF_Unk06 and blend
    /// -1 (none), without ftCo_Fall_Enter's drift clamp or kept fast fall.
    pub(super) fn enter_fall_from_egg(&mut self, assets: &FighterAssets) -> Result<()> {
        self.change_motion_state_with_options(
            CommonMotionState::Fall.into(),
            assets,
            MotionChange {
                rate: 1.0,
                blend_frames: Some(0.0),
                unclamped_fall: true,
                ..Default::default()
            },
        )?;
        self.physics.fast_fall = false;
        Ok(())
    }

    /// Fighter_ChangeMotionState with Ft_MF_SkipAnimVel: a mid-animation
    /// grounded start keeps the fighter's velocity (ftCo_TurnRun_Enter).
    pub(super) fn change_motion_state_keeping_velocity(
        &mut self,
        state: ActionId,
        assets: &FighterAssets,
        start: f32,
    ) -> Result<()> {
        self.change_motion_state_with_options(
            state,
            assets,
            MotionChange {
                start,
                rate: 1.0,
                skip_animation_velocity: true,
                ..Default::default()
            },
        )
    }

    /// Fighter_ChangeMotionState (800693AC), fighter.c:1268-1296:
    /// install the caller's rate before frame-zero animation and commands.
    pub fn change_motion_state_with_rate(
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

    /// ftCo_80098D90: 0x10D4, start0/rate1/default blend.
    /// KeepColAnimHitStatus preserves body and global capsule status, not
    /// per-capsule overrides. SkipModel/SkipMatAnim/SkipColAnim retain their
    /// owners: retain model selections, texture animation and the modeled
    /// primary color bank. Unk06 has no entry consumer.
    /// This is not a ground/air counterpart: ordinary visibility and commands reset.
    pub(super) fn change_shield_break_fall(&mut self, assets: &FighterAssets) -> Result<()> {
        self.change_shield_break_motion(CommonMotionState::ShieldBreakFall, assets)
    }

    /// ftCo_80098E3C / ftCo_80098F3C use the same preservation, flags 0x1094.
    pub(super) fn change_shield_break_motion(
        &mut self,
        state: CommonMotionState,
        assets: &FighterAssets,
    ) -> Result<()> {
        self.change_motion_state_with_options(
            state.into(),
            assets,
            MotionChange {
                rate: 1.0,
                preserve_material_animation: true,
                keep_secondary_color: true,
                preserve: MotionPreservation {
                    hit_status: true,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
    }

    /// ftCo_800D4FF4 (800D50D0..50E8): KeepGfx, SkipColAnim and anim_blend=-1.
    /// The -1 caller sentinel means immediate animation, not the asset blend.
    pub(super) fn change_revival_motion(&mut self, assets: &FighterAssets) -> Result<()> {
        self.change_motion_state_with_options(
            CommonMotionState::Rebirth.into(),
            assets,
            MotionChange {
                start: 0.0,
                rate: 1.0,
                blend_frames: Some(0.0),
                keep_secondary_color: true,
                preserve: MotionPreservation {
                    effects: true,
                    ..Default::default()
                },
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

    /// Fighter_ChangeMotionState with UpdateCmd (0x4000), optionally SkipColAnim.
    /// Control-flow seek without replaying effects/hitboxes; ordinary visibility resets.
    /// Secondary-bank playback is not modeled; neither policy clears the modeled
    /// primary bank (revival, electric damage and powershield flash).
    pub fn change_motion_with_updated_commands(
        &mut self,
        state: ActionId,
        assets: &FighterAssets,
        start: f32,
        color: MotionColorPolicy,
    ) -> Result<()> {
        self.change_motion_state_with_options(
            state,
            assets,
            MotionChange {
                start,
                rate: 1.0,
                keep_secondary_color: matches!(color, MotionColorPolicy::Preserve),
                update_commands: true,
                ..Default::default()
            },
        )
    }

    /// ftCo_8009388C: SkipAnim | KeepGfx resumes the incoming animation clock.
    pub(super) fn change_motion_without_animation(
        &mut self,
        state: ActionId,
        assets: &FighterAssets,
    ) -> Result<()> {
        self.change_motion_state_with_options(
            state,
            assets,
            MotionChange {
                start: self.animation.frame,
                rate: 1.0,
                skip_animation: true,
                preserve: MotionPreservation {
                    effects: true,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
    }

    /// Fighter_ChangeMotionState with Ft_MF_SkipAnim from frame zero: the
    /// motion has no animation of its own (cur_anim_frame stays -1), as in
    /// the GuardOn and GuardReflect entries (ftCo_800924C0 / ftCo_80093A50).
    pub(super) fn change_motion_skipping_animation(
        &mut self,
        state: ActionId,
        assets: &FighterAssets,
    ) -> Result<()> {
        self.change_motion_state_with_options(
            state,
            assets,
            MotionChange {
                rate: 1.0,
                skip_animation: true,
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

    /// ftCo_80090780 (80090780): DamageFall entry with 0x18001. KeepFastFall
    /// and SkipNametagVis are independent of ground/air command preservation.
    /// KeepColAnimPartHitStatus leaves timed ledge status untouched; it does
    /// not preserve ordinary scripted hurt status/capsule overrides.
    pub(super) fn enter_damage_fall(&mut self, assets: &FighterAssets) -> Result<()> {
        if self.physics.ground_or_air == GroundOrAir::Ground {
            self.leave_ground();
        }
        self.change_motion_state_with_options(
            CommonMotionState::DamageFall.into(),
            assets,
            MotionChange {
                rate: 1.0,
                preserve_name_tag: true,
                // ftCo_80090780: flags 0x18001, KeepFastFall | SkipNametagVis |
                // KeepColAnimPartHitStatus; the effects are not kept.
                preserve: MotionPreservation {
                    fast_fall: true,
                    ..Default::default()
                },
                ..Default::default()
            },
        )?;
        // ftCommon_ClampAirDrift (8007D468): clamp only X, no fused sites.
        let maximum = self.attributes.air.air_drift_max;
        self.physics.self_velocity.x = self.physics.self_velocity.x.clamp(-maximum, maximum);
        self.commands
            .rumble_requests
            .push(super::commands::RumbleRequest {
                all_players: false,
                id: 8,
                duration: 0,
            });
        Ok(())
    }
    /// [`Self::change_ground_air_motion`] into a row that plays another
    /// fighter's animation (ftCo_800BCE64: a thrown row's counterpart with
    /// the captor as the animation source).
    pub(super) fn change_ground_air_motion_with_source(
        &mut self,
        state: ActionId,
        assets: &FighterAssets,
        source: super::grab_throw::ThrowSource<'_>,
    ) -> Result<()> {
        self.change_motion_state_with_options(
            state,
            assets,
            MotionChange {
                start: self.animation.frame,
                rate: 1.0,
                source: Some(source),
                ground_air: true,
                skip_model_part_visibility: true,
                update_commands: true,
                preserve_material_animation: true,
                keep_secondary_color: true,
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
                skip_model_part_visibility: true,
                update_commands: true,
                preserve_material_animation: true,
                keep_secondary_color: true,
                preserve,
                ..Default::default()
            },
        )
    }

    /// Fighter_ChangeMotionState (0x800693AC) with a retail `flags` word,
    /// start frame and rate, for character entries whose flag combinations
    /// have no named helper. Bits without a simulated consumer (model,
    /// sound, rumble and statistics owners) are accepted; bits whose
    /// consumer the port does not model name themselves.
    pub fn change_motion_state_with_flags(
        &mut self,
        state: ActionId,
        assets: &FighterAssets,
        flags: MotionEntryFlags,
        start: f32,
        rate: f32,
    ) -> Result<()> {
        use MotionEntryFlags as F;
        for (flag, line) in [
            (F::SKIP_THROW_EXCEPTION, "fighter.c:962"),
            (F::KEEP_ACCESSORY, "fighter.c:1113"),
            (F::KEEP_COL_ANIM_PART_HIT_STATUS, "fighter.c:1007"),
            (F::KEEP_SWORD_TRAIL, "fighter.c:1152"),
            (F::FREEZE_STATE, "fighter.c:1239"),
        ] {
            if flags.contains(flag) {
                unimplemented!("Fighter_ChangeMotionState flags {:#X}: {line}", flags.0);
            }
        }
        // fighter.c:1087-1095: item and article visibility are separate bits.
        let item = flags.contains(F::SKIP_ITEM_VIS);
        self.change_motion_state_with_options(
            state,
            assets,
            MotionChange {
                start,
                rate,
                ground_air: item,
                skip_model_part_visibility: flags.contains(F::SKIP_MODEL_PART_VIS),
                skip_model: flags.contains(F::SKIP_MODEL),
                update_commands: flags.contains(F::UPDATE_CMD),
                preserve_name_tag: flags.contains(F::SKIP_NAMETAG_VIS),
                skip_animation: flags.contains(F::SKIP_ANIM),
                preserve_material_animation: flags.contains(F::SKIP_MAT_ANIM),
                skip_animation_velocity: flags.contains(F::SKIP_ANIM_VEL),
                keep_hitstun: flags.contains(F::SKIP_HITSTUN),
                keep_secondary_color: flags.contains(F::SKIP_COL_ANIM),
                skip_parasol: flags.contains(F::SKIP_PARASOL),
                skip_ko_credit_countdown: flags.contains(F::UNK06),
                preserve: MotionPreservation {
                    hit_status: flags.contains(F::KEEP_COL_ANIM_HIT_STATUS),
                    hitboxes: flags.contains(F::SKIP_HIT),
                    effects: flags.contains(F::KEEP_GFX),
                    fast_fall: flags.contains(F::KEEP_FAST_FALL),
                },
                ..Default::default()
            },
        )
    }

    /// ftCo_800C18A8's entry: Ft_MF_Unk06 | SkipNametagVis |
    /// KeepColAnimPartHitStatus | SkipHitStun, frame 0, rate 1, no blend.
    pub(super) fn change_fly_reflect_motion(
        &mut self,
        state: CommonMotionState,
        assets: &FighterAssets,
    ) -> Result<()> {
        self.change_reflect_motion(state, assets, true)
    }

    /// fn_800C7DC4's entry (retail 0x800C7F34): flags 0x18040, the same
    /// without Ft_MF_SkipHitStun.
    pub(super) fn change_down_reflect_motion(&mut self, assets: &FighterAssets) -> Result<()> {
        self.change_reflect_motion(CommonMotionState::DownReflect, assets, false)
    }

    fn change_reflect_motion(
        &mut self,
        state: CommonMotionState,
        assets: &FighterAssets,
        keep_hitstun: bool,
    ) -> Result<()> {
        self.change_motion_state_with_options(
            state.into(),
            assets,
            MotionChange {
                rate: 1.0,
                blend_frames: Some(0.0),
                preserve_name_tag: true,
                keep_hitstun,
                ..Default::default()
            },
        )
    }

    /// A ground/air counterpart change at the current frame and `rate`.
    pub fn change_ground_air_motion_at_rate(
        &mut self,
        state: ActionId,
        assets: &FighterAssets,
        rate: f32,
    ) -> Result<()> {
        self.change_motion_state_with_options(
            state,
            assets,
            MotionChange {
                start: self.animation.frame,
                rate,
                ground_air: true,
                skip_model_part_visibility: true,
                update_commands: true,
                preserve_material_animation: true,
                keep_secondary_color: true,
                ..Default::default()
            },
        )
    }

    /// Fighter_ChangeMotionState at the current frame with Ft_MF_SkipColAnim |
    /// Ft_MF_UpdateCmd only (ftFx_MF_SpecialLwEnd_Coll): commands advance
    /// without re-running, but visibility, material animation and owned
    /// effects reset as on an ordinary entry.
    pub fn change_motion_state_updating_commands(
        &mut self,
        state: ActionId,
        assets: &FighterAssets,
    ) -> Result<()> {
        self.change_motion_state_with_options(
            state,
            assets,
            MotionChange {
                start: self.animation.frame,
                rate: 1.0,
                update_commands: true,
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
        // fighter.c:1142-1144: only Wait and the walks keep the jab window.
        use CommonMotionState as C;
        let keeps_jab: [ActionId; 4] = [
            C::Wait.into(),
            C::WalkSlow.into(),
            C::WalkMiddle.into(),
            C::WalkFast.into(),
        ];
        if !keeps_jab.contains(&state) {
            self.core.jab_countdown = 0.0;
        }
        let special = (usize::from(row.action.0) >= super::COMMON_COUNT)
            .then(|| self.character.table().specials_keep_held_item);
        let state = row.id;
        self.core.require_held_item_state(state, special);
        if !change.skip_parasol {
            self.parasol_motion_change();
        }
        self.character.on_motion_change();
        self.core.begin_motion_change(source);
        // The port's attack-proc guard belongs to the state that set it (an
        // attack entry, or hitlag ending during an attack); attack entries
        // set it again after this change.
        if self.core.status.interaction == super::Interaction::Attack {
            self.core.status.interaction = super::Interaction::Idle;
        }
        if self.core.physics.ground_or_air == GroundOrAir::Ground {
            self.character.on_grounded_motion();
            // fighter.c:1138.
            self.core.status.used_tether = false;
            self.core.parasol.restore_on_ground();
            // fighter.c:1183-1192: a neutral grounded motion (x9_b1) starts
            // the KO credit's countdown.
            let counts = if usize::from(row.action.0) < super::COMMON_COUNT {
                super::ko_source::starts_countdown(state)
            } else {
                self.character
                    .table()
                    .ko_countdown_rows
                    .contains(&row.action.0)
            };
            if !change.skip_ko_credit_countdown && counts {
                self.core
                    .combat
                    .ko_source
                    .start_countdown(assets.life.ko_credit_frames);
            }
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
        if row.animation < 0
            || change.skip_animation
            || state == CommonMotionState::Guard
            || (!self.character.animated_shield()
                && matches!(
                    state,
                    CommonMotionState::GuardOn | CommonMotionState::GuardReflect
                ))
        {
            // Ft_MF_SkipAnim, fighter.c:1349-1361: clear AObjs and script.
            let had_root_motion = self.core.animation.flags.contains(
                crate::anim::MotionFlags::ROOT_MOTION | crate::anim::MotionFlags::SECOND_ROOT,
            );
            self.core.clear_animation();
            // Fighter_ChangeMotionState: the no-animation branch (anim_id -1,
            // or Ft_MF_SkipAnim, which leaves x594 clear) still reaches the
            // outgoing root-motion clamp (fighter.c:1363-1368).
            if had_root_motion {
                let max = self.core.attributes.running.dash_max_velocity;
                self.core.physics.ground_velocity =
                    self.core.physics.ground_velocity.clamp(-max, max);
            }
            // Fighter_ChangeMotionState80069C0C: fsubs, also for SkipAnim.
            self.core.animation.frame = change.start - change.rate;
            return Ok(());
        }
        self.core
            .start_motion_animation(assets, row.animation, state, change)
    }
}

/// SkipColAnim controls secondary bank x488, never primary bank x408.
#[derive(Clone, Copy, Debug)]
pub enum MotionColorPolicy {
    ResetSecondary,
    Preserve,
}

impl FighterCore {
    /// Reset support probe and model placement before the death hook.
    fn initialize_spawn_geometry(
        &mut self,
        map: &mut CollMap,
        counter: &mut SpawnCounter,
        initial_scale: Vec3,
    ) -> bool {
        let root = self.animation.root;
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
        self.update_model_scale();
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
        // fighter.c:241, retail 0x80067CE8: fmadds. OnLoad set x40 before
        // Fighter_UnkProcessDeath (zero unless the kind sets it).
        let offset = capabilities.spawn_offset * player.scale; // ftCommon_800804EC: fmuls.
        let position = Vec3::new(
            gekko_math::fma::fmadds(player.facing, offset, player.position.x),
            player.position.y,
            player.position.z,
        );
        let mut physics = FighterPhysics::standing(position, player.facing);
        physics.percent = player.damage;
        let mut animation = FighterAnimation::new(&skeleton, root);
        // ftParts_8007506C: each conditional part's mask bit. A part OnLoad
        // grafted (ftParts_800753D4) is also flags_b2; the costume model
        // numbers its own joints around it.
        for (bit, conditional) in assets.conditional_parts.iter().enumerate() {
            if let Some(part) = animation.parts.get_mut(usize::from(conditional.part)) {
                part.motion_mask = 1 << bit;
                part.flags.0 |= PartFlags::CONDITIONAL;
            }
        }
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
            partner: None,
            spawn_number: 0,
            physics,
            animation,
            input: FighterInput::default(),
            collision: EnvironmentCollision::new(data, map.grkind()),
            attributes: assets.attributes.clone(),
            bones: assets.bones.clone(),
            skeleton,
            motion_state,
            state_data: MotionData::None,
            combat: super::damage::CombatState {
                capture_geometry,
                stale: super::attack::stale::StaleHistory::for_fighter(player.secondary),
                ..Default::default()
            },
            shield: super::shield::ShieldState::default(),
            effect_state: super::effects::FighterEffects::default(),
            effects: melee_ef::request::EffectQueue::default(),
            item_requests: Default::default(),
            effects_after_items: Default::default(),
            capabilities,
            cpu: CpuState::prepared(player.cpu_mode, player.cpu_level),
            status: Status::reset(assets.shield_health),
            commands: commands::CommandState::default(),
            ground_pose: GroundPoseFlags::default(),
            dynamics_first_bone: vec![0x100; assets.bones.dynamics_roots.len()],
            stage_wind: Default::default(),
            player_position: position,
            player_facing: player.facing,
            joystick_count: 0,
            previous_collision_bounds: Vec3::ZERO,
            camera: melee_cm::Subject::default(),
            offscreen: Offscreen::default(),
            quake_request: None,
            fell: false,
            fall_credit: None,
            fatal_action: super::ActionId(0),
            released_link: None,
            held_item: None,
            grafted_part: None,
            holds_by_graft: false,
            tether_article: false,
            transformation_requested: false,
            parasol: Default::default(),
            pending_forward_smash: false,
            article_in_hand: None,
            stowed_item: None,
            pickup_pose_pending: false,
            swing_hand_armed: false,
            drop_pose_pending: false,
            pickup_candidates: Default::default(),
            owned_article: None,
            partner_position: None,
            nearest_fighter: None,
            ledge_holders: Default::default(),
            accessory4_armed: false,
            item_catch_locked: false,
            catch_window: 0,
            jab_countdown: 0.0,
            last_jab: None,
            hurtboxes: assets.hurtboxes.clone(),
            hurtboxes_replaced: false,
            dynamic_colliders: assets.dynamic_colliders.clone(),
            thrown_hitbox: assets.thrown_hitbox.clone(),
            grab_handicap: 9, // gm default handicap, before any saved-player override.
            standing_rank: 0,
            player,
        }
    }
    /// Fighter_ChangeMotionState (fighter.c:933-949), before OnGroundedMotion.
    fn begin_motion_change(&mut self, source: Option<super::grab_throw::ThrowSource<'_>>) {
        self.status.require_supported();
        // fighter.c:948.
        self.physics.entry_facing = self.physics.facing;
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
        // fighter.c:983 / ftAnim_80070654: reset costume texture requests before
        // the new animation and its frame-zero commands can install overrides.
        if !change.preserve_material_animation {
            self.commands.reset_texture_animation();
        }
        self.shield.clear_collision();
        self.combat.reflector_enabled = false;
        // fighter.c:1057: x2218_b6.
        self.combat.absorb.active = false;
        self.cancel_cape_turn();
        self.clear_cape_turn_end();
        self.effect_state.hitlag_callbacks = false;
        self.effect_state.article_hitlag = None;
        // fighter.c:1381-1383: hitlag_cb / post_hitlag_cb reset with the row.
        self.combat.hitlag_callbacks = super::damage::HitlagCallbacks::None;
        self.status.unconditional_top_exit = false; // fighter.c:1075
        self.combat.armor = 0.0;
        self.status.ignore_fighter_nudge = false;
        // fighter.c:1069, 1085: x2220_b3 and x2224_b4.
        self.status.no_hit_reaction = false;
        self.status.buried = false;
        // Clearing ownership does not call OnKnockbackExit on interruption.
        if self.status.in_hitstun && !change.keep_hitstun {
            self.status.in_hitstun = false;
            self.combat.combo.grace = assets.combo.grace_frames;
        }
        self.status.on_ledge = false;
        // fighter.c:1038: every entry re-enables the procs (x221F_b3), so a
        // transformation partner wakes when its arrival motion starts.
        self.status.disabled = false;
        // fighter.c:1063: every entry shows the fighter again.
        self.effect_state.invisible = false;
        self.catch_window = 0; // fighter.c:1072
                               // Fighter_ChangeMotionState, fighter.c1128: retained throughout air.
        if self.physics.ground_or_air == GroundOrAir::Ground {
            self.status.ledge_timed_out = false;
            self.item_catch_locked = false;
        }
        self.status.grab_exclusions = ledge::GrabExclusions::NONE;
        self.status.special_grab = None;
        // fighter.c:1063: fp->invisible.
        self.effect_state.invisible = false;
        // fighter.c:979-981: without Ft_MF_SkipModel the selections return
        // to x5F4_arr[i].prev (ftParts_80074A8C). The port keeps no
        // ftParts_80074A4C defaults: its one reader (Samus's bomb jump,
        // idx == 2) sees -1 and 0 alike.
        if !change.skip_model {
            self.commands.model_selections = Default::default();
        }
        // fighter.c:1093-1095: without Ft_MF_SkipModelPartVis the articles
        // show again (x221E_b4).
        if !change.skip_model_part_visibility {
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
            // fighter.c:966-969: a whole-body status returns to normal
            // through ftColl_8007B62C(gobj, 0), colour animation included.
            if self.commands.hurt_status != melee_types::combat::HurtStatus::Normal {
                self.set_body_hurt_status(melee_types::combat::HurtStatus::Normal);
            }
            self.commands.hurt_status = melee_types::combat::HurtStatus::Normal;
            self.commands.capsule_status = melee_types::combat::HurtStatus::Normal;
        }
        self.commands.capsule_overrides.clear();
        // fighter.c:975: whatever the flags, a replaced capsule table returns.
        if self.hurtboxes_replaced {
            self.restore_hurt_capsules(assets);
        }
        // fighter.c:1377: no supported entry passes Ft_MF_KeepAccessory.
        self.accessory4_armed = false;
        self.swing_hand_armed = false;
        // fighter.c:1101-1102: entries without Ft_MF_KeepFastFall clear it.
        if !change.preserve.fast_fall
            && !matches!(
                state,
                CommonMotionState::Fall | CommonMotionState::FallSpecial
            )
        {
            self.physics.fast_fall = false;
        }
        // fighter.c:1105-1107: ftCo_800C0134 unless Ft_MF_SkipColAnim.
        if !change.keep_secondary_color {
            self.clear_secondary_color_overlay(&assets.color_overlays);
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
        if !preserve_name_tag && !change.preserve_name_tag {
            self.status.name_tag_timer = 0;
        }
        if self.effect_state.destroy_on_state_change && !change.preserve.effects {
            self.effect_state.destroy_on_state_change = false;
            self.effects
                .push(melee_ef::request::EffectRequest::DestroyOwned);
        }
        super::commands::reset_parts(&mut self.animation, &mut self.skeleton, assets);
        // fighter.c:1087-1091: without Ft_MF_SkipItemVis (part of
        // ftCommon_GroundAirColl_MF) the held item shows again, leaving the
        // hand pose reset_parts just reinstalled; a counterpart change keeps a
        // hidden item hidden and releases the hand again (ftCommon_8007F578).
        if !change.ground_air {
            self.commands.held_item_hidden = false;
        } else if self.commands.held_item_hidden {
            if let super::commands::HeldItemHand::Light(slot) = self.held_item_hand(assets) {
                super::commands::remove_part_animation(
                    &mut self.animation,
                    &mut self.skeleton,
                    assets,
                    slot,
                );
            }
        }
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
        if animation_id >= 0 {
            // ftCo_8009E7B4 (ftdynamics.c:617-625): while the stage wind
            // blows, Marth and Roy leave every dynamic bone to the solver.
            let windy = super::assets::CommonBehavior::for_kind(assets.kind).stage_wind_dynamics
                && self.stage_wind.code() > 0;
            for (i, set) in self.dynamics.iter_mut().enumerate() {
                let first = if windy {
                    0
                } else {
                    assets.dynamics_motion_starts[&animation_id].starts[i]
                };
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
        let exit_joint = self.animation.flags.blend_exit_joint();
        let had_root_motion = self.animation.flags.contains(
            crate::anim::MotionFlags::ROOT_MOTION | crate::anim::MotionFlags::SECOND_ROOT,
        );
        // fighter.c:1266-1298: every nonzero motion entry loads the preceding pose first.
        let animation_start = if start != 0.0 { start - rate } else { start };
        let mut animated = true;
        if let Some(source) = source {
            if let Some((motion, remap)) = source.animation {
                self.animation.set_animation_remapped(
                    &mut self.skeleton,
                    motion,
                    animation_start,
                    rate,
                    Some(remap),
                    change.blend_frames,
                )?;
            } else {
                // fighter.c:1243-1289 with x590 NULL: ftAnim_8006EBE8 is
                // skipped; the row's id and flags change, the attached
                // AObjs (and their rates) keep playing.
                self.animation.motion_id = animation_id;
                self.animation.flags = source.flags;
                animated = false;
            }
        } else if let Some(&(flags, _)) = assets.unanimated.get(&animation_id) {
            // The fighter's own row without animation data (Yoshi's egg
            // shield stun): x590 NULL, as for a borrowed unanimated row.
            self.animation.motion_id = animation_id;
            self.animation.flags = flags;
            animated = false;
        } else {
            let motion = &assets.motions[&animation_id];
            self.animation.set_animation_remapped(
                &mut self.skeleton,
                motion,
                animation_start,
                rate,
                motion.remap.as_ref().map(|remap| remap.view()),
                change.blend_frames,
            )?;
        }
        if animated {
            self.animation.set_rate(&mut self.skeleton, rate, false);
        } else {
            self.animation.speed = rate; // frame_speed_mul only
        }
        self.animation.frame = start - rate;
        self.animation.remainder = 0.0;
        self.commands.restart(
            source
                .map_or(assets, |source| source.assets)
                .command_entries[&animation_id],
        );
        if start != 0.0 {
            // 80069D84/90: sample start-rate even on an ordinary walk tier entry.
            // This also advances the blend, independent of the eventual frame clock.
            self.animation
                .advance_main::<RetailTrig>(&mut self.skeleton);
            let flags = self.animation.flags;
            if let Some(root) = &mut self.animation.root_motion {
                for (flag, history) in [
                    (
                        crate::anim::MotionFlags::ROOT_MOTION,
                        &mut root.primary_history,
                    ),
                    (
                        crate::anim::MotionFlags::SECOND_ROOT,
                        &mut root.secondary_history,
                    ),
                ] {
                    // 80069D94..80069E10: only authored extraction owners reset.
                    if flags.contains(flag) {
                        history.previous = history.position;
                        history.offset = Vec3::ZERO;
                        history.previous_offset = Vec3::ZERO;
                    }
                }
            }
        }
        // Fighter_ChangeMotionState (0x800693AC), fighter.c:1298,1342-1347:
        // main animation then commands. Part blends run only in the ordinary
        // ftAnim_8006EBA4 tick (ftanim.c:380-385), not again on motion entry.
        self.animation
            .advance_main::<RetailTrig>(&mut self.skeleton);
        // Fighter_ChangeMotionState80069EE0..80069FAC: a departing motion
        // may request one joint to bypass the incoming animation blend.
        // fighter.c:1245-1250: a borrowed motion reads the thrower's table
        // (the victim may author no animation of its own for this id).
        let blend_frames = source.map_or_else(
            || match assets.motions.get(&animation_id) {
                Some(motion) => motion.blend_frames,
                None => assets.unanimated[&animation_id].1,
            },
            |source| source.blend_frames,
        );
        if exit_joint != 0 && blend_frames != 0.0 {
            let joint = self.animation.parts[usize::from(exit_joint)].joint;
            let pose = self.animation.blend_tree.get(joint);
            let translation = pose.translate;
            let rotation = pose.rotate;
            self.skeleton.set_translate(joint, &translation);
            self.skeleton.set_rotation(joint, &rotation);
            self.skeleton
                .clear_flags(joint, hsd_anim::jobj::JOBJ_USE_QUATERNION);
        }
        // fighter.c:1309-1336: a frame-zero entry discards the extracted
        // offset; a grounded mid-animation entry turns the offset of this
        // second sample into velocity unless Ft_MF_SkipAnimVel.
        let flags = self.animation.flags;
        let grounded = self.physics.ground_or_air == GroundOrAir::Ground;
        let facing = self.physics.facing;
        if let Some(root) = &mut self.animation.root_motion {
            for (flag, history) in [
                (
                    crate::anim::MotionFlags::ROOT_MOTION,
                    &mut root.primary_history,
                ),
                (
                    crate::anim::MotionFlags::SECOND_ROOT,
                    &mut root.secondary_history,
                ),
            ] {
                if !flags.contains(flag) {
                    continue;
                }
                if start == 0.0 {
                    history.previous = history.position;
                    history.offset = Vec3::ZERO;
                    history.previous_offset = Vec3::ZERO;
                } else if !change.skip_animation_velocity && grounded {
                    // fighter.c:1320 / 1336: fmuls, stored to self_vel.x and gr_vel.
                    let velocity = history.offset.z * facing;
                    self.physics.self_velocity.x = velocity;
                    self.physics.ground_velocity = velocity;
                }
            }
        }
        if change.update_commands {
            // fighter.c:1295, before ftAction_8007349C decrements by speed.
            self.commands.timer = -start;
            self.commands.advance_control(&self.animation, assets);
        } else if start != 0.0 {
            let hand = self.held_item_hand(assets);
            self.commands.seek(
                &mut self.animation,
                &mut self.skeleton,
                &mut self.ground_pose,
                assets,
                hand,
            );
        } else {
            self.advance_color_overlay(assets);
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
        }
        // A frame-zero set ground/air command takes effect inside the entry.
        self.apply_airborne_commands();
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
            if !change.unclamped_fall {
                let max = self.attributes.air.air_drift_max;
                self.physics.self_velocity.x = self.physics.self_velocity.x.clamp(-max, max);
            }
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
