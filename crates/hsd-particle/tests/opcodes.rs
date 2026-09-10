//! Behavior tests cover every supported opcode byte, including channel masks.
mod common;
use common::*;
use gekko_math::rng::HsdRng;
use hsd_particle::{
    particle::{Particle, PAUSED, TEXTURED},
    rng_sites::{DrawLog, PRIMARY_COLOR},
    Error,
};

fn particle(program: Vec<u8>) -> Particle {
    Particle::new(&descriptor(program), 0, 0).unwrap()
}
fn tick(p: &mut Particle) -> (bool, DrawLog) {
    let mut log = DrawLog::default();
    let alive = p
        .update::<common::RetailTrig>(&mut HsdRng::new(1), &mut log)
        .unwrap();
    (alive, log)
}

#[test]
fn all_short_and_extended_wait_and_texture_opcodes() {
    for opcode in 0..0x80u8 {
        let mut program = vec![opcode];
        if opcode & 0x20 != 0 {
            program.push(5);
        }
        if opcode & 0x40 != 0 {
            program.push(1);
        }
        let expected = if opcode & 0x20 != 0 {
            (u16::from(opcode & 31) << 8) | 5
        } else {
            u16::from(opcode & 31)
        };
        let next_pc = program.len() as u16;
        program.push(0xff);
        let mut p = particle(program);
        p.texture_images = vec![false, true].into();
        let (alive, log) = tick(&mut p);
        assert!(log.0.is_empty());
        assert_eq!(p.wait, expected, "opcode {opcode:x}");
        assert_eq!(alive, expected != 0);
        assert_eq!(p.pc, next_pc + u16::from(expected == 0));
        if opcode & 0x40 != 0 {
            assert_eq!(p.pose, 1);
            assert_ne!(p.kind & TEXTURED, 0);
        }
    }
}

#[test]
fn every_vector_opcode_preserves_unselected_axes_and_operand_alignment() {
    for opcode in 0x80..0xa0u8 {
        let mut program = vec![opcode];
        for axis in 0..3 {
            if opcode & (1 << axis) != 0 {
                float(&mut program, 10.0 + axis as f32);
            }
        }
        program.push(1);
        let mut p = particle(program);
        p.position = [1.0, 2.0, 3.0];
        p.velocity = [4.0, 5.0, 6.0];
        let mut position = p.position;
        let mut velocity = p.velocity;
        let vector = if opcode < 0x90 {
            &mut position
        } else {
            &mut velocity
        };
        for (axis, value) in vector.iter_mut().enumerate() {
            if opcode & (1 << axis) != 0 {
                *value = if opcode & 8 == 0 {
                    10.0 + axis as f32
                } else {
                    *value + 10.0 + axis as f32
                };
            }
        }
        tick(&mut p);
        assert_eq!(p.velocity, velocity, "opcode {opcode:x}");
        assert_eq!(
            p.position,
            std::array::from_fn(|i| position[i] + velocity[i]),
            "opcode {opcode:x}"
        );
    }
}

#[test]
fn every_color_channel_mask_and_timer_countdown() {
    for opcode in 0xc0..0xe0u8 {
        let mut program = vec![opcode, 0x80, 3];
        for channel in 0..4 {
            if opcode & (1 << channel) != 0 {
                program.push(30 + channel);
            }
        }
        program.push(10);
        let mut p = particle(program);
        tick(&mut p);
        let track = if opcode < 0xd0 {
            &p.primary
        } else {
            &p.environment
        };
        let initial = if opcode < 0xd0 { 255 } else { 0 };
        let expected = std::array::from_fn(|i| {
            if opcode & (1 << i) != 0 {
                30 + i as u8
            } else {
                initial
            }
        });
        assert_eq!(track.current, [initial; 4]);
        assert_eq!(track.target, expected);
        assert_eq!(track.remaining, 3);
        for _ in 0..3 {
            tick(&mut p);
        }
        let track = if opcode < 0xd0 {
            &p.primary
        } else {
            &p.environment
        };
        assert_eq!(track.current, expected);
        assert_eq!(track.duration, 0);
    }
}

