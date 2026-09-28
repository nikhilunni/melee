//! CpuFighter (ft/types.h:991, fp+1A88): the CPU controller's state. Every
//! fighter initializes it (ftCo_800A101C, human slots included); melee-cpu
//! runs it for fighters whose input it supplies (Nana, CPU players).
use gekko_math::rng::HsdRng;
use hsd_types::{Vec2, Vec3};

/// CpuFighter.xC for Nana (ftCo_800A101C: FTKIND_NANA forces it).
pub const PARTNER_MODE: i32 = 6;
/// Fighter_x1A88_xFC_t entries in the partner's input ring.
pub const FOLLOW_ENTRIES: usize = 30;
/// ftCo_800A101C: the write cursor starts this far ahead of the read cursor,
/// the delay with which Nana replays her player's inputs.
pub const FOLLOW_DELAY: usize = 5;
/// sizeof(CpuFighter.buffer): the command script area.
pub const SCRIPT_BYTES: usize = 0x100;

/// Fighter_x1A88_xFC_t: one sample of the player's own fighter, as the
/// partner replays it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FollowSample {
    /// +0: held buttons.
    pub buttons: u32,
    /// +4/+5: both triggers carry the analog trigger (ftCo_800B0918).
    pub triggers: [u8; 2],
    /// +6: lstick, in CPU stick units.
    pub stick: [i8; 2],
    /// +8: cstick.
    pub cstick: [i8; 2],
    /// +C.
    pub position: Vec3,
    /// +18.
    pub facing: f32,
}

/// CpuFighter.xFC[30] with its write (x444) and read (x448) cursors.
#[derive(Clone, Debug, PartialEq)]
pub struct FollowRing {
    pub entries: [FollowSample; FOLLOW_ENTRIES],
    pub write: usize,
    pub read: usize,
}

/// The command script: `buffer` (+454), `write_pos` (+554), `csP` (+450)
/// and `command_duration` (+44C), ftcmdscript.c.
#[derive(Clone, Debug, PartialEq)]
pub struct CommandScript {
    pub buffer: [u8; SCRIPT_BYTES],
    pub write: usize,
    /// csP: the next command to run, or None.
    pub cursor: Option<usize>,
    pub duration: u32,
}
impl Default for CommandScript {
    fn default() -> Self {
        Self {
            buffer: [0; SCRIPT_BYTES],
            write: 0,
            cursor: None,
            duration: 0,
        }
    }
}

/// CpuFighter. Fields named by offset keep the decomp's unknowns; a
/// fighter reference is its fighter-list index.
#[derive(Clone, Debug)]
pub struct CpuState {
    /// buttons, +1A88.
    pub buttons: u32,
    /// lstick.x/y, +1A8C/+1A8D.
    pub stick: [i8; 2],
    /// cstick.x/y, +1A8E/+1A8F.
    pub cstick: [i8; 2],
    /// ltrigger, rtrigger (+8, +9): 0..255.
    pub triggers: [u8; 2],
    /// xC, +1A94; not PlayerKind.
    pub mode: i32,
    /// level, +1A98.
    pub level: i32,
    /// x14: ftCo_800A101C's fourth argument.
    pub x14: i32,
    /// x18, +1AA0: the running behaviour.
    pub behavior: i32,
    /// x1C: the behaviour x18 returns to.
    pub home_behavior: i32,
    /// x20.
    pub x20: i32,
    pub x24: i32,
    pub x28: i32,
    pub x2c: i32,
    /// x30: frames the locked target (x48) stays chosen.
    pub target_lock_timer: i32,
    /// CpuFighter.x34 (+1ABC): delay before choosing another attack.
    pub attack_delay: i32,
    /// x38: how near the destination counts as arrived.
    pub destination_radius: f32,
    pub x3c: f32,
    pub x40: f32,
    /// x44: the fighter the CPU is dealing with.
    pub target: Option<usize>,
    /// x48: the locked target of ftCo_800A4BEC.
    pub locked_target: Option<usize>,
    /// x4C: the item the CPU is going for (Item*).
    pub item_target: Option<u32>,
    pub x50: u32,
    /// x54: where the CPU is heading.
    pub destination: Vec2,
    pub x5c: f32,
    /// x60: frames before the stage route's waypoint (x64) replaces x54.
    pub route_timer: i32,
    /// x64: the destination restored when `route_timer` runs out.
    pub route_destination: Vec2,
    pub x6c: Vec2,
    pub x74: Vec2,
    /// x7C, +1B04: the CPU's tick counter.
    pub reaction_timer: i32,
    /// x80: behaviour decisions taken.
    pub x80: i32,
    /// x84: consecutive ticks the collision position barely moved.
    pub still_ticks: i32,
    pub x88: i32,
    pub x8c: i32,
    pub x90: i32,
    pub x94: i32,
    /// x98: the position at reset.
    pub spawn_position: Vec3,
    pub xa4: i32,
    /// xC8/xEC: the defend and attack move queue counts.
    pub defend_queue_len: u8,
    pub attack_queue_len: u8,
    /// xF8 bit 0.
    pub xf8_b0: bool,
    /// xF8 bits 1-2.
    pub xf8_b12: u8,
    /// xF8 bits 3-4: the edge-guard style ftCo_800B732C chose.
    pub xf8_b34: u8,
    pub xf8_b5: bool,
    /// xF8 bit 6: the target is within twice the kind's reach
    /// (ftCo_800A20A0).
    pub xf8_b6: bool,
    pub xf8_b7: bool,
    /// xF9 bit 0: `locked_target` holds.
    pub target_locked: bool,
    pub xf9_b1: bool,
    pub xf9_b2: bool,
    pub xf9_b3: bool,
    pub xf9_b4: bool,
    pub xf9_b5: bool,
    pub xf9_b6: bool,
    pub xf9_b7: bool,
    pub xfa_b1: bool,
    pub xfa_b2: bool,
    /// xFA bits 3-4.
    pub xfa_b34: u8,
    /// xFA bit 5: the floor under the CPU lies inside the blast zones.
    pub over_stage: bool,
    pub xfa_b6: bool,
    /// xFA bit 7: the partner replays its player's inputs.
    pub following: bool,
    pub xfb_b0: bool,
    /// xFC..x448: the partner's input ring.
    pub follow: FollowRing,
    pub script: CommandScript,
    /// x558: the height of an aerial jump.
    pub jump_height: f32,
    /// cpu.x55C/x560/x564/x568 (+1FE4..1FF0), current hurtbox extents.
    pub hurtbox_extents: [f32; 4],
    /// x56C.
    pub x56c: f32,
    /// x570: the recovery skill scale (ftCo_800B33B0).
    pub x570: f32,
    /// half_width, half_height (+574, +578).
    pub half_size: [f32; 2],
}

