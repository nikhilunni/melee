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
    ThrowAccessory,
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
    SpawnHitbox {
        id: usize,
        descriptor: melee_types::combat::HitboxDescriptor,
    },
    ClearHitbox(usize),
    /// it_80279544: update an active item's capsule damage without respawning it.
    SetHitboxDamage {
        id: usize,
        damage: f32,
    },
    ClearHitboxes,
    JabFollowup(bool),
    RapidJab(bool),
    /// ftAction_80071A58 / 80071A9C: all capsules or one bone.
    HurtCapsuleStatus {
        bone: Option<usize>,
        status: melee_types::combat::HurtStatus,
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
    FootstepSound {
        behavior: u8,
        id: u32,
        volume: u8,
        pan: u8,
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