#[test]
fn colors_materialize_the_old_interpolation_before_retargeting() {
    let mut p = particle(vec![0xcf, 4, 0, 0, 0, 0, 2, 0xc1, 0, 100, 1]);
    tick(&mut p);
    tick(&mut p);
    tick(&mut p);
    assert_eq!(p.primary.current, [100, 127, 127, 127]);
    assert_eq!(p.primary.duration, 0);
}

#[test]
fn size_and_random_size_targets_include_zero_range_draw() {
    for opcode in [0xa0, 0xac] {
        let mut program = vec![opcode, 2];
        float(&mut program, 5.0);
        if opcode == 0xac {
            float(&mut program, 0.0);
        }
        program.push(10);
        let mut p = particle(program);
        let (_, log) = tick(&mut p);
        assert_eq!(log.0.len(), usize::from(opcode == 0xac));
        assert_eq!(p.size, 1.0);
        tick(&mut p);
        assert_eq!(p.size, 3.0);
        tick(&mut p);
        assert_eq!(p.size, 5.0);
    }
}

#[test]
fn gravity_and_friction_commands_toggle_physics() {
    for (gravity, friction) in [(0.0, 1.0), (2.0, 0.5)] {
        let mut program = vec![0xa2];
        float(&mut program, gravity);
        program.push(0xa3);
        float(&mut program, friction);
        program.push(1);
        let mut p = particle(program);
        p.velocity = [4.0, 6.0, 8.0];
        tick(&mut p);
        assert_eq!(p.kind & 3, if gravity == 0.0 { 0 } else { 3 });
        assert_eq!(
            p.velocity,
            [4.0 * friction, (6.0 - gravity) * friction, 8.0 * friction]
        );
        assert_eq!(p.position, p.velocity);
    }
}

#[test]
fn random_life_and_conditional_death_use_integer_truncation() {
    let mut p = particle(vec![0xa6, 0, 7, 0, 0, 1]);
    let (_, log) = tick(&mut p);
    assert_eq!(p.life, 6);
    assert_eq!(log.0, [0x8039_A3F4]);
    let mut p = particle(vec![0xa7, 100, 0x89, 0, 0, 0, 0, 1]);
    let (alive, log) = tick(&mut p);
    assert!(!alive);
    assert_eq!(p.pc, 2);
    assert_eq!(log.0, [0x8039_A430]);
}

#[test]
fn random_position_draws_all_axes_even_with_zero_ranges() {
    let mut program = vec![0xa8];
    for _ in 0..3 {
        float(&mut program, 0.0);
    }
    program.push(1);
    let mut p = particle(program);
    p.position = [1.0, 2.0, 3.0];
    let (_, log) = tick(&mut p);
    assert_eq!(p.position, [1.0, 2.0, 3.0]);
    assert_eq!(log.0, [0x8039_A490, 0x8039_A4D4, 0x8039_A51C]);
}

#[test]
fn scalar_and_component_velocity_scales() {
    for opcode in [0xab, 0xbe] {
        let mut program = vec![opcode];
        float(&mut program, 2.0);
        if opcode == 0xbe {
            float(&mut program, 3.0);
            float(&mut program, 4.0);
        }
        program.push(1);
        let mut p = particle(program);
        p.velocity = [1.0, 2.0, 3.0];
        tick(&mut p);
        assert_eq!(
            p.velocity,
            if opcode == 0xab {
                [2.0, 4.0, 6.0]
            } else {
                [2.0, 6.0, 12.0]
            }
        );
    }
}

#[test]
fn flag_commands_preserve_unrelated_flags() {
    for (opcode, mask, expected) in [
        (0xa1, TEXTURED, 0),
        (0xad, 0x80, 0x80),
        (0xae, 0x60, 0),
        (0xaf, 0x60, 0x20),
        (0xb0, 0x60, 0x40),
        (0xb1, 0x60, 0x60),
        (0xe6, 1 << 21, 1 << 21),
        (0xe7, 1 << 21, 0),
    ] {
        let mut p = particle(vec![opcode, 1]);
        p.kind = mask | 0x100;
        tick(&mut p);
        assert_eq!(p.kind, expected | 0x100, "opcode {opcode:x}");
    }
}

