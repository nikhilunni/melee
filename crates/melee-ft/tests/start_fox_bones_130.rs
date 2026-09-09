//! Strict tick-boundary SRT and separately aligned post-render matrices.
mod fighter_support;
#[test]
fn start_fox_bones_130() {
    fighter_support::replay::replay("start", 130, true);
}
#[test]
fn idle_fox_bones_8() {
    fighter_support::replay::replay("idle", 8, true);
}

#[test]
fn excludes_only_unused_euler_w() {
    use fighter_support::replay::compared_component;
    use hsd_anim::jobj::JOBJ_USE_QUATERNION;
    assert!(!compared_component(0, "rotate", 3));
    assert!(compared_component(JOBJ_USE_QUATERNION, "rotate", 3));
    for flags in [0, JOBJ_USE_QUATERNION] {
        for (field, count) in [("mtx", 12), ("rotate", 3), ("scale", 3), ("translate", 3)] {
            for index in 0..count {
                assert!(
                    compared_component(flags, field, index),
                    "{field}[{index}] flags={flags:X}"
                );
            }
        }
    }
}

#[test]
fn start_fox_matrices_vi_130() {
    fighter_support::rendered_pose::compare_start();
}
