//! Inputs extracted from the production melee-sim dash replay at the
//! Effect -> ParticleSystem::spawn boundary at 19938f5. Children come from bytecode.
//! Reproduce with the dash gate, logging SpawnRequest immediately before the
//! spawn calls in effects/dust.rs and Effect::animate (float words via to_bits).
//! The three requests are at ticks 34/49/56; this fixture contains no emitted
//! positions, velocities, generator fields or particle bytecode outputs.
use crate::common::RetailTrig;
use gekko_math::rng::HsdRng;
use hsd_particle::{
    bank::ParticleBank,
    generator::ApplicationTransform,
    rng_sites::DrawLog,
    system::{ParticleSystem, SpawnRequest},
};
use hsd_types::{Mtx, Vec3};
use std::collections::BTreeMap;

pub struct Spawns;
impl Spawns {
    pub fn new() -> Self {
        Self
    }
    pub fn before_main(
        &mut self,
        tick: usize,
        system: &mut ParticleSystem,
        banks: &BTreeMap<u8, ParticleBank>,
        rng: &mut HsdRng,
        draws: &mut DrawLog,
    ) {
        // Dash's model effect is destroyed when Run is entered.
        if tick == 42 {
            system.expire_joint(65538);
        }
        let (kind, translation_x) = match tick {
            34 => (9, None),
            49 => (263, Some(3248823986)),
            56 => (90, Some(3232382294)),
            _ => return,
        };
        let mut request = SpawnRequest::new(0, kind, 0);
        if let Some(x) = translation_x {
            request.application_transform = Some(ApplicationTransform {
                translation: Vec3::new(f32::from_bits(x), f32::from_bits(953267991), 0.0),
                rotation: Vec3::new(0.0, std::f32::consts::FRAC_PI_2, 0.0),
                scale: Vec3::new(1.0, 1.0, 1.0),
                status: 1,
                ..Default::default()
            });
        } else {
            request.joint = Some((
                65538,
                Mtx([
                    [3003349086, 0, 1057803469, 3257687954],
                    [0, 1057803469, 0, 953267991],
                    [3205287117, 2147483648, 3003349086, 3038613672],
                ]
                .map(|row| row.map(f32::from_bits))),
            ));
        }
        system
            .spawn::<RetailTrig>(&banks[&0], request, rng, draws)
            .unwrap();
    }
}
