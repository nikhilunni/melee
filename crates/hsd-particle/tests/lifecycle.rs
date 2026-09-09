mod common;
use common::*;
use gekko_math::rng::HsdRng;
use hsd_particle::{
    generator::Generator,
    particle::PAUSED,
    rng_sites::{DrawLog, EMISSION_COUNT, FD_EMISSION},
    system::ParticleSystem,
    Error,
};

fn generator(d: &hsd_particle::bank::Descriptor, link: u8) -> Generator {
    Generator::new::<RetailTrig>(d, 0, link, &mut HsdRng::new(1), &mut DrawLog::default()).unwrap()
}
fn run(system: &mut ParticleSystem) -> Vec<u32> {
    let mut log = DrawLog::default();
    system
        .proc_main::<RetailTrig>(&mut HsdRng::new(1), &mut log)
        .unwrap();
    log.0
}

#[test]
fn creation_initial_count_conditions_and_no_draw_fast_paths() {
    for (kind, rate, count, draws) in [
        (0, -1.0, 0.0, 0),
        (0, 0.0, 41.0 / 65536.0, 1),
        (0x100, 0.0, f32::from_bits(0x3f7ffffe), 0),
        (0x100, -0.5, 1.0, 0),
        (0x100, -1.0, 0.0, 0),
    ] {
        let mut d = descriptor(vec![]);
        d.kind = kind;
        d.emission_rate = rate;
        let mut log = DrawLog::default();
        let g = Generator::new::<RetailTrig>(&d, 0, 0, &mut HsdRng::new(1), &mut log).unwrap();
        assert_eq!(g.count.to_bits(), count.to_bits());
        assert_eq!(log.0.len(), draws);
    }
}

#[test]
fn generator_creation_order_inserts_after_head_not_at_either_end() {
    let mut system = ParticleSystem::default();
    let d = descriptor(vec![]);
    let a = system.insert_generator(generator(&d, 0));
    let b = system.insert_generator(generator(&d, 0));
    let c = system.insert_generator(generator(&d, 0));
    assert_eq!(
        system.generators.iter().map(|g| g.id).collect::<Vec<_>>(),
        [a, c, b]
    );
    run(&mut system);
    let fourth = system.insert_generator(generator(&d, 0));
    assert_eq!(
        system.generators.iter().map(|g| g.id).collect::<Vec<_>>(),
        [a, fourth, c, b]
    );
}

#[test]
fn particle_heads_reverse_emission_order_and_keep_generator_order() {
    let mut system = ParticleSystem::default();
    let d = descriptor(vec![]);
    let a = system.insert_generator(generator(&d, 0));
    let b = system.insert_generator(generator(&d, 0));
    run(&mut system);
    assert_eq!(
        system.particles[0]
            .iter()
            .map(|p| p.generator_id)
            .collect::<Vec<_>>(),
        [Some(b), Some(a)]
    );
}

#[test]
fn expired_generator_draws_zero_rate_until_last_child_deletion_tick() {
    let mut d = descriptor(vec![10]);
    d.generator_life = 1;
    d.particle_life = 3;
    let mut system = ParticleSystem::default();
    system.insert_generator(generator(&d, 0));
    let first = run(&mut system);
    assert_eq!(first, [0x8039_E3D4]);
    assert_eq!(system.live_particles(), 1);
    assert_eq!(
        (
            system.generators[0].emission_rate,
            system.generators[0].remaining_life
        ),
        (0.0, 1)
    );
    for life in [2, 1] {
        assert_eq!(run(&mut system), [EMISSION_COUNT]);
        assert_eq!(system.particles[0][0].life, life);
    }
    assert_eq!(run(&mut system), [EMISSION_COUNT]);
    assert!(system.generators.is_empty());
    assert_eq!(system.live_particles(), 0);
    assert!(run(&mut system).is_empty());
}

