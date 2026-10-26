//! Mute City's models, splines and parameters come from GrMc.dat.
use hsd_archive::{desc::spline::SplineKind, Archive};
use melee_gr::{desc, mutecity::procs};
use std::path::PathBuf;

fn archive(name: &str) -> Option<Vec<u8>> {
    let root = std::env::var_os("MELEE_DATA_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
    let path = root.join("harness/roms/files").join(name);
    if !melee_test_support::require_files([&path]) {
        return None;
    }
    Some(std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display())))
}

#[test]
fn mute_city_models_splines_and_parameters_come_from_the_archive() {
    let Some(bytes) = archive("GrMc.dat") else {
        return;
    };
    let archive = Archive::parse(&bytes).unwrap();
    let stage = desc::read_mute_city(&archive).unwrap();
    assert_eq!(stage.kind, melee_types::GrKind::MuteCity);
    assert_eq!(stage.parameters.map_scale, 0.6);
    // The city (0x1E) plays the route; the platform (0x1D) carries every
    // collision joint but the track's (4) and plays two animations, the
    // second its tilting bone's.
    let city = &stage.models[usize::from(procs::CITY_MAP)];
    let platform = &stage.models[usize::from(procs::PLATFORM_MAP)];
    assert_eq!((city.animations.len(), city.animation_loops[0]), (1, true));
    assert!(city.joint_mappings.is_empty());
    assert_eq!(platform.animations.len(), 2);
    let joints: Vec<_> = platform
        .joint_mappings
        .iter()
        .map(|m| (m.joint_index, m.extra))
        .collect();
    assert_eq!(joints, [(3, 1), (0, 2), (1, 3), (2, 4), (5, 7)]);
    // GrMc.dat's map 0x1E animation names an unrelocated child (0xFFFFFFFF)
    // under bones 4 and 8, the spline joints' parents.
    let unrelocated: Vec<usize> = {
        fn walk(a: &hsd_archive::desc::AnimJoint, index: &mut usize, out: &mut Vec<usize>) {
            if a.unrelocated_child {
                out.push(*index);
            }
            *index += 1;
            for child in a.children() {
                walk(child, index, out);
            }
        }
        let mut out = Vec::new();
        walk(&city.animations[0], &mut 0, &mut out);
        out
    };
    assert_eq!(unrelocated.len(), 2);
    for bone in [procs::RIGHT_SPLINE_BONE, procs::LEFT_SPLINE_BONE] {
        let spline = desc::bone_spline(&archive, city, bone).unwrap();
        assert_eq!((spline.kind, spline.knots, spline.points.len()), (SplineKind::Cardinal, 49, 51));
        assert_eq!(spline.tension, 0.5);
    }
    let hazards = desc::read_mute_city_hazards(&archive).unwrap();
    println!("{hazards:?}");
    let p = desc::read_mute_city_parameters(&archive).unwrap();
    assert_eq!((p.smoke_damage, p.explode_damage), (8.0, 16.0));
    assert_eq!((p.top_speed, p.leader_top_speed, p.trailer_top_speed), (0.001, 0.0008, 0.0012));
    assert_eq!((p.acceleration, p.lane_step, p.lane_centering, p.braking), (0.00002, 0.01, 0.002, 0.0001));
}
