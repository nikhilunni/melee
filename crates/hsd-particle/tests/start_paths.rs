//! Regression coverage for the match-start paths, with synthetic bank data.
mod common;
use common::{descriptor, float, RetailTrig};
use gekko_math::rng::HsdRng;
use hsd_particle::{
    bank::ParticleBank,
    particle::Particle,
    rng_sites::DrawLog,
    system::{ParticleSystem, SpawnRequest},
};
use hsd_types::Mtx;

fn bank(descriptors: Vec<hsd_particle::bank::Descriptor>) -> ParticleBank {
    ParticleBank {
        version: 0x42,
        first_descriptor_id: 0,
        descriptors: descriptors.into_iter().map(Some).collect(),
        textures: vec![],
    }
}

#[test]
fn spawn_request_sets_owned_parameters_after_descriptor_initialization() {
    let mut d = descriptor(vec![10]);
    d.generator_type = 8;
    d.velocity = [0.0, 0.0, 2.0];
    d.emission_rate = 0.0;
    let bank = bank(vec![d]);
    let mut system = ParticleSystem::default();
    let mut request = SpawnRequest::new(3, 0, 1);
    request.position = [4.0, 5.0, 6.0];
    request.velocity = Some([7.0, 8.0, 9.0]);
    request.joint = Some((42, Mtx::IDENTITY));
    let mut rng = HsdRng::new(1);
    let mut draws = DrawLog::default();
    let id = system
        .spawn::<RetailTrig>(&bank, request, &mut rng, &mut draws)
        .unwrap()
        .unwrap();
    let generator = system.generator_mut(id).unwrap();
    assert_eq!(generator.position, [4.0, 5.0, 6.0]);
    assert_eq!(generator.descriptor.velocity, [7.0, 8.0, 9.0]);
    assert_eq!(
        (
            generator.bank,
            generator.link,
            generator.family_id,
            generator.flags
        ),
        (3, 1, 257, 0x708)
    );
    assert!(matches!(
        generator.shape,
        hsd_particle::generator::EmissionShape::Sphere { speed: 2.0, .. }
    ));
    assert_eq!(draws.0, [0x8039_f250]);
    assert_eq!(rng.seed, 2_745_024);
    assert!(system
        .spawn::<RetailTrig>(&bank, SpawnRequest::new(65, 0, 0), &mut rng, &mut draws)
        .unwrap()
        .is_none());
    assert_eq!(system.family_counter, 257);
    assert_eq!(draws.0.len(), 1);
}

#[test]
fn a5_and_ef_children_run_in_current_emission_pass_after_parent_dies() {
    for opcode in [0xa5, 0xef] {
        let mut program = vec![opcode, 0, 1];
        if opcode == 0xef {
            program.push(7);
        }
        program.push(0xff);
        let mut parent = descriptor(program);
        parent.particle_life = 0;
        parent.generator_life = 2;
        let mut child = descriptor(vec![10]);
        child.generator_life = 1;
        child.particle_life = 2;
        let bank = bank(vec![parent, child]);
        let mut system = ParticleSystem::default();
        let mut request = SpawnRequest::new(0, 0, 0);
        request.joint = Some((99, Mtx::IDENTITY));
        let mut rng = HsdRng::new(1);
        let mut draws = DrawLog::default();
        system
            .spawn::<RetailTrig>(&bank, request, &mut rng, &mut draws)
            .unwrap();
        system
            .proc_main::<RetailTrig>(&mut rng, &mut draws)
            .unwrap();
        assert_eq!(draws.0, [0x8039_e3d4, 0x8039_e3d4]);
        assert_eq!(system.family_counter, 258); // allocate even though family is inherited
        assert_eq!(system.live_particles(), 1);
        assert_eq!(system.particles[0][0].family_id, 257);
        assert_eq!(system.generators[0].children, 0);
        let child = &system.generators[1];
        assert_eq!(
            (child.family_id, child.remaining_life, child.children),
            (257, 1, 1)
        );
        assert_eq!(child.emission_rate, 0.0);
        assert_eq!(child.attachment_id, Some(99));
        assert_eq!(child.flags, 0x700);
        assert_eq!(
            child.descriptor.kind >> 25,
            if opcode == 0xef { 7 } else { 0 }
        );
    }
}