#[test]
fn kill_children_on_expiry_and_u16_life_wrapping() {
    let mut d = descriptor(vec![]);
    d.generator_type = 0x80;
    d.generator_life = 1;
    let mut system = ParticleSystem::default();
    system.insert_generator(generator(&d, 0));
    run(&mut system);
    assert!(system.generators.is_empty());
    assert_eq!(system.live_particles(), 0);
    d.generator_type = 0;
    d.generator_life = 0;
    d.particle_life = u16::MAX;
    system.insert_generator(generator(&d, 0));
    run(&mut system);
    assert_eq!(system.particles[0][0].life, u16::MAX);
}

#[test]
fn masks_and_pause_freeze_count_lifetime_and_rng() {
    let mut d = descriptor(vec![]);
    d.emission_rate = 10.0;
    d.generator_life = 8;
    let mut system = ParticleSystem::default();
    let id = system.insert_generator(generator(&d, 1));
    let before = system.generators[0].count;
    assert!(run(&mut system).is_empty());
    assert_eq!(system.generators[0].count, before);
    assert_eq!(system.generators[0].remaining_life, 8);
    let g = system.generator_mut(id).unwrap();
    g.link = 0;
    g.descriptor.kind |= PAUSED;
    assert!(run(&mut system).is_empty());
    assert_eq!(system.generators[0].remaining_life, 8);
}

#[test]
fn main_aux_masks_reproduce_double_update_of_links_three_through_seven() {
    let d = descriptor(vec![]);
    let mut system = ParticleSystem::default();
    for link in 0..4 {
        system.insert_generator(generator(&d, link));
    }
    let mut rng = HsdRng::new(1);
    let mut log = DrawLog::default();
    system.proc_main::<RetailTrig>(&mut rng, &mut log).unwrap();
    assert_eq!(system.particles.each_ref().map(Vec::len)[..4], [1, 0, 0, 1]);
    system.proc_aux::<RetailTrig>(&mut rng, &mut log).unwrap();
    assert_eq!(system.particles.each_ref().map(Vec::len)[..4], [1, 1, 1, 2]);
    assert_eq!(system.particles[3][1].life, 99);
}

#[test]
fn sphere_draw_conditions_and_immediate_color_order() {
    for (latitude, radius, negative_angle, expected_geometry) in [
        (0.5, -2.0, false, 2),
        (0.0, -2.0, false, 3),
        (std::f32::consts::PI, 2.0, false, 4),
        (0.5, 2.0, true, 4),
    ] {
        let mut d = descriptor(vec![0xba, 0, 0, 0, 0, 1]);
        d.generator_type = 8;
        d.parameters = [latitude, 0.0, 0.0];
        d.radius = radius;
        d.angle = if negative_angle { -0.5 } else { 0.5 };
        let mut system = ParticleSystem::default();
        system.insert_generator(generator(&d, 0));
        let draws = run(&mut system);
        assert_eq!(draws.len(), expected_geometry + 4);
        if latitude == 0.5 && radius < 0.0 {
            assert_eq!(draws, FD_EMISSION);
        }
    }
}

#[test]
fn disc_line_and_cone_shape_helpers_have_conditional_draws() {
    for shape in [0, 1, 3, 4, 6, 7] {
        for negative_angle in [false, true] {
            for negative_radius in [false, true] {
                let mut d = descriptor(vec![]);
                d.generator_type = shape;
                d.angle = if negative_angle { -0.25 } else { 0.25 };
                d.radius = if negative_radius { -2.0 } else { 2.0 };
                d.parameters = [0.0, 1.0, 3.0];
                d.velocity = [0.0, 0.0, 1.0];
                let mut system = ParticleSystem::default();
                system.insert_generator(generator(&d, 0));
                let draws = run(&mut system);
                let expected = if shape == 1 {
                    1 + usize::from(negative_angle)
                } else {
                    1 + usize::from(!negative_radius) + usize::from(shape == 6 || shape == 7)
                };
                assert_eq!(draws.len(),expected,"shape{shape}, negative angle{negative_angle}, negative radius{negative_radius}");
                assert_eq!(system.live_particles(), 1);
                assert!(system.particles[0][0]
                    .position
                    .iter()
                    .all(|v| v.is_finite()));
            }
        }
    }
}

