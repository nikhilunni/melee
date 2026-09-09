use hsd_archive::Archive;
use melee_gr::desc;
use std::path::PathBuf;

#[test]
fn battlefield_archive_geometry_positions_and_environment() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files/GrNBa.dat");
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("skipping Battlefield asset test: {} absent", path.display());
            return;
        }
        Err(error) => panic!("reading Battlefield archive: {error}"),
    };
    let archive = Archive::parse(&bytes).unwrap();
    let desc = desc::read_battlefield(&archive).unwrap();
    assert_eq!(desc.section_counts, [1, 7, 0, 34, 0, 4]);
    assert_eq!(desc.parameters.map_scale.to_bits(), 0.8_f32.to_bits());
    assert_eq!(desc.position_bindings.len(), 21);
    assert!(desc
        .models
        .iter()
        .all(|model| model.joint_mappings.is_empty()));
    let lights = melee_gr::battle::lights::load(&archive, &desc).unwrap();
    assert_eq!(lights.len(), 2);
    assert_eq!(
        lights[0],
        melee_gr::battle::lights::Light::Ambient {
            color: [128, 128, 128, 255]
        }
    );
    assert!(matches!(
        lights[1],
        melee_gr::battle::lights::Light::Directional {
            color: [255, 255, 255, 255],
            ..
        }
    ));
    let music = desc::read_music(&archive, 31).unwrap();
    assert_eq!(
        (music.primary, music.alternate, music.alternate_chance),
        (81, 38, 12)
    );
    let mut map = desc::load_collision(&archive, &desc).unwrap();
    for (x, from_y, height, line, soft) in [
        (0.0, 10.0, 0.0_f32, 1, false),
        (0.0, 60.0, 54.4, 3, true),
        (-40.0, 35.0, 27.2, 2, true),
        (40.0, 35.0, 27.2, 4, true),
    ] {
        let hit = map
            .check_floor(x, from_y, x, -10.0, 0.0, -1, -1, -1, None)
            .unwrap();
        assert_eq!(hit.pos.y.to_bits(), height.to_bits());
        assert_eq!(hit.line_id, line);
        assert_eq!(hit.flags & 0x100 != 0, soft);
    }
    let joints = desc.models[0].joint.descendants();
    for (position, height) in [(0, 8.0_f32), (1, 62.4)] {
        let binding = desc
            .position_bindings
            .iter()
            .find(|b| b.stage_position == position)
            .unwrap();
        let marker = joints[binding.joint_index as usize].position;
        assert_eq!(
            (marker.y * desc.parameters.map_scale).to_bits(),
            height.to_bits()
        );
        assert_eq!(marker.x.to_bits(), 0);
    }
}