#[test]
fn b6_rotation_retargets_then_interpolates_without_rng() {
    let mut program = vec![0xb6, 2];
    float(&mut program, 4.0);
    program.extend([2, 0xb6, 0]);
    float(&mut program, -1.0);
    program.push(10);
    let mut p = Particle::new(&descriptor(program), 0, 0).unwrap();
    p.rotation = 2.0;
    p.rotation_target = 2.0;
    let mut rng = HsdRng::new(1);
    let mut draws = DrawLog::default();
    for (rotation, target, timer) in [(2.0, 6.0, 2), (4.0, 6.0, 1), (5.0, 5.0, 0)] {
        p.update(&mut rng, &mut draws).unwrap();
        assert_eq!(
            (p.rotation, p.rotation_target, p.rotation_timer),
            (rotation, target, timer)
        );
    }
    assert!(draws.0.is_empty());
}

#[test]
fn bd_normalizes_nonzero_velocity_and_draws_even_when_zero() {
    for (velocity, expected) in [([3.0, 4.0, 0.0], [6.0, 8.0, 0.0]), ([0.0; 3], [0.0; 3])] {
        let mut program = vec![0xbd];
        float(&mut program, 10.0);
        float(&mut program, 0.0);
        program.push(10);
        let mut p = Particle::new(&descriptor(program), 0, 0).unwrap();
        p.velocity = velocity;
        let mut draws = DrawLog::default();
        p.update(&mut HsdRng::new(1), &mut draws).unwrap();
        assert_eq!(p.velocity.map(f32::to_bits), expected.map(f32::to_bits));
        assert_eq!(p.position, expected);
        assert_eq!(draws.0, [0x8039_b5e0]);
    }
}

#[test]
fn e0_shares_four_random_deltas_between_color_tracks() {
    let mut p = Particle::new(&descriptor(vec![0xe0, 0, 127, 128, 0, 10]), 0, 0).unwrap();
    p.primary.target = [10, 250, 5, 128];
    p.environment.target = [50, 0, 100, 0];
    let mut draws = DrawLog::default();
    p.update(&mut HsdRng::new(1), &mut draws).unwrap();
    assert_eq!(
        draws.0,
        [0x8039_bb28, 0x8039_bbe4, 0x8039_bca0, 0x8039_bd5c]
    );
    assert_eq!(p.primary.current, [10, 255, 0, 128]);
    assert_eq!(p.environment.current, [50, 198, 75, 0]);
}

#[test]
fn display_sort_keeps_equal_buckets_stable_and_obeys_link_mask() {
    let mut system = ParticleSystem::default();
    for (tag, blend, edge) in [(1, 7, false), (2, 4, false), (3, 7, false), (4, 7, true)] {
        let mut p = Particle::new(&descriptor(vec![10]), 0, 0).unwrap();
        p.family_id = tag;
        p.kind = blend << 25 | if edge { 8 } else { 0 };
        system.particles[0].push(p);
    }
    system.sort_for_display(2); // link 0 excluded
    assert_eq!(system.particles[0][0].family_id, 1);
    system.sort_for_display(1);
    assert_eq!(
        system.particles[0]
            .iter()
            .map(|p| p.family_id)
            .collect::<Vec<_>>(),
        [4, 2, 1, 3]
    );
}

#[test]
fn full_sphere_draws_latitude_sign_and_azimuth_then_retains_expired_parent() {
    let mut d = descriptor(vec![10]);
    d.generator_type = 8;
    d.parameters = [std::f32::consts::PI, 0.0, 0.0];
    d.velocity = [0.0, -6.0, 0.0];
    d.generator_life = 1;
    d.particle_life = 2;
    let bank = bank(vec![d]);
    let mut system = ParticleSystem::default();
    let mut rng = HsdRng::new(1);
    let mut draws = DrawLog::default();
    system
        .spawn::<RetailTrig>(&bank, SpawnRequest::new(0, 0, 0), &mut rng, &mut draws)
        .unwrap();
    system
        .proc_main::<RetailTrig>(&mut rng, &mut draws)
        .unwrap();
    assert_eq!(draws.0, [0x8039_eb04, 0x8039_eb5c, 0x8039_ebcc]);
    assert_eq!(system.generators[0].remaining_life, 1);
    assert_eq!(system.generators[0].emission_rate, 0.0);
    for remaining in [1, 0] {
        draws.0.clear();
        system
            .proc_main::<RetailTrig>(&mut rng, &mut draws)
            .unwrap();
        assert_eq!(draws.0, [0x8039_ef00]);
        assert_eq!(system.live_particles(), remaining);
    }
    assert!(system.generators.is_empty());
}
