//! The CPU command script (ftcmdscript.c): behaviours write short programs
//! of button, stick and wait commands into CpuFighter.buffer; one runs a
//! step per tick and sets the CPU's pad.
use gekko_math::msl::{cosf, fctiwz, sinf};
use hsd_types::Vec3;
use melee_ft::fighter::cpu::{CommandScript, CpuState, SCRIPT_BYTES};

/// CPUCommand (ftcmdscript.h). Codes up to `ZERO_ARG_END` take no
/// argument, up to `ONE_ARG_END` one, above it two.
pub use melee_types::CpuCmd as Command;

/// HSD_Pad button bits the commands press.
mod pad {
    pub const DPAD_LEFT: u32 = 1 << 0;
    pub const DPAD_RIGHT: u32 = 1 << 1;
    pub const DPAD_DOWN: u32 = 1 << 2;
    pub const DPAD_UP: u32 = 1 << 3;
    pub const Z: u32 = 1 << 4;
    pub const R: u32 = 1 << 5;
    pub const L: u32 = 1 << 6;
    pub const A: u32 = 1 << 8;
    pub const B: u32 = 1 << 9;
    pub const X: u32 = 1 << 10;
    pub const Y: u32 = 1 << 11;
    pub const START: u32 = 1 << 12;
}

/// ftCo_800B462C (800B462C): writing starts over at the buffer's start.
pub fn restart(cpu: &mut CpuState) {
    cpu.script.write = 0;
}

/// ftCo_800B463C (800B463C): append one byte.
pub fn push(cpu: &mut CpuState, byte: u8) {
    let script = &mut cpu.script;
    assert!(
        script.write < SCRIPT_BYTES,
        "ftcmdscript.c:501: command script buffer over flow!"
    );
    script.buffer[script.write] = byte;
    script.write += 1;
}

/// A command without an argument.
pub fn command(cpu: &mut CpuState, command: Command) {
    push(cpu, u8::from(command));
}

/// ftCo_800B46B8 (800B46B8): a command and one argument byte.
pub fn command1(cpu: &mut CpuState, command: Command, argument: u8) {
    push(cpu, u8::from(command));
    push(cpu, argument);
}

/// ftCo_800B4778 (800B4778): a command and two argument bytes.
pub fn command2(cpu: &mut CpuState, command: Command, first: u8, second: u8) {
    push(cpu, u8::from(command));
    push(cpu, first);
    push(cpu, second);
}

/// ftCo_800B4880 (800B4880): append PlCo's stored script `id`, its
/// closing CpuCmd_Done included.
pub fn stored(cpu: &mut CpuState, data: &crate::desc::CpuData, id: usize) {
    for &byte in &data.scripts[id] {
        push(cpu, byte);
    }
}

/// `SetLstickX 0; SetLstickY 0` (ftCo_CpuSetNeutralStick).
pub fn neutral_stick(cpu: &mut CpuState) {
    command1(cpu, Command::SetLstickX, 0);
    command1(cpu, Command::SetLstickY, 0);
}

/// ftCo_CpuFinishWithNeutralStick: a neutral stick, then Done.
pub fn finish_with_neutral_stick(cpu: &mut CpuState) {
    neutral_stick(cpu);
    command(cpu, Command::Done);
}

/// ftCo_CpuReturnToPreviousBehavior: x18 = x1C, then Done.
pub fn return_to_previous(cpu: &mut CpuState) {
    cpu.behavior = cpu.home_behavior;
    command(cpu, Command::Done);
}

/// ftCo_800B49F4 (800B49F4): end the script and run it from the start.
pub fn start(cpu: &mut CpuState) {
    command(cpu, Command::Done);
    cpu.script.cursor = Some(0);
    cpu.script.duration = 1;
}

