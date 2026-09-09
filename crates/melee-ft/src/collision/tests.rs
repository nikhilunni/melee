use super::{
    ecb,
    ground::{map_wait, EnvironmentCollision, WaitGroundResult},
};
use crate::{desc::EcbBones, physics::FighterPhysics};
use hsd_anim::jobj::{JObjTree, JointSpec};
use hsd_types::{Vec2, Vec3};
use melee_mp::{set_ecb_source_fixed, CollMap, CollMapBuilder};
use melee_types::{
    mp::{coll_data_x130, line_flag, SurfaceData},
    GrKind,
};

// Same geometry as melee-mp::tests::fd_data (85.5-wide floor, closed walls
// and bottom, separate platform); kept here to test the fighter wrapper.
fn fd() -> CollMap {
    let mut b = CollMapBuilder::new();
    let left = b.vertex(0, -85.5, 0.0);
    let right = b.vertex(0, 85.5, 0.0);
    let bottom_right = b.vertex(0, 85.5, -50.0);
    let bottom_left = b.vertex(0, -85.5, -50.0);
    b.floor(0, left, right, line_flag::LEDGE as u16);
    b.right_wall(0, right, bottom_right, 0);
    b.ceiling(0, bottom_right, bottom_left, 0);
    b.left_wall(0, bottom_left, left, 0);
    let a = b.vertex(1, -20.0, 30.0);
    let z = b.vertex(1, 20.0, 30.0);
    b.floor(1, a, z, line_flag::PLATFORM as u16);
    CollMap::load(b.build(), 1.0, GrKind::Last)
}
fn bones() -> EcbBones {
    EcbBones {
        joints: [1, 2, 3, 4, 5, 6],
        center_y: 0.5,
        ledge_snap_x: 6.0,
        ledge_snap_y: 8.0,
        ledge_snap_height: 10.0,
    }
}

#[test]
fn ecb_uses_six_world_bones_and_player_scale() {
    let map = fd();
    let mut tree = JObjTree::new();
    let root = tree.load_joint(
        &JointSpec::new()
            .position(20.0, 4.0, 0.0)
            .scale(2.0, 2.0, 2.0)
            .children(
                [
                    (-3.0, 1.0),
                    (3.0, 2.0),
                    (0.0, 10.0),
                    (-2.0, 4.0),
                    (2.0, 8.0),
                    (0.0, 5.0),
                ]
                .map(|(x, y)| JointSpec::new().position(x, y, 0.0))
                .into(),
            ),
    );
    let mut cd = ecb::initialize(&map, Vec3::new(20.0, 4.0, 0.0), &bones(), 1.5, 75.0);
    ecb::load_grounded(&mut cd, &mut tree, root);
    // flags 5 has no padding, grounded bottom, center = 0.5 * PLAYER scale.
    assert_eq!(cd.desired_ecb.top, Vec2::new(0.0, 20.0));
    assert_eq!(cd.desired_ecb.bottom, Vec2::ZERO);
    assert_eq!(cd.desired_ecb.left, Vec2::new(-6.0, 10.75));
    assert_eq!(cd.desired_ecb.right, Vec2::new(6.0, 10.75));
    assert_eq!(cd.ecb_source.x128, 15.0);
    assert_eq!(cd.ledge_snap_x, 9.0);
    assert_eq!(cd.ledge_snap_y, 12.0);
    assert_eq!(cd.ledge_snap_height, 15.0);
}

