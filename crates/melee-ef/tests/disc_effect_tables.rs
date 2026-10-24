//! The loader invariants every effect table on the disc meets, which make
//! three of the effect loader's error arms unreachable (`unreachable!` in
//! `resources::character`, `Effect::load_table` and `animate_joints`).
use hsd_anim::jobj::{HSD_A_J_PATH, HSD_A_J_PTCLTGT, HSD_A_J_SETBYTE0, HSD_A_J_SETFLOAT9};
use hsd_archive::desc::anim::AnimJoint;
use hsd_archive::desc::effect_visual::EffectVisual;
use hsd_archive::desc::jobj::JObjUnion;
use hsd_archive::Archive;
use std::path::PathBuf;

/// The asset directory: `MELEE_DATA_ROOT` (a checkout with harness data, for
/// worktrees) or this checkout.
fn files() -> Option<PathBuf> {
    let root = std::env::var_os("MELEE_DATA_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    );
    let files = root.join("harness/roms/files");
    melee_test_support::require_files([files.join("EfCoData.dat")]).then_some(files)
}

/// `(track type, AObj obj_id)` for every track of an animation tree.
fn tracks(node: &AnimJoint, out: &mut Vec<(u8, u32)>) {
    if let Some(aobj) = &node.aobjdesc {
        out.extend(aobj.tracks().map(|track| (track.type_, aobj.obj_id)));
    }
    for next in [&node.child, &node.next].into_iter().flatten() {
        tracks(next, out);
    }
}

/// A table row (`EF_EffectDesc`, 20 bytes) holds a lifetime and four links;
/// the table ends at the first row whose link words are not all relocated
/// pointers or nulls.
fn rows(archive: &Archive, table: u32) -> u32 {
    let mut count = 0;
    loop {
        let row = table + 8 + count * 20;
        let links = (1..5).map(|word| row + word * 4);
        let is_row = archive.reader().u32(row + 16).is_ok()
            && links
                .clone()
                .all(|at| archive.is_relocated_offset(at) || archive.reader().u32(at) == Ok(0));
        if !is_row {
            return count;
        }
        count += 1;
    }
}

#[test]
fn every_effect_table_on_the_disc_meets_the_loader_invariants() {
    let Some(files) = files() else {
        return;
    };
    let mut names: Vec<String> = std::fs::read_dir(&files)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.starts_with("Ef") && name.ends_with(".dat"))
        .collect();
    names.sort();
    let (mut tables, mut models, mut paths) = (0, 0, 0);
    for name in &names {
        let archive = Archive::parse(&std::fs::read(files.join(name)).unwrap()).unwrap();
        for symbol in archive.publics().iter().filter(|s| s.name.ends_with("DataTable")) {
            tables += 1;
            let table = archive.public(&symbol.name).unwrap();
            // efAsync_OnLoad: particle commands and textures come in pairs.
            let commands = archive.link(table).unwrap();
            let textures = archive.link(table + 4).unwrap();
            assert_eq!(commands.is_some(), textures.is_some(), "{name} {}", symbol.name);
            for index in 0..rows(&archive, table) {
                if archive.link(table + 8 + index * 20 + 4).unwrap().is_none() {
                    continue;
                }
                models += 1;
                let visual = EffectVisual::read(&archive, table, index).unwrap();
                let joints = visual.model.descendants();
                let mut out = Vec::new();
                if let Some(animation) = &visual.animation {
                    tracks(animation, &mut out);
                }
                for (track, object) in out {
                    let at = format!("{name} {} row {index} track {track}", symbol.name);
                    // No callback the port leaves unhandled.
                    assert!(
                        !(HSD_A_J_SETBYTE0..=HSD_A_J_SETFLOAT9).contains(&track)
                            && track != HSD_A_J_PTCLTGT,
                        "{at}"
                    );
                    // An AObj object is the path a PATH track follows.
                    paths += usize::from(track == HSD_A_J_PATH);
                    if object != 0 {
                        let joint = joints.iter().find(|joint| joint.offset == object);
                        assert!(
                            joint.is_some_and(|joint| matches!(joint.u, JObjUnion::Spline(Some(_)))),
                            "{at}: object {object:#x} is not a spline joint"
                        );
                    }
                }
            }
        }
    }
    // 36 tables in 36 files, 138 models; one PATH track (effCommonDataTable
    // row 36).
    assert_eq!((names.len(), tables, models, paths), (36, 36, 138, 1));
}