/// ftCo_800B4A78 (800B4A78): release the pad and drop the script.
pub fn clear(cpu: &mut CpuState) {
    cpu.buttons = 0;
    cpu.stick = [0; 2];
    cpu.cstick = [0; 2];
    cpu.triggers = [0; 2];
    cpu.script.cursor = None;
    cpu.script.duration = 0;
    restart(cpu);
}

/// Whether a script is running (csP != NULL or command_duration != 0,
/// ftCo_800B2790's gate).
pub fn running(script: &CommandScript) -> bool {
    script.cursor.is_some() || script.duration != 0
}

/// The fighter a script steers by.
pub struct Subject {
    pub position: Vec3,
    pub facing: f32,
    pub motion: u32,
    /// CpuFighter.x44's position.
    pub target: Option<Vec3>,
}

/// `magnitude * cosf/sinf(angle)` truncated to a stick byte: fmuls, fctiwz,
/// stb (800B420C..800B4220).
fn stick_component(magnitude: i8, component: f32) -> i8 {
    fctiwz(f32::from(magnitude) * component) as i8
}

/// ftCo_800B3E04 (800B3E04): count down the running command; when it ends
/// run commands until one waits or the script is done.
pub fn run(cpu: &mut CpuState, subject: &Subject) {
    let Some(mut cursor) = cpu.script.cursor else {
        return;
    };
    if cpu.script.duration == 0 {
        return;
    }
    cpu.script.duration -= 1;
    if cpu.script.duration != 0 {
        return;
    }
    let byte = |cursor: &mut usize, cpu: &CpuState| {
        let value = cpu.script.buffer[*cursor];
        *cursor += 1;
        value
    };
    while cpu.script.duration == 0 {
        let code = byte(&mut cursor, cpu);
        match code {
            1 => cpu.buttons |= pad::A,
            2 => cpu.buttons &= !pad::A,
            3 => cpu.buttons |= pad::B,
            4 => cpu.buttons &= !pad::B,
            5 => cpu.buttons |= pad::X,
            6 => cpu.buttons &= !pad::X,
            7 => cpu.buttons |= pad::Y,
            8 => cpu.buttons &= !pad::Y,
            9 => {
                cpu.triggers[1] = 0xFF;
                cpu.buttons |= pad::R;
            }
            10 => {
                cpu.triggers[1] = 0;
                cpu.buttons &= !pad::R;
            }
            11 => cpu.buttons |= pad::L,
            12 => cpu.buttons &= !pad::L,
            13 => cpu.buttons |= pad::Z,
            14 => cpu.buttons &= !pad::Z,
            15 => cpu.buttons |= pad::DPAD_UP,
            16 => cpu.buttons &= !pad::DPAD_UP,
            17 => cpu.buttons |= pad::DPAD_DOWN,
            18 => cpu.buttons &= !pad::DPAD_DOWN,
            19 => cpu.buttons |= pad::DPAD_RIGHT,
            20 => cpu.buttons &= !pad::DPAD_RIGHT,
            21 => cpu.buttons |= pad::DPAD_LEFT,
            22 => cpu.buttons &= !pad::DPAD_LEFT,
            23 => cpu.buttons |= pad::START,
            24 => cpu.buttons &= !pad::START,
            25 => cpu.buttons = 0,
            0x7F => {
                cpu.script.duration = 0;
                cpu.script.cursor = None;
                return;
            }
            0x80 => cpu.stick[0] = byte(&mut cursor, cpu) as i8,
            0x81 => cpu.stick[1] = byte(&mut cursor, cpu) as i8,
            0x82 => cpu.cstick[0] = byte(&mut cursor, cpu) as i8,
            0x83 => cpu.cstick[1] = byte(&mut cursor, cpu) as i8,
            0x84 => cpu.triggers[1] = byte(&mut cursor, cpu),
            0x85 => cpu.triggers[0] = byte(&mut cursor, cpu),
            0x86..=0x8D => {
                let bit = [pad::A, pad::B, pad::X, pad::Y][usize::from((code - 0x86) / 2)];
                if (code - 0x86) % 2 == 0 {
                    cpu.buttons |= bit;
                } else {
                    cpu.buttons &= !bit;
                }
                cpu.script.duration = u32::from(byte(&mut cursor, cpu));
            }
            0x8E => cpu.script.duration = u32::from(byte(&mut cursor, cpu)),
            0x8F => {
                let magnitude = byte(&mut cursor, cpu) as i8;
                let angle = destination_angle(cpu, subject);
                cpu.stick[0] = stick_component(magnitude, cosf(angle));
                cpu.stick[1] = stick_component(magnitude, sinf(angle));
            }
            0x90 => {
                let value = byte(&mut cursor, cpu) as i8;
                cpu.stick[0] = if cpu.destination.x > subject.position.x {
                    value
                } else {
                    value.wrapping_neg()
                };
            }
            0x91 => {
                let value = byte(&mut cursor, cpu) as i8;
                cpu.stick[0] = if f64::from(subject.facing) >= 0.0 {
                    value
                } else {
                    value.wrapping_neg()
                };
            }
            0x92 => {
                let motion = byte(&mut cursor, cpu);
                if subject.motion == u32::from(motion) {
                    // 800B4390: wait a tick on this command; csP stays.
                    cpu.script.duration = 1;
                    return;
                }
            }
            0x93 => cpu.behavior = i32::from(byte(&mut cursor, cpu)),
            0x94 => {
                let magnitude = byte(&mut cursor, cpu) as i8;
                if let Some(target) = subject.target {
                    let angle = melee_lb::trigf::stick_angle(
                        target.y - subject.position.y,
                        target.x - subject.position.x,
                    );
                    cpu.stick[0] = stick_component(magnitude, cosf(angle));
                    cpu.stick[1] = stick_component(magnitude, sinf(angle));
                }
            }
            0x95 => {
                let mut value = byte(&mut cursor, cpu) as i8;
                if let Some(target) = subject.target {
                    if target.x < subject.position.x {
                        value = value.wrapping_neg();
                    }
                    cpu.stick[0] = value;
                }
            }
            0xC0 => {
                let step = byte(&mut cursor, cpu) as i8;
                let limit = byte(&mut cursor, cpu) as i8;
                let angle = destination_angle(cpu, subject);
                let (cos, sin) = (cosf(angle), sinf(angle));
                // 800B4448: x clamps with an else-if ...
                let mut x = i32::from(cpu.stick[0]) + i32::from(stick_component(step, cos));
                let limit_x = i32::from(stick_component(limit, cos));
                if x > limit_x {
                    x = limit_x;
                } else if x < -limit_x {
                    x = -limit_x;
                }
                cpu.stick[0] = x as i8;
                // 800B44DC: ... y with two independent tests.
                let mut y = i32::from(cpu.stick[1]) + i32::from(stick_component(step, sin));
                let limit_y = i32::from(stick_component(limit, sin));
                if y > limit_y {
                    y = limit_y;
                }
                if y < -limit_y {
                    y = -limit_y;
                }
                cpu.stick[1] = y as i8;
            }
            0xC1 | 0xC2 => {
                let step = i32::from(byte(&mut cursor, cpu) as i8);
                let limit = i32::from(byte(&mut cursor, cpu) as i8);
                let toward = if code == 0xC1 {
                    cpu.destination.x > subject.position.x
                } else {
                    f64::from(subject.facing) > 0.0
                };
                let stick = i32::from(cpu.stick[0]);
                let value = if toward { stick + step } else { stick - step };
                let value = if value > limit {
                    limit
                } else if value < -limit {
                    -limit
                } else {
                    value
                };
                cpu.stick[0] = value as i8;
            }
            // 800B3EC4: codes past 0xC2 and unused ones fall through.
            _ => {}
        }
    }
    cpu.script.cursor = Some(cursor);
}

