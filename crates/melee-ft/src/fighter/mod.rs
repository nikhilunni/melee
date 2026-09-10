//! Fighter ownership and scheduler callbacks for grounded, item-free Wait.
//! Retail addresses and unsupported paths are documented at each entry point.
mod character;
pub use character::{CharacterState, CharacterTable};

pub mod air_dodge;
pub mod assets;
pub mod attack;
pub mod caches;
pub mod commands;
pub mod damage;
pub mod dash;
pub mod down;
mod dynamic_commands;
pub mod effects;
pub mod entry;
pub mod escape;
pub mod fall;
pub mod grab;
pub mod grab_escape;
pub mod grab_throw;
pub mod hitbox;
pub mod jump;
pub mod landing;
pub mod ledge;
pub mod life;
pub mod multi_jump;
pub mod overlap;
mod pass;
mod procs;
pub mod run;
pub mod shield;
mod shield_break;
pub mod smash;
mod snapshot;
mod spawn;
pub mod squat;
pub mod state;
pub mod teeter;
pub mod turn;
pub mod turn_run;
pub mod walk;

use crate::{
    anim::FighterAnimation,
    collision::{ground::EnvironmentCollision, pose::GroundPoseFlags},
    desc::{FighterAttributes, FighterBones},
    input::FighterInput,
    physics::FighterPhysics,
};
use hsd_anim::jobj::JObjTree;
use hsd_types::{Vec2, Vec3};
use melee_types::{FighterKind, PlayerKind};
pub use spawn::{PlayerSlot, SpawnContext, SpawnCounter};
pub use state::{
    common_table, interleaved_order, ActionId, FighterProc, MotionRow, MotionState, SpecialSlot,
    COMMON_COUNT,
};

/// Per-candidate character defense callback, before ordinary hurtbox contact.
pub type DefenseContact = fn(&mut Fighter, &mut Fighter, &assets::FighterAssets, usize) -> bool;
/// Deferred character defense reaction at Fighter_ProcessHit.
pub type DefenseHit = fn(&mut Fighter, &assets::FighterAssets);

