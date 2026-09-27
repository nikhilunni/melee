//! Fighter bone sampling for shared attack capsules.
use super::caches::part_position;
use crate::anim::playback::FighterAnimation;
use hsd_anim::jobj::JObjTree;
use hsd_types::Vec3;
pub fn update(
    hit: &mut melee_coll::hitbox::HitCapsule,
    tree: &mut JObjTree,
    animation: &FighterAnimation,
    scale: f32,
) {
    let mut offset = hit.descriptor.offset;
    if hit.descriptor.ignore_scale {
        let inverse = 1.0 / scale;
        offset = Vec3::new(offset.x * inverse, offset.y * inverse, offset.z * inverse);
    }
    let position = part_position(tree, animation, hit.descriptor.bone, offset);
    hit.update_position(position);
}