/// lb_8000D008 toward the destination: atan2 of (x54 - cur_pos), each
/// component an fsubs (800B41D4/800B41E4).
fn destination_angle(cpu: &CpuState, subject: &Subject) -> f32 {
    melee_lb::trigf::stick_angle(
        cpu.destination.y - subject.position.y,
        cpu.destination.x - subject.position.x,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use melee_ft::fighter::cpu::CpuSetup;

    fn cpu() -> CpuState {
        CpuState::prepared(4, 1)
    }

    fn subject(x: f32, facing: f32) -> Subject {
        Subject {
            position: Vec3::new(x, 0.0, 0.0),
            facing,
            motion: 14,
            target: None,
        }
    }

    #[test]
    fn a_started_script_runs_until_a_wait_and_resumes_after_it() {
        let mut cpu = cpu();
        command(&mut cpu, Command::PressA);
        command1(&mut cpu, Command::WaitFor, 2);
        command(&mut cpu, Command::ReleaseA);
        command1(&mut cpu, Command::SetLstickX, 0x7F);
        start(&mut cpu);
        assert!(running(&cpu.script));
        let s = subject(0.0, 1.0);
        run(&mut cpu, &s);
        assert_eq!((cpu.buttons, cpu.script.duration), (0x100, 2));
        run(&mut cpu, &s);
        assert_eq!(cpu.buttons, 0x100);
        run(&mut cpu, &s);
        // Released, stick set, then Done ends the script.
        assert_eq!((cpu.buttons, cpu.stick[0]), (0, 0x7F));
        assert!(!running(&cpu.script));
    }

    #[test]
    fn destination_commands_steer_by_its_side_and_clamp_their_step() {
        let mut cpu = cpu();
        cpu.destination = hsd_types::Vec2::new(-10.0, 0.0);
        command1(&mut cpu, Command::LstickXTowardDestination, 0x40);
        start(&mut cpu);
        run(&mut cpu, &subject(0.0, 1.0));
        assert_eq!(cpu.stick[0], -0x40);
        // 0xC1: step toward the destination, clamped to the limit.
        restart(&mut cpu);
        cpu.stick[0] = -0x70;
        command2(
            &mut cpu,
            Command::LstickXTowardDestinationClamped,
            0x20,
            0x7F,
        );
        start(&mut cpu);
        run(&mut cpu, &subject(0.0, 1.0));
        assert_eq!(cpu.stick[0], -0x7F);
    }

    #[test]
    fn waiting_on_a_motion_holds_the_cursor_on_that_command() {
        let mut cpu = cpu();
        command1(&mut cpu, Command::WaitIfMotionId, 14);
        command(&mut cpu, Command::PressB);
        start(&mut cpu);
        run(&mut cpu, &subject(0.0, 1.0));
        assert_eq!((cpu.script.cursor, cpu.buttons), (Some(0), 0));
        let mut moved = subject(0.0, 1.0);
        moved.motion = 15;
        run(&mut cpu, &moved);
        assert_eq!(cpu.buttons, 0x200);
    }

    #[test]
    fn clear_releases_the_pad_and_drops_the_script() {
        let mut cpu = CpuState::initialize(
            &CpuSetup {
                mode: 6,
                level: 0,
                partner: true,
                position: Vec3::ZERO,
                gravity: 0.1,
                jump_velocity: 2.0,
                air_jump_multiplier: 1.0,
                floor_below: None,
            },
            &mut gekko_math::HsdRng::new(1),
        );
        command(&mut cpu, Command::PressA);
        start(&mut cpu);
        run(&mut cpu, &subject(0.0, 1.0));
        cpu.stick = [5, 5];
        clear(&mut cpu);
        assert_eq!(
            (cpu.buttons, cpu.stick, cpu.script.cursor),
            (0, [0; 2], None)
        );
        assert_eq!(cpu.script.write, 0);
        // Nana's mode and follow radius (ftCo_800A101C's FTKIND_NANA arm).
        assert_eq!((cpu.mode, cpu.x3c, cpu.x56c), (6, 15.0, 1.0));
    }
}