/// Character-owned load/reset hooks (`ftData_OnLoad`/`ftData_OnDeath`).
/// Implementations live in ft-<character>; common fighter code never loads a
/// character crate. The implementation owns its typed special attributes.
pub trait CharacterCallbacks: Sized + Send + Sync + 'static {
    const TABLE: CharacterTable = CharacterTable::new::<Self>();

    fn table() -> &'static CharacterTable {
        &Self::TABLE
    }

    fn into_state(self) -> CharacterState {
        CharacterState::new(self)
    }

    /// ftCo_AttackAir.c: decideFighter. Link/Young Link and Game & Watch
    /// supply their character entry here when their aerials are ported.
    const ENTER_AERIAL: fn(&mut Fighter, &assets::FighterAssets) -> assets::Result<()> =
        attack::aerial::enter;

    /// Character-owned table (retail's per-kind MotionState table), indexed
    /// from action 341. Specials will form its bulk; existing multijumps and
    /// character shield states also live here.
    const SPECIAL_ROWS: &'static [MotionRow] = &[];
    /// ftData special-row move IDs, indexed from action 341.
    const SPECIAL_MOVES: &'static [Option<attack::stale::GroundMove>] = &[];

    fn special_rows() -> &'static [MotionRow] {
        Self::SPECIAL_ROWS
    }

    fn enter_special(
        _fighter: &mut Fighter,
        _slot: SpecialSlot,
        _airborne: bool,
        _assets: &assets::FighterAssets,
    ) {
        // retail: ftData_SpecialN[kind] etc.
    }

    /// Fighter_8006C80C: character-owned accessory4, after the deferred effect flush.
    fn accessory(_fighter: &mut Fighter, _assets: &assets::FighterAssets) {}
    fn item_muzzle(_fighter: &mut Fighter, _assets: &assets::FighterAssets) -> Option<(Vec3, f32)> {
        None
    }

    fn item_owner(fighter: &mut Fighter, _assets: &assets::FighterAssets) -> melee_it::ItemOwner {
        melee_it::ItemOwner {
            illusion: None,
            position: fighter.physics.position,
            facing: fighter.physics.facing,
            hold_position: fighter.physics.position,
            blaster_action: 9,
            remove_blaster: true,
        }
    }

    fn kind(&self) -> FighterKind;
    /// ftCo_AttackS4.c:145-166, decideFighter (8008C348): nonstandard entry.
    fn forward_smash_variant(&self) {
        if Self::descriptor().common_behavior.forward_smash_entry {
            unimplemented!("ftCo_AttackS4: character entry hook");
        }
    }
    /// ftCo_Catch.c / CatchPull.c: ordinary body grab by default. Tether and
    /// character-specific capture variants override this boundary.
    fn catch_variant(&mut self) {}
    /// ftCo_Throw.c:145-157,346-353: special capture and Fox laser callbacks.
    fn throw_variant(&self) {
        if Self::descriptor().common_behavior.throw_callback {
            unimplemented!("ftCo_Throw.c: character throw callback hook");
        }
    }

    /// ftColl candidate boundary: special defense may consume an eligible hit.
    const DEFENSE_CONTACT: Option<DefenseContact> = None;
    /// Fighter_ProcessHit: deferred special defense reaction, before hitlag.
    const PROCESS_DEFENSE_HIT: Option<DefenseHit> = None;

    /// Explicit boundary for a character-owned hurt-capsule layout.
    fn check_hurtbox_interaction(&self) {}
    /// ftCo_Attack1.c:89-110, decideAttack11 / getMotionFlags (8008AB84 / 8008ABC0).
    fn jab_variant(&self) {
        if Self::descriptor().common_behavior.jab_entry {
            unimplemented!("ftCo_Attack1.c:89-110: character jab entry hook");
        }
    }
    /// ftCo_Attack1 doAttack13 (8008B194): Marth restarts Attack11.
    fn third_jab_state(&self) -> melee_types::CommonMotionState {
        melee_types::CommonMotionState::Attack13
    }
    /// The character's on-disc resources (`ft<Char>_Init_*` strings, part and
    /// animation counts). The scene loads archives through this.
    fn descriptor() -> &'static assets::CharacterDescriptor
    where
        Self: Sized;
    /// Build the character from its data archive (`ftData.ext_attr` and
    /// whatever else its `OnLoad` reads). Mirrors `ft<Char>_Init_OnLoad`'s
    /// PUSH_ATTRS without the runtime allocation.
    fn from_archive(data: &hsd_archive::Archive) -> Result<Self, crate::desc::FighterDescError>
    where
        Self: Sized;
    /// Restore character-owned fields from a retail Fighter dump when a scene
    /// starts from a savestate (the shared fields are restored by the scene).
    fn restore_saved(&mut self, _raw_fighter: &[u8]) {}
    fn on_load(&mut self, capabilities: &mut Capabilities);
    fn on_reset(&mut self);
    /// Costume-dependent OnLoad work (e.g. material animation end frames).
    fn on_costume_loaded(
        &mut self,
        _archive: &hsd_archive::Archive,
        _costume: u8,
    ) -> assets::Result<()> {
        Ok(())
    }
    /// OnLoad work requiring decoded animation resources and costume identity.
    fn on_resources_loaded(&mut self, _assets: &assets::FighterAssets, _player: &PlayerSlot) {}
    /// Fighter_ChangeMotionState, fighter.c:1120-1123: restore ground resources.
    fn on_grounded_motion(&mut self) {}
    /// ftCo_8009DD94, ftdynamics.c:397-419: first joint affected by forces.
    fn dynamics_first_force_bone(&self, _set: usize, _count: usize) -> usize {
        0
    }

    /// ftCo_800C3B10 (800C3B10), ftCo_AirCatch.c:54-79.
    fn air_dodge_tether(&self) {
        if Self::descriptor().common_behavior.air_dodge_tether {
            unimplemented!("ftCo_AirCatch.c:54-79: character tether hook");
        }
    }

    /// ftCo_Landing_Enter (800D5AEC), ftCo_Landing.c:51-83.
    /// Character crates reset their airborne special resources here.
    fn on_landing(&mut self, _allow_interrupt: bool) {
        if Self::descriptor().common_behavior.landing_reset {
            unimplemented!("ftCo_Landing.c:51-83: character landing reset hook");
        }
    }

    /// ftCo_Guard.c:335-350, 917-934: egg shield and sword model hooks.
    fn guard_variant(&self, _commands: &mut commands::CommandState) {}
    /// ftCo_Escape.c: per-character setup at its retail motion-entry boundary.
    fn escape_variant(
        _fighter: &mut Fighter,
        _assets: &assets::FighterAssets,
        rolling: bool,
    ) -> assets::Result<()>
    where
        Self: Sized,
    {
        if rolling && Self::descriptor().common_behavior.morph_ball_roll {
            unimplemented!("ftCo_Escape.c:83-85: Samus morph-ball roll");
        }
        Ok(())
    }
    /// ftPe_8011BA54 / ftPe_8011BAD8: float selection surrounding the
    /// aerial-jump predicate. Characters without float do nothing.
    fn check_float_input(
        &self,
        _input: &crate::input::FighterInput,
        _assets: &assets::FighterAssets,
        _vertical_velocity: f32,
        _phase: FloatInputPhase,
    ) {
    }

    /// Which double-jump entry the character uses
    /// (ftCo_JumpAerial.c:103-119 `switch (fp->kind)`). The default arm is
    /// the ordinary `ftCo_JumpAerial_Enter_Basic`; Ness, Yoshi, Peach and
    /// Mewtwo override this in their own crates.
    fn aerial_jump_style(&self) -> AerialJumpStyle {
        AerialJumpStyle::Basic
    }
    /// ftCo_800CB870 / ftCo_800D730C: shared multijump attributes, when enabled.
    fn multi_jump_attributes(&self) -> Option<&multi_jump::MultiJumpAttributes> {
        None
    }
    /// ftCo_800D7268: ordinary family by default; Kirby's copied helmet uses
    /// the second family and must override this hook when that path is ported.
    fn multi_jump_family(&self) -> usize {
        0
    }
    fn multi_jump_animation(&self, _jump: usize) -> i32 {
        unimplemented!("ftCo_JumpAerialF1.c: character multijump animation table")
    }
    /// Character-owned setup after the common aerial-jump entry and Anim.
    fn aerial_jump_entered(_fighter: &mut Fighter)
    where
        Self: Sized,
    {
    }
    fn aerial_jump_animated(_fighter: &mut Fighter)
    where
        Self: Sized,
    {
    }

    /// Retail action identity for a shared semantic state. Character tables
    /// may reuse common callbacks with their own action numbers.
    fn action_id(&self, state: melee_types::CommonMotionState) -> i32 {
        state.into()
    }
    /// GuardOn/Reflect usually skip animation and blend the guard pose.
    /// Animated shields instead run the actual GuardOn motion and script.
    fn animated_shield(&self) -> bool {
        false
    }
    /// Complete character-owned shield callbacks; None selects shared behavior.
    fn enter_shield(
        _fighter: &mut Fighter,
        _assets: &assets::FighterAssets,
        _reflect: bool,
    ) -> Option<assets::Result<()>>
    where
        Self: Sized,
    {
        None
    }
    fn animate_shield(
        _fighter: &mut Fighter,
        _assets: &assets::FighterAssets,
    ) -> Option<assets::Result<()>>
    where
        Self: Sized,
    {
        None
    }
    /// Character-owned Guard IASA; None selects the shared input order.
    fn input_shield(
        _fighter: &mut Fighter,
        _assets: &assets::FighterAssets,
    ) -> Option<assets::Result<()>>
    where
        Self: Sized,
    {
        None
    }
    fn enter_guard_hold(
        _fighter: &mut Fighter,
        _assets: &assets::FighterAssets,
    ) -> Option<assets::Result<()>>
    where
        Self: Sized,
    {
        None
    }
    fn enter_guard_off(
        _fighter: &mut Fighter,
        _assets: &assets::FighterAssets,
    ) -> Option<assets::Result<()>>
    where
        Self: Sized,
    {
        None
    }
    /// ftCo_Escape.c: character setup after motion entry, and completion.
    fn escape_finished(
        _fighter: &mut Fighter,
        _assets: &assets::FighterAssets,
    ) -> Option<assets::Result<()>>
    where
        Self: Sized,
    {
        None
    }
    fn escape_animated(_fighter: &mut Fighter)
    where
        Self: Sized,
    {
    }
}

