//! it_802AE63C / it_802AE200: opening and recoil offsets sampled before
//! advancing the item counters. These tables are shared by Fox and Falco.
use hsd_anim::jobj::{JObjId, JObjTree};
use melee_it::{ItemCore, ItemScratch};
pub(super) fn blaster(item: &ItemCore, tree: &mut JObjTree) -> bool {
    let ItemScratch::Held(state) = &item.scratch else {
        return false;
    };
    let mut joint = JObjId(0);
    for _ in 0..4 {
        joint = tree.get(joint).child.expect("blaster joint hierarchy");
    }
    const OPENING: [f32; 5] = [-0.425, -0.595, -0.765, -0.935, -0.85];
    let mut position = tree.get(joint).translate;
    position.y = OPENING[state.opening_pose_frame];
    tree.set_translate(joint, &position);
    let recoil = tree.get(joint).next.expect("blaster recoil joint");
    const RECOIL: [f32; 14] = [
        0.0, -0.51, -1.02, -1.53, -1.39, -1.251, -1.112, -0.973, -0.834, -0.695, -0.556, -0.417,
        -0.278, -0.139,
    ];
    position = tree.get(recoil).translate;
    position.z = RECOIL[state.recoil_pose_frame];
    tree.set_translate(recoil, &position);
    let barrel = tree.get(recoil).next.expect("blaster barrel joint");
    const SCALE: [f32; 14] = [
        0.5, 0.5, 1.75, 3.0, 2.375, 1.75, 1.125, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5,
    ];
    let mut scale = tree.get(barrel).scale;
    scale.y = SCALE[state.recoil_pose_frame];
    scale.z = scale.y;
    tree.set_scale(barrel, &scale);
    let flap = tree.get(barrel).next.expect("blaster flap joint");
    const ROTATION: [f32; 14] = [
        0.0, -42.0, -20.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];
    tree.set_rotation_x(flap, ROTATION[state.recoil_pose_frame]);
    state.visibility == 1
}
