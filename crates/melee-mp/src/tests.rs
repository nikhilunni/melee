//! Tests over a hand-built Final-Destination-like map: one flat floor with a
//! ledge at each end, closed by two walls and a bottom ceiling, plus one
//! drop-through platform in a second group. Expected values are computed in
//! the tests with the same op sequence the port uses.

use hsd_anim::mtx::vec_normalize;
use hsd_types::{Mtx, Vec3};
use melee_types::mp::{joint_flag, line_flag, line_kind, LineSection, MapCollData, NO_ID};
use melee_types::GrKind;

use crate::geom::{line_intersection_h, line_intersection_v, remap_2d, sq};
use crate::{CollMap, CollMapBuilder, JobjState};

/// Line ids of the standard map, in the order the builder lays them out
/// (floors first, then ceiling, right wall, left wall).
const FLOOR: i32 = 0;
const PLATFORM: i32 = 1;
const CEILING: i32 = 2;
const RIGHT_WALL: i32 = 3;
const LEFT_WALL: i32 = 4;

const STAGE_HALF: f32 = 85.5;
const STAGE_BOTTOM: f32 = -50.0;
const PLAT_HALF: f32 = 20.0;
const PLAT_Y: f32 = 30.0;

fn fd_data() -> MapCollData {
    let mut b = CollMapBuilder::new();
    let v0 = b.vertex(0, -STAGE_HALF, 0.0);
    let v1 = b.vertex(0, STAGE_HALF, 0.0);
    let v2 = b.vertex(0, STAGE_HALF, STAGE_BOTTOM);
    let v3 = b.vertex(0, -STAGE_HALF, STAGE_BOTTOM);
    b.floor(0, v0, v1, line_flag::LEDGE as u16);
    b.right_wall(0, v1, v2, 0);
    b.ceiling(0, v2, v3, 0);
    b.left_wall(0, v3, v0, 0);
    let p0 = b.vertex(1, -PLAT_HALF, PLAT_Y);
    let p1 = b.vertex(1, PLAT_HALF, PLAT_Y);
    b.floor(1, p0, p1, line_flag::PLATFORM as u16);
    b.build()
}

fn fd() -> CollMap {
    CollMap::load(fd_data(), 1.0, GrKind::Last)
}

fn normalized(x: f32, y: f32) -> Vec3 {
    let src = Vec3::new(x, y, 0.0);
    let mut dst = Vec3::ZERO;
    vec_normalize(&src, &mut dst);
    dst
}

#[test]
fn builder_layout_matches_ids() {
    let d = fd_data();
    assert_eq!(d.lines.len(), 5);
    assert_eq!((d.floor_start, d.floor_count), (0, 2));
    assert_eq!((d.ceiling_start, d.ceiling_count), (2, 1));
    assert_eq!((d.right_wall_start, d.right_wall_count), (3, 1));
    assert_eq!((d.left_wall_start, d.left_wall_count), (4, 1));
    assert_eq!(d.joints.len(), 2);
    assert_eq!(d.joints[0].section(LineSection::Floor), (0, 1));
    assert_eq!(d.joints[1].section(LineSection::Floor), (1, 1));
    assert_eq!((d.joints[0].vtx_start, d.joints[0].vtx_count), (0, 4));
    assert_eq!((d.joints[1].vtx_start, d.joints[1].vtx_count), (4, 2));
    assert_eq!(d.joints[0].left_bound, -STAGE_HALF);
    assert_eq!(d.joints[0].bottom_bound, STAGE_BOTTOM);
    assert_eq!(d.joints[1].top_bound, PLAT_Y);
    // Chain: floor -> right wall -> ceiling -> left wall -> floor.
    assert_eq!(d.lines[FLOOR as usize].next_id0, RIGHT_WALL as i16);
    assert_eq!(d.lines[RIGHT_WALL as usize].next_id0, CEILING as i16);
    assert_eq!(d.lines[CEILING as usize].next_id0, LEFT_WALL as i16);
    assert_eq!(d.lines[LEFT_WALL as usize].next_id0, FLOOR as i16);
    assert_eq!(d.lines[FLOOR as usize].prev_id0, LEFT_WALL as i16);
    assert_eq!(d.lines[PLATFORM as usize].next_id0, -1);
}

#[test]
fn load_sets_flags_bounds_and_scale() {
    let m = fd();
    for id in 0..5 {
        assert_ne!(m.coll_lines()[id].flags & line_flag::ENABLED, 0);
    }
    assert_eq!(
        m.coll_lines()[FLOOR as usize].flags & line_kind::KIND_MASK,
        line_kind::FLOOR
    );
    assert_eq!(
        m.coll_lines()[CEILING as usize].flags & line_kind::KIND_MASK,
        line_kind::CEILING
    );
    assert_eq!(m.joint_list(), &[0, 1]);
    assert_eq!(m.joints()[0].flags, joint_flag::ENABLED);
    assert!(m.joints()[0].xe);
    // mpLibLoad initialises box 0 to right/top = +F32_MAX and left/bottom =
    // -F32_MAX (mplib.c:912-915), so its own `top < y` / `bottom > y`
    // updates never fire and the box stays at the extremes. Retail does
    // this; the box is only made finite by mpLib_80058820.
    let b = m.bounds()[0];
    assert_eq!(
        (b.left, b.right, b.bottom, b.top),
        (-f32::MAX, f32::MAX, -f32::MAX, f32::MAX)
    );
    assert!(!m.checked_bounding());

    // Stage scale multiplies vertex positions and joint bounds.
    let m2 = CollMap::load(fd_data(), 2.0, GrKind::Last);
    assert_eq!(m2.vertices()[1].pos.x, 2.0 * STAGE_HALF);
    assert_eq!(m2.vertices()[1].x0, STAGE_HALF);
    assert_eq!(m2.vertices()[1].x10, 2.0 * STAGE_HALF);
    assert_eq!(m2.joints()[0].bounding_min.y, 2.0 * STAGE_BOTTOM);
}

#[test]
fn floor_hit_on_straight_down_sweep() {
    let mut m = fd();
    let hit = m
        .check_floor(0.0, 5.0, 0.0, -5.0, 0.0, NO_ID, NO_ID, NO_ID, None)
        .expect("floor under origin");
    // The flat floor takes the horizontal-line path over the extended
    // endpoints (each pushed 1 unit outward because neighbours exist).
    let (x0, y0, x1, _) = m.line_endpoints_extended(FLOOR);
    assert!(x0 < -STAGE_HALF && x1 > STAGE_HALF);
    let (ex, ey) = line_intersection_h(x0, y0, x1, 0.0, 5.0, 0.0, -5.0).unwrap();
    assert_eq!(hit.pos, Vec3::new(ex, ey, 0.0));
    assert_eq!(hit.pos, Vec3::new(0.0, 0.0, 0.0));
    assert_eq!(hit.line_id, FLOOR);
    assert_eq!(hit.flags, line_flag::LEDGE);
    assert_eq!(hit.normal, Vec3::new(0.0, 1.0, 0.0));
    // Bounding state is restored after the sweep.
    assert!(!m.checked_bounding());
    assert_eq!(m.joints()[0].flags & joint_flag::TOO_FAR, 0);

    // The platform is the nearer floor for a sweep starting above it.
    let hit = m
        .check_floor(0.0, 35.0, 0.0, -5.0, 0.0, NO_ID, NO_ID, NO_ID, None)
        .unwrap();
    assert_eq!(hit.line_id, PLATFORM);
    assert_eq!(hit.pos, Vec3::new(0.0, PLAT_Y, 0.0));
    assert_eq!(hit.flags, line_flag::PLATFORM);

    // Skipping the platform line or its joint falls through to the floor.
    let hit = m
        .check_floor(0.0, 35.0, 0.0, -5.0, 0.0, PLATFORM, NO_ID, NO_ID, None)
        .unwrap();
    assert_eq!(hit.line_id, FLOOR);
    let hit = m
        .check_floor(0.0, 35.0, 0.0, -5.0, 0.0, NO_ID, 1, NO_ID, None)
        .unwrap();
    assert_eq!(hit.line_id, FLOOR);
    // joint_id_only restricts to that joint.
    let hit = m
        .check_floor(0.0, 35.0, 0.0, -5.0, 0.0, NO_ID, NO_ID, 1, None)
        .unwrap();
    assert_eq!(hit.line_id, PLATFORM);
    // A line filter callback can veto the platform.
    let mut veto = |_: &CollMap, id: i32| id != PLATFORM;
    let hit = m
        .check_floor(
            0.0,
            35.0,
            0.0,
            -5.0,
            0.0,
            NO_ID,
            NO_ID,
            NO_ID,
            Some(&mut veto),
        )
        .unwrap();
    assert_eq!(hit.line_id, FLOOR);

    // y_offset raises the floor.
    let hit = m
        .check_floor(0.0, 5.0, 0.0, -5.0, 2.0, NO_ID, NO_ID, NO_ID, None)
        .unwrap();
    assert_eq!(hit.pos.y, 2.0);

    // Moving upward through a floor is not a hit (ay >= by gate).
    assert!(m
        .check_floor(0.0, -5.0, 0.0, 5.0, 0.0, NO_ID, NO_ID, NO_ID, None)
        .is_none());
}

#[test]
fn no_floor_hit_off_stage() {
    let mut m = fd();
    assert!(m
        .check_floor(100.0, 5.0, 100.0, -5.0, 0.0, NO_ID, NO_ID, NO_ID, None)
        .is_none());
    assert!(m
        .check_floor(-100.0, 5.0, -100.0, -5.0, 0.0, NO_ID, NO_ID, NO_ID, None)
        .is_none());
    // Below the stage.
    assert!(m
        .check_floor(0.0, -60.0, 0.0, -70.0, 0.0, NO_ID, NO_ID, NO_ID, None)
        .is_none());
    // A sweep that stops short of the floor.
    assert!(m
        .check_floor(0.0, 10.0, 0.0, 1.0, 0.0, NO_ID, NO_ID, NO_ID, None)
        .is_none());
    assert!(!m.checked_bounding());
}