/// Float predicates run on either side of the ordinary aerial-jump check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FloatInputPhase {
    BeforeAerialJump,
    AfterAerialJump,
}

/// Double-jump entry variants of ftCo_JumpAerial.c:103-119. Basic, Peach and
/// Yoshi are ported; the others retain explicit unsupported boundaries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AerialJumpStyle {
    /// ftCo_JumpAerialF1: shared Kirby/Jigglypuff multijumps.
    MultiJump,
    /// ftCo_JumpAerial_Enter_Basic (0x800CBBC0).
    Basic,
    /// ftNs_JumpAerial_Enter: Ness's multi-frame double jump.
    Ness,
    /// ftYs_JumpAerial_Enter: Yoshi's armoured double jump.
    Yoshi,
    /// ftPe_JumpAerial_Enter: Peach's float-capable double jump.
    Peach,
    /// ftMt_JumpAerial_Enter: Mewtwo's teleport-style double jump.
    Mewtwo,
}

#[derive(Clone, Debug, Default)]
pub struct Capabilities {
    /// Fighter +2226 bit 1: ftCo_DownBound selects the grounded-damage variant.
    pub grounded_down_bound: bool,
    /// can_walljump, Fighter +2224 bit 0; Fox OnLoad sets this.
    pub can_walljump: bool,
    /// ftData special callback presence, S/Hi/N/Lw.
    pub specials: [bool; 4],
}