#[test]
fn ecb_minimums_fixed_source_and_locked_bottom_use_mp() {
    let map = fd();
    let mut tree = JObjTree::new();
    let root =
        tree.load_joint(&JointSpec::new().children((0..6).map(|_| JointSpec::new()).collect()));
    let mut cd = ecb::initialize(&map, Vec3::ZERO, &bones(), 1.0, 75.0);
    ecb::load_grounded(&mut cd, &mut tree, root);
    assert_eq!(cd.desired_ecb.top.y, 1.0); // sanitizer raises zero height
    assert_eq!(cd.desired_ecb.right, Vec2::new(2.0, 0.5));
    assert_eq!(cd.desired_ecb.left, Vec2::new(-2.0, 0.5));
    set_ecb_source_fixed(&mut cd, 1.0, 1.0, 1.0, 1.0);
    ecb::load_grounded(&mut cd, &mut tree, root);
    assert_eq!(cd.desired_ecb.top.y, 1.5);
    assert_eq!(cd.desired_ecb.bottom.y, -1.5);
    assert_eq!(cd.desired_ecb.right.x, 1.5);
    cd.x130_flags |= coll_data_x130::LOCKED;
    cd.desired_ecb.bottom.y = -0.5;
    ecb::load_grounded(&mut cd, &mut tree, root);
    assert_eq!(cd.desired_ecb.bottom.y, -0.5);
}

#[test]
fn wait_collision_distinguishes_support_teeter_fall_and_backward_nudge() {
    for (facing, stick, nudge, wanted, x) in [
        (1.0, 0.0, 0.0, WaitGroundResult::EnterTeeter, 85.5),
        (1.0, 1.0, 0.0, WaitGroundResult::EnterFall, 90.0),
        (-1.0, 0.0, 0.0, WaitGroundResult::EnterFall, 90.0),
        (1.0, 0.0, -1.0, WaitGroundResult::Supported, 85.5),
    ] {
        let mut map = fd();
        let mut tree = JObjTree::new();
        let root = tree.load_joint(&JointSpec::new());
        let mut motion = FighterPhysics::standing(Vec3::new(80.0, 0.0, 0.0), facing);
        let mut cd = ecb::initialize(&map, motion.position, &bones(), 1.0, 75.0);
        set_ecb_source_fixed(&mut cd, 8.0, 0.0, 4.0, 4.0);
        cd.floor = SurfaceData {
            index: 0,
            flags: line_flag::LEDGE,
            normal: map.line_get_normal(0),
        };
        let mut environment = EnvironmentCollision::new(cd);
        motion.position.x = 90.0;
        motion.player_nudge.x = nudge;
        let actual = map_wait(
            &mut motion,
            &mut environment,
            &mut map,
            &mut tree,
            root,
            stick,
        );
        assert_eq!(
            actual, wanted,
            "facing={facing} stick={stick} nudge={nudge}"
        );
        assert_eq!(motion.position.x, x);
        assert_eq!(tree.translation(root), motion.position);
    }
}

#[test]
fn map_snaps_from_perturbed_height_and_unlocks_ecb() {
    let mut map = fd();
    let mut tree = JObjTree::new();
    let root = tree.load_joint(&JointSpec::new());
    let mut motion = FighterPhysics::standing(Vec3::new(0.0, 0.25, 0.0), 1.0);
    let mut cd = ecb::initialize(&map, motion.position, &bones(), 1.0, 75.0);
    set_ecb_source_fixed(&mut cd, 8.0, 0.0, 4.0, 4.0);
    cd.floor = SurfaceData {
        index: 0,
        flags: line_flag::LEDGE,
        normal: map.line_get_normal(0),
    };
    cd.x130_flags |= coll_data_x130::LOCKED;
    let mut environment = EnvironmentCollision::new(cd);
    environment.lock_frames = 1;
    environment.collision_flag = true;
    assert_eq!(
        map_wait(
            &mut motion,
            &mut environment,
            &mut map,
            &mut tree,
            root,
            0.0
        ),
        WaitGroundResult::Supported
    );
    // Snapping from a perturbed height does not land on the standing height
    // 0x38d1_b717: the floor probe rounds the displacement to f32 before the
    // collision code adds it to the position (mplib.c:1161, mpcoll.c:2956;
    // retail 0x8004E038-0x8004E044).
    assert_eq!(motion.position.y.to_bits(), 0x38d1_b800);
    assert_eq!(environment.lock_frames, 0);
    assert_eq!(environment.data.x130_flags & coll_data_x130::LOCKED, 0);
    assert!(!environment.collision_flag);
    assert_eq!(tree.translation(root), motion.position);
}