#[test]
fn ledge_detection_at_both_edges() {
    let mut m = fd();
    // dir > 0 selects the line's left end.
    let left = m
        .find_ledge(NO_ID, NO_ID, NO_ID, 1, -90.0, -5.0, -85.0, 5.0)
        .expect("left ledge");
    assert_eq!(left.line_id, FLOOR);
    assert_eq!(left.pos, Vec3::new(-STAGE_HALF, 0.0, 0.0));
    // dir < 0 selects the line's right end.
    let right = m
        .find_ledge(NO_ID, NO_ID, NO_ID, -1, 85.0, -5.0, 90.0, 5.0)
        .expect("right ledge");
    assert_eq!(right.line_id, FLOOR);
    assert_eq!(right.pos, Vec3::new(STAGE_HALF, 0.0, 0.0));
    // The returned x is clamped into the search box.
    let clamped = m
        .find_ledge(NO_ID, NO_ID, NO_ID, 1, -85.0, -5.0, -80.0, 5.0)
        .unwrap();
    assert_eq!(clamped.pos.x, -85.0);
    // No ledge far away, none on the platform (no LEDGE flag), none when
    // the floor line is skipped.
    assert!(m
        .find_ledge(NO_ID, NO_ID, NO_ID, 1, 200.0, -5.0, 210.0, 5.0)
        .is_none());
    assert!(m
        .find_ledge(NO_ID, NO_ID, NO_ID, 1, -25.0, 25.0, -15.0, 35.0)
        .is_none());
    assert!(m
        .find_ledge(FLOOR, NO_ID, NO_ID, 1, -90.0, -5.0, -85.0, 5.0)
        .is_none());
    assert!(m
        .find_ledge(NO_ID, 0, NO_ID, 1, -90.0, -5.0, -85.0, 5.0)
        .is_none());
}

#[test]
#[should_panic(expected = "dir == 0")]
fn ledge_search_rejects_dir_zero() {
    let mut m = fd();
    let _ = m.find_ledge(NO_ID, NO_ID, NO_ID, 0, -90.0, -5.0, -85.0, 5.0);
}

#[test]
fn wall_hits() {
    let mut m = fd();
    // Right wall (normal +x): approached moving left.
    let hit = m
        .check_right_wall(90.0, -10.0, 80.0, -10.0, NO_ID, NO_ID)
        .expect("right wall");
    let (ex, ey) =
        line_intersection_v(STAGE_HALF, 0.0, STAGE_BOTTOM, 90.0, -10.0, 80.0, -10.0).unwrap();
    assert_eq!(hit.pos, Vec3::new(ex, ey, 0.0));
    assert_eq!(hit.pos, Vec3::new(STAGE_HALF, -10.0, 0.0));
    assert_eq!(hit.line_id, RIGHT_WALL);
    assert_eq!(hit.normal, Vec3::new(1.0, 0.0, 0.0));
    // Moving away from it is not a hit (ax >= bx gate).
    assert!(m
        .check_right_wall(80.0, -10.0, 90.0, -10.0, NO_ID, NO_ID)
        .is_none());
    // Left wall (normal -x): approached moving right.
    let hit = m
        .check_left_wall(-90.0, -10.0, -80.0, -10.0, NO_ID, NO_ID)
        .expect("left wall");
    assert_eq!(hit.pos, Vec3::new(-STAGE_HALF, -10.0, 0.0));
    assert_eq!(hit.line_id, LEFT_WALL);
    assert_eq!(hit.normal, Vec3::new(-1.0, 0.0, 0.0));
    assert!(m
        .check_left_wall(-80.0, -10.0, -90.0, -10.0, NO_ID, NO_ID)
        .is_none());
    // Above the wall's extent: miss.
    assert!(m
        .check_right_wall(90.0, 10.0, 80.0, 10.0, NO_ID, NO_ID)
        .is_none());

    // Point probes give the signed x distance to the wall.
    let p = m
        .right_wall_probe(RIGHT_WALL, &Vec3::new(90.0, -10.0, 0.0))
        .unwrap();
    assert_eq!(p.line_id, RIGHT_WALL);
    assert_eq!(p.delta, STAGE_HALF - 90.0);
    let p = m
        .left_wall_probe(LEFT_WALL, &Vec3::new(-90.0, -10.0, 0.0))
        .unwrap();
    assert_eq!(p.delta, -STAGE_HALF - -90.0);
    // Off the end of the wall chain by more than 0.1: none.
    assert!(m
        .right_wall_probe(RIGHT_WALL, &Vec3::new(90.0, 5.0, 0.0))
        .is_none());

    // Vertex sweep: a wall vertex moving into the fighter across the ECB
    // edge is detected. The right wall (normal +x) moves 5 units right,
    // from x = 85.5 to 90.5, through a vertical ECB edge at x = 88.
    m.joint_snapshot_prev_pos(0);
    m.vtx_set_pos(1, STAGE_HALF + 5.0, 0.0);
    m.vtx_set_pos(2, STAGE_HALF + 5.0, STAGE_BOTTOM);
    // The joint's box must cover the moved vertices or the sweep culls it.
    m.joint_update_bounding(0);
    let id = m.right_wall_vertex_sweep(
        88.0, -20.0, 88.0, 20.0, 88.0, -20.0, 88.0, 20.0, NO_ID, NO_ID,
    );
    assert_eq!(id, Some(RIGHT_WALL));
    // The wall retreating from the fighter is not a hit.
    m.joint_snapshot_prev_pos(0);
    m.vtx_set_pos(1, STAGE_HALF, 0.0);
    m.vtx_set_pos(2, STAGE_HALF, STAGE_BOTTOM);
    let id = m.right_wall_vertex_sweep(
        88.0, -20.0, 88.0, 20.0, 88.0, -20.0, 88.0, 20.0, NO_ID, NO_ID,
    );
    assert_eq!(id, None);
    // And a left wall (normal -x) uses the mirrored sweep. mpcoll passes
    // the ECB's right-side edge top to bottom here (`prev_right ->
    // prev_bottom`, mpcoll.c:2058), so the edge runs downward.
    m.joint_snapshot_prev_pos(0);
    m.vtx_set_pos(3, -STAGE_HALF - 5.0, STAGE_BOTTOM);
    m.vtx_set_pos(0, -STAGE_HALF - 5.0, 0.0);
    m.joint_update_bounding(0);
    let id = m.left_wall_vertex_sweep(
        -88.0, 20.0, -88.0, -20.0, -88.0, 20.0, -88.0, -20.0, NO_ID, NO_ID,
    );
    assert_eq!(id, Some(LEFT_WALL));
    // The upward orientation is the right-wall convention and misses.
    let id = m.left_wall_vertex_sweep(
        -88.0, -20.0, -88.0, 20.0, -88.0, -20.0, -88.0, 20.0, NO_ID, NO_ID,
    );
    assert_eq!(id, None);
}

#[test]
fn ceiling_hit() {
    let mut m = fd();
    let hit = m
        .check_ceiling(0.0, -60.0, 0.0, -40.0, NO_ID, NO_ID)
        .expect("ceiling");
    let (x0, y0, x1, _) = m.line_endpoints_extended(CEILING);
    let (ex, ey) = line_intersection_h(x0, y0, x1, 0.0, -60.0, 0.0, -40.0).unwrap();
    assert_eq!(hit.pos, Vec3::new(ex, ey, 0.0));
    assert_eq!(hit.pos, Vec3::new(0.0, STAGE_BOTTOM, 0.0));
    assert_eq!(hit.line_id, CEILING);
    assert_eq!(hit.normal, Vec3::new(0.0, -1.0, 0.0));
    // Moving down through a ceiling is not a hit.
    assert!(m
        .check_ceiling(0.0, -40.0, 0.0, -60.0, NO_ID, NO_ID)
        .is_none());
    let p = m
        .ceiling_probe(CEILING, &Vec3::new(10.0, -55.0, 0.0))
        .unwrap();
    let f = (STAGE_BOTTOM - STAGE_BOTTOM) * (10.0 - STAGE_HALF) / (-STAGE_HALF - STAGE_HALF)
        + STAGE_BOTTOM
        - -55.0;
    assert_eq!(p.delta, (f64::from(f) - 0.0001) as f32);
}

#[test]
fn line_flags_kinds_and_chain_queries() {
    let mut m = fd();
    assert_eq!(m.line_get_kind(FLOOR), line_kind::FLOOR);
    assert_eq!(m.line_get_kind(PLATFORM), line_kind::FLOOR);
    assert_eq!(m.line_get_kind(CEILING), line_kind::CEILING);
    assert_eq!(m.line_get_kind(RIGHT_WALL), line_kind::RIGHT_WALL);
    assert_eq!(m.line_get_kind(LEFT_WALL), line_kind::LEFT_WALL);
    assert_eq!(m.line_get_flags(FLOOR), line_flag::LEDGE);
    assert_eq!(m.line_get_flags(PLATFORM), line_flag::PLATFORM);
    assert_eq!(m.line_get_flags(CEILING), 0);
    assert_eq!(m.joint_from_line(FLOOR), 0);
    assert_eq!(m.joint_from_line(PLATFORM), 1);
    assert_eq!(m.joint_from_line(NO_ID), NO_ID);
    assert!(m.line_is_active(FLOOR));
    assert!(!m.line_is_active(NO_ID));

    // Chain walking.
    assert_eq!(m.line_next(FLOOR), RIGHT_WALL);
    assert_eq!(m.line_next(RIGHT_WALL), CEILING);
    assert_eq!(m.line_prev(FLOOR), LEFT_WALL);
    assert_eq!(m.line_next(PLATFORM), NO_ID);
    assert_eq!(m.line_next_non_floor(FLOOR), RIGHT_WALL);
    assert_eq!(m.line_prev_non_floor(FLOOR), LEFT_WALL);
    assert_eq!(m.line_next_non_right_wall(RIGHT_WALL), CEILING);
    assert_eq!(m.line_prev_non_ceiling(CEILING), RIGHT_WALL);
    assert_eq!(m.line_next_non_floor_id0(FLOOR), RIGHT_WALL);
    assert_eq!(m.line_prev_non_floor_id0(FLOOR), LEFT_WALL);
    assert_eq!(m.line_next_non_floor(PLATFORM), NO_ID);
    // Single floor: no floor neighbour in either direction.
    assert_eq!(m.floor_next_across_joint(FLOOR), NO_ID);
    assert_eq!(m.floor_prev_across_joint(FLOOR), NO_ID);
    assert!(m.lines_connected(FLOOR, FLOOR));
    assert!(!m.lines_connected(FLOOR, RIGHT_WALL));
    assert!(!m.lines_connected(FLOOR, PLATFORM));

    // Endpoint finders.
    assert_eq!(m.floor_get_right(FLOOR), Vec3::new(STAGE_HALF, 0.0, 0.0));
    assert_eq!(m.floor_get_left(FLOOR), Vec3::new(-STAGE_HALF, 0.0, 0.0));
    assert_eq!(
        m.right_wall_get_top(RIGHT_WALL),
        Vec3::new(STAGE_HALF, 0.0, 0.0)
    );
    assert_eq!(
        m.right_wall_get_bottom(RIGHT_WALL),
        Vec3::new(STAGE_HALF, STAGE_BOTTOM, 0.0)
    );
    assert_eq!(
        m.ceiling_get_right(CEILING),
        Vec3::new(STAGE_HALF, STAGE_BOTTOM, 0.0)
    );
    assert_eq!(
        m.ceiling_get_left(CEILING),
        Vec3::new(-STAGE_HALF, STAGE_BOTTOM, 0.0)
    );
    assert_eq!(
        m.left_wall_get_bottom(LEFT_WALL),
        Vec3::new(-STAGE_HALF, STAGE_BOTTOM, 0.0)
    );
    assert_eq!(
        m.left_wall_get_top(LEFT_WALL),
        Vec3::new(-STAGE_HALF, 0.0, 0.0)
    );
    assert_eq!(
        m.floor_chain_right_end_id0(FLOOR),
        Vec3::new(STAGE_HALF, 0.0, 0.0)
    );
    assert_eq!(
        m.floor_chain_left_end_id0(FLOOR),
        Vec3::new(-STAGE_HALF, 0.0, 0.0)
    );
    assert_eq!(
        m.line_get_v0_pos(PLATFORM),
        Vec3::new(-PLAT_HALF, PLAT_Y, 0.0)
    );
    assert_eq!(
        m.line_get_v1_pos(PLATFORM),
        Vec3::new(PLAT_HALF, PLAT_Y, 0.0)
    );

    // Normals come from PSVECNormalize of (-(y1 - y0), x1 - x0, 0).
    assert_eq!(m.line_get_normal(FLOOR), normalized(0.0, 2.0 * STAGE_HALF));
    assert_eq!(
        m.line_get_normal(RIGHT_WALL),
        normalized(-STAGE_BOTTOM, 0.0)
    );
    assert_eq!(
        m.line_get_normal(CEILING),
        normalized(0.0, -2.0 * STAGE_HALF)
    );

    // Material byte.
    m.line_set_material(FLOOR, 5);
    assert_eq!(m.line_get_flags(FLOOR), line_flag::LEDGE | 5);
    m.line_set_material(FLOOR, 2);
    assert_eq!(m.line_get_flags(FLOOR), line_flag::LEDGE | 2);
}