/// CpuFighter's recorded fields. Human slots still initialize this block.
#[derive(Clone, Debug)]
pub struct CpuState {
    /// buttons, +1A88.
    pub buttons: u32,
    /// lstick.x/y, +1A8C/+1A8D.
    pub stick: [i8; 2],
    /// xC, +1A94; not PlayerKind.
    pub mode: i32,
    /// level, +1A98.
    pub level: i32,
    /// x18, +1AA0.
    pub behavior: i32,
    /// x7C, +1B04; frozen for human slots.
    pub reaction_timer: i32,
    /// CpuFighter.x34 (+1ABC): delay before choosing another attack.
    pub attack_delay: i32,
    /// cpu.x55C/x560/x564/x568 (+1FE4..1FF0), current hurtbox extents.
    pub hurtbox_extents: [f32; 4],
}

/// Unsupported interactions are represented explicitly, never inferred from
/// zero controller samples. These gates correspond to fighter.c's proc arms.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Interaction {
    #[default]
    Idle,
    Hitlag,
    HeldItem,
    Grab,
    Shield,
    Damage,
    Death,
    StatusEffect,
    Accessory,
    Attack,
    AsyncEffect,
    StageHazard,
    FighterOverlap,
    CoinMatch,
}

/// Reset sentinels used by the ordinary Wait proc path.
#[derive(Clone, Debug)]
pub struct Status {
    /// x221F_b3 (+221F mask 10).
    pub disabled: bool,
    /// x221D_b4: reset input during match startup.
    pub input_frozen: bool,
    /// x221D_b5: skip fighter-overlap nudge while dodging (ftcommon.c:850).
    pub ignore_fighter_nudge: bool,
    pub interaction: Interaction,
    /// dmg.x18ac_time_since_hit (+18AC), reset -1.
    pub time_since_hit: i32,
    /// smash_attrs.x2138_smashSinceHitbox (+2138), reset -1.
    pub time_since_smash: f32,
    /// shield_health (+1998), reset PlCo.x260.
    pub shield_health: f32,
    /// x2064_ledgeCooldown (+2064).
    pub ledge_cooldown: i32,
    /// x2222_b3: count a top exit without upward knockback.
    pub unconditional_top_exit: bool,
    /// x2228_b2: suppress grabs while another state owns ledge detection.
    pub ledge_grab_disabled: bool,
    /// x221D_b7: hanging/ledge-option state, cleared on motion entry.
    pub on_ledge: bool,
    /// x1A6A, ftCommon_8007E2F4: excluded grab categories.
    pub grab_exclusions: ledge::GrabExclusions,
    /// x1990: timed intangibility, independent of subaction hurt status.
    pub ledge_intangibility: i32,

