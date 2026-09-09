//! Fighter ownership and scheduler callbacks for grounded, item-free Wait.
//! Retail addresses and unsupported paths are documented at each entry point.
pub mod air_dodge;
pub mod assets;
pub mod caches;
pub mod commands;
pub mod dash;
pub mod effects;
pub mod entry;
pub mod escape;
pub mod fall;
pub mod jump;
pub mod landing;
pub mod ledge;
mod procs;
pub mod run;
pub mod shield;
mod snapshot;
mod spawn;
pub mod squat;
pub mod state;
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
pub use state::{interleaved_order, FighterProc, MotionState};

/// Character-owned load/reset hooks (`ftData_OnLoad`/`ftData_OnDeath`).
/// Implementations live in ft-<character>; common fighter code never loads a
/// character crate. The implementation owns its typed special attributes.
pub trait CharacterCallbacks {
    fn kind(&self) -> FighterKind;
    fn on_load(&mut self, capabilities: &mut Capabilities);
    fn on_reset(&mut self);
    /// ftCo_800C3B10 (800C3B10), ftCo_AirCatch.c:54-79.
    fn air_dodge_tether(&self) {
        if matches!(
            self.kind(),
            FighterKind::Link | FighterKind::CLink | FighterKind::Samus
        ) {
            unimplemented!("ftCo_AirCatch.c:54-79: character tether hook");
        }
    }

    /// ftCo_Landing_Enter (800D5AEC), ftCo_Landing.c:51-83.
    /// Character crates reset their airborne special resources here.
    fn on_landing(&mut self, _allow_interrupt: bool) {
        if matches!(
            self.kind(),
            FighterKind::Mario
                | FighterKind::DrMario
                | FighterKind::Peach
                | FighterKind::Emblem
                | FighterKind::GameWatch
                | FighterKind::Popo
                | FighterKind::Nana
                | FighterKind::Kirby
                | FighterKind::Mewtwo
        ) {
            unimplemented!("ftCo_Landing.c:51-83: character landing reset hook");
        }
    }

    /// ftCo_Guard.c:335-350, 917-934: egg shield and sword model hooks.
    fn guard_variant(&self) {
        if self.kind() == FighterKind::Yoshi {
            unimplemented!("ftCo_Guard.c:339-341: Yoshi egg shield");
        }
    }
    /// ftCo_Escape.c:78-94, 228-241: per-character escape setup.
    fn escape_variant(&self, rolling: bool) {
        if rolling && self.kind() == FighterKind::Samus {
            unimplemented!("ftCo_Escape.c:83-85: Samus morph-ball roll");
        }
        if self.kind() == FighterKind::Yoshi {
            unimplemented!("ftCo_Escape.c:86-88, 232-234: Yoshi egg escape");
        }
    }
    /// Which double-jump entry the character uses
    /// (ftCo_JumpAerial.c:103-119 `switch (fp->kind)`). The default arm is
    /// the ordinary `ftCo_JumpAerial_Enter_Basic`; Ness, Yoshi, Peach and
    /// Mewtwo override this in their own crates.
    fn aerial_jump_style(&self) -> AerialJumpStyle {
        AerialJumpStyle::Basic
    }
}

/// Double-jump entry variants of ftCo_JumpAerial.c:103-119. Only `Basic`
/// is ported; the others exist so character crates can name them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AerialJumpStyle {
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
            ledge_grab_disabled: false,
            on_ledge: false,
            grab_exclusions: ledge::GrabExclusions::NONE,
            ledge_intangibility: 0,
            sword_trail: -1,
            camera_shift: Vec2::ZERO,
            name_tag_timer: 0,
        }
    }
    fn require_idle(&self) {
        match self.interaction {
            Interaction::Idle | Interaction::Shield => {}
            Interaction::Hitlag => unimplemented!("fighter.c:1398-1437: hitlag/SDI"),
            Interaction::HeldItem => unimplemented!("fighter.c:1523-1532: held-item lifetime"),
            Interaction::Grab => unimplemented!("fighter.c:1560-1578,2602-2626: capture/grab"),
            Interaction::Damage => unimplemented!("fighter.c:2853-2998: damage/hitlag"),
            Interaction::Death => unimplemented!("fighter.c:914-929: death/entry states"),
            Interaction::StatusEffect => unimplemented!("fighter.c:1463-1641: status/item effects"),
            Interaction::Accessory => {
                unimplemented!("fighter.c:2533-2576: accessory callbacks/model")
            }
            Interaction::Attack => unimplemented!("ftcoll.c:3032: active attack hitboxes"),
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

/// Fighter (ft/types.h), composed from the verified subsystem owners.
/// Physical position/facing have one owner, `physics`, rather than mirrors.
pub struct Fighter<C: CharacterCallbacks> {
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
    pub motion_state: MotionState,
    pub state_data: MotionData,
    pub shield: shield::ShieldState,
    pub effect_state: effects::FighterEffects,
    pub effects: Vec<effects::EffectRequest>,
    pub character: C,
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
    pub hurtboxes: Vec<caches::Hurtbox>,
    pub dynamic_colliders: Vec<caches::DynamicCollider>,
    /// x1064_thrownHitbox: its pose advances even without a throw.
    pub thrown_hitbox: caches::ThrownHitbox,
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
    #[default]
    None,
    Entry(entry::EntryState),
    Guard(shield::GuardState),
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
    JumpAerial {
        retained_drop_timer: f32,
    },
    Fall {
        blend: f32,
    },
    Landing {
        allow_interrupt: bool,
        retained_drop_timer: f32,
    },
}
