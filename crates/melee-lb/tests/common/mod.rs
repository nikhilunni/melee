use hsd_anim::jobj::{JObjId, JObjTree};
use hsd_anim::mtx::InverseTrig;

pub struct RetailTrig;
impl InverseTrig for RetailTrig {
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

/// Scale, Euler rotation, translation, as bits (including signed zeros).
pub fn srt(tree: &JObjTree, id: JObjId) -> [u32; 9] {
    let j = tree.get(id);
    [
        j.scale.x,
        j.scale.y,
        j.scale.z,
        j.rotate.x,
        j.rotate.y,
        j.rotate.z,
        j.translate.x,
        j.translate.y,
        j.translate.z,
    ]
    .map(f32::to_bits)
}
