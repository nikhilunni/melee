//! Exact rational branch cases for lbColl_80006094; no sampled game data.
use hsd_types::Vec3;
use melee_coll::geometry::{hitbox_pair_contact, Capsule};

fn capsule(a: [f32; 3], b: [f32; 3], radius: f32) -> Capsule {
    Capsule {
        start: Vec3::new(a[0], a[1], a[2]),
        end: Vec3::new(b[0], b[1], b[2]),
        radius,
    }
}
fn bits(point: Vec3) -> [u32; 3] {
    [point.x.to_bits(), point.y.to_bits(), point.z.to_bits()]
}
fn contact(a: Capsule, b: Capsule, x: [f32; 3], y: [f32; 3]) {
    let actual = hitbox_pair_contact(a, b).expect("contact");
    assert_eq!(bits(actual.0), x.map(f32::to_bits));
    assert_eq!(bits(actual.1), y.map(f32::to_bits));
}

#[test]
fn clank_geometry_degenerate_segment_branches_exact() {
    contact(
        capsule([0., 0., 0.], [0., 0., 0.], 1.),
        capsule([0., 1., 0.], [0., 1., 0.], 0.),
        [0., 0., 0.],
        [0., 1., 0.],
    );
    // Nondegenerate receiver, point incoming: interior projection.
    contact(
        capsule([0., 0., 0.], [4., 0., 0.], 1.),
        capsule([1., 1., 0.], [1., 1., 0.], 0.),
        [1., 0., 0.],
        [1., 1., 0.],
    );
    // Point receiver goes through the parallel branch instead.
    contact(
        capsule([1., 1., 0.], [1., 1., 0.], 1.),
        capsule([0., 0., 0.], [4., 0., 0.], 0.),
        [1., 1., 0.],
        [1., 0., 0.],
    );
}

#[test]
fn clank_geometry_parallel_midpoint_tie_selects_receiver_end() {
    contact(
        capsule([0., 0., 0.], [2., 0., 0.], 1.),
        capsule([0., 1., 0.], [2., 1., 0.], 0.),
        [2., 0., 0.],
        [2., 1., 0.],
    );
    // Reversing the receiver deliberately changes the tie-selected points.
    contact(
        capsule([2., 0., 0.], [0., 0., 0.], 1.),
        capsule([0., 1., 0.], [2., 1., 0.], 0.),
        [0., 0., 0.],
        [0., 1., 0.],
    );
    contact(
        capsule([0., 0., 0.], [8., 0., 0.], 1.),
        capsule([0., 1., 0.], [2., 1., 0.], 0.),
        [0., 0., 0.],
        [0., 1., 0.],
    );
}

#[test]
fn clank_geometry_crossing_and_endpoint_repair_exact() {
    contact(
        capsule([0., 0., 0.], [4., 0., 0.], 0.),
        capsule([1., -1., 0.], [1., 3., 0.], 0.),
        [1., 0., 0.],
        [1., 0., 0.],
    );
    contact(
        capsule([0., 0., 0.], [1., 0., 0.], 3.),
        capsule([2., 2., 0.], [2., 3., 0.], 0.),
        [1., 0., 0.],
        [2., 2., 0.],
    );
    // h outside / u interior: first repaired candidate wins strictly.
    contact(
        capsule([0., 0., 0.], [1., 0., 0.], 1.),
        capsule([2., -1., 0.], [2., 1., 0.], 0.),
        [1., 0., 0.],
        [2., 0., 0.],
    );
    // h interior / u outside: second repaired candidate wins strictly.
    contact(
        capsule([0., 0., 0.], [4., 0., 0.], 1.),
        capsule([1., 1., 0.], [1., 2., 0.], 0.),
        [1., 0., 0.],
        [1., 1., 0.],
    );
}

#[test]
fn clank_geometry_touch_is_inclusive_broadphase_and_distance() {
    let a = capsule([0., 0., 0.], [4., 0., 0.], 1.);
    let b = capsule([1., 1., 0.], [1., 1., 0.], 0.);
    assert!(hitbox_pair_contact(a, b).is_some());
    let below = Capsule {
        radius: f32::from_bits(1.0f32.to_bits() - 1),
        ..a
    };
    assert!(hitbox_pair_contact(below, b).is_none());
    // Diagonal separation passes all axis bounds but fails squared distance.
    let c = capsule([0., 0., 0.], [0., 0., 0.], 1.);
    assert!(hitbox_pair_contact(c, capsule([1., 1., 0.], [1., 1., 0.], 0.)).is_none());
}