#[test]
fn floor_probe_walks_and_clamps() {
    let m = fd();
    let p = m.floor_probe(FLOOR, &Vec3::new(10.0, 3.0, 0.0)).unwrap();
    assert_eq!(p.line_id, FLOOR);
    let f = (0.0 - 0.0) * (10.0 - -STAGE_HALF) / (STAGE_HALF - -STAGE_HALF) + 0.0 - 3.0;
    assert_eq!(p.delta, (f64::from(f) + 0.0001) as f32);
    assert_eq!(p.flags, line_flag::LEDGE);
    assert_eq!(p.normal, normalized(0.0, 2.0 * STAGE_HALF));
    // Within 0.1 past the end: clamped to the end.
    assert!(m
        .floor_probe(FLOOR, &Vec3::new(STAGE_HALF + 0.05, 0.0, 0.0))
        .is_some());
    // Beyond 0.1 past the end, with no floor neighbour: none.
    assert!(m
        .floor_probe(FLOOR, &Vec3::new(STAGE_HALF + 0.5, 0.0, 0.0))
        .is_none());
    assert!(m
        .floor_probe(FLOOR, &Vec3::new(-STAGE_HALF - 0.5, 0.0, 0.0))
        .is_none());
}

#[test]
#[should_panic(expected = "not found lineID=-1")]
fn floor_probe_rejects_invalid_id() {
    let m = fd();
    let _ = m.floor_probe(NO_ID, &Vec3::ZERO);
}

#[test]
fn joint_enable_disable_and_hide() {
    let mut m = fd();
    let sweep = |m: &mut CollMap| {
        m.check_floor(0.0, 35.0, 0.0, 25.0, 0.0, NO_ID, NO_ID, NO_ID, None)
            .map(|h| h.line_id)
    };
    assert_eq!(sweep(&mut m), Some(PLATFORM));

    m.joint_list_remove(1);
    assert_eq!(m.joint_list(), &[0]);
    assert_eq!(m.joints()[1].flags & joint_flag::ENABLED, 0);
    assert_eq!(
        m.coll_lines()[PLATFORM as usize].flags & line_flag::ENABLED,
        0
    );
    assert_eq!(sweep(&mut m), None);
    // Idempotent.
    m.joint_list_remove(1);
    assert_eq!(m.joint_list(), &[0]);

    m.joint_list_add(1);
    assert_eq!(m.joint_list(), &[0, 1]);
    assert_ne!(
        m.coll_lines()[PLATFORM as usize].flags & line_flag::ENABLED,
        0
    );
    assert_eq!(sweep(&mut m), Some(PLATFORM));
    m.joint_list_add(1);
    assert_eq!(m.joint_list(), &[0, 1]);

    // Hidden joints are culled by the bounding check.
    m.joint_hide(1);
    assert_ne!(
        m.coll_lines()[PLATFORM as usize].flags & line_flag::HIDDEN,
        0
    );
    assert!(!m.line_is_active(PLATFORM));
    assert_eq!(sweep(&mut m), None);
    m.joint_unhide(1);
    assert_eq!(sweep(&mut m), Some(PLATFORM));

    // Per-line enable/disable.
    m.line_disable(PLATFORM);
    assert_eq!(sweep(&mut m), None);
    m.line_enable(PLATFORM);
    assert_eq!(sweep(&mut m), Some(PLATFORM));

    // Unlinking the first joint keeps the list order of the rest.
    m.joint_list_remove(0);
    assert_eq!(m.joint_list(), &[1]);
    m.joint_list_add(0);
    assert_eq!(m.joint_list(), &[1, 0]);
}

#[test]
fn bounding_check_marks_far_joints() {
    let mut m = fd();
    m.bounding_check(200.0, 200.0, 210.0, 210.0);
    assert!(m.checked_bounding());
    assert_ne!(m.joints()[0].flags & joint_flag::TOO_FAR, 0);
    assert_ne!(m.joints()[1].flags & joint_flag::TOO_FAR, 0);
    // With bounding pre-checked, sweeps do not touch it and see the marks.
    assert!(m
        .check_floor(0.0, 5.0, 0.0, -5.0, 0.0, NO_ID, NO_ID, NO_ID, None)
        .is_none());
    assert!(m.checked_bounding());
    m.uncheck_bounding();
    assert!(!m.checked_bounding());
    assert_eq!(m.joints()[0].flags & joint_flag::TOO_FAR, 0);

    // A box over the platform only.
    m.bounding_check_2(-10.0, 25.0, 10.0, 35.0);
    assert_eq!(m.joints()[1].flags & joint_flag::TOO_FAR, 0);
    // The main joint's box spans y -50..0, so it is too far.
    assert_ne!(m.joints()[0].flags & joint_flag::TOO_FAR, 0);
    m.uncheck_bounding();

    // B10 joints are never too far.
    m.joint_set_b10(0);
    m.bounding_check(200.0, 200.0, 210.0, 210.0);
    assert_eq!(m.joints()[0].flags & joint_flag::TOO_FAR, 0);
    m.uncheck_bounding();

    // bounding_check_3 spans four points.
    m.bounding_check_3(-10.0, 25.0, 10.0, 35.0, 0.0, -10.0, 0.0, 40.0);
    assert_eq!(m.joints()[1].flags & joint_flag::TOO_FAR, 0);
    m.uncheck_bounding();
}

#[test]
fn floor_below_point() {
    let mut m = fd();
    assert_eq!(
        m.floor_below(&Vec3::new(0.0, 10.0, 0.0), NO_ID, NO_ID),
        FLOOR
    );
    // Joint-list order: the main floor is found before the platform.
    assert_eq!(
        m.floor_below(&Vec3::new(0.0, 40.0, 0.0), NO_ID, NO_ID),
        FLOOR
    );
    assert_eq!(
        m.floor_below(&Vec3::new(0.0, 40.0, 0.0), NO_ID, 1),
        PLATFORM
    );
    assert_eq!(
        m.floor_below(&Vec3::new(0.0, 40.0, 0.0), 0, NO_ID),
        PLATFORM
    );
    assert_eq!(
        m.floor_below(&Vec3::new(0.0, -10.0, 0.0), NO_ID, NO_ID),
        NO_ID
    );
    assert_eq!(
        m.floor_below(&Vec3::new(100.0, 10.0, 0.0), NO_ID, NO_ID),
        NO_ID
    );
    assert!(!m.checked_bounding());
}

#[test]
fn check_multiple_picks_nearest_surface() {
    let mut m = fd();
    let hit = m.check_all(NO_ID, NO_ID, 0.0, 5.0, 0.0, -5.0).unwrap();
    assert_eq!(hit.line_id, FLOOR);
    let hit = m.check_all(NO_ID, NO_ID, 90.0, -10.0, 80.0, -10.0).unwrap();
    assert_eq!(hit.line_id, RIGHT_WALL);
    let hit = m.check_all(NO_ID, NO_ID, 0.0, -60.0, 0.0, -40.0).unwrap();
    assert_eq!(hit.line_id, CEILING);
    // Diagonal into the bottom-right corner from outside: the wall is
    // reached first.
    let hit = m.check_all(NO_ID, NO_ID, 95.0, -45.0, 75.0, -45.0).unwrap();
    assert_eq!(hit.line_id, RIGHT_WALL);
    // Only ceilings requested: the floor sweep finds nothing.
    assert!(m
        .check_multiple(0.0, 5.0, 0.0, -5.0, 0x2, NO_ID, NO_ID)
        .is_none());
    let hit = m
        .check_all_remap(NO_ID, NO_ID, 0.0, 5.0, 0.0, -5.0)
        .unwrap();
    assert_eq!(hit.line_id, FLOOR);
    assert!(!m.checked_bounding());
}

#[test]
fn walk_along_floor_stops_at_walls() {
    let mut m = fd();
    let start = Vec3::new(0.0, 0.0, 0.0);
    let w = m.walk_along_floor(FLOOR, &start, 10.0, 0.0).unwrap();
    assert!(w.reached);
    assert_eq!(w.line_id, FLOOR);
    // Expected: the probe puts us at y = 0 + 0.0001-ish, then we move
    // t = 10 / |v1 - p| of the way toward v1.
    let p = m.floor_probe(FLOOR, &start).unwrap();
    let sp58 = Vec3::new(0.0, start.y + p.delta, 0.0);
    let sp4c = Vec3::new(STAGE_HALF, 0.0, 0.0);
    let dist = gekko_math::msl::sqrtf(sq(sp58.x - sp4c.x) + sq(sp58.y - sp4c.y));
    let t = 10.0 / dist;
    let expect = Vec3::new(
        (t * (sp4c.x - sp58.x)) + sp58.x,
        (t * (sp4c.y - sp58.y)) + sp58.y,
        (t * (sp4c.z - sp58.z)) + sp58.z,
    );
    assert_eq!(w.pos, expect);
    assert_eq!(w.surface.map(|s| s.0), Some(line_flag::LEDGE));

    // Walking left works the same way from v0.
    let w = m.walk_along_floor(FLOOR, &start, -10.0, 0.0).unwrap();
    assert!(w.reached);
    assert_eq!(w.line_id, FLOOR);
    assert!(w.pos.x < 0.0);

    // Past the right ledge onto the wall with no wall allowance: fails, the
    // end line is not a floor, and the position is the wall's far end.
    let w = m.walk_along_floor(FLOOR, &start, 100.0, 0.0).unwrap();
    assert!(!w.reached);
    assert_eq!(w.line_id, NO_ID);
    assert_eq!(w.surface, None);
    assert_eq!(w.pos, Vec3::new(STAGE_HALF, STAGE_BOTTOM, 0.0));

    // Inactive line: no result at all.
    m.line_disable(FLOOR);
    assert!(m.walk_along_floor(FLOOR, &start, 10.0, 0.0).is_none());
}