    /// x2100 (+2100), -1 disables sword afterimages.
    pub sword_trail: i32,
    /// dmg.x18B8/x18BC: camera damage displacement.
    pub camera_shift: Vec2,
    /// x209A, nametag display countdown; Wait entry uses PlCo.x5F0.
    pub name_tag_timer: u16,
}
impl Status {
    pub fn reset(shield_health: f32) -> Self {
        Self {
            disabled: false,
            input_frozen: false,
            ignore_fighter_nudge: false,
            interaction: Interaction::Idle,
            time_since_hit: -1,
            time_since_smash: -1.0,
            shield_health,
            ledge_cooldown: 0,
            unconditional_top_exit: false,
            ledge_grab_disabled: false,
            on_ledge: false,
            grab_exclusions: ledge::GrabExclusions::NONE,
            ledge_intangibility: 0,
            sword_trail: -1,
            camera_shift: Vec2::ZERO,
            name_tag_timer: 0,
        }
    }
    fn require_supported(&self) {
        match self.interaction {
            Interaction::Idle
            | Interaction::Shield
            | Interaction::Hitlag
            | Interaction::Damage
            | Interaction::Attack => {}
            Interaction::HeldItem => unimplemented!("fighter.c:1523-1532: held-item lifetime"),
            Interaction::Grab => unimplemented!("fighter.c:1560-1578,2602-2626: capture/grab"),
            Interaction::Death => unimplemented!("fighter.c:914-929: death/entry states"),
            Interaction::StatusEffect => unimplemented!("fighter.c:1463-1641: status/item effects"),
            Interaction::Accessory => {
                unimplemented!("fighter.c:2533-2576: accessory callbacks/model")
            }
            Interaction::AsyncEffect => unimplemented!("fighter.c:2560: nonempty efAsync queue"),
            Interaction::StageHazard => {
                unimplemented!("fighter.c:2604,2631: stage collision lists")
            }
            Interaction::FighterOverlap => {
                unimplemented!("ftcommon.c:827-888: fighter overlap nudge")
            }
            Interaction::CoinMatch => {
                unimplemented!("ft_07C6.c:39: coin-match attachment collision")
            }
        }
    }
}

/// Character-owned payload and live callback row (ft/types.h).
/// Calculation owners live in the concrete core.
pub struct Fighter {
    pub core: FighterCore,
    pub character: CharacterState,
    pub motion_row: MotionRow,
}
impl std::ops::Deref for Fighter {
    type Target = FighterCore;
    fn deref(&self) -> &Self::Target {
        &self.core
    }
}
impl std::ops::DerefMut for Fighter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.core
    }
}
impl Fighter {
    pub fn character_accessory(&mut self, assets: &assets::FighterAssets) {
        (self.character.table().accessory)(self, assets);
    }
    pub fn item_muzzle(&mut self, assets: &assets::FighterAssets) -> Option<(Vec3, f32)> {
        (self.character.table().item_muzzle)(self, assets)
    }
    pub fn item_owner(&mut self, assets: &assets::FighterAssets) -> melee_it::ItemOwner {
        (self.character.table().item_owner)(self, assets)
    }
    /// Install callback and scalar state together, including during savestate import.
    pub fn install_motion_row(&mut self, row: MotionRow) {
        self.core.motion_state = MotionState::new(row);
        self.motion_row = row;
    }
}