#[test]
fn random_color_uses_signed_deltas_clamps_and_draws_four_times() {
    for opcode in [0xba, 0xbb] {
        let mut p = particle(vec![opcode, 0, 127, 128, 0, 1]);
        let track = if opcode == 0xba {
            &mut p.primary
        } else {
            &mut p.environment
        };
        track.target = [10, 250, 5, 128];
        let mut rng = HsdRng::new(0x12345678);
        let mut reference = rng;
        let mut expected = [10, 250, 5, 128];
        for (value, delta) in expected.iter_mut().zip([0, 254, -256, 0]) {
            let offset = delta as f32 * reference.randf();
            *value = (*value as f32 + offset).clamp(0.0, 255.0) as u8;
        }
        let mut log = DrawLog::default();
        p.update::<common::RetailTrig>(&mut rng, &mut log).unwrap();
        let track = if opcode == 0xba {
            &p.primary
        } else {
            &p.environment
        };
        assert_eq!(track.current, expected);
        assert_eq!(rng.seed, reference.seed);
        if opcode == 0xba {
            assert_eq!(log.0, PRIMARY_COLOR);
        } else {
            assert_eq!(log.0.len(), 4);
        }
    }
}

#[test]
fn random_texture_pose_and_explicit_palette() {
    let mut p = particle(vec![0xbc, 2, 0, 0xe3, 7, 1]);
    p.texture_images = vec![false, false, true].into();
    let (_, log) = tick(&mut p);
    assert_eq!((p.pose, p.palette), (2, 7));
    assert_ne!(p.kind & TEXTURED, 0);
    assert_eq!(log.0, [0x8039_B4FC]);
}

#[test]
fn texture_flip_each_mode_and_both_axes() {
    for opcode in [0xe4, 0xe5] {
        for mode in 0..4 {
            let flag = if opcode == 0xe4 { 1 << 18 } else { 1 << 19 };
            let mut p = particle(vec![opcode, mode, 1]);
            p.kind = flag;
            let (_, log) = tick(&mut p);
            assert_eq!(p.kind & flag, if mode == 1 { flag } else { 0 });
            assert_eq!(log.0.len(), usize::from(mode == 3));
        }
    }
}

#[test]
fn trail_command_changes_flag_and_retains_last_nonnegative_value() {
    let mut program = vec![0xe8];
    float(&mut program, 3.0);
    program.push(1);
    program.push(0xe8);
    float(&mut program, -1.0);
    program.push(1);
    let mut p = particle(program);
    tick(&mut p);
    assert_eq!(p.trail, 3.0);
    assert_ne!(p.kind & (1 << 20), 0);
    tick(&mut p);
    assert_eq!(p.trail, 3.0);
    assert_eq!(p.kind & (1 << 20), 0);
}

#[test]
fn rotation_randomization_handles_continuous_and_discrete_timing() {
    for timing in [0, 4] {
        let mut program = vec![0xed];
        float(&mut program, 2.0);
        float(&mut program, 0.0);
        program.extend([timing, 1]);
        let mut p = particle(program);
        let (_, log) = tick(&mut p);
        assert_eq!(p.rotation, 2.0);
        assert_eq!(p.rotation_target, 2.0);
        assert_eq!(
            log.0,
            if timing == 0 {
                vec![0x8039_C8D4]
            } else {
                vec![0x8039_C870]
            }
        );
    }
}

#[test]
fn counted_loop_and_mark_jump_preserve_program_counters_across_ticks() {
    let mut p = particle(vec![0xfa, 2, 0xab, 0x40, 0, 0, 0, 1, 0xfb, 2, 0xff]);
    p.velocity = [1.0, 0.0, 0.0];
    tick(&mut p);
    assert_eq!(p.loop_start, 2);
    assert_eq!(p.velocity[0], 2.0);
    tick(&mut p);
    assert_eq!(p.velocity[0], 4.0);
    tick(&mut p);
    assert_eq!(p.loop_count, 0);
    assert_eq!(p.wait, 2);
    let mut p = particle(vec![0xfc, 1, 0xfd]);
    for _ in 0..4 {
        assert!(tick(&mut p).0);
        assert_eq!((p.mark, p.pc, p.wait), (1, 2, 1));
    }
}

