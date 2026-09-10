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
    SetAirborne(melee_types::GroundOrAir),
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
    SpawnHitbox {
        id: usize,
        descriptor: melee_types::combat::HitboxDescriptor,
    },
    ClearHitbox(usize),
    ClearHitboxes,
    JabFollowup(bool),
    /// ftAction_80071AE8: enable the jab combo flag (x2218_b1).
    JabCombo {
        disabled: bool,
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