#[test]
fn remap_sweep_follows_moving_platform() {
    let mut m = fd();
    // Move the platform up 10 units this frame: previous positions stay at
    // y = 30 and the joint is flagged as transformed.
    m.vtx_set_pos(4, -PLAT_HALF, PLAT_Y + 10.0);
    m.vtx_set_pos(5, PLAT_HALF, PLAT_Y + 10.0);
    m.joints[1].flags |= joint_flag::B8;

    // The plain sweep from y=35 down to 25 misses the platform now at 40.
    assert!(m
        .check_floor(0.0, 35.0, 0.0, 25.0, 0.0, NO_ID, NO_ID, NO_ID, None)
        .is_none());
    // The remap sweep carries the start along with the platform and hits.
    let hit = m
        .check_floor_remap(0.0, 35.0, 0.0, 25.0, 0.0, NO_ID, NO_ID, NO_ID, None)
        .expect("remapped hit");
    assert_eq!(hit.line_id, PLATFORM);
    let (ax, ay) = remap_2d(
        -PLAT_HALF,
        PLAT_Y,
        PLAT_HALF,
        PLAT_Y,
        -PLAT_HALF,
        PLAT_Y + 10.0,
        PLAT_HALF,
        PLAT_Y + 10.0,
        0.0,
        35.0,
    );
    let (ex, ey) =
        line_intersection_h(-PLAT_HALF, PLAT_Y + 10.0, PLAT_HALF, ax, ay, 0.0, 25.0).unwrap();
    assert_eq!(hit.pos, Vec3::new(ex, ey, 0.0));
    assert_eq!(hit.pos, Vec3::new(0.0, PLAT_Y + 10.0, 0.0));

    // The platform's speed at a point riding it.
    let speed = m
        .line_speed(PLATFORM, &Vec3::new(0.0, PLAT_Y, 0.0))
        .unwrap();
    assert_eq!(speed, Vec3::new(0.0, 10.0, 0.0));
    m.line_disable(PLATFORM);
    assert!(m.line_speed(PLATFORM, &Vec3::ZERO).is_none());
}

#[test]
fn floor_bounds_track_enabled_floors() {
    let mut m = fd();
    m.update_floor_bounds();
    let b = m.bounds()[1];
    assert_eq!(
        (b.left, b.right, b.bottom, b.top),
        (-STAGE_HALF, STAGE_HALF, 0.0, PLAT_Y)
    );
    assert!(m.joints().iter().all(|j| !j.xe));
    // No dirty joint: nothing recomputed even if lines changed.
    m.lines[PLATFORM as usize].flags &= !line_flag::ENABLED;
    m.update_floor_bounds();
    assert_eq!(m.bounds()[1].top, PLAT_Y);
    m.lines[PLATFORM as usize].flags |= line_flag::ENABLED;
    // Removing the platform joint dirties it and drops it from the bound.
    m.joint_list_remove(1);
    m.update_floor_bounds();
    assert_eq!(m.bounds()[1].top, 0.0);
    m.reset_bounds();
    assert_eq!(m.bounds()[0].right, 10000.0);
    assert_eq!(m.bounds()[1].left, -10000.0);
}

#[test]
fn stitching_links_adjacent_groups() {
    // Two floors meeting at x = 0, in different groups, so the builder gives
    // them no id0 link.
    let mut b = CollMapBuilder::new();
    let a0 = b.vertex(0, -10.0, 0.0);
    let a1 = b.vertex(0, 0.0, 0.0);
    b.floor(0, a0, a1, 0);
    let c0 = b.vertex(1, 0.0, 0.0);
    let c1 = b.vertex(1, 10.0, 0.0);
    b.floor(1, c0, c1, 0);
    let mut m = CollMap::load(b.build(), 1.0, GrKind::Last);
    assert_eq!(m.line_next(0), NO_ID);
    m.stitch_all_joints();
    assert_eq!(m.data().lines[0].next_id1, 1);
    assert_eq!(m.data().lines[1].prev_id1, 0);
    assert_eq!(m.line_next(0), 1);
    assert_eq!(m.line_prev(1), 0);
    assert!(m.lines_connected(0, 1));
    assert_eq!(m.floor_get_right(0), Vec3::new(10.0, 0.0, 0.0));
    // The probe walks across the stitched link.
    let p = m.floor_probe(0, &Vec3::new(5.0, 1.0, 0.0)).unwrap();
    assert_eq!(p.line_id, 1);
    // The alt-link walker stops at the alternate link.
    assert_eq!(m.floor_next_across_joint(0), 1);
    // Moving the second group away invalidates the link on the next
    // line_next (distance >= 2 units), but not the stored id.
    m.vtx_set_pos(2, 3.0, 0.0);
    assert_eq!(m.line_next(0), NO_ID);
    assert_eq!(m.data().lines[0].next_id1, 1);
}

#[test]
fn dynamic_lines_reclassify_by_slope() {
    let mut b = CollMapBuilder::new();
    let d0 = b.vertex(0, 0.0, 0.0);
    let d1 = b.vertex(0, 10.0, 0.0);
    b.dynamic(0, d0, d1, line_flag::DYNAMIC_PLATFORM as u16);
    let mut m = CollMap::load(b.build(), 1.0, GrKind::Last);
    let kind = |m: &CollMap| m.coll_lines()[0].flags & line_kind::KIND_MASK;

    m.joint_update_dynamics(0);
    assert_eq!(kind(&m), line_kind::FLOOR);
    assert_ne!(m.coll_lines()[0].flags & line_flag::PLATFORM, 0);
    assert_ne!(m.line_get_flags(0) & line_flag::PLATFORM, 0);

    m.vtx_set_pos(1, 1.0, 10.0);
    m.joint_update_dynamics_and_island(0);
    assert_eq!(kind(&m), line_kind::LEFT_WALL);
    assert_eq!(m.coll_lines()[0].flags & line_flag::ENABLED, 0);

    m.vtx_set_pos(1, 1.0, -10.0);
    m.joint_update_dynamics(0);
    assert_eq!(kind(&m), line_kind::RIGHT_WALL);

    m.vtx_set_pos(1, -10.0, 0.0);
    m.joint_update_dynamics(0);
    assert_eq!(kind(&m), line_kind::CEILING);

    m.vtx_set_pos(1, 0.0, 5.0);
    m.joint_update_dynamics(0);
    assert_eq!(kind(&m), line_kind::LEFT_WALL);
    m.vtx_set_pos(1, 0.0, -5.0);
    m.joint_update_dynamics(0);
    assert_eq!(kind(&m), line_kind::RIGHT_WALL);

    // Dynamic lines are swept with the floors once they are floors again.
    m.vtx_set_pos(1, 10.0, 0.0);
    m.joint_update_dynamics(0);
    let hit = m
        .check_floor(5.0, 5.0, 5.0, -5.0, 0.0, NO_ID, NO_ID, NO_ID, None)
        .unwrap();
    assert_eq!(hit.line_id, 0);
    assert_eq!(hit.flags, line_flag::DYNAMIC_PLATFORM | line_flag::PLATFORM);
}

#[test]
#[should_panic(expected = "zero-length dynamic line")]
fn zero_length_dynamic_line_asserts() {
    let mut b = CollMapBuilder::new();
    let d0 = b.vertex(0, 0.0, 0.0);
    let d1 = b.vertex(0, 0.0, 0.0);
    b.dynamic(0, d0, d1, 0);
    // Pruning would mark it EMPTY; the kind update still asserts on it.
    let mut m = CollMap::load(b.build(), 1.0, GrKind::Last);
    m.joint_update_dynamics(0);
}

#[test]
fn prune_marks_and_splices_empty_lines() {
    let mut b = CollMapBuilder::new();
    let v0 = b.vertex(0, -10.0, 0.0);
    let v1 = b.vertex(0, 0.0, 0.0);
    let v2 = b.vertex(0, 0.0, 0.0);
    let v3 = b.vertex(0, 10.0, 0.0);
    b.floor(0, v0, v1, 0);
    b.floor(0, v1, v2, 0); // zero length
    b.floor(0, v2, v3, 0);
    let mut data = b.build();
    assert_eq!(data.lines[0].next_id0, 1);
    assert_eq!(data.lines[2].prev_id0, 1);
    let pura = data.clone();
    CollMap::prune_empty_lines(&mut data, GrKind::Last);
    assert_ne!(u32::from(data.lines[1].hi_flags) & line_flag::EMPTY, 0);
    assert_eq!(data.lines[1].next_id0, -1);
    assert_eq!(data.lines[0].next_id0, 2);
    assert_eq!(data.lines[2].prev_id0, 0);
    // Poke Floats skips pruning.
    let mut pura_data = pura.clone();
    CollMap::prune_empty_lines(&mut pura_data, GrKind::Pura);
    assert_eq!(pura_data, pura);

    // Loaded, the empty line is never a hit and the chain skips it.
    let mut m = CollMap::load(pura, 1.0, GrKind::Last);
    assert_ne!(m.coll_lines()[1].flags & line_flag::EMPTY, 0);
    assert_eq!(m.line_next(0), 2);
    let hit = m
        .check_floor(0.0, 5.0, 0.0, -5.0, 0.0, NO_ID, NO_ID, NO_ID, None)
        .unwrap();
    assert_ne!(hit.line_id, 1);
}

