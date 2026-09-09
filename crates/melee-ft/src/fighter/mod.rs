//! Fighter ownership and scheduler callbacks for grounded, item-free Wait.
//! Retail addresses and unsupported paths are documented at each entry point.
pub mod assets;
pub mod caches;
pub mod commands;
mod procs;
mod snapshot;
mod spawn;
pub mod state;

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
    pub interaction: Interaction,
    /// dmg.x18ac_time_since_hit (+18AC), reset -1.
    pub time_since_hit: i32,
    /// smash_attrs.x2138_smashSinceHitbox (+2138), reset -1.
    pub time_since_smash: f32,
    /// shield_health (+1998), reset PlCo.x260.
    pub shield_health: f32,
    /// x2064_ledgeCooldown (+2064).
    pub ledge_cooldown: i32,
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
            interaction: Interaction::Idle,
            time_since_hit: -1,
            time_since_smash: -1.0,
            shield_health,
            ledge_cooldown: 0,
            sword_trail: -1,
            camera_shift: Vec2::ZERO,
            name_tag_timer: 0,
        }
    }
    fn require_idle(&self) {
        match self.interaction {
            Interaction::Idle => {}
            Interaction::Hitlag => unimplemented!("fighter.c:1398-1437: hitlag/SDI"),
            Interaction::HeldItem => unimplemented!("fighter.c:1523-1532: held-item lifetime"),
            Interaction::Grab => unimplemented!("fighter.c:1560-1578,2602-2626: capture/grab"),
            Interaction::Shield => unimplemented!("fighter.c:2822-2842: active shield"),
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
    pub character: C,
    pub capabilities: Capabilities,
    pub cpu: CpuState,
    pub status: Status,
    pub commands: commands::CommandState,
    /// x221C_u16_y, three ground-IK enable bits.
    pub ground_pose: GroundPoseFlags,
    /// dynamic_bone_sets[].bone_id (+2F0, stride 18); 0x100 disables solving.
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
    pub position: Vec3,
    pub bone_position: Vec3,
    pub horizontal: Vec2,
    pub vertical: Vec3,
    pub facing: f32,
}