/// convertStickAxis (ftCo_0A01.c:832, 800A17E4): fdivs by 127 above zero,
/// fmuls by 1/128 otherwise, clamped to [-1, 1].
fn stick_axis(value: i8) -> f32 {
    let axis = if value > 0 {
        f32::from(value) / 127.0
    } else {
        f32::from(value) * (1.0 / 128.0)
    };
    if f64::from(axis) > 1.0 {
        1.0
    } else if f64::from(axis) < -1.0 {
        -1.0
    } else {
        axis
    }
}

/// ftCo_GetCpuLTrigger / RTrigger (800A1904): a double divide by 255,
/// rounded, clamped to 1.
fn trigger(value: u8) -> f32 {
    let analog = (f64::from(value) / 255.0) as f32;
    if f64::from(analog) > 1.0 {
        1.0
    } else {
        analog
    }
}

/// What ftCo_800A101C reads from the fighter.
pub struct CpuSetup {
    /// Player_GetCpuType, overridden for a partner.
    pub mode: i32,
    pub level: i32,
    /// FTKIND_NANA: follow the player's own fighter.
    pub partner: bool,
    pub position: Vec3,
    /// co_attrs.gravity, jump_v_initial_velocity and air_jump_v_multiplier.
    pub gravity: f32,
    pub jump_velocity: f32,
    pub air_jump_multiplier: f32,
    /// ftCo_800A0FB0 at the fighter: the floor 10 above to 1000 below,
    /// unless the stage ignores it.
    pub floor_below: Option<Vec3>,
}

impl CpuState {
    /// ftCo_800A101C (0x800A101C), ftCo_0A01.c:647-830.
    /// Human and CPU slots both consume the reaction and attack-delay draws.
    pub fn initialize(setup: &CpuSetup, rng: &mut HsdRng) -> Self {
        // Retail 0x800A124C fmul / 0x800A1258 fctiwz: double 10.0 * Randf then fctiwz.
        // asm.py ftCo_800A101C --fused: no multiply-add sites.
        let reaction_timer = (10.0_f64 * f64::from(rng.randf())) as i32;
        let mut cpu = Self::with_draws(setup, reaction_timer, 0);
        cpu.attack_delay = Self::initial_attack_delay(cpu.mode, setup.level, rng);
        cpu
    }

    /// The block before ftCo_800A101C runs (Fighter_Create's prepare, or a
    /// savestate import that restores it): no draws taken.
    pub fn prepared(mode: i32, level: i32) -> Self {
        Self::with_draws(
            &CpuSetup {
                mode,
                level,
                partner: false,
                position: Vec3::ZERO,
                gravity: 0.0,
                jump_velocity: 0.0,
                air_jump_multiplier: 0.0,
                floor_below: None,
            },
            0,
            0,
        )
    }