#[test]
fn joint_transform_uniform_scale_and_hide() {
    let mut m = fd();
    let mut mtx = Mtx::IDENTITY;
    mtx.0[0][3] = 5.0;
    mtx.0[1][3] = -2.0;
    m.update_joint_transform(1, Some(JobjState { hidden: false, mtx }));
    assert_eq!(m.transform_update_count(), 1);
    let v = m.vertices()[4];
    assert_eq!(v.pos.x, -PLAT_HALF * 1.0 + 5.0);
    assert_eq!(v.pos.y, PLAT_Y * 1.0 + -2.0);
    // Previous positions were snapshotted before the move.
    assert_eq!((v.x10, v.x14), (-PLAT_HALF, PLAT_Y));
    let j = m.joints()[1];
    assert_ne!(j.flags & joint_flag::B8, 0);
    assert_eq!(j.flags & joint_flag::B9, 0);
    assert_eq!(j.bounding_min.x, (-PLAT_HALF * 1.0 + 5.0) - 30.0);
    assert_eq!(j.bounding_max.y, 30.0 + (PLAT_Y * 1.0 + -2.0));
    assert!(j.xe);

    // A hidden JObj hides the joint once; unhiding happens on the next
    // visible update.
    m.update_joint_transform(1, Some(JobjState { hidden: true, mtx }));
    assert_ne!(m.joints()[1].flags & joint_flag::HIDDEN, 0);
    assert_ne!(
        m.coll_lines()[PLATFORM as usize].flags & line_flag::HIDDEN,
        0
    );
    m.update_joint_transform(1, Some(JobjState { hidden: false, mtx }));
    assert_eq!(m.joints()[1].flags & joint_flag::HIDDEN, 0);
    assert_eq!(m.transform_update_count(), 3);

    // No JObj: only the previous-position snapshot happens.
    m.vtx_set_pos(4, 1.0, 2.0);
    m.update_joint_transform(1, None);
    assert_eq!((m.vertices()[4].x10, m.vertices()[4].x14), (1.0, 2.0));

    // General matrix path: a rotation sets B9 and pads the box by 30.
    let mut rot = Mtx::IDENTITY;
    rot.0[0][0] = 0.0;
    rot.0[0][1] = -1.0;
    rot.0[1][0] = 1.0;
    rot.0[1][1] = 0.0;
    m.update_joint_transform(
        1,
        Some(JobjState {
            hidden: false,
            mtx: rot,
        }),
    );
    let j = m.joints()[1];
    assert_ne!(j.flags & joint_flag::B9, 0);
    // (x, y) -> (-y, x): the platform now spans x in [-30, -30], y in
    // [-20, 20], padded by 30.
    assert_eq!(j.bounding_min.x, -PLAT_Y - 30.0);
    assert_eq!(j.bounding_max.x, -PLAT_Y + 30.0);
    assert_eq!(j.bounding_min.y, -PLAT_HALF - 30.0);
    assert_eq!(j.bounding_max.y, PLAT_HALF + 30.0);
    // The dynamics pass reclassified nothing (no dynamic lines) and the
    // platform, now vertical, still reports as a floor line by section.
    assert_eq!(m.line_get_kind(PLATFORM), line_kind::FLOOR);

    // joint_update_bounding grows the box to cover vertices.
    m.joints[1].bounding_min = hsd_types::Vec2::new(0.0, 0.0);
    m.joints[1].bounding_max = hsd_types::Vec2::new(0.0, 0.0);
    m.joint_update_bounding(1);
    assert_eq!(m.joints()[1].bounding_min.x, -PLAT_Y - 30.0);
}

#[test]
fn joint_callbacks_and_ledge_notification() {
    use melee_types::mp::CollData;
    use std::sync::atomic::{AtomicI32, Ordering};
    static CALLS: AtomicI32 = AtomicI32::new(0);
    fn cb(user_data: u32, joint_id: i32, coll: &mut CollData, x50: i32, kind: i32, dy: f32) {
        assert_eq!(user_data, 77);
        assert_eq!(joint_id, 0);
        assert_eq!(x50, 3);
        assert_eq!(kind, 3);
        assert_eq!(dy, 0.0);
        coll.x38 = 1;
        CALLS.fetch_add(1, Ordering::SeqCst);
    }
    let mut m = fd();
    assert_eq!(m.joint_get_cb1(0), (None, 0));
    m.joint_set_cb1(0, 77, cb);
    assert!(m.joint_get_cb1(0).0.is_some());
    m.joint_set_cb2(1, 5, cb);
    assert_eq!(m.joint_get_cb2(1).1, 5);
    let mut coll = CollData {
        x50: 3.9,
        ..Default::default()
    };
    m.notify_ledge_grab(&mut coll, FLOOR);
    assert_eq!(CALLS.load(Ordering::SeqCst), 1);
    assert_eq!(coll.x38, 1);
    // No callback on the platform's joint, and -1 is ignored.
    m.notify_ledge_grab(&mut coll, PLATFORM);
    m.notify_ledge_grab(&mut coll, NO_ID);
    assert_eq!(CALLS.load(Ordering::SeqCst), 1);
    m.joint_clear_cb1(0);
    m.notify_ledge_grab(&mut coll, FLOOR);
    assert_eq!(CALLS.load(Ordering::SeqCst), 1);
    // B11 toggles.
    m.joint_set_b11(0);
    assert_ne!(m.joints()[0].flags & joint_flag::B11, 0);
    m.joint_clear_b11(0);
    assert_eq!(m.joints()[0].flags & joint_flag::B11, 0);
    m.joint_refresh_island(0);
}

#[test]
fn vertex_and_line_position_writes() {
    let mut m = fd();
    assert_eq!(m.vtx_get_pos(1), (STAGE_HALF, 0.0));
    m.line_set_pos(PLATFORM, -5.0, 1.0, 5.0, 2.0);
    assert_eq!(m.vtx_get_pos(4), (-5.0, 1.0));
    assert_eq!(m.vtx_get_pos(5), (5.0, 2.0));
    // Offsets are from the archive position, not the current one.
    m.line_set_offset(PLATFORM, 1.0, 1.0, 1.0, 1.0);
    assert_eq!(m.vtx_get_pos(4), (-PLAT_HALF + 1.0, PLAT_Y + 1.0));
    assert_eq!(m.vtx_get_pos(5), (PLAT_HALF + 1.0, PLAT_Y + 1.0));
}

#[test]
fn intersection_primitives() {
    use crate::geom::line_intersection;
    // A sloped floor from (0,0) to (10,10) swept by a vertical drop at x=5.
    let (x, y) = line_intersection(0.0, 0.0, 10.0, 10.0, 5.0, 8.0, 5.0, 2.0).unwrap();
    assert_eq!((x, y), (5.0, 5.0));
    // Approaching from below (behind the line) is rejected.
    assert!(line_intersection(0.0, 0.0, 10.0, 10.0, 5.0, 2.0, 5.0, 8.0).is_none());
    // Entirely to one side.
    assert!(line_intersection(0.0, 0.0, 10.0, 10.0, 20.0, 8.0, 20.0, 2.0).is_none());
    // Colinear.
    assert!(line_intersection(0.0, 0.0, 10.0, 0.0, 2.0, 0.0, 8.0, 0.0).is_none());
    // Zero-length sweep.
    assert!(line_intersection(0.0, 0.0, 10.0, 10.0, 5.0, 5.0, 5.0, 5.0).is_none());
    // t == 1 takes the `a1` clamp branch.
    let (x, y) = line_intersection(0.0, 0.0, 10.0, 0.0, 10.0, 1.0, 10.0, -1.0).unwrap();
    assert_eq!((x, y), (10.0, 0.0));
    // Entirely past the end of a in x: rejected by the range test.
    assert!(line_intersection(0.0, 0.0, 10.0, 0.0, 10.05, 1.0, 10.05, -1.0).is_none());

    // H: a sweep straddling the end whose crossing lands within 0.1 past
    // it snaps to the end (9.5 + 1.125/2 = 10.0625); beyond 0.1 misses
    // (9.5 + 1.5/2 = 10.25).
    assert_eq!(
        line_intersection_h(0.0, 0.0, 10.0, 9.5, 1.0, 10.625, -1.0),
        Some((10.0, 0.0))
    );
    assert!(line_intersection_h(0.0, 0.0, 10.0, 9.5, 1.0, 11.0, -1.0).is_none());
    // A sweep entirely past the end is rejected by the range test first.
    assert!(line_intersection_h(0.0, 0.0, 10.0, 10.05, 1.0, 10.05, -1.0).is_none());
    // Horizontal sweep against a horizontal line: parallel, none.
    assert!(line_intersection_h(0.0, 0.0, 10.0, 2.0, 0.0, 8.0, 0.0).is_none());
    // V mirrors H.
    assert_eq!(
        line_intersection_v(0.0, 0.0, 10.0, -1.0, 9.5, 1.0, 10.625),
        Some((0.0, 10.0))
    );
    assert!(line_intersection_v(0.0, 0.0, 10.0, -1.0, 9.5, 1.0, 11.0).is_none());
    assert!(line_intersection_v(0.0, 0.0, 10.0, 1.0, 5.0, -1.0, 5.0).is_none());

    // remap_2d: point midway along a moves midway along b.
    assert_eq!(
        remap_2d(0.0, 0.0, 10.0, 0.0, 0.0, 5.0, 10.0, 5.0, 5.0, 1.0),
        (5.0, 6.0)
    );
    // Degenerate a: the C's asymmetric fallback.
    assert_eq!(
        remap_2d(1.0, 1.0, 1.0, 1.0, 2.0, 3.0, 4.0, 5.0, 0.0, 0.0),
        (
            0.0 + (2.0 - 1.0) + (4.0 - 1.0),
            0.0 + (3.0 - 1.0) + (5.0 - 1.0)
        )
    );
}

// ---------------------------------------------------------------------------
// mpcoll.c
// ---------------------------------------------------------------------------

mod mpcoll {
    use super::*;
    use crate::{
        air_flags, copy_coll_data, interpolate_ecb, load_ecb, load_ecb_fixed, load_ecb_jobj,
        sanitize_desired_ecb, set_ecb_angle, set_ecb_source_fixed, set_ecb_source_jobj,
        set_facing_dir, set_ledge_snap, squeeze_horizontal, squeeze_vertical, terrain_speed_scale,
        TERRAIN_SPEED_SCALE,
    };
    use gekko_math::msl::{cosf, sinf};
    use hsd_types::Vec2;
    use melee_types::mp::{collide, CollData, EcbSourceParams, FtCollisionBox};

    /// A fighter-like `CollData` with a fixed 8-tall, 4-wide ECB, standing
    /// still at `pos`.
    fn fixed_body(m: &CollMap, pos: Vec3) -> CollData {
        let mut cd = CollData {
            cur_pos: pos,
            ..Default::default()
        };
        m.coll_data_init(&mut cd);
        set_ecb_source_fixed(&mut cd, 8.0, 0.0, 4.0, 4.0);
        cd
    }

    /// Move the body so the next pass sweeps from `from` to `to`.
    fn sweep(cd: &mut CollData, from: Vec3, to: Vec3) {
        cd.last_pos = from;
        cd.prev_pos = from;
        cd.cur_pos = to;
    }

