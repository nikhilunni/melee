//! Shared subaction payloads (lb/types.h); no archive or fighter ownership.
use hsd_types::Vec3;
#[derive(Clone, Debug)]
pub struct HitboxDescriptor {
    pub group: u8,
    pub bone: usize,
    pub common_bone: bool,
    pub requires_throw_owner: bool,
    pub damage: f32,
    pub shield_damage: i8,
    pub sound_severity: u8,
    pub radius: f32,
    pub offset: Vec3,
    pub angle: u16,
    pub growth: u16,
    pub weight_knockback: u16,
    pub base_knockback: u16,
    pub element: crate::HitElement,
    pub hit_ground: bool,
    pub hit_air: bool,
    pub ignore_scale: bool,
    pub clank: bool,
    pub rebound: bool,
}
/// ftAction_80071E04 (80071E04): throw/pummel damage records have no geometry.
#[derive(Clone, Debug)]
pub struct ThrowHitbox {
    pub damage: f32,
    pub angle: u16,
    pub growth: u16,
    pub weight_knockback: u16,
    pub base_knockback: u16,
    pub element: crate::HitElement,
    pub sound_severity: u8,
    pub sound_kind: u8,
}
/// ftColl_8007B0C0 / Fighter.x1988: subaction-controlled vulnerability.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HurtStatus {
    #[default]
    Normal,
    Invincible,
    Intangible,
}
/// ftAction_80071028's five command words, decoded at the archive boundary.
#[derive(Clone, Debug)]
pub struct GraphicsCommand {
    pub bone: usize,
    pub common_bone: bool,
    pub item_bone: bool,
    pub destroy_on_state_change: bool,
    pub id: u16,
    pub parameter: f32,
    pub offset: Vec3,
    pub range: Vec3,
}

/// Move identity shared by fighters and their projectile attacks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StaleMove {
    Jab1,
    Jab2,
    Jab3,
    RapidJab,
    Dash,
    SideTilt,
    UpTilt,
    DownTilt,
    SideSmash,
    UpSmash,
    DownSmash,
    NeutralAir,
    ForwardAir,
    BackAir,
    UpAir,
    DownAir,
    SpecialNeutral,
    SpecialSide,
    SpecialUp,
    SpecialDown,
    /// FtMoveId_DownAttackU / DownAttackD: get-up attacks.
    GetupAttackFaceUp,
    GetupAttackFaceDown,
    /// FtMoveId_CliffAttackSlow / CliffAttackQuick: ledge attacks.
    LedgeAttackSlow,
    LedgeAttackQuick,
    Pummel,
    ThrowForward,
    ThrowBack,
    ThrowUp,
    ThrowDown,
    /// FtMoveId_Parasol: the parasol's open and fall states.
    Parasol,
}

/// One attack instance retained by projectiles after their owner changes motion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttackInstance {
    pub move_id: StaleMove,
    pub serial: u64,
}