    fn with_draws(setup: &CpuSetup, reaction_timer: i32, attack_delay: i32) -> Self {
        let mode = if setup.partner {
            PARTNER_MODE
        } else {
            setup.mode
        };
        let (behavior, x20) = match mode {
            1 | 25 => (12, 0),
            3 => (1, 11),
            15 => (0, 0),
            _ => (1, 10),
        };
        let destination = setup
            .floor_below
            .map_or(Vec2::new(setup.position.x, setup.position.y), |floor| {
                Vec2::new(floor.x, floor.y)
            });
        // 800A16E0 fmuls, then 800A1718 fmuls, fmul (double 2.0 * gravity),
        // fdiv and frsp.
        let gravity = setup.gravity;
        let jump = setup.jump_velocity * setup.air_jump_multiplier;
        let jump_height = if gravity < 0.00001 && gravity > -0.00001 {
            10.0
        } else {
            (f64::from(jump * jump) / (2.0 * f64::from(gravity))) as f32
        };
        // 800A1398..800A13A0: fadds, then fmuls by 0.5.
        let extents = [1.0_f32, 1.0];
        let middle = 0.5 * (extents[0] + extents[1]);
        Self {
            buttons: 0,
            stick: [0; 2],
            cstick: [0; 2],
            triggers: [0; 2],
            mode,
            level: setup.level,
            x14: 0,
            behavior,
            home_behavior: behavior,
            x20,
            x24: 0x12C,
            x28: 0,
            x2c: 5,
            target_lock_timer: 0,
            attack_delay,
            destination_radius: 5.0,
            x3c: if setup.partner { 15.0 } else { 40.0 },
            x40: 50.0,
            target: None,
            locked_target: None,
            item_target: None,
            x50: 0,
            destination,
            x5c: 0.0,
            route_timer: 0,
            route_destination: destination,
            x6c: Vec2::ZERO,
            x74: Vec2::ZERO,
            reaction_timer,
            x80: reaction_timer,
            still_ticks: 0,
            x88: 0,
            x8c: 0,
            x90: 0,
            x94: 0,
            spawn_position: setup.position,
            xa4: 0,
            defend_queue_len: 0,
            attack_queue_len: 0,
            xf8_b0: false,
            xf8_b12: 0,
            xf8_b34: 0,
            xf8_b5: false,
            xf8_b6: false,
            xf8_b7: false,
            target_locked: false,
            xf9_b1: false,
            // ftCo_800A101C sets xF9_b2 for Nana, then clears it for everyone.
            xf9_b2: false,
            xf9_b3: false,
            xf9_b4: false,
            xf9_b5: false,
            xf9_b6: false,
            xf9_b7: false,
            xfa_b1: false,
            xfa_b2: false,
            xfa_b34: 1,
            over_stage: false,
            xfa_b6: false,
            following: false,
            xfb_b0: false,
            follow: FollowRing {
                entries: [FollowSample {
                    facing: 1.0,
                    ..FollowSample::default()
                }; FOLLOW_ENTRIES],
                write: FOLLOW_DELAY,
                read: 0,
            },
            script: CommandScript::default(),
            jump_height,
            hurtbox_extents: [extents[0], extents[1], middle, 2.0],
            // x56C: Donkey Kong and Bowser 8.5, Giga Bowser 17 (unported
            // kinds), Nana 1, everyone else 3.5.
            x56c: if setup.partner { 1.0 } else { 3.5 },
            x570: 1.0,
            half_size: [10.0, 10.0],
        }
    }

    /// ftCo_800A101C's FTKIND_NANA loop: every ring entry starts at the
    /// player's own fighter (ftCo_800A589C), with no inputs.
    pub fn seed_follow_ring(&mut self, position: Vec3, facing: f32) {
        for entry in &mut self.follow.entries {
            *entry = FollowSample {
                position,
                facing,
                ..FollowSample::default()
            };
        }
    }

    /// What Fighter_Spaghetti_8006AD10 samples in place of the pad when
    /// the CPU drives the fighter: ftCo_GetCpuLStickX/Y, CStickX/Y,
    /// LTrigger/RTrigger and Buttons.
    pub fn pad_sample(&self) -> crate::input::PadSample {
        use crate::input::{Buttons, PadSample, Stick};
        let stick = |[x, y]: [i8; 2]| Stick {
            x: stick_axis(x),
            y: stick_axis(y),
        };
        PadSample {
            buttons: Buttons(self.buttons),
            stick: stick(self.stick),
            cstick: stick(self.cstick),
            left_trigger: trigger(self.triggers[0]),
            right_trigger: trigger(self.triggers[1]),
        }
    }

    /// ftCo_800B9704 (ftcpuattack.c:2098-2106), called at 0x800A1744.
    fn initial_attack_delay(mode: i32, level: i32, rng: &mut HsdRng) -> i32 {
        // Retail 0x800B9718 draws even for humans. 0x800B9734/0x800B974C
        // are fmadds; 0x800B9750 truncates with fctiwz before mode-7 halving.
        let random_delay = gekko_math::fma::fmadds(15.0, rng.randf(), 15.0);
        let delay = gekko_math::msl::fctiwz(gekko_math::fma::fmadds(
            (10 - level) as f32,
            random_delay,
            10.0,
        ));
        if mode == 7 {
            delay / 2
        } else {
            delay
        }
    }
}
