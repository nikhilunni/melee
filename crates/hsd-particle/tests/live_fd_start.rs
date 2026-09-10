mod common;
#[path = "support/dust_replay.rs"]
mod dust_replay;
#[path = "support/restore.rs"]
mod restore;
#[path = "support/fixture_spawns.rs"]
mod spawns;
use hsd_archive::Archive;
use std::{fs, path::PathBuf};
const SPAWN_FIXTURE: &str = include_str!("data/start_fd_spawns.json");
#[test]
fn live_fd_start_600_ticks_match_every_field_and_rng_draw() {
    dust_replay::replay_strict("start_fd_fox", 600);
}

#[test]
fn effect_animation_keys_identify_external_generator_requests() {
    use hsd_anim::fobj::FObj;
    use hsd_archive::desc::AnimJoint;
    fn tracks(node: &AnimJoint, output: &mut Vec<hsd_archive::desc::FObjDesc>) {
        for sibling in node.siblings() {
            if let Some(aobj) = &sibling.aobjdesc {
                output.extend(aobj.tracks().filter(|f| f.type_ == 0x28).cloned());
            }
            if let Some(child) = &sibling.child {
                tracks(child, output);
            }
        }
    }
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files/EfCoData.dat");
    if !path.exists() {
        eprintln!("skipping: {} absent", path.display());
        return;
    }
    let archive = Archive::parse(&fs::read(path).unwrap()).unwrap();
    for (kind, expected) in [
        (0x24, vec![(0, 0, 445), (0, 0, 449), (9, 0, 448)]),
        (0x18, vec![(0, 0, 10)]),
    ] {
        let desc = archive.public("effCommonDataTable").unwrap() + 8 + kind * 20;
        let offset = archive.link(desc + 8).unwrap().expect("effect animation");
        let anim = AnimJoint::read(&archive, offset).unwrap();
        let mut all = Vec::new();
        tracks(&anim, &mut all);
        let mut events = Vec::new();
        for track in all {
            let mut f = FObj::new(
                &track.ad,
                track.startframe,
                track.type_,
                track.frac_value,
                track.frac_slope,
            );
            f.req_anim(0.0);
            for tick in 0..60 {
                f.interpret_anim(
                    Some(&mut |_, value: f32| {
                        let bits = value.to_bits();
                        events.push((tick, bits & 63, (bits >> 6) & 0xffffff));
                    }),
                    1.0,
                );
            }
        }
        events.sort();
        assert_eq!(events, expected, "effect {kind:#x} spawn keys");
    }
}
