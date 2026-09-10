#![allow(dead_code)]
use hsd_particle::bank::Descriptor;
pub struct RetailTrig;
impl hsd_anim::mtx::InverseTrig for RetailTrig {
    fn atan2f(y: f32, x: f32) -> f32 {
        melee_lb::trigf::atan2f(y, x)
    }
    fn asinf(x: f32) -> f32 {
        melee_lb::trigf::asinf(x)
    }
    fn acosf(x: f32) -> f32 {
        melee_lb::trigf::acosf(x)
    }
}
pub fn descriptor(program: Vec<u8>) -> Descriptor {
    Descriptor {
        generator_type: 0,
        texture_group: 0,
        generator_life: 0,
        particle_life: 100,
        kind: 0,
        gravity: 0.0,
        friction: 1.0,
        velocity: [0.0; 3],
        radius: -1.0,
        angle: 0.0,
        emission_rate: -1.0,
        size: 1.0,
        parameters: [0.0; 3],
        program: program.into(),
    }
}
pub fn float(program: &mut Vec<u8>, value: f32) {
    program.extend(value.to_be_bytes());
}
