//! Production melee-sim inputs, captured by running the jump gate after M4-T3.
//! Instrument ParticleSystem::spawn/update_joint at the effect boundary with
//! f32::to_bits and label calls with Simulation::tick's frame; remove logging
//! afterwards. The JSON retains only joints referenced by external requests.
//! These are five spawn requests plus attachment matrices, never particle outputs.
//! Child descriptors, velocities, lifetimes and bytecode remain archive-derived.
use crate::common::RetailTrig;
use gekko_math::HsdRng;
use hsd_particle::{
    bank::ParticleBank,
    rng_sites::DrawLog,
    system::{ParticleSystem, SpawnRequest},
};
use hsd_types::Mtx;
use std::collections::BTreeMap;

pub struct Spawns(serde_json::Value);
impl Spawns {
    pub fn new() -> Self {
        Self(serde_json::from_str(include_str!("jump_fd_spawns.json")).unwrap())
    }
    pub fn after_particles(
        &mut self,
        _tick: usize,
        _system: &mut ParticleSystem,
        _banks: &BTreeMap<u8, ParticleBank>,
        _rng: &mut HsdRng,
        _draws: &mut DrawLog,
    ) {
    }
    pub fn before_main(
        &mut self,
        tick: usize,
        system: &mut ParticleSystem,
        banks: &BTreeMap<u8, ParticleBank>,
        rng: &mut HsdRng,
        draws: &mut DrawLog,
    ) {
        let Some(events) = self.0.get(tick.to_string()).and_then(|v| v.as_array()) else {
            return;
        };
        for event in events {
            let joint = event["joint"].as_u64().unwrap() as usize;
            let matrix = Mtx(std::array::from_fn(|row| {
                std::array::from_fn(|col| {
                    f32::from_bits(event["matrix"][row][col].as_u64().unwrap() as u32)
                })
            }));
            if let Some(kind) = event.get("spawn") {
                let mut request = SpawnRequest::new(0, kind.as_u64().unwrap() as u32, 0);
                request.joint = Some((joint, matrix));
                system
                    .spawn::<RetailTrig>(&banks[&0], request, rng, draws)
                    .unwrap();
            } else {
                system.update_joint(joint, matrix);
            }
        }
    }
}