/// Shared fighter state and calculations, compiled independently of character.
/// Position and facing have one owner: `physics`.
pub struct FighterCore {
    /// kind (+004), initialized by Fighter_UnkInitLoad_80068914.
    pub kind: FighterKind,
    /// player_id (+00C) and Player slot metadata (pl/player.c).
    pub player: PlayerSlot,
    /// x8_spawnNum (+008); allocated only by cold spawning, never import.
    pub spawn_number: u32,
    pub physics: FighterPhysics,
    pub animation: FighterAnimation,
    pub input: FighterInput,
    pub collision: EnvironmentCollision,
    /// co_attrs (+110); copied from ftData.x0.
    pub attributes: FighterAttributes,
    pub bones: FighterBones,
    /// GObj.hsd_obj: main skeleton; animation owns the secondary tree.
    pub skeleton: JObjTree,
    pub revival_platform: life::RevivalPlatform,
    pub revival_platform_active: bool,
    pub motion_state: MotionState,
    pub state_data: MotionData,
    pub combat: damage::CombatState,
    pub shield: shield::ShieldState,
    pub effect_state: effects::FighterEffects,
    pub effects: melee_ef::request::EffectQueue,
    /// Ordered item operations drained by the scene after each fighter phase.
    pub item_requests: melee_types::fixed::FixedVec<melee_it::ItemRequest, 64>,
    pub capabilities: Capabilities,
    pub cpu: CpuState,
    pub status: Status,
    pub commands: commands::CommandState,
    /// x221C_u16_y, three ground-IK enable bits.
    pub ground_pose: GroundPoseFlags,
    /// Owned DynamicsDesc chains from dynamic_bone_sets[].dyn_desc.
    pub dynamics: Vec<melee_lb::dynamics::DynamicBoneSet>,
    /// Fighter +2228 bit 1; use the fighter-height plane instead of mpCheckFloor.
    pub dynamics_use_floor_plane: bool,
    /// dynamic_bone_sets[].bone_id (+2F0, stride 0x18); 0x100 disables solving.
    pub dynamics_first_bone: Vec<u32>,
    /// Player_80032828 / Player_SetFacingDirectionConditional mirror.
    pub player_position: Vec3,
    pub player_facing: f32,
    /// Player_UpdateJoystickCountByIndex events.
    pub joystick_count: u64,
    /// dmg.x1930.x0: swept collision bounds sampled at s_link 0.
    pub previous_collision_bounds: Vec3,
    /// x890_cameraBox: target subject updated at s_link 18.
    pub camera: CameraSubject,
    pub hurtboxes: Vec<melee_coll::hurtbox::HurtCapsule>,
    pub dynamic_colliders: Vec<caches::DynamicCollider>,
    /// x1064_thrownHitbox: its pose advances even without a throw.
    pub thrown_hitbox: caches::ThrownHitbox,
    /// Player_GetHandicap; initialized by match setup or the saved player boundary.
    pub grab_handicap: u8,
}

/// Retail inverse trig adapter used by HSD animation.
pub struct RetailTrig;
impl hsd_anim::mtx::InverseTrig for RetailTrig {
    fn atan2f(y: f32, x: f32) -> f32 {
        melee_lb::trigf::atan2f(y, x)
    }
    fn asinf(x: f32) -> f32 {
        melee_lb::trigf::asinf(x)
    }
    fn acosf(x: f32) -> f32 {
        melee_lb::trigf::acosf(x)
    }
}

#[derive(Clone, Debug, Default)]
pub struct CameraSubject {
    /// CameraBox.on_ledge, set by ftCo_Cliff_Cam (80081644).
    pub on_ledge: bool,
    pub position: Vec3,
    pub bone_position: Vec3,
    pub horizontal: Vec2,
    pub vertical: Vec3,
    pub facing: f32,
}

/// State-local data; the retail union starts at Fighter +2340.
#[derive(Clone, Debug, Default)]
pub enum MotionData {
    Life(life::LifeState),
    Smash,
    Down {
        wait_remaining: f32,
    },
    Catch,
    Capture(grab_escape::CaptureState),
    #[default]
    None,
    Entry(entry::EntryState),
    Jab(attack::JabState),
    RapidJab(attack::RapidJabState),
    DownTilt {
        repeat_pressed: bool,
    },
    Tilt,
    Aerial {
        retained_drop_timer: f32,
    },
    Damage(damage::DamageState),
    Guard(shield::GuardState),
    Dizzy(shield_break::DizzyState),
    Escape(escape::EscapeState),
    EscapeAir(air_dodge::AirDodgeState),
    Cliff(ledge::CliffState),
    CliffJump(ledge::CliffJumpState),
    Squat(squat::SquatState),
    Turn(turn::TurnState),
    Walk(walk::WalkState),
    Dash(dash::DashState),
    Run(run::RunState),
    RunBrake(run::RunBrakeState),
    TurnRun(turn_run::TurnRunState),
    KneeBend(jump::KneeBendState),
    Jump(jump::JumpState),
    MultiJump(multi_jump::MultiJumpState),
    JumpAerial {
        retained_drop_timer: f32,
    },
    Pass {
        retained_drop_timer: f32,
    },
    Fall(fall::FallState),
    FallSpecial(fall::SpecialFallState),
    Landing {
        allow_interrupt: bool,
        retained_drop_timer: f32,
    },
}
