mod common;

use std::path::Path;

use common::{srt, RetailTrig};
use hsd_anim::load::load_joint_tree;
use hsd_archive::desc::{read_public_figatree, read_public_jobj};
use hsd_archive::Archive;
use melee_ft::desc::{read_fox_animations, FOX_ANIMATION_COUNT, WAIT1_ANIMATION_INDEX};
use melee_lb::anim::{animation_frames, attach_figatree, request_frame};

const WAIT1_PUBLIC: &str = "PlyFox5K_Share_ACTION_Wait1_figatree";
const FOX_JOINT_COUNT: usize = 73;

#[test]
fn real_fox_wait1_attaches_and_advances() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files");
    if !dir.is_dir() {
        eprintln!("skipping: {} not found (disc not extracted)", dir.display());
        return;
    }
    let fighter = Archive::parse(&std::fs::read(dir.join("PlFx.dat")).unwrap()).unwrap();
    let table = read_fox_animations(&fighter).unwrap();
    assert_eq!(table.table_offset, Some(0x771C));
    assert_eq!(table.entries.len(), FOX_ANIMATION_COUNT as usize);
    assert_eq!(
        table.entries.iter().filter(|row| row.aj_size != 0).count(),
        278
    );
    let wait = &table.entries[WAIT1_ANIMATION_INDEX];
    assert_eq!(wait.symbol_name.as_deref(), Some(WAIT1_PUBLIC));
    assert_eq!((wait.aj_offset, wait.aj_size), (0, 5077));
    assert_eq!(wait, &table.entries[6]); // Shared motion, different omitted scripts.
    let aj = std::fs::read(dir.join("PlFxAJ.dat")).unwrap();
    let archive = Archive::parse(wait.sub_archive(&aj).unwrap().unwrap()).unwrap();
    let animation = read_public_figatree(&archive, WAIT1_PUBLIC).unwrap();
    assert_eq!(animation.nodes.len(), FOX_JOINT_COUNT);
    assert_eq!(animation.tracks.len(), 121);
    assert_eq!(animation.nodes.iter().filter(|&&n| n != 0).count(), 49);
    assert_eq!(
        animation_frames(Some(&animation)).to_bits(),
        120.0f32.to_bits()
    );
    eprintln!(
        "{WAIT1_PUBLIC}: type={}, flags={:#x}, frames={}, nodes={}, tracks={}, animated nodes={}",
        animation.type_,
        animation.flags,
        animation.frames,
        animation.nodes.len(),
        animation.tracks.len(),
        animation.nodes.iter().filter(|&&n| n != 0).count()
    );

    let costume = Archive::parse(&std::fs::read(dir.join("PlFxNr.dat")).unwrap()).unwrap();
    let descriptor = read_public_jobj(&costume, "PlyFox5K_Share_joint").unwrap();
    let (mut tree, root) = load_joint_tree(&costume, &descriptor).unwrap();
    let ids: Vec<_> = tree.depth_first(root).collect();
    let rest: Vec<_> = ids.iter().map(|&id| srt(&tree, id)).collect();
    assert_eq!(
        attach_figatree(&mut tree, root, &animation).unwrap(),
        FOX_JOINT_COUNT
    );
    for (&id, &count) in ids.iter().zip(&animation.nodes) {
        assert_eq!(tree.get(id).aobj.is_some(), count != 0);
    }
    request_frame(&mut tree, root, 0.0);
    tree.anim_all::<RetailTrig>(root);
    let frame0: Vec<_> = ids.iter().map(|&id| srt(&tree, id)).collect();
    eprintln!(
        "root frame 0: S/R/T {:?}; bits {:08x?}",
        srt(&tree, root).map(f32::from_bits),
        srt(&tree, root)
    );
    assert!(frame0
        .iter()
        .flatten()
        .all(|&bits| f32::from_bits(bits).is_finite()));
    assert_ne!(
        frame0, rest,
        "frame 0 must change at least one joint from rest"
    );
    // No second request: exercise AObj FIRST_PLAY and the normal rate-1 step.
    tree.anim_all::<RetailTrig>(root);
    let frame1: Vec<_> = ids.iter().map(|&id| srt(&tree, id)).collect();
    eprintln!(
        "root frame 1: S/R/T {:?}; bits {:08x?}",
        srt(&tree, root).map(f32::from_bits),
        srt(&tree, root)
    );
    assert!(frame1
        .iter()
        .flatten()
        .all(|&bits| f32::from_bits(bits).is_finite()));
    assert_ne!(frame0, frame1, "frame 1 must advance at least one joint");
    for id in ids {
        if let Some(aobj) = &tree.get(id).aobj {
            assert_eq!(aobj.curr_frame.to_bits(), 1.0f32.to_bits());
        }
    }
}
