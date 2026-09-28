//! Behaviour 1's movement toward the destination: ftCo_800A9CB4 in the
//! air, ftCo_800AB224 on the ground.
use crate::{
    script::{self, Command as C},
    world::Scene,
};
use gekko_math::{
    fma::{fmadds, fnmsub},
    msl::{fabsf, fctiwz, sqrtf},
};
use melee_ft::fighter::Fighter;
use melee_types::{mp::collide, GrKind, GroundOrAir};

/// ftCo_IsNearlyZero.
fn nearly_zero(x: f32) -> bool {
    x < 0.00001 && x > -0.00001
}

/// ftCo_CpuFinishWithNeutralY: lstick y back to 0, then Done.
pub fn finish_with_neutral_y(fp: &mut Fighter) {
    let cpu = &mut fp.core.cpu;
    script::command1(cpu, C::SetLstickY, 0);
    script::command(cpu, C::Done);
}

/// ftCo_800A3234 (0x800A3234): falling toward the destination below a
/// ledge-high angle, and blocked by a wall or with no floor between the
/// ECB bottom's last and current positions below the destination.
fn should_jump_back(fp: &mut Fighter, scene: &mut Scene) -> bool {
    let cpu = &fp.core.cpu;
    if cpu.xfa_b6 {
        return fp.core.physics.position_delta.y < 0.1;
    }
    if fp.core.physics.ground_or_air != GroundOrAir::Air {
        return false;
    }
    if f64::from(fp.core.physics.position_delta.y) > 0.0 {
        return false;
    }
    if fp.core.motion_state.action.0 == 0xF4 {
        return false;
    }
    if fp.core.kind == melee_types::FighterKind::Yoshi {
        unimplemented!("ftCo_800A3234: Yoshi's scaled early-match branch");
    }
    let position = fp.core.physics.position;
    let angle = melee_lb::trigf::stick_angle(
        cpu.destination.y - position.y,
        fabsf(cpu.destination.x - position.x),
    );
    if angle < -1.308_996_9 {
        return false;
    }
    let data = &fp.core.collision.data;
    let wall = if fp.core.physics.facing > 0.0 {
        collide::LEFT_WALL_MASK
    } else {
        collide::RIGHT_WALL_MASK
    };
    if data.env_flags as u32 & wall != 0 {
        return true;
    }
    let bottom = data.ecb.bottom;
    // 800A33A0..800A33BC: fadds.
    let from_x = data.last_pos.x + bottom.x;
    let from_y = data.last_pos.y + bottom.y;
    let to_x = data.cur_pos.x + bottom.x;
    let to_y = data.cur_pos.y + bottom.y;
    let destination_y = cpu.destination.y;
    if scene
        .check_usable_floor(from_x, from_y, to_x, to_y)
        .is_some()
    {
        return false;
    }
    destination_y > from_y
}

/// ftCo_800A9CB4 (0x800A9CB4): airborne movement toward the destination:
/// jump back when falling short, else drift, steering by where the fall
/// will pass the destination's x.
pub fn toward_destination_in_air(fp: &mut Fighter, scene: &mut Scene) {
    // ftCo_800A1CA8: the hammer (x2168), which only its item sets.
    if should_jump_back(fp, scene) {
        let attributes = &fp.core.attributes;
        if attributes.jumping.max_jumps > i32::from(fp.core.physics.jumps_used) {
            use_available_jump(fp);
            return;
        }
        match scene.stage() {
            GrKind::Shrine | GrKind::Fourside => {
                unimplemented!("ftCo_800A9CB4: the Temple and Fourside recoveries")
            }
            _ => {}
        }
        // ftCo_800A9CB4_inline1: not under a ceiling (xFA_b6), and shrunk
        // (x34_scale.y < 1), or on Icicle Mountain or Rainbow Cruise.
        if !fp.core.cpu.xfa_b6 {
            if fp.core.player.scale < 1.0 {
                unimplemented!("ftCo_800A96B8: recovery special");
            }
            if matches!(scene.stage(), GrKind::Icemt | GrKind::RCruise) {
                unimplemented!("ftCo_800A9CB4: Icicle Mountain and Rainbow Cruise recovery");
            }
        }
    }
    if fp.core.cpu.xfa_b6 {
        let cpu = &mut fp.core.cpu;
        script::command1(cpu, C::SetLstickX, 0x81);
        finish_with_neutral_y(fp);
        return;
    }
    let position = fp.core.physics.position;
    let delta = fp.core.physics.position_delta;
    let attributes = &fp.core.attributes;
    let gravity = attributes.air.gravity;
    let terminal = attributes.air.terminal_velocity;
    let destination = fp.core.cpu.destination;
    let dx = destination.x - position.x;
    let distance = fabsf(dx);
    // 800AA0C8: fdivs.
    let x_time = if !nearly_zero(delta.x) {
        dx / delta.x
    } else if f64::from(distance) > 0.0 {
        -1.0
    } else {
        0.0
    };
    let terminal_time = if nearly_zero(gravity) {
        1000.0
    } else {
        // 800AA12C..800AA138: fneg, fsubs, fneg, fdivs.
        -(-terminal - delta.y) / gravity
    };
    let y = if terminal_time <= 0.0 {
        // 800AA154: fmadds.
        fmadds(delta.y, x_time, position.y)
    } else if x_time < terminal_time {
        // The decomp names this a square root of the time; retail takes one.
        let root = sqrtf(x_time);
        // 800AA1C8/800AA1D0 fmuls, 800AA1D8 fnmsub, fadd, frsp.
        let fall = fnmsub(0.5, f64::from(gravity * root), f64::from(delta.y * x_time));
        (f64::from(position.y) + fall) as f32
    } else {
        let root = sqrtf(terminal_time);
        let after = x_time - terminal_time;
        // 800AA250..800AA260 fmuls; 800AA264 fnmsub; fsub; fadd; frsp.
        let fall = fnmsub(
            0.5,
            f64::from(gravity * root),
            f64::from(delta.y * terminal_time),
        );
        let fall = fall - f64::from(after * terminal);
        (f64::from(position.y) + fall) as f32
    };
    let cpu = &mut fp.core.cpu;
    if f64::from(x_time) < 0.0 {
        script::command1(cpu, C::LstickXTowardDestination, 0x7F);
    } else if y < destination.y {
        // 800AA2A4..800AA2C0: fsubs, fctiwz, doubled, at most 0x20.
        let step = (fctiwz(destination.y - y) << 1).min(0x20);
        script::command2(cpu, C::LstickXTowardDestinationClamped, step as u8, 0x7F);
    } else {
        script::command1(cpu, C::SetLstickX, 0);
    }
    finish_with_neutral_y(fp);
}

/// ftCo_CpuUseAvailableJump: an aerial jump toward the destination (or
/// forward under a ceiling).
fn use_available_jump(fp: &mut Fighter) {
    if fp.core.cpu.xfa_b6 {
        let cpu = &mut fp.core.cpu;
        script::command1(cpu, C::SetLstickY, 0);
        script::command1(cpu, C::LstickXForward, 0x7F);
    } else if fp.core.kind == melee_types::FighterKind::Yoshi {
        unimplemented!("ftCo_CpuUseAvailableJump: Yoshi's scaled branch (ftCo_800A0508)");
    } else {
        script::command1(&mut fp.core.cpu, C::LstickTowardDestination, 0x7F);
    }
    let cpu = &mut fp.core.cpu;
    script::command(cpu, C::PressY);
    script::command1(cpu, C::WaitFor, 1);
    script::command(cpu, C::ReleaseY);
    script::command(cpu, C::Done);
}