    #[test]
    fn coll_data_init_and_fixed_source() {
        let m = fd();
        let cd = fixed_body(&m, Vec3::new(1.0, 2.0, 0.0));
        assert_eq!(cd.prev_pos, Vec3::new(1.0, 2.0, 0.0));
        assert_eq!(cd.floor.index, NO_ID);
        assert_eq!(cd.floor.normal, Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(cd.ceiling.normal, Vec3::new(0.0, -1.0, 0.0));
        assert_eq!(cd.joint_id_skip, NO_ID);
        assert_eq!(cd.x38, m.transform_update_count());
        // The fixed source installed the default diamond and faces left.
        assert_eq!(cd.ecb.top, Vec2::new(0.0, 8.0));
        assert_eq!(cd.ecb.right, Vec2::new(4.0, 4.0));
        assert_eq!(cd.prev_ecb, cd.ecb);
        assert_eq!(cd.facing_dir, -1);
        assert!(matches!(
            cd.ecb_source.params,
            EcbSourceParams::Fixed {
                up: 8.0,
                down: 0.0,
                front: 4.0,
                back: 4.0,
                angle: 0.0
            }
        ));
    }

    #[test]
    fn air_collide_lands_on_floor() {
        let mut m = fd();
        let mut cd = fixed_body(&m, Vec3::ZERO);
        sweep(&mut cd, Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.0, -1.0, 0.0));
        let landed = m.air_collide_pass(&mut cd, None);
        assert!(landed);
        assert!(cd.x34_flags.b5);
        assert_eq!(cd.floor.index, FLOOR);
        assert_eq!(cd.floor.flags, line_flag::LEDGE);
        let env = cd.env_flags as u32;
        assert_ne!(env & collide::FLOOR_PUSH, 0);
        assert_ne!(env & collide::FLOOR_HUG, 0);
        assert_eq!(env & collide::WALL_MASK, 0);
        // Snapped: the ECB bottom lands on the floor with the probe's
        // +0.0001 bias, computed the same way.
        let probe = m.floor_probe(FLOOR, &Vec3::new(0.0, -1.0, 0.0)).unwrap();
        assert_eq!(cd.cur_pos.y, -1.0 + probe.delta);
        assert_eq!(cd.cur_pos.x, 0.0);
        assert_eq!(cd.contact, Vec3::new(0.0, 0.0, 0.0));
        // The ECB reached its desired shape in one step (6 units of travel).
        assert_eq!(cd.ecb, cd.desired_ecb);
        assert!(!m.is_on_platform(&cd));
        assert_eq!(m.floor_speed_scale(&cd), 1.0);

        // Standing well above the floor: no landing, position unchanged.
        let mut cd = fixed_body(&m, Vec3::ZERO);
        sweep(
            &mut cd,
            Vec3::new(0.0, 40.0, 0.0),
            Vec3::new(0.0, 39.0, 0.0),
        );
        assert!(!m.air_collide_pass(&mut cd, None));
        assert_eq!(cd.cur_pos, Vec3::new(0.0, 39.0, 0.0));
        assert_eq!(cd.env_flags, 0);
    }

    #[test]
    fn long_moves_are_stepped() {
        let mut m = fd();
        let mut cd = fixed_body(&m, Vec3::ZERO);
        // 40 units of fall in one frame (off to the side of the platform):
        // 7 steps of 40/7, and the landing stops the loop early with b5.
        sweep(
            &mut cd,
            Vec3::new(50.0, 30.0, 0.0),
            Vec3::new(50.0, -10.0, 0.0),
        );
        assert!(m.air_collide_pass(&mut cd, None));
        assert_eq!(cd.floor.index, FLOOR);
        // The step that landed started above the floor.
        assert!(cd.prev_pos.y > 0.0);
        assert!(cd.cur_pos.y < 0.01 && cd.cur_pos.y >= 0.0);
    }

    #[test]
    fn platform_pass_callback_vetoes_platform() {
        let mut m = fd();
        // Falling through the platform at y = 30 with the pass callback
        // refusing it: no landing.
        let mut cd = fixed_body(&m, Vec3::ZERO);
        sweep(
            &mut cd,
            Vec3::new(0.0, 33.0, 0.0),
            Vec3::new(0.0, 28.0, 0.0),
        );
        let mut veto = |_: &CollMap, id: i32| id != PLATFORM;
        assert!(!m.air_collide_platform_pass(&mut cd, Some(&mut veto), None));
        assert_eq!(cd.cur_pos.y, 28.0);
        // Without the veto the platform catches the body.
        let mut cd = fixed_body(&m, Vec3::ZERO);
        sweep(
            &mut cd,
            Vec3::new(0.0, 33.0, 0.0),
            Vec3::new(0.0, 28.0, 0.0),
        );
        let mut allow = |_: &CollMap, _id: i32| true;
        assert!(m.air_collide_platform_pass(&mut cd, Some(&mut allow), None));
        assert_eq!(cd.floor.index, PLATFORM);
        assert!(m.is_on_platform(&cd));
        // And floor_skip on the platform also lets the body through.
        let mut cd = fixed_body(&m, Vec3::ZERO);
        cd.floor_skip = PLATFORM;
        sweep(
            &mut cd,
            Vec3::new(0.0, 33.0, 0.0),
            Vec3::new(0.0, 28.0, 0.0),
        );
        assert!(!m.air_collide_pass(&mut cd, None));
    }

    #[test]
    fn air_collide_pushes_out_of_right_wall() {
        let mut m = fd();
        let mut cd = fixed_body(&m, Vec3::ZERO);
        // Moving left into the stage's right wall (x = 85.5) from outside;
        // the ECB's left point (x - 4) crosses it.
        sweep(
            &mut cd,
            Vec3::new(95.0, -25.0, 0.0),
            Vec3::new(80.0, -25.0, 0.0),
        );
        assert!(!m.air_collide_pass(&mut cd, None));
        assert_eq!(cd.cur_pos.x, STAGE_HALF + 4.0);
        assert_eq!(cd.cur_pos.y, -25.0);
        assert_eq!(cd.right_facing_wall.index, RIGHT_WALL);
        assert_eq!(cd.right_facing_wall.normal, Vec3::new(1.0, 0.0, 0.0));
        let env = cd.env_flags as u32;
        assert_ne!(env & collide::RIGHT_WALL_PUSH, 0);
        assert_ne!(env & collide::RIGHT_WALL_HUG, 0);
        assert_eq!(env & collide::LEFT_WALL_MASK, 0);

        // Mirror: moving right into the left wall.
        let mut cd = fixed_body(&m, Vec3::ZERO);
        sweep(
            &mut cd,
            Vec3::new(-95.0, -25.0, 0.0),
            Vec3::new(-80.0, -25.0, 0.0),
        );
        assert!(!m.air_collide_pass(&mut cd, None));
        assert_eq!(cd.cur_pos.x, -STAGE_HALF - 4.0);
        assert_eq!(cd.left_facing_wall.index, LEFT_WALL);
        let env = cd.env_flags as u32;
        assert_ne!(env & collide::LEFT_WALL_PUSH, 0);
        assert_eq!(env & collide::RIGHT_WALL_MASK, 0);
    }

    #[test]
    fn ceiling_pushes_body_down() {
        let mut m = fd();
        let mut cd = fixed_body(&m, Vec3::ZERO);
        // Rising into the bottom ceiling at y = -50 from below.
        sweep(
            &mut cd,
            Vec3::new(0.0, -62.0, 0.0),
            Vec3::new(0.0, -55.0, 0.0),
        );
        assert!(!m.air_collide_pass(&mut cd, None));
        assert_eq!(cd.ceiling.index, CEILING);
        let env = cd.env_flags as u32;
        assert_ne!(env & collide::CEILING_PUSH, 0);
        assert_ne!(env & collide::CEILING_HUG, 0);
        // The ECB top (8 above the position) sits just under the ceiling.
        let probe = m
            .ceiling_probe(CEILING, &Vec3::new(0.0, -55.0 + 8.0, 0.0))
            .unwrap();
        assert_eq!(cd.cur_pos.y, -55.0 + probe.delta);
        assert!(cd.cur_pos.y + 8.0 < STAGE_BOTTOM);
    }

    #[test]
    fn ground_collide_walks_off_or_stops_at_edge() {
        let mut m = fd();
        // Standing on the floor at x = 82, walking to x = 88 past the right
        // ledge with no edge flag (one 6-unit step): the body walks off (no
        // floor) and the ledge-slip flag is raised.
        let mut cd = fixed_body(&m, Vec3::ZERO);
        cd.floor.index = FLOOR;
        cd.floor.flags = line_flag::LEDGE;
        sweep(
            &mut cd,
            Vec3::new(82.0, 0.0, 0.0),
            Vec3::new(88.0, 0.0, 0.0),
        );
        assert!(!m.ground_collide_pass(&mut cd, None));
        assert_eq!(cd.cur_pos.x, 88.0);
        let env = cd.env_flags as u32;
        assert_ne!(env & collide::RIGHT_LEDGE_SLIP, 0);
        assert_eq!(env & collide::FLOOR_PUSH, 0);

        // The same walk with the stop-at-edge flag halts at the ledge.
        let mut cd = fixed_body(&m, Vec3::ZERO);
        cd.floor.index = FLOOR;
        cd.floor.flags = line_flag::LEDGE;
        sweep(
            &mut cd,
            Vec3::new(80.0, 0.0, 0.0),
            Vec3::new(90.0, 0.0, 0.0),
        );
        assert!(m.ground_collide_stop_at_edge(&mut cd, None));
        assert_eq!(cd.cur_pos, Vec3::new(STAGE_HALF, 0.0, 0.0));
        assert_eq!(cd.floor.index, FLOOR);
        let env = cd.env_flags as u32;
        // The C names the right end of a floor `Collide_LeftEdge`.
        assert_ne!(env & collide::LEFT_EDGE, 0);
        assert_ne!(env & collide::FLOOR_PUSH, 0);

        // Walking within the floor follows it.
        let mut cd = fixed_body(&m, Vec3::ZERO);
        cd.floor.index = FLOOR;
        sweep(&mut cd, Vec3::new(0.0, 0.0, 0.0), Vec3::new(5.0, 0.0, 0.0));
        assert!(m.ground_collide_pass(&mut cd, None));
        assert_eq!(cd.cur_pos.x, 5.0);
        assert_eq!(cd.floor.index, FLOOR);

        // Teeter: facing right at the right edge with the stick neutral.
        let mut cd = fixed_body(&m, Vec3::ZERO);
        cd.floor.index = FLOOR;
        set_facing_dir(&mut cd, 1);
        cd.lstick_x = 0.0;
        sweep(
            &mut cd,
            Vec3::new(80.0, 0.0, 0.0),
            Vec3::new(90.0, 0.0, 0.0),
        );
        // teeter leaves `touching_floor` false: the body is held at the edge
        // but reports airborne-style, with Collide_Edge set.
        assert!(!m.ground_collide_teeter(&mut cd, None));
        assert_eq!(cd.cur_pos.x, STAGE_HALF);
        assert_ne!(cd.env_flags as u32 & collide::EDGE, 0);
        // Holding the stick outward past 0.75 walks off instead.
        let mut cd = fixed_body(&m, Vec3::ZERO);
        cd.floor.index = FLOOR;
        set_facing_dir(&mut cd, 1);
        cd.lstick_x = 1.0;
        sweep(
            &mut cd,
            Vec3::new(80.0, 0.0, 0.0),
            Vec3::new(90.0, 0.0, 0.0),
        );
        assert!(!m.ground_collide_teeter(&mut cd, None));
        assert_eq!(cd.cur_pos.x, 90.0);
    }

