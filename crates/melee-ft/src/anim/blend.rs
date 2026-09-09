//! Blend timing and SRT transfer (`ftanim.c` and `lb_00B0.c`).
use gekko_math::fma::fmadds;
use hsd_anim::jobj::{JObj, JObjId, JObjTree, JOBJ_MTX_DIRTY, JOBJ_USE_QUATERNION};
use hsd_anim::mtx::InverseTrig;
use hsd_anim::quat::{euler_to_quat, quat_slerp, Quaternion};
use hsd_types::Vec3;

/// `ftAnim_8006E9B4` 0x8006EA70..0x8006EAB0 and `ftAnim_800707B0`:
/// single-precision add/subtract/divide, no fused sites (asm.py --fused).
/// Progress saturates, but duration stays nonzero after blending completes.
pub fn advance_blend(duration: f32, progress: &mut f32, rate: f32) -> (f32, f32) {
    *progress += rate;
    if duration <= *progress {
        *progress = duration;
        (1.0, 0.0)
    } else {
        let remaining = duration - *progress;
        let weight = rate / (rate + remaining);
        (weight, 1.0 - weight)
    }
}

/// `lbCopyJObjSRT` (`lb_00B0.c`, 0x8000C7BC).
pub fn copy_pose(source: &JObj, tree: &mut JObjTree, destination: JObjId) {
    let joint = tree.get_mut(destination);
    joint.translate = source.translate;
    joint.scale = source.scale;
    joint.rotate = source.rotate;
    joint.flags = (joint.flags & !JOBJ_USE_QUATERNION) | (source.flags & JOBJ_USE_QUATERNION);
    tree.set_flags(destination, JOBJ_MTX_DIRTY);
}

/// `lb_8000C490` (0x8000C490): target weight first, existing pose second.
/// The destination aliases the second input in fighter playback.
pub fn blend_pose<T: InverseTrig>(
    source: &JObj,
    tree: &mut JObjTree,
    id: JObjId,
    weight: f32,
    inverse: f32,
) {
    let old = tree.get(id);
    // retail 0x8000C4BC/0x8000C4C0, ...C4D4, ...C4E8, ...C4FC,
    // ...C510, ...C524: existing * inverse rounds BEFORE fmadds(target, weight, product).
    let mix = |a, b| fmadds(a, weight, b * inverse);
    let translate = Vec3::new(
        mix(source.translate.x, old.translate.x),
        mix(source.translate.y, old.translate.y),
        mix(source.translate.z, old.translate.z),
    );
    let scale = Vec3::new(
        mix(source.scale.x, old.scale.x),
        mix(source.scale.y, old.scale.y),
        mix(source.scale.z, old.scale.z),
    );
    let (rotation, quaternion) = blend_rotation::<T>(source, old, inverse);
    let joint = tree.get_mut(id);
    joint.translate = translate;
    joint.scale = scale;
    joint.rotate = rotation;
    if quaternion {
        tree.set_flags(id, JOBJ_USE_QUATERNION);
    } else {
        tree.clear_flags(id, JOBJ_USE_QUATERNION);
    }
    tree.set_flags(id, JOBJ_MTX_DIRTY);
}

fn blend_rotation<T: InverseTrig>(a: &JObj, b: &JObj, inverse: f32) -> (Quaternion, bool) {
    let is_quat = |j: &JObj| j.flags & JOBJ_USE_QUATERNION != 0;
    // ABS macro retains NaNs and negative zero; no std libm substitution.
    let abs = |x: f32| if x < 0.0 { -x } else { x };
    if !is_quat(a)
        && !is_quat(b)
        && abs(a.rotate.x - b.rotate.x) <= 0.0001
        && abs(a.rotate.y - b.rotate.y) <= 0.0001
        && abs(a.rotate.z - b.rotate.z) <= 0.0001
    {
        return (a.rotate, false);
    }
    let rotation = |j: &JObj| {
        let mut q = j.rotate;
        if !is_quat(j) {
            euler_to_quat(&Vec3::new(q.x, q.y, q.z), &mut q);
        }
        q
    };
    let a = rotation(a);
    let mut b = rotation(b);
    let square = |x: f32| x * x;
    // retail 0x8000C6D4..0x8000C750: separate squares and left-associated
    // fadds; --fused shows only the six translation/scale instructions above.
    let sums = ((square(a.x + b.x) + square(a.y + b.y)) + square(a.z + b.z)) + square(a.w + b.w);
    let differences =
        ((square(a.x - b.x) + square(a.y - b.y)) + square(a.z - b.z)) + square(a.w - b.w);
    if differences > sums {
        b = Quaternion::new(-b.x, -b.y, -b.z, -b.w);
    }
    let mut out = Quaternion::default();
    quat_slerp::<T>(&a, &b, &mut out, inverse);
    (out, true)
}
