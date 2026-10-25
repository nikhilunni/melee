//! Character-owned choices in ftCo_0A01.c and ftcpuattack.c. Profiles contain
//! only static data; the CPU subsystem implements their shared strategies.
#[derive(Clone, Copy, Debug)]
pub struct MotionRange(pub u16, pub u16);
impl MotionRange {
    pub fn contains(self, motion: u16) -> bool {
        self.0 <= motion && motion <= self.1
    }
}
#[derive(Clone, Copy, Debug)]
pub struct ExclusionRule {
    pub range: f32,
    pub scripts: &'static [i32],
    /// Airborne, exclude when not over the stage instead of probing ahead.
    pub require_ground_or_stage: bool,
}
/// ftCo_800A96B8's arm for the kind.
#[derive(Clone, Copy, Debug)]
pub enum Recovery {
    /// ftCo_CpuRecoverDiagonally (Samus's ftCo_800A963C writes the same).
    Diagonal,
    /// The up special on the diagonal, then a wait (Fox, Falco).
    TimedSpecial(u8),
    /// The side special level toward the stage, then a wait (Luigi).
    SideSpecial(u8),
    /// ftCo_800A92CC: Quick Attack's two legs (Pikachu, Pichu).
    QuickAttack,
    /// ftCo_800A8EB0: PK Thunder into Ness at level 9, once under the
    /// destination.
    PkThunder,
    /// ftCo_800A949C: Farore's Wind (Zelda).
    FaroresWind,
    /// No special: the up special is no recovery (Yoshi).
    None,
}
#[derive(Clone, Copy, Debug)]
pub enum Ranged {
    None,
    RemoteProjectile,
    RollingAttack,
    DownwardProjectile(i32),
}
#[derive(Clone, Copy, Debug)]
pub enum FollowUpAction {
    ReleaseAtTargetHeight,
    SteerStickTowardTarget,
    RollAcrossSharedIsland,
    TapShield,
    SteerOwnedProjectile,
}
#[derive(Clone, Copy, Debug)]
pub struct FollowUpRule {
    pub motions: MotionRange,
    pub action: FollowUpAction,
    /// Also runs before the ranged attack's continuation.
    pub ranged: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Charge {
    pub loops: [u16; 2],
    pub release_wait: bool,
    pub store_full: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Finisher {
    pub high_level: bool,
    pub script: usize,
    pub at_high_percent: Option<usize>,
}
#[derive(Clone, Copy, Debug)]
pub struct CpuProfile {
    pub exclusions: &'static [ExclusionRule],
    pub copy_exclusions: bool,
    pub attacking_from: u16,
    /// ftCo_800A101C: initial extra reach (x56C), before periodic rerolls.
    pub initial_reach: f32,
    pub reach_reroll_scale: f64,
    pub recovery: Recovery,
    pub ranged: Ranged,
    pub follow_ups: &'static [FollowUpRule],
    pub charge: Option<Charge>,
    pub plunge_threat_motions: &'static [u16],
    pub column_threat: Option<(u16, f32)>,
    pub rising_threat: Option<(u16, f64)>,
    pub ignores_threats: bool,
    pub absorb_item_threats: bool,
    pub blaster_defence: bool,
    pub cargo_motions: Option<MotionRange>,
    pub mouthful_motions: Option<MotionRange>,
    pub klaw_motion: Option<u16>,
    pub finisher: Option<Finisher>,
    pub scaled_jump: bool,
    /// ftCo_800ABA34 / ftCo_800ACD5C: consume the requested transformation script.
    pub transform_on_request: bool,
}
impl CpuProfile {
    /// ftCommon's ordinary arms: no character-owned attack probes or follow-ups.
    pub const DEFAULT: Self = Self {
        exclusions: &[],
        copy_exclusions: false,
        attacking_from: 0x155,
        initial_reach: 3.5,
        reach_reroll_scale: 4.0,
        recovery: Recovery::Diagonal,
        ranged: Ranged::None,
        follow_ups: &[],
        charge: None,
        plunge_threat_motions: &[],
        column_threat: None,
        rising_threat: None,
        ignores_threats: false,
        absorb_item_threats: false,
        blaster_defence: false,
        cargo_motions: None,
        mouthful_motions: None,
        klaw_motion: None,
        finisher: None,
        scaled_jump: false,
        transform_on_request: false,
    };
}
