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
    grafted: Option<super::GraftedPart>,
) {
    let mut offset = hit.descriptor.offset;
    if hit.descriptor.ignore_scale {
        let inverse = 1.0 / scale;
        offset = Vec3::new(offset.x * inverse, offset.y * inverse, offset.z * inverse);
    }
    // lb_8000B1CC with a NULL jobj returns the offset itself.
    let position = match grafted {
        _ if hit.world => offset,
        Some(grafted) if grafted.part == hit.descriptor.bone => {
            // lb_8000B1CC on the article's model: only its translation is
            // kept, which a zero offset reads alone.
            assert!(
                offset == Vec3::ZERO,
                "lb_8000B1CC: an offset capsule on a grafted part"
            );
            grafted.position
        }
        // ftAction_80071708 installs fp->parts[bone].joint unchecked; the
        // parts pool holds MAX_FT_PARTS bones, and those past the kind's
        // parts_num hold no joint unless an article's model fills one (Link's
        // and Young Link's Catch box names bone 139).
        _ if hit.descriptor.bone >= animation.parts.len() => offset,
        _ => part_position(tree, animation, hit.descriptor.bone, offset),
    };
    hit.update_position(position);
}