#[test]
fn pool_exhaustion_keeps_geometry_draws_but_skips_particle_script() {
    let mut d = descriptor(vec![0xba, 0, 0, 0, 0, 1]);
    d.generator_type = 8;
    d.parameters = [0.5, 0.0, 0.0];
    let mut system = ParticleSystem::default();
    system.particle_capacity = 0;
    system.insert_generator(generator(&d, 0));
    assert_eq!(run(&mut system), FD_EMISSION[..2]);
    assert_eq!(system.generators[0].children, 0);
}

#[test]
fn unsupported_shapes_and_app_srt_are_explicit_errors() {
    for shape in [2, 5, 9, 15] {
        let mut d = descriptor(vec![]);
        d.generator_type = shape;
        assert!(
            matches!(Generator::new::<RetailTrig>(&d,0,0,&mut HsdRng::new(1),&mut DrawLog::default()),Err(Error::UnsupportedGenerator{shape:s}) if s==shape)
        );
    }
    let mut d = descriptor(vec![]);
    d.kind = 0x20000;
    assert!(matches!(
        Generator::new::<RetailTrig>(&d, 0, 0, &mut HsdRng::new(1), &mut DrawLog::default()),
        Err(Error::UnsupportedFeature(_))
    ));
}

#[test]
fn restored_render_tracks_keep_current_bytes_until_countdown_completes() {
    use hsd_particle::particle::{BytePairTrack, Particle};
    let mut particle = Particle::new(&descriptor(vec![]), 0, 0).unwrap();
    let track = BytePairTrack {
        current: [7, 19],
        target: [113, 211],
        duration: 2,
        remaining: 2,
    };
    particle.material = track.clone();
    particle.ambient = track.clone();
    particle.alpha_compare = track;
    let mut rng = HsdRng::new(123);
    let mut log = DrawLog::default();
    particle.kind |= PAUSED;
    assert!(particle
        .update::<common::RetailTrig>(&mut rng, &mut log)
        .unwrap());
    assert_eq!(particle.material.remaining, 2);
    particle.kind &= !PAUSED;
    for remaining in [1, 0] {
        assert!(particle
            .update::<common::RetailTrig>(&mut rng, &mut log)
            .unwrap());
        for track in [
            &particle.material,
            &particle.ambient,
            &particle.alpha_compare,
        ] {
            assert_eq!(track.remaining, remaining);
            assert_eq!(track.duration, if remaining == 0 { 0 } else { 2 });
            assert_eq!(
                track.current,
                if remaining == 0 { [113, 211] } else { [7, 19] }
            );
        }
    }
    assert_eq!(rng.seed, 123);
    assert!(log.0.is_empty());
}

#[test]
fn alpha_compare_command_materializes_old_interpolation_before_restarting() {
    use hsd_particle::particle::{BytePairTrack, Particle};
    // B3 duration 2, mode 0x12, targets 80/100; then wait 10 ticks.
    let d = descriptor(vec![0xB3, 2, 0x12, 80, 100, 10]);
    let mut particle = Particle::new(&d, 0, 0).unwrap();
    particle.alpha_compare = BytePairTrack {
        current: [20, 240],
        target: [100, 40],
        duration: 4,
        remaining: 3,
    };
    let mut rng = HsdRng::new(1);
    let mut draws = DrawLog::default();
    particle
        .update::<common::RetailTrig>(&mut rng, &mut draws)
        .unwrap();
    assert_eq!(particle.alpha_compare.current, [60, 140]);
    assert_eq!(particle.alpha_compare.remaining, 2);
    assert_eq!(particle.alpha_compare_mode, 0x12);
    particle
        .update::<common::RetailTrig>(&mut rng, &mut draws)
        .unwrap();
    assert_eq!(particle.alpha_compare.current, [60, 140]);
    particle
        .update::<common::RetailTrig>(&mut rng, &mut draws)
        .unwrap();
    assert_eq!(particle.alpha_compare.current, [80, 100]);
    assert_eq!(particle.alpha_compare.duration, 0);
    assert!(draws.0.is_empty());
    assert_eq!(rng.seed, 1);
}
