//! Shared subaction decoding and timed execution. Consumers apply emitted commands.
pub mod decode;
mod state;
pub use state::{CommandLoop, ScriptState};
/// ftAction_80072A5C (80072A5C) -> ftCo_800BFFD0 (800BFFD0).
/// Color animation is renderer output; the command retains its ID and duration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorAnimationRequest {
    pub id: u8,
    pub duration: u32,
}

#[derive(Clone, Debug)]
pub enum Command {
    SmashCharge(SmashCharge),
    SetAirborne(AirborneMode),
    WindEffect(WindEffect),
    SmashSound,
    /// ftAction_80072BF4 (opcode 51): Fighter_TakeDamage_8006CC7C with the
    /// signed 26-bit amount, e.g. Roy's fully charged Flare Blade recoil.
    SelfDamage(i32),
    ThrowAccessory,
    /// ftAction_80071908: throw_flags_b1, a one-shot cue the current motion's
    /// callbacks read and clear (Falcon Punch's effect spawn and removal).
    MoveCue,
    GrabRelease,
    ThrowReverse,
    SetThrowHitbox {
        id: usize,
        descriptor: melee_types::combat::ThrowHitbox,
    },
    BeginLoop(u32),
    EndLoop,
    /// ftAction_80071F78 (80071F78), Fighter +221E bit 4.
    ArticleVisibility(bool),
    /// ftAction_80071FA0: Fighter +221E bit 5; true hides the fighter model.
    FighterVisibility(bool),
    /// ftAction_80071F34: the ordinary held-item visibility flag.
    HeldItemVisibility(bool),
    /// ftAction_80072894 -> ftCommon_8007E83C: play the held parasol's
    /// animation `index` so that it lasts `frames` (zero: at the fighter's rate).
    ParasolAnimation { index: usize, frames: f32 },
    SpawnHitbox {
        id: usize,
        descriptor: melee_types::combat::HitboxDescriptor,
    },
    ClearHitbox(usize),
    /// it_80279544 / ftAction_8007162C: update an active capsule's damage
    /// without respawning it.
    SetHitboxDamage {
        id: usize,
        damage: f32,
    },
    /// ftAction_8007169C: set a capsule's size (HitCapsule.scale).
    SetHitboxRadius {
        id: usize,
        radius: f32,
    },
    ClearHitboxes,
    JabFollowup(bool),
    RapidJab(bool),
    /// ftAction_80071A58 / 80071A9C: all capsules or one bone.
    HurtCapsuleStatus {
        bone: Option<usize>,
        status: melee_types::combat::HurtStatus,
    },
    /// ftAction_800729D4 (opcode 45): a held Beam Sword's blade grows
    /// (mode 0, it_80284FC4 over `frames` to `length` / 256) or returns
    /// (mode 1, it_80285024 over `frames`) (ftCommon_8007EEC8/8007EF5C).
    SwordBlade {
        mode: u8,
        frames: u16,
        length: u16,
    },
    SwordTrail {
        duration: i32,
        reverse: bool,
    },
    ColorAnimation(ColorAnimationRequest),
    /// ftAction_80072B94: toggle animation ownership of a dynamic joint.
    ToggleDynamics(i32),
    ModelSelection {
        group: i32,
        variant: i32,
    },
    HurtStatus(melee_types::combat::HurtStatus),
    AllowInterrupt,
    End,
    Graphics(melee_types::combat::GraphicsCommand),
    SetVariable {
        index: usize,
        value: u32,
    },
    Goto(usize),
    WaitAnimationLoop,
    LandingEffect(u16),
    Rumble {
        all_players: bool,
        id: u16,
        duration: u16,
    },
    /// ftAction_80071FC8: seven-word random sound selection.
    RandomSound(RandomSound),
    /// ftAction_80072320 (opcode 39): a four-word sound panned by a
    /// direction (lbAudioAx_800263E8); `handle` is its sfx_base (which
    /// Fighter sound handle keeps it). It draws nothing.
    DirectionalSound { handle: u8, id: u32 },
    /// ftAction_80071B50 (opcode 17), or ftAction_80072CD8 (opcode 54,
    /// `terrain`), whose sound the floor's terrain may replace.
    FootstepSound {
        behavior: u8,
        id: u32,
        volume: u8,
        pan: u8,
        terrain: bool,
        /// `footstep_fx_0.use_alt_bone` (lb/types.h:881): the terrain effect
        /// goes to the second foot.
        alt_foot: bool,
    },
    Wait(f32),
    AtFrame(f32),
    Call {
        target: usize,
        continuation: usize,
    },
    Return,
    Part {
        group: usize,
        variant: usize,
        blend: f32,
    },
    GroundPose(u8),
    Texture {
        indices: Vec<usize>,
        frame: f32,
    },
    /// ftAction_80071708 (opcode 14): hitbox `id` may (not) hit fighters
    /// (HitCapsule.x42_b5) or items (x42_b7).
    HitboxTargets {
        id: usize,
        items: bool,
        enabled: bool,
    },
    /// Item script opcode 16 (it_8027978C): play (sub-opcodes 0..2) or
    /// stop (10, 11) an item sound. Item scripts only.
    ItemSound { sub: u8, id: u32 },
    /// An ftAction opcode the port does not decode yet. Its length is unknown,
    /// so decoding stops here; reaching it at run time is `unimplemented!`.
    Unported(u32),
}
#[derive(Clone, Copy, Debug)]
pub enum ChargePhase {
    PreCharge,
    Charging,
    Release,
}
#[derive(Clone, Copy, Debug)]
pub struct SmashCharge {
    pub phase: ChargePhase,
    pub frames: f32,
    pub maximum_frames: f32,
    pub maximum_multiplier: f32,
    pub saved_rate: f32,
    pub color_animation: u8,
}
impl SmashCharge {
    /// ftCo_800DEEB8: retail 800DEEDC fmadds, then fmuls.
    pub fn scale_damage(&self, damage: f32) -> f32 {
        if !matches!(self.phase, ChargePhase::Release) {
            return damage;
        }
        damage
            * gekko_math::fma::fmadds(
                self.maximum_multiplier - 1.0,
                self.frames / self.maximum_frames,
                1.0,
            )
    }
}

/// Opcode 38 payload; selection consumes one HSD_Randi at the command boundary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RandomSound {
    pub ids: [u32; 6],
    pub range: u8,
    pub behavior: u8,
    pub volume: u8,
    pub pan: u8,
}

/// ftAction_80071998 selects one of the three common conversion helpers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AirborneMode {
    Ground,
    Air,
    AirUseAllJumps,
}

/// ftAction_80073118: signed fixed-point wind command payload.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindEffect {
    pub bone: u8,
    pub x: i16,
    pub y: i16,
    pub magnitude: i16,
    pub decay: i16,
    pub timer: i16,
    pub angle: i16,
}