    #[test]
    fn resumed_ground_step_preserves_ecb_and_environment_history() {
        let mut complete_map = fd();
        let mut before = fixed_body(&complete_map, Vec3::ZERO);
        before.floor.index = FLOOR;
        before.env_flags = collide::FLOOR_PUSH as i32;
        // The next pose grows, so replaying interpolation would destroy history.
        let EcbSourceParams::Fixed { ref mut up, .. } = before.ecb_source.params else {
            unreachable!()
        };
        *up = 10.0;
        let mut complete = before;
        assert!(complete_map.ground_collide_teeter(&mut complete, None));

        let mut suspended = before;
        crate::coll_prev(&mut suspended);
        crate::load_ecb_with_flags(&mut suspended, 5, None);
        suspended.prev_env_flags = suspended.env_flags;
        suspended.env_flags = 0;
        suspended.x34_flags.b5 = false;
        interpolate_ecb(&mut suspended, 1.0);
        suspended.prev_pos = suspended.cur_pos;
        assert_ne!(suspended.ecb, suspended.prev_ecb);
        let mut resumed_map = fd();
        assert!(resumed_map.resume_ground_teeter(&mut suspended, None, true));
        assert_eq!(suspended, complete);
    }

    #[test]
    fn ledge_grabs_both_sides() {
        let mut m = fd();
        // Falling beside the stage's left edge, facing right.
        let mut cd = fixed_body(&m, Vec3::ZERO);
        set_facing_dir(&mut cd, 1);
        set_ledge_snap(&mut cd, 5.0, 0.0, 10.0);
        sweep(
            &mut cd,
            Vec3::new(-90.0, 5.0, 0.0),
            Vec3::new(-90.0, -3.0, 0.0),
        );
        assert!(!m.air_collide_ledge(&mut cd, None));
        let env = cd.env_flags as u32;
        assert_ne!(env & collide::LEFT_LEDGE_GRAB, 0);
        assert_eq!(env & collide::RIGHT_LEDGE_GRAB, 0);
        assert_eq!(cd.ledge_id_left, FLOOR);
        assert_eq!(cd.ledge_id_right, NO_ID);
        assert_eq!(cd.contact, Vec3::new(-STAGE_HALF, 0.0, 0.0));

        // Mirror: beside the right edge, facing left.
        let mut cd = fixed_body(&m, Vec3::ZERO);
        set_facing_dir(&mut cd, -1);
        set_ledge_snap(&mut cd, 5.0, 0.0, 10.0);
        sweep(
            &mut cd,
            Vec3::new(90.0, 5.0, 0.0),
            Vec3::new(90.0, -3.0, 0.0),
        );
        assert!(!m.air_collide_ledge(&mut cd, None));
        let env = cd.env_flags as u32;
        assert_ne!(env & collide::RIGHT_LEDGE_GRAB, 0);
        assert_eq!(cd.ledge_id_right, FLOOR);
        assert_eq!(cd.contact, Vec3::new(STAGE_HALF, 0.0, 0.0));

        // Facing away from the ledge: no grab.
        let mut cd = fixed_body(&m, Vec3::ZERO);
        set_facing_dir(&mut cd, -1);
        set_ledge_snap(&mut cd, 5.0, 0.0, 10.0);
        sweep(
            &mut cd,
            Vec3::new(-90.0, 5.0, 0.0),
            Vec3::new(-90.0, -3.0, 0.0),
        );
        assert!(!m.air_collide_ledge(&mut cd, None));
        assert_eq!(cd.env_flags as u32 & collide::LEDGE_GRAB_MASK, 0);
        // Rising: no grab.
        let mut cd = fixed_body(&m, Vec3::ZERO);
        set_facing_dir(&mut cd, 1);
        set_ledge_snap(&mut cd, 5.0, 0.0, 10.0);
        sweep(
            &mut cd,
            Vec3::new(-90.0, -3.0, 0.0),
            Vec3::new(-90.0, 2.0, 0.0),
        );
        assert!(!m.air_collide_ledge(&mut cd, None));
        assert_eq!(cd.env_flags as u32 & collide::LEDGE_GRAB_MASK, 0);
        // Without CAN_GRAB_LEDGE the same fall grabs nothing.
        let mut cd = fixed_body(&m, Vec3::ZERO);
        set_facing_dir(&mut cd, 1);
        set_ledge_snap(&mut cd, 5.0, 0.0, 10.0);
        sweep(
            &mut cd,
            Vec3::new(-90.0, 5.0, 0.0),
            Vec3::new(-90.0, -3.0, 0.0),
        );
        assert!(!m.air_collide_pass(&mut cd, None));
        assert_eq!(cd.env_flags as u32 & collide::LEDGE_GRAB_MASK, 0);
        // The ledge grab fires the joint's cb_0 with ground kind 3.
        static KINDS: std::sync::Mutex<Vec<i32>> = std::sync::Mutex::new(Vec::new());
        fn cb(_u: u32, _j: i32, _c: &mut CollData, _x: i32, kind: i32, _dy: f32) {
            KINDS.lock().unwrap().push(kind);
        }
        m.joint_set_cb1(0, 1, cb);
        m.notify_ledge_grab(&mut cd, FLOOR);
        assert_eq!(KINDS.lock().unwrap().as_slice(), &[3]);
    }

    #[test]
    fn point_collide_snaps_to_first_surface() {
        let mut m = fd();
        let mut cd = fixed_body(&m, Vec3::ZERO);
        sweep(&mut cd, Vec3::new(0.0, 3.0, 0.0), Vec3::new(0.0, -2.0, 0.0));
        assert!(m.point_collide_pass(&mut cd));
        assert_eq!(cd.floor.index, FLOOR);
        let probe = m.floor_probe(FLOOR, &Vec3::new(0.0, 0.0, 0.0)).unwrap();
        assert_eq!(cd.cur_pos.y, 0.0 + probe.delta);
        assert_ne!(cd.env_flags as u32 & collide::FLOOR_PUSH, 0);
        // Into the right wall from outside.
        let mut cd = fixed_body(&m, Vec3::ZERO);
        sweep(
            &mut cd,
            Vec3::new(88.0, -25.0, 0.0),
            Vec3::new(84.0, -25.0, 0.0),
        );
        assert!(m.point_collide_pass(&mut cd));
        assert_eq!(cd.right_facing_wall.index, RIGHT_WALL);
        assert_eq!(cd.cur_pos.x, STAGE_HALF);
        // Nothing crossed.
        let mut cd = fixed_body(&m, Vec3::ZERO);
        sweep(
            &mut cd,
            Vec3::new(0.0, 20.0, 0.0),
            Vec3::new(0.0, 15.0, 0.0),
        );
        assert!(!m.point_collide_pass(&mut cd));
    }

    #[test]
    fn point_inside_solid_uses_vertical_squeeze() {
        // A 12-tall slab: floor at y = 0, ceiling at y = 12. The probe body
        // is 20 tall (fixed 10 up, 10 down), so the ECB change forces two
        // steps in mpColl_80043754. mpCollInterpolateECB clears `b6` at the
        // start of every step and mpCollSqueezeVertical clears `b5`, so the
        // squeeze reaches the return value only when the shrunken body is
        // squeezed again in the second step: near the ceiling it is, in the
        // middle of the slab it fits after the first squeeze.
        let mut b = CollMapBuilder::new();
        let f0 = b.vertex(0, -50.0, 0.0);
        let f1 = b.vertex(0, 50.0, 0.0);
        let c0 = b.vertex(0, 50.0, 12.0);
        let c1 = b.vertex(0, -50.0, 12.0);
        b.floor(0, f0, f1, 0);
        b.ceiling(0, c0, c1, 0);
        let mut m = CollMap::load(b.build(), 1.0, GrKind::Last);
        assert!(m.is_point_inside_solid(&Vec3::new(0.0, 11.5, 0.0)));
        assert!(!m.is_point_inside_solid(&Vec3::new(0.0, 6.0, 0.0)));
        assert!(!m.is_point_inside_solid(&Vec3::new(0.0, 40.0, 0.0)));
        // Open FD interior is 50 tall: a 20-tall probe body is not squeezed.
        let mut fdm = fd();
        assert!(!fdm.is_point_inside_solid(&Vec3::new(0.0, -25.0, 0.0)));
    }