#[test]
fn both_end_opcodes_delete_before_physics_and_pause_freezes_every_timer() {
    for opcode in [0xfe, 0xff] {
        let mut p = particle(vec![opcode]);
        p.velocity = [1.0; 3];
        assert!(!tick(&mut p).0);
        assert_eq!(p.position, [0.0; 3]);
    }
    let mut p = particle(vec![0xff]);
    p.kind = PAUSED;
    p.size_timer = 3;
    let before = (p.life, p.wait, p.size_timer);
    assert!(tick(&mut p).0);
    assert_eq!((p.life, p.wait, p.size_timer), before);
    let mut p = particle(vec![]);
    p.life = 0;
    tick(&mut p);
    assert_eq!(p.life, u16::MAX);
}

#[test]
fn unported_opcodes_and_malformed_programs_fail_explicitly() {
    let supported = [
        0xa0, 0xa1, 0xa2, 0xa3, 0xa5, 0xa6, 0xa7, 0xa8, 0xa9, 0xab, 0xac, 0xad, 0xae, 0xaf, 0xb0,
        0xb1, 0xb3, 0xb6, 0xb8, 0xba, 0xbb, 0xbc, 0xbd, 0xbe, 0xe0, 0xe3, 0xe4, 0xe5, 0xe6, 0xe7,
        0xe8, 0xed, 0xef, 0xfa, 0xfb, 0xfc, 0xfd, 0xfe, 0xff,
    ];
    for opcode in 0xa0..=0xff {
        if supported.contains(&opcode) || (0xc0..0xe0).contains(&opcode) {
            continue;
        }
        let mut p = particle(vec![opcode]);
        assert_eq!(
            p.update::<common::RetailTrig>(&mut HsdRng::new(1), &mut DrawLog::default()),
            Err(Error::UnsupportedOpcode { opcode, pc: 0 })
        );
    }
    for program in [
        vec![0xa9],
        vec![0xa9, 0, 0, 0],
        vec![0xa2, 0],
        vec![0xb8],
        vec![0xb8, 0, 0, 0],
        vec![0xb3],
        vec![0xb3, 2, 0x12, 80],
    ] {
        let mut p = particle(program);
        assert!(matches!(
            p.update::<common::RetailTrig>(&mut HsdRng::new(1), &mut DrawLog::default()),
            Err(Error::TruncatedProgram { .. })
        ));
    }
    let mut p = particle(vec![0xfc, 0xfd]);
    assert!(matches!(
        p.update::<common::RetailTrig>(&mut HsdRng::new(1), &mut DrawLog::default()),
        Err(Error::InstructionLimit { .. })
    ));
}

#[test]
fn alpha_comparison_rebases_before_retargeting_and_snaps_on_zero_timer() {
    // B3 starts a 4-tick blend; two ticks later its 16.16 halfway value
    // becomes the next blend's start. A zero-duration command snaps both bytes.
    let mut p = particle(vec![
        0xb3, 4, 0x33, 101, 55, 2, 0xb3, 0x80, 4, 0x12, 201, 155, 1, 0xb3, 0, 0x34, 7, 9, 20,
    ]);
    assert!(tick(&mut p).1 .0.is_empty());
    assert_eq!(p.alpha_compare.current, [1, 255]);
    tick(&mut p);
    tick(&mut p);
    assert_eq!(p.alpha_compare.current, [51, 155]);
    assert_eq!(p.alpha_compare.remaining, 4);
    assert_eq!(p.alpha_compare_mode, 0x12);
    tick(&mut p);
    assert_eq!(p.alpha_compare.current, [7, 9]);
    assert_eq!(p.alpha_compare.duration, 0);
    assert_eq!(p.alpha_compare.remaining, 0);
    assert_eq!(p.alpha_compare_mode, 0x34);
}
