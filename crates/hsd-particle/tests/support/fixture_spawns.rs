//! Effect-boundary inputs logged from each production melee-sim scenario gate.
//! Float inputs are stored as words; particle outputs and child spawns are never
//! fixtures. See tests/data/README.md for capture provenance and reproduction.
use crate::common::RetailTrig;
use gekko_math::HsdRng;
use hsd_particle::{
    bank::ParticleBank,
    rng_sites::DrawLog,
    system::{ParticleSystem, SpawnRequest},
};
use hsd_types::{Mtx, Vec3};
use serde_json::Value;
use std::collections::BTreeMap;

pub struct Spawns(Value);
impl Spawns {
    pub fn new() -> Self {
        Self(serde_json::from_str(crate::SPAWN_FIXTURE).unwrap())
    }
    pub fn before_main(
        &mut self,
        tick: usize,
        system: &mut ParticleSystem,
        banks: &BTreeMap<u8, ParticleBank>,
        rng: &mut HsdRng,
        draws: &mut DrawLog,
    ) {
        let Some(events) = self.0.get(tick.to_string()).and_then(Value::as_array) else {
            return;
        };
        for event in events {
            if let Some(kind) = event.get("spawn") {
                let bank = word(&event["bank"]) as u8;
                let mut request = SpawnRequest::new(bank, word(kind), word(&event["link"]) as u8);
                request.position = std::array::from_fn(|axis| float(&event["position"][axis]));
                if !event["joint"].is_null() {
                    request.joint = Some((
                        word(&event["joint"]["id"]) as usize,
                        matrix(&event["joint"]["matrix"]),
                    ));
                }
                if let Some(t) = event.get("transform").filter(|t| !t.is_null()) {
                    request.application_transform =
                        Some(hsd_particle::generator::ApplicationTransform {
                            translation: vector(&t["translation"]),
                            rotation: vector(&t["rotation"]),
                            scale: vector(&t["scale"]),
                            status: t["status"].as_i64().unwrap() as i32,
                            ..Default::default()
                        });
                }
                request.mirror = event
                    .get("mirror")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                system
                    .spawn::<RetailTrig>(&banks[&bank], request, rng, draws)
                    .unwrap();
            } else if let Some(joint) = event.get("update") {
                system.update_joint(word(joint) as usize, matrix(&event["matrix"]));
            } else if let Some(joint) = event.get("expire") {
                system.expire_joint(word(joint) as usize);
            } else if let Some(joint) = event.get("flags_joint") {
                // efLib_SpawnParticleEffect: the caller adjusts the just-created
                // shield generator before either particle proc runs.
                let generator = system
                    .generators
                    .iter_mut()
                    .find(|g| g.attachment_id == Some(word(joint) as usize))
                    .unwrap();
                generator.flags = (generator.flags & !(word(&event["clear"]) as u16))
                    | word(&event["set"]) as u16;
            } else {
                panic!("unknown effect input {event}");
            }
        }
    }
}
fn word(value: &Value) -> u32 {
    value.as_u64().unwrap().try_into().unwrap()
}
fn float(value: &Value) -> f32 {
    f32::from_bits(word(value))
}
fn matrix(value: &Value) -> Mtx {
    Mtx(std::array::from_fn(|row| {
        std::array::from_fn(|col| float(&value[row][col]))
    }))
}

fn vector(v: &Value) -> Vec3 {
    Vec3::new(float(&v[0]), float(&v[1]), float(&v[2]))
}