    #[test]
    fn coll_end_applies_dynamic_attribute() {
        let mut m = fd();
        fn hook(_pos: &Vec3, floor_id: i32) -> i32 {
            if floor_id == FLOOR {
                7
            } else {
                0
            }
        }
        m.dynamic_attr_hook = Some(hook);
        let mut cd = fixed_body(&m, Vec3::ZERO);
        sweep(&mut cd, Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.0, -1.0, 0.0));
        assert!(m.air_collide_pass(&mut cd, None));
        assert_eq!(cd.floor.flags & line_flag::MATERIAL_MASK, 7);
        assert_ne!(cd.floor.flags & line_flag::LEDGE, 0);
        assert_eq!(m.floor_speed_scale(&cd), TERRAIN_SPEED_SCALE[7]);
    }

    #[test]
    fn ecb_loaders() {
        // Fixed, unrotated, facing right: front is +x.
        let mut cd = CollData::default();
        cd.x34_flags.b0 = true;
        set_ecb_source_fixed(&mut cd, 10.0, 2.0, 6.0, 3.0);
        set_facing_dir(&mut cd, 1);
        load_ecb_fixed(&mut cd);
        assert_eq!(cd.desired_ecb.top, Vec2::new(0.0, 10.0));
        assert_eq!(cd.desired_ecb.bottom, Vec2::new(0.0, -2.0));
        assert_eq!(cd.desired_ecb.right, Vec2::new(6.0, 4.0));
        assert_eq!(cd.desired_ecb.left, Vec2::new(-3.0, 4.0));
        assert!(!cd.x34_flags.b0);
        // Facing left swaps front and back.
        set_facing_dir(&mut cd, -1);
        load_ecb_fixed(&mut cd);
        assert_eq!(cd.desired_ecb.right.x, 3.0);
        assert_eq!(cd.desired_ecb.left.x, -6.0);
        // Too thin is widened to 1.5 each side.
        set_ecb_source_fixed(&mut cd, 1.0, 1.0, 1.0, 1.0);
        load_ecb_fixed(&mut cd);
        assert_eq!(cd.desired_ecb.top.y, 1.5);
        assert_eq!(cd.desired_ecb.bottom.y, -1.5);
        assert_eq!(cd.desired_ecb.right.x, 1.5);
        assert_eq!(cd.desired_ecb.left.x, -1.5);

        // Rotated by a quarter turn: the extents swap axes. Expected values
        // follow the C's op order with MSL sinf/cosf.
        set_ecb_source_fixed(&mut cd, 10.0, 2.0, 6.0, 3.0);
        set_facing_dir(&mut cd, 1);
        let angle = 1.5707964f32;
        set_ecb_angle(&mut cd, angle);
        load_ecb_fixed(&mut cd);
        let (s, c) = (sinf(angle), cosf(angle));
        let mid_x = 0.5 * (6.0f32 + -3.0f32);
        let pts_x = [
            -10.0f32 * s,
            -(-2.0f32) * s,
            (6.0 * c) - (mid_x * s),
            (-3.0 * c) - (mid_x * s),
        ];
        let pts_y = [
            10.0f32 * c,
            -2.0f32 * c,
            (6.0 * s) + (mid_x * c),
            (-3.0 * s) + (mid_x * c),
        ];
        let (mut lx, mut rx, mut by, mut ty) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
        for i in 0..4 {
            if rx < pts_x[i] {
                rx = pts_x[i];
            } else if lx > pts_x[i] {
                lx = pts_x[i];
            }
            if ty < pts_y[i] {
                ty = pts_y[i];
            } else if by > pts_y[i] {
                by = pts_y[i];
            }
        }
        assert_eq!(cd.desired_ecb.top.y, ty);
        assert_eq!(cd.desired_ecb.bottom.y, by);
        assert_eq!(cd.desired_ecb.right.x, rx);
        assert_eq!(cd.desired_ecb.left.x, lx);
        // Angle wraps once past tau, in f64 as the C does with its double
        // M_TAU literal.
        set_ecb_angle(&mut cd, 7.0);
        assert!(
            matches!(cd.ecb_source.params, EcbSourceParams::Fixed { angle, .. }
            if angle == (7.0f64 - core::f64::consts::TAU) as f32)
        );

        // JObj source: the ECB bounds six bone positions, padded by 2.
        let mut cd = CollData::default();
        cd.x34_flags.b0 = true;
        cd.cur_pos = Vec3::new(10.0, 20.0, 0.0);
        set_ecb_source_jobj(
            &mut cd,
            Some(0),
            [Some(1), Some(2), Some(3), Some(4), Some(5), Some(6)],
            0.5,
        );
        let bones = |slot: Option<u32>| -> Vec3 {
            match slot {
                Some(1) => Vec3::new(10.0, 21.0, 0.0),
                Some(2) => Vec3::new(13.0, 25.0, 0.0),
                Some(3) => Vec3::new(7.0, 25.0, 0.0),
                Some(4) => Vec3::new(10.0, 30.0, 0.0),
                Some(5) => Vec3::new(11.0, 24.0, 0.0),
                Some(6) => Vec3::new(9.0, 24.0, 0.0),
                _ => Vec3::ZERO,
            }
        };
        load_ecb_jobj(&mut cd, 0, &bones);
        // x in [-3, 3] -> [-5, 5]; y in [1, 10] -> [-1, 12], bottom clamped
        // to 0.
        assert_eq!(cd.desired_ecb.left.x, -5.0);
        assert_eq!(cd.desired_ecb.right.x, 5.0);
        assert_eq!(cd.desired_ecb.bottom.y, 0.0);
        assert_eq!(cd.desired_ecb.top.y, 12.0);
        assert_eq!(cd.desired_ecb.right.y, 0.5 + 0.5 * (0.0 + 12.0));
        // Grounded flag zeroes the bottom; 0x10 forces a 2-tall body.
        load_ecb_jobj(&mut cd, 0x11, &bones);
        assert_eq!(cd.desired_ecb.bottom.y, 0.0);
        assert_eq!(cd.desired_ecb.top.y, 2.0);
        // Ledge-grab flag skips the padding; 8 forces +-1 width.
        load_ecb_jobj(&mut cd, 0x8 | air_flags::CAN_GRAB_LEDGE, &bones);
        assert_eq!(cd.desired_ecb.left.x, -1.0);
        assert_eq!(cd.desired_ecb.right.x, 1.0);
        assert_eq!(cd.desired_ecb.top.y, 10.0);
        // The generic loader picks the JObj path and needs bones. Its flags
        // (6) include CAN_GRAB_LEDGE, so the 2-unit padding is skipped.
        load_ecb(&mut cd, Some(&bones));
        assert_eq!(cd.desired_ecb.top.y, 10.0);
        assert_eq!(cd.desired_ecb.left.x, -3.0);

        // Box loader.
        let mut cd = CollData::default();
        crate::load_ecb_box(
            &mut cd,
            &FtCollisionBox {
                top: 9.0,
                bottom: 1.0,
                left: Vec2::new(-2.0, 5.0),
                right: Vec2::new(2.0, 5.0),
            },
        );
        assert_eq!(cd.desired_ecb.top, Vec2::new(0.0, 9.0));
        assert_eq!(cd.desired_ecb.left, Vec2::new(-2.0, 5.0));

        // Sanitize: a degenerate diamond is opened up.
        let mut cd = CollData::default();
        cd.desired_ecb.top = Vec2::new(0.0, 0.5);
        cd.desired_ecb.bottom = Vec2::new(0.0, 0.0);
        cd.desired_ecb.right = Vec2::new(0.2, 0.25);
        cd.desired_ecb.left = Vec2::new(-0.2, 0.25);
        sanitize_desired_ecb(&mut cd);
        assert_eq!(cd.desired_ecb.top.y, 1.5);
        assert_eq!(cd.desired_ecb.right.x, 1.0);
        assert_eq!(cd.desired_ecb.left.x, -1.0);
        assert_eq!(cd.desired_ecb.right.y, 0.75);
    }

    #[test]
    fn interpolation_and_squeezes() {
        let mut cd = CollData::default();
        cd.ecb.top = Vec2::new(0.0, 8.0);
        cd.desired_ecb.top = Vec2::new(0.0, 12.0);
        interpolate_ecb(&mut cd, 0.5);
        assert_eq!(cd.prev_ecb.top.y, 8.0);
        assert_eq!(cd.ecb.top.y, 8.0 + 0.5 * (12.0 - 8.0));
        // b6 restores the pre-squeeze ECB first.
        cd.x34_flags.b6 = true;
        cd.x64_ecb.top = Vec2::new(0.0, 4.0);
        interpolate_ecb(&mut cd, 1.0);
        assert!(!cd.x34_flags.b6);
        assert_eq!(cd.ecb.top.y, 12.0);

        let mut cd = CollData::default();
        cd.ecb.right = Vec2::new(4.0, 4.0);
        cd.ecb.left = Vec2::new(-4.0, 4.0);
        cd.cur_pos.x = 10.0;
        squeeze_horizontal(&mut cd, true, 8.0, 12.0);
        // Gap 4 wide plus ECB width 8: half is 6; centred on the gap.
        assert_eq!(cd.ecb.right.x, 6.0);
        assert_eq!(cd.ecb.left.x, -6.0);
        assert_eq!(cd.cur_pos.x, (12.0 + 4.0) - 6.0);
        assert!(cd.x34_flags.b6);
        assert_eq!(cd.x64_ecb.right.x, 4.0);

        let mut cd = CollData::default();
        cd.ecb.top = Vec2::new(0.0, 8.0);
        cd.ecb.bottom = Vec2::new(0.0, 0.0);
        squeeze_vertical(&mut cd, false, 5.0, 2.0);
        // Grounded: sit on the floor, top pulled down to the ceiling.
        assert_eq!(cd.cur_pos.y, 2.0);
        assert_eq!(cd.ecb.top.y, (5.0 - 2.0 + 8.0 - 0.0) + 0.0);
        assert_eq!(cd.ecb.right.y, 0.5 * (cd.ecb.top.y + cd.ecb.bottom.y));
        let mut cd = CollData::default();
        cd.ecb.top = Vec2::new(0.0, 8.0);
        squeeze_vertical(&mut cd, true, 5.0, 2.0);
        // Airborne: centred between them.
        assert_eq!(cd.cur_pos.y, 0.5 * (5.0 + 2.0));
        let mut cd = CollData::default();
        cd.ecb.top = Vec2::new(0.0, 8.0);
        squeeze_vertical(&mut cd, true, 2.0, 8.0);
        // Height would be under 3: collapse to a min-height body on the
        // floor.
        assert_eq!(cd.cur_pos.y, 8.0);
        assert_eq!(cd.ecb.bottom.y, 0.0);
        assert_eq!(cd.ecb.top.y, 2.0);
    }

    #[test]
    fn copy_and_terrain_table() {
        let src = CollData {
            cur_pos: Vec3::new(1.0, 2.0, 3.0),
            floor: melee_types::mp::SurfaceData {
                index: 4,
                ..Default::default()
            },
            x50: 9.0,
            joint_id_only: 2,
            ..Default::default()
        };
        let mut dst = CollData::default();
        copy_coll_data(&src, &mut dst, 0);
        assert_eq!(dst.cur_pos, src.cur_pos);
        assert_eq!(dst.floor.index, 4);
        // Not copied by the C.
        assert_eq!(dst.x50, 0.0);
        assert_eq!(dst.joint_id_only, 0);

        assert_eq!(terrain_speed_scale(0), 1.0);
        assert_eq!(terrain_speed_scale(2), 1.5);
        assert_eq!(terrain_speed_scale(12 | line_flag::LEDGE), 0.1);
        assert_eq!(terrain_speed_scale(15), 0.2);
    }

    /// The decomp writes `M_TAU` as the double literal 6.283185307179586;
    /// the port uses the std constant, which must be the same f64.
    #[test]
    #[allow(clippy::approx_constant)]
    fn m_tau_literal_matches_std() {
        assert_eq!(
            6.283185307179586f64.to_bits(),
            core::f64::consts::TAU.to_bits()
        );
    }

    #[test]
    #[should_panic(expected = "not support rotate")]
    fn jobj_source_cannot_rotate() {
        let mut cd = CollData::default();
        set_ecb_source_jobj(&mut cd, None, [None; 6], 0.0);
        set_ecb_angle(&mut cd, 1.0);
    }
}
