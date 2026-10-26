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
/// A stage line's hit: what StageData.on_touch_line (Ground_801C5700)
/// returns for a line, read as `lbColl_80008D30_arg1` (lb/forward.h:94)
/// and turned into a HitCapsule by lbColl_80008D30 (0x80008D30).
/// ftCo_800C0B20 logs it when a fighter touches the line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineHazard {
    /// +0x4: the damage, also the knockback formula's count.
    pub damage: u32,
    /// +0x8.
    pub angle: i32,
    /// +0xC / +0x10 / +0x14: knockback growth, weight-dependent (fixed)
    /// knockback, base knockback.
    pub growth: u32,
    pub weight_knockback: u32,
    pub base_knockback: u32,
    /// +0x18.
    pub element: crate::HitElement,
    /// +0x1C / +0x20.
    pub sound_severity: i32,
    pub sound_kind: i32,
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
    /// The owner's facing when the command ran. ftCo_09F7.c passes
    /// `&fp->facing_dir` to efAsync_Spawn, which copies the value into the
    /// queued request then, so a facing flip later in the same proc (a
    /// back throw's reversal) does not reach it. None until issued.
    pub issued_facing: Option<f32>,
    /// Whether the owner stood on the ground when the command ran, after
    /// the set-ground/air commands before it in the same script step.
    /// ftCo_09F7.c reads `fp->ground_or_air` for the floor angle as the
    /// command runs; the port applies those commands after the step.
    /// None until issued.
    pub issued_grounded: Option<bool>,
    /// The joint and its world matrix when the command ran in a proc at or
    /// past link 9, where efAsync_Spawn (0x800679B0) creates the effect at
    /// once and lb_8000B1CC reads the joint then: an animation step later
    /// in the same proc (ftCo_8008DCE0's ftAnim_8006EBA4 after its motion
    /// change) does not move it. None while the effect reads the joint at
    /// its flush.
    pub issued_joint: Option<IssuedJoint>,
}

/// [`GraphicsCommand::issued_joint`]: the resolved effect bone and its matrix.
#[derive(Clone, Copy, Debug)]
pub struct IssuedJoint {
    pub bone: usize,
    pub matrix: hsd_types::Mtx,
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
    /// FtMoveId_CargoThrowF..Lw (0x39..0x3C): Donkey Kong's cargo throws.
    CargoThrowForward,
    CargoThrowBack,
    CargoThrowUp,
    CargoThrowDown,
    /// FtMoveId_SwordSwing1..SwordSwingDash (0x40..0x43): the Beam Sword's
    /// swings.
    SwordSwing1,
    SwordSwing3,
    SwordSwing4,
    SwordSwingDash,
    /// FtMoveId_BatSwing1..BatSwingDash (0x44..0x47): the Home-Run Bat's.
    BatSwing1,
    BatSwing3,
    BatSwing4,
    BatSwingDash,
    /// FtMoveId_ParasolSwing1..ParasolSwingDash (0x48..0x4B).
    ParasolSwing1,
    ParasolSwing3,
    ParasolSwing4,
    ParasolSwingDash,
    /// FtMoveId_HarisenSwing1..HarisenSwingDash (0x4C..0x4F): the Fan's.
    HarisenSwing1,
    HarisenSwing3,
    HarisenSwing4,
    HarisenSwingDash,
    /// FtMoveId_StarRodSwing1..StarRodSwingDash (0x50..0x53).
    StarRodSwing1,
    StarRodSwing3,
    StarRodSwing4,
    StarRodSwingDash,
    /// FtMoveId_LipstickSwing1..LipstickSwingDash (0x54..0x57).
    LipstickSwing1,
    LipstickSwing3,
    LipstickSwing4,
    LipstickSwingDash,
    /// FtMoveId_KbSpecialNMr..KbSpecialNGk (0x16..0x2F): Kirby's copied
    /// neutral specials, one id per copied kind in retail's order (the two
    /// Bowsers, the two Links and the rest each stale on their own).
    CopiedMario,
    CopiedFox,
    CopiedCaptain,
    CopiedDonkey,
    CopiedKoopa,
    CopiedLink,
    CopiedSeak,
    CopiedNess,
    CopiedPeach,
    CopiedPopo,
    CopiedPikachu,
    CopiedSamus,
    CopiedYoshi,
    CopiedPurin,
    CopiedMewtwo,
    CopiedLuigi,
    CopiedMars,
    CopiedZelda,
    CopiedCLink,
    CopiedDrMario,
    CopiedFalco,
    CopiedPichu,
    CopiedGameWatch,
    CopiedGanon,
    CopiedEmblem,
    CopiedGkoops,
}

/// One attack instance retained by projectiles after their owner changes motion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttackInstance {
    pub move_id: StaleMove,
    pub serial: u64,
}
